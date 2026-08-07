use common::configuration::EffectiveRoutingBudget;
use common::consts::ROUTING_MAX_SWITCH_SPEND_PCT_HEADER;
use hyper::header::HeaderMap;
use tracing::warn;

/// Read `x-routing-max-switch-spend-pct` from request headers.
pub fn max_switch_spend_pct_from_headers(headers: &HeaderMap) -> Option<f64> {
    let raw = headers
        .get(ROUTING_MAX_SWITCH_SPEND_PCT_HEADER)
        .and_then(|h| h.to_str().ok())?;
    match EffectiveRoutingBudget::parse_max_switch_spend_pct_header_value(raw) {
        Some(pct) => Some(pct),
        None => {
            warn!(
                header = ROUTING_MAX_SWITCH_SPEND_PCT_HEADER,
                value = raw,
                "ignoring invalid max_switch_spend_pct header value"
            );
            None
        }
    }
}

/// Instance routing budget with an optional per-request `max_switch_spend_pct` override.
pub fn routing_budget_for_request(
    base: Option<EffectiveRoutingBudget>,
    headers: &HeaderMap,
) -> Option<EffectiveRoutingBudget> {
    EffectiveRoutingBudget::for_request(base, max_switch_spend_pct_from_headers(headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::consts::ROUTING_MAX_SWITCH_SPEND_PCT_HEADER;
    use hyper::header::HeaderValue;

    #[test]
    fn parses_canonical_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            ROUTING_MAX_SWITCH_SPEND_PCT_HEADER,
            HeaderValue::from_static("15"),
        );
        assert_eq!(max_switch_spend_pct_from_headers(&headers), Some(15.0));
    }

    #[test]
    fn header_overrides_configured_budget() {
        let base = EffectiveRoutingBudget {
            max_switch_spend_pct: 20.0,
            replenish_on_rebind: true,
            cache_read_discount: 0.1,
            record_counterfactual: true,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            ROUTING_MAX_SWITCH_SPEND_PCT_HEADER,
            HeaderValue::from_static("5"),
        );
        let resolved = routing_budget_for_request(Some(base), &headers).unwrap();
        assert_eq!(resolved.max_switch_spend_pct, 5.0);
        assert!(resolved.record_counterfactual);
    }
}
