#!/bin/bash
set -e

# Routing Budget demo — drives the real /v1/chat/completions endpoint (the
# same path production traffic takes) to show:
#   1. implicit session pinning (same session across turns, going warm)
#   2. the routing budget vetoing an unaffordable model switch
#
# Starts Plano itself with tracing enabled, then after each request prints
# the matching trace via `planoai trace <trace_id>`, plus a second view
# filtered down to just the `plano.*` attributes — session_id, cache warmth,
# and the switch decision — so the pinning behavior is obvious without having
# to know brightstaff's internals.
#
# We generate our own W3C `traceparent` header and send it on each request:
# brightstaff only derives the `trace_id` it returns in the response body from
# an incoming `traceparent` header (see extract_or_generate_traceparent in
# crates/brightstaff/src/handlers/mod.rs) — without one, that field and the
# real exported span end up with two independently-random, uncorrelated IDs.
# Supplying our own keeps both in sync so we can look up the exact trace.
#
# Prereqs: `curl`, `jq`, `uuidgen`, and the `planoai` CLI on PATH.
# See GUIDE.md for the full walkthrough and how to flip the veto into an allow.

PLANO_URL="${PLANO_URL:-http://localhost:12000}"
METRICS_URL="${METRICS_URL:-http://localhost:9092}"

gen_trace_id() { uuidgen | tr '[:upper:]' '[:lower:]' | tr -d '-'; }
gen_span_id() { uuidgen | tr '[:upper:]' '[:lower:]' | tr -d '-' | cut -c1-16; }

# Poll for a specific trace_id to show up in the local OTLP listener, then
# keep polling until its span count stops growing for two checks in a row.
# Spans of the same trace can land in different OTLP batch-export flushes
# (routing_decision's parent span can finish - and export - slightly after
# its outbound child), so stopping at the first non-empty result can print a
# partial trace that's missing the root span.
wait_for_trace_by_id() {
  local id="$1"
  local tries=0
  local last_count=-1
  local stable_checks=0
  while [ "$tries" -lt 30 ]; do
    local json
    json="$(planoai trace "$id" --json --no-interactive 2>/dev/null)"
    local count
    count="$(printf '%s' "$json" | jq '(.traces[0].spans // []) | length' 2>/dev/null)"
    local has_root
    has_root="$(printf '%s' "$json" | jq -e '(.traces[0].spans // []) | any(.service == "plano(llm)")' 2>/dev/null)"
    if [ -n "$count" ] && [ "$count" -gt 0 ] 2>/dev/null && [ "$has_root" = "true" ]; then
      if [ "$count" = "$last_count" ]; then
        stable_checks=$((stable_checks + 1))
        [ "$stable_checks" -ge 2 ] && return 0
      else
        stable_checks=0
        last_count="$count"
      fi
    fi
    sleep 0.5
    tries=$((tries + 1))
  done
  return 1
}

# Right after startup, the very first outbound call can race Envoy/brightstaff
# still finishing their upstream connection warmup and come back as a plain-text
# "Failed to send request" instead of JSON. Poll with a throwaway request until
# a real completion comes back before running the actual demo turns.
wait_for_upstream_ready() {
  local tries=0
  while [ "$tries" -lt 20 ]; do
    if curl -s "$PLANO_URL/v1/chat/completions" \
        -H 'Content-Type: application/json' \
        -d '{"model": "openai/gpt-4o-mini", "messages": [{"role": "user", "content": "ping"}]}' \
        | jq -e '.model' >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
    tries=$((tries + 1))
  done
  return 1
}

echo "=== Routing Budget Demo ==="
echo ""
echo "Starting Plano with tracing enabled (planoai up config.yaml --with-tracing)..."
planoai up config.yaml --with-tracing
trap 'echo ""; echo "Stopping Plano..."; planoai down' EXIT
echo ""
echo "Waiting for the upstream connection to warm up..."
wait_for_upstream_ready || echo "    (still not ready, continuing anyway)"
echo ""
echo "Both turns hit the real /v1/chat/completions endpoint. Watch the"
echo "'plano.*' trace attributes below each request: session_id stays the"
echo "same across turns, plano.cache.warm flips false -> true, and"
echo "plano.switch.decision shows whether the budget followed the router or"
echo "retained the warm anchor."
echo ""

