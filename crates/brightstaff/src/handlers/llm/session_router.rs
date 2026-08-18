//! Session-cache-aware routing.
//!
//! Routing itself stays cache-blind: the `llm_router` (quality) still picks a
//! candidate model for every request. This module then decides whether to *honor*
//! that candidate or stick to the session's warm anchor, based on:
//!
//! * **Cache warmth** — inferred structurally from how long ago the session was last
//!   used vs. the provider's cache window ([`hermesllm::provider_cache_capability`]),
//!   so it works on the decision path with no provider response in hand.
//! * **A cumulative per-session spend cap** — a paid switch (the candidate must
//!   re-ingest the context at its uncached rate) is allowed only while total switch
//!   spend stays within `max_switch_spend_pct`% of the session's running *never-switch*
//!   baseline (what staying on the session's `default_model` would have cost — priced
//!   independently of the current anchor, which drifts as switches happen). An
//!   outright-cheaper switch is free but never reduces that spend. The promise: this
//!   conversation bills at most `max_switch_spend_pct`% above never-switching. A warm
//!   anchor is priced at its *cached* rate and the switch pays the cache-loss delta:
//!   provider caches are assumed real whenever the budget is active (most providers
//!   cache automatically), independent of `prompt_caching.enabled`, which only
//!   controls Plano's own marker injection and caching-without-budget affinity.
//!
//! The default posture is to stick. Quality and cost stay separate: the router decides
//! whether a switch *improves quality*; the overhead cap decides whether it is *affordable*.
//!
//! Prompt-cache *marker injection* is a separate concern — see [`super::prompt_caching`].

use std::time::{Duration, SystemTime};

use common::configuration::EffectiveRoutingBudget;
use hermesllm::apis::openai::{ContentPart, Message, MessageContent, Role};
use hermesllm::transforms::lib::ExtractText;
use hermesllm::{provider_cache_capability, ProviderCacheCapability, ProviderId};
use opentelemetry::trace::get_active_span;
use opentelemetry::KeyValue;
use tracing::{debug, info};

use crate::affinity::derive_implicit_affinity;
use crate::metrics as bs_metrics;
use crate::metrics::labels as metric_labels;
use crate::router::orchestrator::OrchestratorService;
use crate::session_cache::{record_route_visit, RouteVisit, SessionBinding};
use crate::tracing::plano as tracing_plano;

/// Resolved session identity for one request.
pub struct SessionResolution {
    /// Stable prefix hash (system + tools + first user message), independent of
    /// `prompt_caching.enabled` so it can still drive the `x-plano-prefix-hash`
    /// RING_HASH replica-stickiness header. `None` when the request opted out or has
    /// no anchorable prompt.
    pub request_prefix_hash: Option<u64>,
    /// Session key: the explicit `X-Model-Affinity` value, or the implicit prefix-hash
    /// key when implicit affinity is active. `None` when there is nothing to anchor to.
    pub session_id: Option<String>,
}

/// Resolve the session key and prefix hash from the (already filtered / state-merged)
/// request. An explicit affinity header always anchors; the implicit key is derived
/// when `implicit_affinity_enabled` is set — true when either prompt caching's
/// `session_affinity` or the routing budget is active, so stickiness works whether or
/// not prompt caching is enabled. The prefix hash is derived regardless (only
/// `X-Plano-Cache: off` or an unanchorable prompt suppresses it) so the
/// `x-plano-prefix-hash` RING_HASH replica-stickiness header still works.
pub fn resolve_session(
    explicit_session_id: Option<String>,
    messages: &[Message],
    tool_names: Option<&[String]>,
    tenant_id: Option<&str>,
    implicit_affinity_enabled: bool,
    cache_off_for_request: bool,
) -> SessionResolution {
    let implicit_affinity = if cache_off_for_request {
        None
    } else {
        derive_implicit_affinity(messages, tool_names, tenant_id)
    };
    let request_prefix_hash = implicit_affinity.as_ref().map(|a| a.prefix_hash);

    let session_id = match explicit_session_id {
        Some(sid) => Some(sid),
        None if implicit_affinity_enabled && !cache_off_for_request => {
            implicit_affinity.as_ref().map(|a| a.session_key.clone())
        }
        None => None,
    };

    SessionResolution {
        request_prefix_hash,
        session_id,
    }
}

/// Envelopes Claude Code injects as user-role text next to tool results. They are
/// written by the harness, not spoken by the user, so a tail made up only of these
/// does not open a new turn.
const INJECTED_USER_ENVELOPES: [(&str, &str); 2] = [
    ("<system-reminder>", "</system-reminder>"),
    ("<user-prompt-submit-hook>", "</user-prompt-submit-hook>"),
];

/// Remove every [`INJECTED_USER_ENVELOPES`] span, leaving whatever the user actually typed.
/// An unterminated opening tag swallows the rest of the text — a truncated envelope is
/// still harness output.
fn strip_injected_envelopes(text: &str) -> String {
    let mut remaining = text.to_string();

    for (open, close) in INJECTED_USER_ENVELOPES {
        while let Some(start) = remaining.find(open) {
            let body_start = start + open.len();
            let end = match remaining[body_start..].find(close) {
                Some(offset) => body_start + offset + close.len(),
                None => remaining.len(),
            };
            remaining.replace_range(start..end, "");
        }
    }

    remaining
}

/// Whether the message carries user-supplied content that is not text — an image today.
/// A pasted screenshot with no caption is still the user speaking, so it opens a turn
/// even though there is no text to check.
fn has_non_text_content(content: &Option<MessageContent>) -> bool {
    matches!(content, Some(MessageContent::Parts(parts))
        if parts.iter().any(|part| !matches!(part, ContentPart::Text { .. })))
}

/// Whether this request is a fresh user turn and should go through the quality router.
///
/// Only the tail is examined. A trailing [`Role::User`] is a new utterance when it
/// carries an attachment, or text that survives stripping the harness envelopes —
/// including Anthropic packing new user text alongside `tool_result` blocks, which
/// normalizes to a trailing user message. Anything else (tool results, assistant
/// output, an empty or reminder-only user tail, empty history) is not a user turn;
/// routing is skipped and the prior decision is replayed when a warm binding exists.
pub fn is_user_turn(messages: &[Message]) -> bool {
    let Some(last) = messages.last() else {
        return false;
    };
    if last.role != Role::User {
        return false;
    }
    if has_non_text_content(&last.content) {
        return true;
    }

    !strip_injected_envelopes(&last.content.extract_text())
        .trim()
        .is_empty()
}

/// Whether this request should skip the quality router and replay the prior decision.
/// Off unless `routing.route_on_user_only` is enabled, and only then for non-user tails.
pub fn should_reuse_prior_decision(enabled: bool, messages: &[Message]) -> bool {
    enabled && !is_user_turn(messages)
}

/// A routing decision carried over from an earlier request in the same session.
pub struct LoopReuse {
    /// The model this session is already running on.
    pub model: String,
    /// The route that picked it, replayed so telemetry stays attributed to it.
    pub route_name: Option<String>,
}

/// The decision to reuse when this is not a user turn, or `None` when the request must
/// go through the quality router.
///
/// Reuse is deliberately conservative; anything unexpected falls through to a normal
/// route rather than risk pinning a request to the wrong model:
///
/// * no session key (nothing to reuse from, e.g. `X-Plano-Cache: off`),
/// * no binding, or one whose cache went cold or whose prompt prefix drifted,
/// * a different model lane than the one that wrote the binding (see
///   [`SessionBinding::requested_model`]).
pub async fn reuse_prior_decision(
    orchestrator: &OrchestratorService,
    session_id: Option<&str>,
    tenant_id: Option<&str>,
    prefix_hash: Option<u64>,
    requested_model: &str,
) -> Option<LoopReuse> {
    let session_id = session_id?;
    let binding = orchestrator.peek_binding(session_id, tenant_id).await?;

    if binding.requested_model != requested_model {
        debug!(
            binding_lane = %binding.requested_model,
            request_lane = %requested_model,
            "prior-decision reuse declined — request is on a different model lane"
        );
        return None;
    }

    let (warm, _) = warmth(
        &binding,
        &capability_for_model(&binding.anchor_model),
        SystemTime::now(),
    );
    let drifted = matches!(
        (binding.prefix_hash, prefix_hash),
        (Some(stored), Some(current)) if stored != current
    );
    if !warm || drifted {
        debug!(
            warm,
            drifted, "prior-decision reuse declined — session is no longer warm on this prefix"
        );
        return None;
    }

    Some(LoopReuse {
        model: binding.anchor_model,
        route_name: binding.route_name,
    })
}