# --- Turn 1: pin the session (a code-generation prompt) ---
echo "--- 1. Turn 1: pin the session (creates the binding) ---"
echo ""
turn1_trace_id="$(gen_trace_id)"
turn1_traceparent="00-${turn1_trace_id}-$(gen_span_id)-01"
turn1_response="$(curl -s "$PLANO_URL/v1/chat/completions" \
  -H 'Content-Type: application/json' \
  -H "traceparent: $turn1_traceparent" \
  -d '{"model": "openai/gpt-4o-mini", "messages": [
    {"role": "system", "content": "You are a senior Rust engineer."},
    {"role": "user", "content": "Write a Rust function that reverses a linked list."}
  ]}')"
printf '%s' "$turn1_response" | jq '{model, id}'
echo ""
echo "    Expect: model=anthropic/claude-sonnet-4-6 (code generation)"
echo ""
echo "    Trace for turn 1 (trace_id=$turn1_trace_id, waiting for export)..."
echo ""
wait_for_trace_by_id "$turn1_trace_id" || true
planoai trace "$turn1_trace_id"
echo "    Session/pinning signal:"
planoai trace "$turn1_trace_id" --filter "plano.*"
echo ""

# --- Turn 2: same system prompt + same first message, one turn later ---
echo "--- 2. Turn 2: same session, warm, router proposes a different model ---"
echo ""
turn2_trace_id="$(gen_trace_id)"
turn2_traceparent="00-${turn2_trace_id}-$(gen_span_id)-01"
turn2_response="$(curl -s "$PLANO_URL/v1/chat/completions" \
  -H 'Content-Type: application/json' \
  -H "traceparent: $turn2_traceparent" \
  -d '{"model": "openai/gpt-4o-mini", "messages": [
    {"role": "system", "content": "You are a senior Rust engineer."},
    {"role": "user", "content": "Write a Rust function that reverses a linked list."},
    {"role": "assistant", "content": "Here is an idiomatic in-place reversal for a singly linked list:\n\n```rust\ntype Link = Option<Box<Node>>;\n\nstruct Node {\n    val: i32,\n    next: Link,\n}\n\nfn reverse(mut head: Link) -> Link {\n    let mut prev: Link = None;\n    while let Some(mut node) = head {\n        head = node.next.take();\n        node.next = prev;\n        prev = Some(node);\n    }\n    prev\n}\n```\n\nIt walks the list once, moving the next pointer of each node to its predecessor."},
    {"role": "user", "content": "Now explain its time complexity in plain English — no code."}
  ]}')"
printf '%s' "$turn2_response" | jq '{model, id}'
echo ""
echo "    Expect: same model as turn 1 (anthropic/claude-sonnet-4-6). If the"
echo "    router proposed openai/gpt-4o (code understanding) this turn, the"
echo "    budget vetoed the switch and retained the warm anchor instead —"
echo "    check plano.switch.decision below."
echo ""
echo "    Trace for turn 2 (trace_id=$turn2_trace_id, waiting for export)..."
echo ""
wait_for_trace_by_id "$turn2_trace_id" || true
planoai trace "$turn2_trace_id"
echo "    Session/pinning signal:"
planoai trace "$turn2_trace_id" --filter "plano.*"
echo ""

# --- Switch decisions metric ---
echo "--- 3. Switch decisions (why the budget decided what it did) ---"
echo ""
curl -s "$METRICS_URL/metrics" | grep session_switch_decisions || true
echo ""
echo "    over_cap  = switch vetoed, anchor retained"
echo "    free      = cheaper/affordable switch allowed"
echo "    same_anchor = router did not propose a switch this turn"
echo ""

echo "=== Demo Complete ==="
echo ""
echo "To see the switch ALLOWED instead of vetoed: comment out the routing_budget"
echo "block in config.yaml (or raise max_switch_spend_pct), then 'planoai down &&"
echo "planoai up config.yaml' and re-run — turn 2 will follow the router to"
echo "openai/gpt-4o. See GUIDE.md for details."