/// Extra memory retention beyond the warmth window, so a still-warm binding is never
/// GC'd out from under the router before it could plausibly go cold.
const GC_SLACK: Duration = Duration::from_secs(60);

/// Stable request facts the router reasons about. Independent of the transport (full
/// proxy vs. decision endpoint) so both paths route identically.
pub struct RouteFacts<'a> {
    /// Session key (explicit `X-Model-Affinity` or the implicit prefix key). `None`
    /// disables stickiness for this request (nothing to anchor to).
    pub session_id: Option<&'a str>,
    pub tenant_id: Option<&'a str>,
    /// Stable prompt-prefix hash; a mismatch vs. the stored binding means the provider
    /// cache is already lost, so a switch is free.
    pub prefix_hash: Option<u64>,
    /// Context size in tokens (the tokens a switch would re-ingest). The request-side
    /// count of the real messages; the binding's usage-refined count is preferred when
    /// warm (see [`actual_context_tokens`]).
    pub context_tokens: u64,
    /// The model the quality router picked for this request.
    pub candidate_model: &'a str,
    pub candidate_route: Option<&'a str>,
    /// The model the client asked for, after alias resolution and before routing had a
    /// say. Persisted as the binding's lane so a later non-user turn on a
    /// different lane doesn't inherit this decision.
    pub requested_model: &'a str,
}

/// The routing decision plus the session state to carry into the response side.
pub struct RouteDecision {
    /// The model to actually dispatch to (the anchor when a switch was vetoed).
    pub model: String,
    pub route_name: Option<String>,
    /// The session's never-switch model for this episode — carried to the response side
    /// so the usage-refresh preserves it on the binding.
    pub default_model: String,
    /// The binding's model lane — carried to the response side so the usage-refresh
    /// preserves it (see [`SessionBinding::requested_model`]).
    pub requested_model: String,
    /// Whether the session's cache was inferred warm at decision time.
    pub warm: bool,
    /// Whether a model switch was allowed this turn — mirrors `plano.switch.decision=allowed`
    /// on the span when the routing budget evaluated a switch.
    pub switched: bool,
    /// Cumulative never-switch baseline (USD) after this decision.
    pub baseline_usd: f64,
    /// Cumulative switch spend (USD) after this decision.
    pub switch_spend_usd: f64,
    /// Cumulative actual conversation cost (USD) so far — carried to the response side,
    /// which adds this turn's real cost and re-persists it.
    pub session_cost_usd: f64,
    /// Cumulative switches taken this session (after this decision).
    pub switches: u32,
    /// Bounded per-model route history after this decision — carried to the response
    /// side so the usage-refresh preserves it (and refines the anchor's token count).
    pub history: Vec<RouteVisit>,
    /// Context-token estimate persisted with the binding (refined later from usage).
    pub cached_tokens: u64,
    /// GC bound the binding was stored with (reused when the response side refreshes).
    pub gc_ttl: Duration,
}

/// Count the request's context size in tokens from the real message content, using the
/// tiktoken-based counter when available and falling back to the chars/4 heuristic. This
/// is the request-side figure; on the full-proxy path the binding is later refined with
/// the provider's own reported prompt-token count (see [`SessionBinding::cached_tokens`]),
/// which the router prefers when present.
pub fn actual_context_tokens(messages: &[Message], model: &str) -> u64 {
    let text: String = messages
        .iter()
        .filter_map(|m| m.content.as_ref().map(|c| c.to_string()))
        .collect::<Vec<_>>()
        .join("\n");
    match common::tokenizer::token_count(model, &text) {
        Ok(count) => count as u64,
        Err(_) => (text.len() / 4) as u64,
    }
}

/// Resolve a provider-qualified model id (e.g. `openai/gpt-4o`) to its cache window.
/// Unknown providers fall back to the conservative default.
fn capability_for_model(model: &str) -> ProviderCacheCapability {
    let provider_part = model.split_once('/').map(|(p, _)| p).unwrap_or(model);
    ProviderId::try_from(provider_part)
        .map(provider_cache_capability)
        .unwrap_or_default()
}

/// How long a binding on this model can sit idle before its cache is certainly cold.
fn warmth_window(cap: &ProviderCacheCapability) -> Duration {
    if cap.extended_retention {
        cap.extended_ttl
    } else {
        cap.idle_ttl.min(cap.hard_ttl)
    }
}

/// Whether the session's provider cache is plausibly still warm given how long ago it
/// was last used. Returns the warmth verdict and the measured idle gap.
fn warmth(
    binding: &SessionBinding,
    cap: &ProviderCacheCapability,
    now: SystemTime,
) -> (bool, Duration) {
    let idle = now
        .duration_since(binding.last_used)
        .unwrap_or(Duration::ZERO);
    let warm = if cap.extended_retention {
        idle <= cap.extended_ttl
    } else {
        idle <= cap.idle_ttl && idle <= cap.hard_ttl
    };
    (warm, idle)
}

/// Decide the final model for this request and persist the updated session binding.
///
/// Never overrides the router on a *cold* session — it only protects a warm cache. The
/// returned [`RouteDecision`] carries the model to dispatch plus the session state the
/// response side reuses when it refreshes the binding from real usage.
pub async fn route(
    orchestrator: &OrchestratorService,
    routing_budget: Option<&EffectiveRoutingBudget>,
    facts: RouteFacts<'_>,
) -> RouteDecision {
    let now = SystemTime::now();
    let candidate_gc_ttl = warmth_window(&capability_for_model(facts.candidate_model)) + GC_SLACK;

    // No session to anchor to: honor the candidate, persist nothing.
    let Some(session_id) = facts.session_id else {
        return RouteDecision {
            model: facts.candidate_model.to_string(),
            route_name: facts.candidate_route.map(str::to_string),
            default_model: facts.candidate_model.to_string(),
            requested_model: facts.requested_model.to_string(),
            warm: false,
            switched: false,
            baseline_usd: 0.0,
            switch_spend_usd: 0.0,
            session_cost_usd: 0.0,
            switches: 0,
            history: Vec::new(),
            cached_tokens: facts.context_tokens,
            gc_ttl: candidate_gc_ttl,
        };
    };

    let existing = orchestrator.get_binding(session_id, facts.tenant_id).await;

    // Warmth + prefix drift. A drifted prefix means the cache is already cold.
    let (warm, idle) = match &existing {
        Some(b) => warmth(b, &capability_for_model(&b.anchor_model), now),
        None => (false, Duration::ZERO),
    };
    let drifted = match (
        existing.as_ref().and_then(|b| b.prefix_hash),
        facts.prefix_hash,
    ) {
        (Some(stored), Some(current)) => stored != current,
        _ => false,
    };
    let effective_warm = warm && !drifted;

    // Cumulative actual conversation cost so far (through prior turns). Conversation-
    // level: preserved across warm/cold re-binds; the response side adds this turn.
    let session_cost_usd = existing.as_ref().map(|b| b.session_cost_usd).unwrap_or(0.0);

    // Resolve the final model, cumulative baseline/spend, switch count, and telemetry.
    let mut model = facts.candidate_model.to_string();
    let mut route_name = facts.candidate_route.map(str::to_string);
    // The session's never-switch model for this episode — priced into the baseline.
    let default_model;
    let baseline_usd;
    let mut switch_spend_usd;
    let mut switches;
    let mut switched = false;
    let mut cost_opt: Option<f64> = None;
    let mut ceiling_opt: Option<f64> = None;
    let mut candidate_warm_tokens: u64 = 0;
    let mut counterfactual: Option<String> = None;
    let decision_label: &'static str;
    let reason: &'static str;

    match existing.as_ref() {
        Some(b) if effective_warm => {
            switches = b.switches;
            // The model the session would have stayed on had it never switched. Older
            // bindings (persisted before this field existed) fall back to the anchor.
            let session_default = if b.default_model.is_empty() {
                b.anchor_model.clone()
            } else {
                b.default_model.clone()
            };
            // Prefer the provider's real prompt-token count from the prior turn over the
            // request-side estimate — it's the actual context the session carries.
            let context_tokens = if b.cached_tokens > 0 {
                b.cached_tokens
            } else {
                facts.context_tokens
            };
            // Grow the never-switch baseline by this turn's read cost on the *default*
            // model — the money the session would spend by never switching. This is the
            // denominator the overhead cap is measured against. Missing pricing → no
            // growth this turn.
            let turn_baseline = match routing_budget {
                Some(cfg) => orchestrator
                    .context_read_cost_in_usd(
                        context_tokens,
                        &session_default,
                        cfg.cache_read_discount,
                    )
                    .await
                    .unwrap_or(0.0),
                None => 0.0,
            };
            baseline_usd = b.baseline_usd + turn_baseline;
            switch_spend_usd = b.switch_spend_usd;
            default_model = session_default;

            if facts.candidate_model == b.anchor_model {
                // Router agrees with the anchor — stick, no cost.
                decision_label = metric_labels::SWITCH_DECISION_ALLOWED;
                reason = metric_labels::SWITCH_REASON_SAME_ANCHOR;
            } else if let Some(cfg) = routing_budget {
                // Ceiling: at most `max_switch_spend_pct`% of the cumulative baseline may be
                // spent on switching over this warm episode.
                let ceiling = (cfg.max_switch_spend_pct / 100.0) * baseline_usd;
                ceiling_opt = Some(ceiling);
                // Credit any context the candidate still has cached from an earlier visit
                // this session: a return to a still-warm model re-reads only the tokens
                // appended since, not the whole context (the A→B→A case).
                candidate_warm_tokens = b
                    .history
                    .iter()
                    .find(|v| v.model == facts.candidate_model)
                    .filter(|v| {
                        now.duration_since(v.last_used).unwrap_or(Duration::MAX)
                            <= warmth_window(&capability_for_model(facts.candidate_model))
                    })
                    .map(|v| v.cached_tokens.min(context_tokens))
                    .unwrap_or(0);
                match orchestrator
                    .estimate_switch_cost_in_usd(
                        context_tokens,
                        &b.anchor_model,
                        facts.candidate_model,
                        candidate_warm_tokens,
                        cfg.cache_read_discount,
                    )
                    .await
                {
                    // No pricing for one side — fail open (switch freely) rather than
                    // veto the router on guesswork.
                    None => {
                        switches += 1;
                        switched = true;
                        decision_label = metric_labels::SWITCH_DECISION_ALLOWED;
                        reason = metric_labels::SWITCH_REASON_NO_PRICING;
                        debug!(
                            anchor = %b.anchor_model,
                            candidate = %facts.candidate_model,
                            "switch allowed — missing pricing data, cannot gate"
                        );
                    }
                    Some(cost) => {
                        cost_opt = Some(cost);
                        if cost <= 0.0 {
                            // Outright cheaper: allowed for free. Does NOT reduce spend —
                            // the "saving" is vs a path we didn't take, not real money.
                            switches += 1;
                            switched = true;
                            decision_label = metric_labels::SWITCH_DECISION_ALLOWED;
                            reason = metric_labels::SWITCH_REASON_FREE;
                            info!(
                                anchor = %b.anchor_model,
                                candidate = %facts.candidate_model,
                                switch_cost_in_usd = cost,
                                "switch allowed — candidate is no more expensive than staying"
                            );
                        } else if switch_spend_usd + cost <= ceiling {
                            switch_spend_usd += cost;
                            switches += 1;
                            switched = true;
                            decision_label = metric_labels::SWITCH_DECISION_ALLOWED;
                            reason = metric_labels::SWITCH_REASON_WITHIN_CAP;
                            info!(
                                anchor = %b.anchor_model,
                                candidate = %facts.candidate_model,
                                switch_cost_in_usd = cost,
                                switch_spend_in_usd = switch_spend_usd,
                                overhead_ceiling_in_usd = ceiling,
                                "switch allowed — within session overhead cap"
                            );
                        } else {
                            // Unaffordable: retain the warm anchor.
                            if cfg.record_counterfactual {
                                counterfactual = Some(match route_name.as_deref() {
                                    Some(rn) if !rn.is_empty() && rn != "none" => {
                                        format!("{} ({rn})", facts.candidate_model)
                                    }
                                    _ => facts.candidate_model.to_string(),
                                });
                            }
                            model = b.anchor_model.clone();
                            route_name = b.route_name.clone();
                            decision_label = metric_labels::SWITCH_DECISION_RETAINED;
                            reason = metric_labels::SWITCH_REASON_OVER_CAP;
                            info!(
                                anchor = %b.anchor_model,
                                candidate = %facts.candidate_model,
                                switch_cost_in_usd = cost,
                                switch_spend_in_usd = switch_spend_usd,
                                overhead_ceiling_in_usd = ceiling,
                                "switch vetoed — would exceed session overhead cap, retaining anchor"
                            );
                        }
                    }
                }
            } else {
                // Warm but no budget configured — follow the router freely.
                switches += 1;
                switched = true;
                decision_label = metric_labels::SWITCH_DECISION_ALLOWED;
                reason = metric_labels::SWITCH_REASON_FREE;
            }
            bs_metrics::record_session_switch_decision(decision_label, reason);
        }
        _ => {
            // Cold (or no binding, or drifted): honor the candidate and (re)start a
            // fresh warm episode. Switches reset — this is a new cache lifetime. On
            // rebind we reset the running totals unless replenish_on_rebind is off, in
            // which case the prior episode's baseline/spend carry over.
            let (base, spend) = match (routing_budget, existing.as_ref()) {
                (Some(cfg), Some(b)) if !cfg.replenish_on_rebind => {
                    (b.baseline_usd, b.switch_spend_usd)
                }
                _ => (0.0, 0.0),
            };
            baseline_usd = base;
            switch_spend_usd = spend;
            switches = 0;
            // Fresh episode anchors on the model we're about to dispatch. When totals are
            // carried across a rebind (replenish off), keep the prior default so the
            // baseline lineage stays consistent.
            default_model = match (routing_budget, existing.as_ref()) {
                (Some(cfg), Some(b)) if !cfg.replenish_on_rebind && !b.default_model.is_empty() => {
                    b.default_model.clone()
                }
                _ => model.clone(),
            };
        }
    }

    // Context count persisted with the binding (refined later from real usage).
    let cached_tokens = if facts.context_tokens > 0 {
        facts.context_tokens
    } else {
        existing.as_ref().map(|b| b.cached_tokens).unwrap_or(0)
    };
    let gc_ttl = warmth_window(&capability_for_model(&model)) + GC_SLACK;

    // Route history: a drifted prefix invalidates every model's cache, so start fresh;
    // otherwise carry it forward. Record this turn's dispatched model (refined with the
    // real token count on the response side). Stale entries decay via the warmth check.
    let mut history = if drifted {
        Vec::new()
    } else {
        existing
            .as_ref()
            .map(|b| b.history.clone())
            .unwrap_or_default()
    };
    record_route_visit(&mut history, &model, now, cached_tokens);

    // Observability: cache warmth + budget/switch state on the current span.
    get_active_span(|span| {
        span.set_attribute(KeyValue::new(tracing_plano::CACHE_WARM, effective_warm));
        span.set_attribute(KeyValue::new(
            tracing_plano::CACHE_IDLE_MS,
            idle.as_millis() as i64,
        ));
        if routing_budget.is_some() {
            // Consumed overhead as a percentage of the never-switch baseline — directly
            // comparable to the configured max_switch_spend_pct. Zero before any baseline.
            let overhead_pct = if baseline_usd > 0.0 {
                100.0 * switch_spend_usd / baseline_usd
            } else {
                0.0
            };
            span.set_attribute(KeyValue::new(
                tracing_plano::SESSION_OVERHEAD_PCT,
                overhead_pct,
            ));
            span.set_attribute(KeyValue::new(
                tracing_plano::SESSION_SWITCH_SPEND_IN_USD,
                switch_spend_usd,
            ));
            span.set_attribute(KeyValue::new(
                tracing_plano::SESSION_BASELINE_IN_USD,
                baseline_usd,
            ));
            span.set_attribute(KeyValue::new(
                tracing_plano::SESSION_SWITCHES,
                switches as i64,
            ));
        }
        // Cumulative actual conversation cost (through prior turns) — emitted for every
        // session, independent of the routing budget.
        span.set_attribute(KeyValue::new(
            tracing_plano::SESSION_TOTAL_COST_IN_USD,
            session_cost_usd,
        ));
        if let Some(cost) = cost_opt {
            span.set_attribute(KeyValue::new(tracing_plano::SWITCH_COST_IN_USD, cost));
            span.set_attribute(KeyValue::new(
                tracing_plano::SWITCH_CANDIDATE_WARM_TOKENS,
                candidate_warm_tokens as i64,
            ));
            if let Some(ceiling) = ceiling_opt {
                span.set_attribute(KeyValue::new(
                    tracing_plano::SWITCH_OVERHEAD_CEILING_IN_USD,
                    ceiling,
                ));
            }
            span.set_attribute(KeyValue::new(
                tracing_plano::SWITCH_DECISION,
                if model == facts.candidate_model {
                    metric_labels::SWITCH_DECISION_ALLOWED
                } else {
                    metric_labels::SWITCH_DECISION_RETAINED
                },
            ));
        }
        if let Some(ref cf) = counterfactual {
            span.set_attribute(KeyValue::new(
                tracing_plano::SWITCH_COUNTERFACTUAL_ROUTE,
                cf.clone(),
            ));
        }
    });

    orchestrator
        .store_binding(
            session_id,
            facts.tenant_id,
            SessionBinding {
                anchor_model: model.clone(),
                default_model: default_model.clone(),
                requested_model: facts.requested_model.to_string(),
                route_name: route_name.clone(),
                prefix_hash: facts.prefix_hash,
                last_used: now,
                cached_tokens,
                baseline_usd,
                switch_spend_usd,
                switches,
                session_cost_usd,
                history: history.clone(),
            },
            Some(gc_ttl),
        )
        .await;

    RouteDecision {
        model,
        route_name,
        default_model,
        requested_model: facts.requested_model.to_string(),
        warm: effective_warm,
        switched,
        baseline_usd,
        switch_spend_usd,
        session_cost_usd,
        switches,
        history,
        cached_tokens,
        gc_ttl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap_5m_1h() -> ProviderCacheCapability {
        ProviderCacheCapability {
            idle_ttl: Duration::from_secs(300),
            hard_ttl: Duration::from_secs(3600),
            extended_retention: false,
            extended_ttl: Duration::from_secs(3600),
        }
    }

    /// The model lane test requests run on — what the client asked for, before routing.
    const CLIENT_MODEL: &str = "anthropic/claude-sonnet-4-5";

    fn binding_used_ago(secs: u64) -> SessionBinding {
        SessionBinding {
            anchor_model: "anthropic/claude-sonnet-4-5".to_string(),
            default_model: "anthropic/claude-sonnet-4-5".to_string(),
            requested_model: CLIENT_MODEL.to_string(),
            route_name: None,
            prefix_hash: Some(1),
            last_used: SystemTime::now() - Duration::from_secs(secs),
            cached_tokens: 100_000,
            baseline_usd: 1.0,
            switch_spend_usd: 0.0,
            switches: 0,
            session_cost_usd: 0.0,
            history: Vec::new(),
        }
    }

    #[test]
    fn warm_within_idle_window() {
        let (warm, _) = warmth(&binding_used_ago(60), &cap_5m_1h(), SystemTime::now());
        assert!(warm);
    }

    #[test]
    fn cold_past_idle_window() {
        let (warm, _) = warmth(&binding_used_ago(600), &cap_5m_1h(), SystemTime::now());
        assert!(!warm);
    }

    #[test]
    fn extended_retention_keeps_warm_past_idle() {
        let cap = ProviderCacheCapability {
            extended_retention: true,
            ..cap_5m_1h()
        };
        // 10 minutes idle: cold under 5m, warm under the 1h extended window.
        let (warm, _) = warmth(&binding_used_ago(600), &cap, SystemTime::now());
        assert!(warm);
    }

    #[test]
    fn capability_resolves_from_model_prefix() {
        // Known provider prefix resolves; unknown falls back to the default.
        let anthropic = capability_for_model("anthropic/claude-sonnet-4-5");
        assert_eq!(anthropic, ProviderCacheCapability::default());
        let unknown = capability_for_model("madeup/model-x");
        assert_eq!(unknown, ProviderCacheCapability::default());
    }

    // ---- route() budget behavior ----

    use crate::router::model_metrics::{ModelMetricsService, ModelRates};
    use crate::session_cache::memory::MemorySessionCache;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Anchor `expensive` cached rate 0.3, candidate `pricey` input 5.0, candidate `cheap`
    // input 0.1. With a 100k-token context the paid switch to `pricey` costs
    // 0.1M * (5.0 - 0.3) = $0.47; the `cheap` switch is 0.1M * (0.1 - 0.3) = -$0.02 (free).
    // Each warm turn grows the never-switch baseline by 0.1M * 0.3 = $0.03.
    fn orch_with_rates() -> OrchestratorService {
        let mut rates = HashMap::new();
        rates.insert(
            "anthropic/expensive".to_string(),
            ModelRates {
                input_per_million: 3.0,
                output_per_million: 15.0,
                cache_read_per_million: Some(0.3),
            },
        );
        rates.insert(
            "openai/pricey".to_string(),
            ModelRates {
                input_per_million: 5.0,
                output_per_million: 15.0,
                cache_read_per_million: Some(0.5),
            },
        );
        rates.insert(
            "google/cheap".to_string(),
            ModelRates {
                input_per_million: 0.1,
                output_per_million: 0.4,
                cache_read_per_million: Some(0.01),
            },
        );
        let metrics = Arc::new(ModelMetricsService::from_rates_for_test(rates));
        let cache = Arc::new(MemorySessionCache::new(100));
        OrchestratorService::with_routing(
            "http://localhost/v1/chat/completions".to_string(),
            "m".to_string(),
            "p".to_string(),
            None,
            Some(metrics),
            Some(600),
            cache,
            None,
            8192,
        )
    }

    fn routing_budget(pct: f64) -> EffectiveRoutingBudget {
        EffectiveRoutingBudget {
            max_switch_spend_pct: pct,
            replenish_on_rebind: true,
            cache_read_discount: 0.1,
            record_counterfactual: false,
        }
    }

    /// Seed a warm binding on the `expensive` anchor with a pre-accumulated never-switch
    /// baseline (`baseline_usd`) and switch spend (`switch_spend_usd`), simulating a
    /// session that has already run for some turns.
    async fn seed_warm_binding(
        orch: &OrchestratorService,
        baseline_usd: f64,
        switch_spend_usd: f64,
        idle_secs: u64,
    ) {
        orch.store_binding(
            "s1",
            None,
            SessionBinding {
                anchor_model: "anthropic/expensive".to_string(),
                default_model: "anthropic/expensive".to_string(),
                requested_model: CLIENT_MODEL.to_string(),
                route_name: None,
                prefix_hash: Some(1),
                last_used: SystemTime::now() - Duration::from_secs(idle_secs),
                cached_tokens: 100_000,
                baseline_usd,
                switch_spend_usd,
                switches: 0,
                session_cost_usd: 0.0,
                history: Vec::new(),
            },
            Some(Duration::from_secs(3600)),
        )
        .await;
    }

    fn facts_for<'a>(candidate: &'a str) -> RouteFacts<'a> {
        RouteFacts {
            session_id: Some("s1"),
            tenant_id: None,
            prefix_hash: Some(1),
            context_tokens: 0,
            candidate_model: candidate,
            candidate_route: None,
            requested_model: CLIENT_MODEL,
        }
    }

    #[tokio::test]
    async fn paid_switch_within_cap_accrues_spend() {
        let orch = orch_with_rates();
        // Baseline $2.00 already accrued; this turn adds $0.03 -> $2.03. At 25% the
        // ceiling is $0.5075, which covers the $0.47 switch to `pricey`.
        seed_warm_binding(&orch, 2.0, 0.0, 30).await;
        let st = routing_budget(25.0);
        let d = route(&orch, Some(&st), facts_for("openai/pricey")).await;

        assert_eq!(d.model, "openai/pricey");
        assert!(d.warm);
        assert!(d.switched);
        assert_eq!(d.switches, 1);
        assert!(
            (d.switch_spend_usd - 0.47).abs() < 1e-6,
            "spend {} != 0.47",
            d.switch_spend_usd
        );
        assert!(
            (d.baseline_usd - 2.03).abs() < 1e-6,
            "baseline {} != 2.03",
            d.baseline_usd
        );
    }

    #[tokio::test]
    async fn paid_switch_over_cap_retains_anchor() {
        let orch = orch_with_rates();
        // Baseline $1.00 (+$0.03 this turn). At 25% the ceiling is ~$0.2575 < $0.47.
        seed_warm_binding(&orch, 1.0, 0.0, 30).await;
        let st = routing_budget(25.0);
        let d = route(&orch, Some(&st), facts_for("openai/pricey")).await;

        assert_eq!(d.model, "anthropic/expensive");
        assert!(d.warm);
        assert!(!d.switched);
        assert_eq!(d.switches, 0);
        // Vetoed switch spends nothing.
        assert!((d.switch_spend_usd - 0.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn cheaper_switch_is_free() {
        let orch = orch_with_rates();
        seed_warm_binding(&orch, 1.0, 0.10, 30).await;
        let st = routing_budget(25.0);
        let d = route(&orch, Some(&st), facts_for("google/cheap")).await;

        assert_eq!(d.model, "google/cheap");
        assert!(d.warm);
        assert_eq!(d.switches, 1);
        // Free switches never touch the running spend — it stays at 0.10.
        assert!(
            (d.switch_spend_usd - 0.10).abs() < 1e-6,
            "spend {} != 0.10",
            d.switch_spend_usd
        );
    }

    #[tokio::test]
    async fn cold_session_resets_and_follows_router() {
        let orch = orch_with_rates();
        // 10 minutes idle: past Anthropic's 5m idle window -> cold. Prior episode spent
        // its overhead; the fresh episode resets baseline and spend to zero.
        seed_warm_binding(&orch, 5.0, 3.0, 600).await;
        let st = routing_budget(25.0);
        let d = route(&orch, Some(&st), facts_for("openai/pricey")).await;

        assert_eq!(d.model, "openai/pricey");
        assert!(!d.warm);
        assert_eq!(d.switches, 0);
        assert!((d.baseline_usd - 0.0).abs() < 1e-6, "baseline reset");
        assert!((d.switch_spend_usd - 0.0).abs() < 1e-6, "spend reset");
    }

    #[tokio::test]
    async fn no_session_honors_candidate() {
        let orch = orch_with_rates();
        let st = routing_budget(1.0);
        let facts = RouteFacts {
            session_id: None,
            tenant_id: None,
            prefix_hash: Some(1),
            context_tokens: 0,
            candidate_model: "openai/pricey",
            candidate_route: None,
            requested_model: CLIENT_MODEL,
        };
        let d = route(&orch, Some(&st), facts).await;
        assert_eq!(d.model, "openai/pricey");
        assert!(!d.warm);
    }

    #[tokio::test]
    async fn baseline_priced_on_default_not_anchor() {
        // A session that started on `expensive` (default) but has since switched to
        // `cheap` (current anchor). The never-switch baseline must keep growing at the
        // *default* model's cached rate (0.3/M), not the cheap anchor's (0.01/M).
        let orch = orch_with_rates();
        orch.store_binding(
            "s1",
            None,
            SessionBinding {
                anchor_model: "google/cheap".to_string(),
                default_model: "anthropic/expensive".to_string(),
                requested_model: CLIENT_MODEL.to_string(),
                route_name: None,
                prefix_hash: Some(1),
                last_used: SystemTime::now(),
                cached_tokens: 100_000,
                baseline_usd: 0.0,
                switch_spend_usd: 0.0,
                switches: 1,
                session_cost_usd: 0.0,
                history: Vec::new(),
            },
            Some(Duration::from_secs(3600)),
        )
        .await;
        let st = routing_budget(25.0);
        // Router agrees with the current anchor -> same-anchor, no switch.
        let d = route(&orch, Some(&st), facts_for("google/cheap")).await;

        assert_eq!(d.model, "google/cheap");
        assert!(d.warm);
        assert_eq!(d.switches, 1);
        // 100_000 tokens x $0.30/M (default `expensive`) = $0.03 — not $0.001 (cheap).
        assert!(
            (d.baseline_usd - 0.03).abs() < 1e-6,
            "baseline {} != 0.03 (should price the default model, not the anchor)",
            d.baseline_usd
        );
        assert_eq!(d.default_model, "anthropic/expensive");
    }

    #[tokio::test]
    async fn warm_return_is_affordable() {
        // Warm on `expensive`, but `pricey` was used recently and still holds the whole
        // 100k context. Baseline $0.40 (+$0.03 this turn = $0.43); at 25% the ceiling is
        // ~$0.1075. Returning to `pricey` re-reads the 100k at its *cached* rate (0.5),
        // so the switch costs 100k x (0.5 - 0.3)/1M = $0.02 — under the ceiling, allowed.
        // A cold switch would re-read at 5.0 -> $0.47 and be vetoed.
        let orch = orch_with_rates();
        orch.store_binding(
            "s1",
            None,
            SessionBinding {
                anchor_model: "anthropic/expensive".to_string(),
                default_model: "anthropic/expensive".to_string(),
                requested_model: CLIENT_MODEL.to_string(),
                route_name: None,
                prefix_hash: Some(1),
                last_used: SystemTime::now() - Duration::from_secs(30),
                cached_tokens: 100_000,
                baseline_usd: 0.40,
                switch_spend_usd: 0.0,
                switches: 1,
                session_cost_usd: 0.0,
                history: vec![RouteVisit {
                    model: "openai/pricey".to_string(),
                    last_used: SystemTime::now() - Duration::from_secs(30),
                    cached_tokens: 100_000,
                }],
            },
            Some(Duration::from_secs(3600)),
        )
        .await;
        let st = routing_budget(25.0);
        let d = route(&orch, Some(&st), facts_for("openai/pricey")).await;

        assert_eq!(d.model, "openai/pricey", "warm return should be allowed");
        assert_eq!(d.switches, 2);
        assert!(
            (d.switch_spend_usd - 0.02).abs() < 1e-6,
            "spend {} != 0.02 (warm return should charge only the cached-rate delta)",
            d.switch_spend_usd
        );
        // The candidate's own history entry is refreshed to the current model.
        assert!(d.history.iter().any(|v| v.model == "openai/pricey"));
    }

    /// End-to-end cost validation over a full session lifetime: drive `route()` turn by
    /// turn through the real session cache and pricing math, and check the feature's core
    /// promise at every turn — cumulative switch spend never exceeds `max_switch_spend_pct`%
    /// of the never-switch baseline.
    ///
    /// With a 100k context, each warm turn grows the baseline by 100k x $0.30/M = $0.03
    /// (the `expensive` default's cached rate) and a switch to `pricey` costs
    /// 100k x (5.0 - 0.3)/M = $0.47. At a 25% cap the ceiling is 0.25 x $0.03 x k after
    /// k warm turns, so the switch must be vetoed through warm turn 62
    /// (ceiling $0.465 < $0.47) and allowed exactly at warm turn 63 (ceiling $0.4725).
    #[tokio::test]
    async fn multi_turn_session_cost_stays_within_overhead_cap() {
        let orch = orch_with_rates();
        let st = routing_budget(25.0);
        let cap_fraction = 25.0 / 100.0;
        let facts_with_context = |candidate: &'static str| RouteFacts {
            session_id: Some("s1"),
            tenant_id: None,
            prefix_hash: Some(1),
            context_tokens: 100_000,
            candidate_model: candidate,
            candidate_route: None,
            requested_model: CLIENT_MODEL,
        };

        // Turn 1 — cold start: no binding yet, the candidate is honored and becomes both
        // the anchor and the session's never-switch default.
        let d = route(&orch, Some(&st), facts_with_context("anthropic/expensive")).await;
        assert_eq!(d.model, "anthropic/expensive");
        assert!(!d.warm);
        assert_eq!(d.default_model, "anthropic/expensive");

        // Warm turns: the router keeps proposing the pricier model every turn. The gate
        // must veto until the accrued baseline makes the switch affordable.
        let mut first_switch_warm_turn: Option<u32> = None;
        let mut warm_turns = 0u32;
        let mut last = None;
        for turn in 1..=70u32 {
            let d = route(&orch, Some(&st), facts_with_context("openai/pricey")).await;
            assert!(d.warm, "turn {turn} should be warm (used moments ago)");
            warm_turns = turn;

            // The invariant under validation: spend never exceeds the cap.
            assert!(
                d.switch_spend_usd <= cap_fraction * d.baseline_usd + 1e-9,
                "turn {turn}: spend {} exceeds {}% of baseline {}",
                d.switch_spend_usd,
                st.max_switch_spend_pct,
                d.baseline_usd
            );
            // Baseline must track the independently computed never-switch cost:
            // $0.03 per warm turn on the default model's cached rate.
            let expected_baseline = 0.03 * turn as f64;
            assert!(
                (d.baseline_usd - expected_baseline).abs() < 1e-6,
                "turn {turn}: baseline {} != expected never-switch cost {expected_baseline}",
                d.baseline_usd
            );

            if d.model == "openai/pricey" {
                first_switch_warm_turn = Some(turn);
                assert_eq!(d.switches, 1);
                assert!(
                    (d.switch_spend_usd - 0.47).abs() < 1e-6,
                    "allowed switch should charge the full cache-loss delta"
                );
                last = Some(d);
                break;
            }
            // Vetoed turns retain the anchor and spend nothing.
            assert_eq!(d.model, "anthropic/expensive");
            assert_eq!(d.switches, 0);
            assert!((d.switch_spend_usd - 0.0).abs() < 1e-9);
        }

        // ceiling(k) = 0.25 x 0.03k first covers $0.47 at k = 63.
        assert_eq!(
            first_switch_warm_turn,
            Some(63),
            "switch should flip from veto to allow exactly when the ceiling covers its cost"
        );
        let after_switch = last.unwrap();

        // Same-anchor turn after the switch: no further spend accrues.
        let d = route(&orch, Some(&st), facts_with_context("openai/pricey")).await;
        warm_turns += 1;
        assert_eq!(d.model, "openai/pricey");
        assert_eq!(d.switches, 1);
        assert!((d.switch_spend_usd - after_switch.switch_spend_usd).abs() < 1e-9);

        // A -> B -> A return: `expensive` was dispatched last turn-but-one, so its cache
        // is still warm — the return re-reads at cached rates and is free (never vetoed,
        // never accrues spend).
        let d = route(&orch, Some(&st), facts_with_context("anthropic/expensive")).await;
        warm_turns += 1;
        assert_eq!(
            d.model, "anthropic/expensive",
            "warm return must be allowed"
        );
        assert_eq!(d.switches, 2);
        assert!(
            (d.switch_spend_usd - after_switch.switch_spend_usd).abs() < 1e-9,
            "free return must not change the running spend"
        );

        // Final end-to-end check of the promise: total switch overhead across the whole
        // session is within max_switch_spend_pct% of the independently computed
        // never-switch baseline.
        let never_switch_cost = 0.03 * warm_turns as f64;
        assert!(
            (d.baseline_usd - never_switch_cost).abs() < 1e-6,
            "final baseline {} != independently computed never-switch cost {never_switch_cost}",
            d.baseline_usd
        );
        assert!(
            d.switch_spend_usd <= cap_fraction * d.baseline_usd + 1e-9,
            "session overhead {} exceeds the promised {}% of {}",
            d.switch_spend_usd,
            st.max_switch_spend_pct,
            d.baseline_usd
        );
    }

    // ---- is_user_turn() ----
    //
    // Driven through the real client-API parsers rather than hand-built `Message`s: the
    // gate is "last role is user", and every agent wire format has to normalize to that.

    use hermesllm::clients::SupportedAPIsFromClient;
    use hermesllm::{ProviderRequest, ProviderRequestType};

    fn messages_from(endpoint: &str, body: serde_json::Value) -> Vec<Message> {
        let bytes = serde_json::to_vec(&body).unwrap();
        let client_api = SupportedAPIsFromClient::from_endpoint(endpoint).unwrap();
        ProviderRequestType::try_from((&bytes[..], &client_api))
            .expect("request should parse")
            .get_messages()
    }

    /// Claude Code posting a tool result back for the next step of the same turn.
    #[test]
    fn anthropic_tool_result_is_a_continuation() {
        let msgs = messages_from(
            "/v1/messages",
            serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1024,
                "messages": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"role": "assistant", "content": [
                        {"type": "tool_use", "id": "toolu_1", "name": "read_file",
                         "input": {"path": "main.rs"}}
                    ]},
                    {"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "fn main() {}"}
                    ]}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    /// Anthropic packs a tool result and new user text into one message. The user spoke
    /// again, so this opens a fresh turn and must route normally.
    #[test]
    fn anthropic_tool_result_with_new_user_text_is_a_fresh_turn() {
        let msgs = messages_from(
            "/v1/messages",
            serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1024,
                "messages": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"role": "assistant", "content": [
                        {"type": "tool_use", "id": "toolu_1", "name": "read_file",
                         "input": {"path": "main.rs"}}
                    ]},
                    {"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "fn main() {}"},
                        {"type": "text", "text": "actually, explain the error handling too"}
                    ]}
                ]
            }),
        );
        assert!(is_user_turn(&msgs));
    }

    /// Cline / Cursor on OpenAI Chat Completions.
    #[test]
    fn chat_tool_role_is_a_continuation() {
        let msgs = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"role": "assistant", "tool_calls": [
                        {"id": "call_1", "type": "function",
                         "function": {"name": "read_file", "arguments": "{\"path\":\"main.rs\"}"}}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": "fn main() {}"}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    /// Tool traffic from earlier turns stays in the history forever; only the tail marks
    /// a user turn, otherwise every later turn of a long session would be pinned.
    #[test]
    fn chat_fresh_user_turn_after_earlier_tools_is_not_a_continuation() {
        let msgs = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"role": "assistant", "tool_calls": [
                        {"id": "call_1", "type": "function",
                         "function": {"name": "read_file", "arguments": "{}"}}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": "fn main() {}"},
                    {"role": "assistant", "content": "Fixed it."},
                    {"role": "user", "content": "now write the tests"}
                ]
            }),
        );
        assert!(is_user_turn(&msgs));
    }

    /// Codex / OpenCode on the Responses API.
    #[test]
    fn responses_function_call_output_is_a_continuation() {
        let msgs = messages_from(
            "/v1/responses",
            serde_json::json!({
                "model": "gpt-5.3-codex",
                "input": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"type": "function_call", "id": "fc_1", "call_id": "call_1",
                     "name": "exec_command", "arguments": "{\"cmd\":\"cat main.rs\"}",
                     "status": "completed"},
                    {"type": "function_call_output", "id": "fc_out_1", "call_id": "call_1",
                     "output": {"stdout": "fn main() {}"}}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    /// Codex registers custom tools by default, so most of its continuations carry
    /// `custom_tool_call_output` rather than `function_call_output`.
    #[test]
    fn responses_custom_tool_call_output_is_a_continuation() {
        let msgs = messages_from(
            "/v1/responses",
            serde_json::json!({
                "model": "gpt-5.3-codex",
                "input": [
                    {"role": "user", "content": "run the tests"},
                    {"type": "custom_tool_call", "id": "ctc_1", "call_id": "call_1",
                     "name": "shell", "input": "cargo test", "status": "completed"},
                    {"type": "custom_tool_call_output", "call_id": "call_1",
                     "output": "test result: ok"}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    #[test]
    fn plain_user_turn_routes_and_empty_history_does_not() {
        let msgs = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "write a haiku"}]
            }),
        );
        assert!(is_user_turn(&msgs));
        assert!(!is_user_turn(&[]));
    }

    /// An assistant-only tail is not a user turn, so routing is skipped.
    #[test]
    fn reuse_is_off_unless_route_on_user_only_is_enabled() {
        let tool_tail = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "tool_calls": [
                        {"id": "call_1", "type": "function",
                         "function": {"name": "read_file", "arguments": "{}"}}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": "ok"}
                ]
            }),
        );
        let user_tail = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "fix the bug"}]
            }),
        );
        assert!(!should_reuse_prior_decision(false, &tool_tail));
        assert!(should_reuse_prior_decision(true, &tool_tail));
        assert!(!should_reuse_prior_decision(true, &user_tail));
    }

    #[test]
    fn assistant_tail_is_not_a_user_turn() {
        let msgs = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "content": "Looking at main.rs"}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    /// A user-role message with nothing in it is a harness artifact, not an utterance.
    #[test]
    fn empty_and_whitespace_user_tails_are_not_user_turns() {
        for content in ["", "   \n\t "] {
            let msgs = messages_from(
                "/v1/chat/completions",
                serde_json::json!({
                    "model": "gpt-4o",
                    "messages": [
                        {"role": "user", "content": "fix the bug"},
                        {"role": "assistant", "content": "Fixed it."},
                        {"role": "user", "content": content}
                    ]
                }),
            );
            assert!(!is_user_turn(&msgs), "content {content:?} opened a turn");
        }
    }

    /// Every client API can deliver a user-role message with nothing in it — a null or
    /// missing `content`, an empty parts array, an empty text block. None of them open a
    /// turn, whatever the wire format.
    #[test]
    fn empty_user_content_is_not_a_user_turn_on_any_client_api() {
        let chat = |content: serde_json::Value| {
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "content": "Fixed it."},
                    {"role": "user", "content": content}
                ]
            })
        };
        let anthropic = |content: serde_json::Value| {
            serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1024,
                "messages": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "content": "Fixed it."},
                    {"role": "user", "content": content}
                ]
            })
        };
        let responses = |content: serde_json::Value| {
            serde_json::json!({
                "model": "gpt-5.3-codex",
                "input": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "content": "Fixed it."},
                    {"role": "user", "content": content}
                ]
            })
        };

        let cases = [
            ("/v1/chat/completions", chat(serde_json::Value::Null)),
            ("/v1/chat/completions", chat(serde_json::json!(""))),
            ("/v1/chat/completions", chat(serde_json::json!([]))),
            (
                "/v1/chat/completions",
                chat(serde_json::json!([{"type": "text", "text": "  "}])),
            ),
            ("/v1/messages", anthropic(serde_json::json!(""))),
            ("/v1/messages", anthropic(serde_json::json!([]))),
            (
                "/v1/messages",
                anthropic(serde_json::json!([{"type": "text", "text": "\n"}])),
            ),
            ("/v1/responses", responses(serde_json::json!(""))),
            ("/v1/responses", responses(serde_json::json!([]))),
        ];

        for (endpoint, body) in cases {
            let msgs = messages_from(endpoint, body.clone());
            assert!(
                !is_user_turn(&msgs),
                "{endpoint} opened a turn on {body}, normalized to {msgs:?}"
            );
        }
    }

    /// A pasted screenshot with no caption has no text to check, but the user did speak.
    #[test]
    fn attachment_only_user_tail_is_a_user_turn() {
        const PNG: &str = "data:image/png;base64,iVBORw0KGgo=";

        let cases = [
            (
                "/v1/chat/completions",
                serde_json::json!({
                    "model": "gpt-4o",
                    "messages": [
                        {"role": "user", "content": "fix the bug"},
                        {"role": "assistant", "content": "Fixed it."},
                        {"role": "user", "content": [
                            {"type": "image_url", "image_url": {"url": PNG}}
                        ]}
                    ]
                }),
            ),
            (
                "/v1/messages",
                serde_json::json!({
                    "model": "claude-sonnet-4-6",
                    "max_tokens": 1024,
                    "messages": [
                        {"role": "user", "content": "fix the bug"},
                        {"role": "assistant", "content": "Fixed it."},
                        {"role": "user", "content": [
                            {"type": "image", "source": {
                                "type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="
                            }}
                        ]}
                    ]
                }),
            ),
        ];

        for (endpoint, body) in cases {
            let msgs = messages_from(endpoint, body.clone());
            assert!(
                is_user_turn(&msgs),
                "{endpoint} pinned an attachment-only turn, normalized to {msgs:?}"
            );
        }
    }

    /// Bedrock reaches the same gate through the upstream shape rather than a client
    /// endpoint, so cover its empty user messages directly.
    #[test]
    fn empty_bedrock_user_content_is_not_a_user_turn() {
        use hermesllm::apis::amazon_bedrock::{
            ContentBlock, ConversationRole, ConverseRequest, Message as BedrockMessage,
        };

        for tail in [
            Vec::new(),
            vec![ContentBlock::Text {
                text: String::new(),
            }],
            vec![ContentBlock::Text {
                text: "  \n".to_string(),
            }],
        ] {
            let request = ConverseRequest {
                model_id: "anthropic.claude-3-sonnet".to_string(),
                messages: Some(vec![
                    BedrockMessage {
                        role: ConversationRole::User,
                        content: vec![ContentBlock::Text {
                            text: "fix the bug".to_string(),
                        }],
                    },
                    BedrockMessage {
                        role: ConversationRole::Assistant,
                        content: vec![ContentBlock::Text {
                            text: "Fixed it.".to_string(),
                        }],
                    },
                    BedrockMessage {
                        role: ConversationRole::User,
                        content: tail.clone(),
                    },
                ]),
                ..Default::default()
            };

            let msgs = ProviderRequestType::BedrockConverse(request).get_messages();
            assert!(
                !is_user_turn(&msgs),
                "bedrock tail {tail:?} opened a turn, normalized to {msgs:?}"
            );
        }
    }

    /// Companion to the OpenAI and Anthropic attachment cases above: a Bedrock image
    /// with no caption is a real user turn and must open a route.
    #[test]
    fn attachment_only_bedrock_user_tail_is_a_user_turn() {
        use hermesllm::apis::amazon_bedrock::{
            ContentBlock, ConversationRole, ConverseRequest, ImageBlock, ImageSource,
            Message as BedrockMessage,
        };

        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet".to_string(),
            messages: Some(vec![
                BedrockMessage {
                    role: ConversationRole::User,
                    content: vec![ContentBlock::Text {
                        text: "fix the bug".to_string(),
                    }],
                },
                BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::Text {
                        text: "Fixed it.".to_string(),
                    }],
                },
                BedrockMessage {
                    role: ConversationRole::User,
                    content: vec![ContentBlock::Image {
                        image: ImageBlock {
                            source: ImageSource::Base64 {
                                media_type: "image/png".to_string(),
                                data: "iVBORw0KGgo=".to_string(),
                            },
                        },
                    }],
                },
            ]),
            ..Default::default()
        };

        let msgs = ProviderRequestType::BedrockConverse(request).get_messages();
        assert!(
            is_user_turn(&msgs),
            "bedrock pinned an attachment-only turn, normalized to {msgs:?}"
        );
    }

    /// Claude Code injects reminders and hook output as user-role text. On their own they
    /// are not a new utterance, so the loop stays pinned.
    #[test]
    fn envelope_only_user_tail_is_not_a_user_turn() {
        for content in [
            "<system-reminder>\nYour todo list is empty.\n</system-reminder>",
            "<user-prompt-submit-hook>blocked by hook</user-prompt-submit-hook>\n",
            "<system-reminder>a</system-reminder> <system-reminder>b</system-reminder>",
            // Truncated envelope: still harness output, not user text.
            "<system-reminder>\nYour todo list is empty.",
        ] {
            let msgs = messages_from(
                "/v1/chat/completions",
                serde_json::json!({
                    "model": "gpt-4o",
                    "messages": [
                        {"role": "user", "content": "fix the bug"},
                        {"role": "assistant", "content": "Fixed it."},
                        {"role": "user", "content": content}
                    ]
                }),
            );
            assert!(!is_user_turn(&msgs), "content {content:?} opened a turn");
        }
    }

    /// A reminder rides alongside real user text on genuine turns; that still routes.
    #[test]
    fn envelope_plus_user_text_is_a_user_turn() {
        let msgs = messages_from(
            "/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "user", "content": "fix the bug"},
                    {"role": "assistant", "content": "Fixed it."},
                    {"role": "user", "content": "<system-reminder>\nTodo list is empty.\n</system-reminder>\nnow write the tests"}
                ]
            }),
        );
        assert!(is_user_turn(&msgs));
    }

    /// Claude Code's usual continuation shape: a tool result packed with a reminder and
    /// no user text. The reminder must not be mistaken for a new turn.
    #[test]
    fn anthropic_tool_result_with_reminder_only_is_a_continuation() {
        let msgs = messages_from(
            "/v1/messages",
            serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1024,
                "messages": [
                    {"role": "user", "content": "fix the bug in main.rs"},
                    {"role": "assistant", "content": [
                        {"type": "tool_use", "id": "toolu_1", "name": "read_file",
                         "input": {"path": "main.rs"}}
                    ]},
                    {"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "fn main() {}"},
                        {"type": "text", "text": "<system-reminder>\nYour todo list has changed.\n</system-reminder>"}
                    ]}
                ]
            }),
        );
        assert!(!is_user_turn(&msgs));
    }

    // ---- reuse_prior_decision() ----

    /// Seed the binding a first turn would have written: the client asked for
    /// `requested_model`, routing sent the turn to `anchor`.
    async fn seed_lane_binding(
        orch: &OrchestratorService,
        requested_model: &str,
        anchor: &str,
        route_name: Option<&str>,
        idle_secs: u64,
    ) {
        orch.store_binding(
            "s1",
            None,
            SessionBinding {
                anchor_model: anchor.to_string(),
                default_model: anchor.to_string(),
                requested_model: requested_model.to_string(),
                route_name: route_name.map(str::to_string),
                prefix_hash: Some(1),
                last_used: SystemTime::now() - Duration::from_secs(idle_secs),
                cached_tokens: 100_000,
                baseline_usd: 0.0,
                switch_spend_usd: 0.0,
                switches: 0,
                session_cost_usd: 0.0,
                history: Vec::new(),
            },
            Some(Duration::from_secs(3600)),
        )
        .await;
    }

    /// The core case: the client keeps asking for Sonnet, routing sent turn 1 to a
    /// different model, and the rest of the loop replays that model instead of re-routing.
    #[tokio::test]
    async fn warm_binding_on_the_same_lane_is_reused() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", Some("code gen"), 5).await;

        let reuse = reuse_prior_decision(&orch, Some("s1"), None, Some(1), CLIENT_MODEL)
            .await
            .expect("warm same-lane binding should be reused");
        assert_eq!(reuse.model, "openai/pricey");
        assert_eq!(reuse.route_name.as_deref(), Some("code gen"));
    }

    /// When no route matched, the first turn dispatches the client's own model and stores
    /// it as the anchor — continuations must replay that too.
    #[tokio::test]
    async fn unrouted_first_turn_is_reused() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, CLIENT_MODEL, None, 5).await;

        let reuse = reuse_prior_decision(&orch, Some("s1"), None, Some(1), CLIENT_MODEL)
            .await
            .expect("an unrouted turn still anchors the loop");
        assert_eq!(reuse.model, CLIENT_MODEL);
        assert!(reuse.route_name.is_none());
    }

    /// Known limitation of a *shared explicit* `X-Model-Affinity` id: one session key
    /// holds one binding, so a side call (side chat, summarizer, subagent) overwrites the
    /// main loop's lane and prefix. The guards keep the side call from being pinned to the
    /// wrong model, but the loop's pin is evicted rather than kept alongside it, and the
    /// next continuation re-routes. Implicit affinity does not have this problem: the side
    /// call's different prompt prefix derives its own key.
    #[tokio::test]
    async fn a_side_call_on_a_shared_affinity_id_evicts_the_loop_pin() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", Some("code gen"), 5).await;

        // Side call: same affinity header, different model lane and prompt prefix.
        route(
            &orch,
            None,
            RouteFacts {
                session_id: Some("s1"),
                tenant_id: None,
                prefix_hash: Some(2),
                context_tokens: 1_000,
                candidate_model: "google/cheap",
                candidate_route: Some("summarize"),
                requested_model: "anthropic/small-fast",
            },
        )
        .await;

        // The main loop's next continuation can no longer find its decision.
        assert!(
            reuse_prior_decision(&orch, Some("s1"), None, Some(1), CLIENT_MODEL)
                .await
                .is_none(),
            "side call should have overwritten the loop binding"
        );
    }

    /// Claude Code's `ANTHROPIC_SMALL_FAST_MODEL` side calls ride the same prompt prefix
    /// as the main loop. They must not inherit the main loop's model.
    #[tokio::test]
    async fn a_different_model_lane_is_not_reused() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", Some("code gen"), 5).await;

        let reuse = reuse_prior_decision(
            &orch,
            Some("s1"),
            None,
            Some(1),
            "anthropic/claude-haiku-4-5",
        )
        .await;
        assert!(reuse.is_none(), "the small-fast lane must route on its own");
    }

    /// The loop paused long enough for the provider cache to lapse: the binding no longer
    /// describes a live loop, so fall back to a fresh routing decision.
    #[tokio::test]
    async fn a_cold_binding_is_not_reused() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", None, 24 * 3600).await;

        let reuse = reuse_prior_decision(&orch, Some("s1"), None, Some(1), CLIENT_MODEL).await;
        assert!(reuse.is_none());
    }

    /// A changed system prompt or tool set means this is a different conversation that
    /// merely collided on the session key.
    #[tokio::test]
    async fn a_drifted_prefix_is_not_reused() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", None, 5).await;

        let reuse = reuse_prior_decision(&orch, Some("s1"), None, Some(999), CLIENT_MODEL).await;
        assert!(reuse.is_none());
    }

    /// No session key (`X-Plano-Cache: off`, or nothing to anchor on) and no binding both
    /// mean there is nothing to replay.
    #[tokio::test]
    async fn without_a_session_or_binding_there_is_nothing_to_reuse() {
        let orch = orch_with_rates();
        assert!(
            reuse_prior_decision(&orch, None, None, Some(1), CLIENT_MODEL)
                .await
                .is_none()
        );
        assert!(
            reuse_prior_decision(&orch, Some("never-seen"), None, Some(1), CLIENT_MODEL)
                .await
                .is_none()
        );
    }

    /// Skipping the router must not skip session bookkeeping: feeding the replayed model
    /// back through `route()` keeps the session on it and refreshes the binding.
    #[tokio::test]
    async fn replaying_the_loop_model_refreshes_the_binding_without_switching() {
        let orch = orch_with_rates();
        seed_lane_binding(&orch, CLIENT_MODEL, "openai/pricey", Some("code gen"), 30).await;

        let reuse = reuse_prior_decision(&orch, Some("s1"), None, Some(1), CLIENT_MODEL)
            .await
            .unwrap();
        let d = route(&orch, None, facts_for(&reuse.model)).await;

        assert_eq!(d.model, "openai/pricey");
        assert_eq!(d.switches, 0, "replaying the anchor is not a switch");
        assert!(d.warm);

        let stored = orch.get_binding("s1", None).await.unwrap();
        assert_eq!(stored.anchor_model, "openai/pricey");
        assert_eq!(stored.requested_model, CLIENT_MODEL);
        assert!(
            stored.last_used.elapsed().unwrap() < Duration::from_secs(5),
            "the binding should have been refreshed"
        );
    }
}
