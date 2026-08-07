use common::configuration::EffectiveRoutingBudget;
use common::consts::{MAX_SWITCH_SPEND_PCT_HEADER_ALIAS, PLANO_MAX_SWITCH_SPEND_PCT_HEADER};
use hyper::header::HeaderMap;
use tracing::warn;

/// Read `x-plano-max-switch-spend-pct` or the `max_switch_spend_pct` alias from request headers.
pub fn max_switch_spend_pct_from_headers(headers: &HeaderMap) -> Option<f64> {
    for name in [
        PLANO_MAX_SWITCH_SPEND_PCT_HEADER,
        MAX_SWITCH_SPEND_PCT_HEADER_ALIAS,
    ] {
        let Some(raw) = headers.get(name).and_then(|h| h.to_str().ok()) else {
            continue;
        };
        match EffectiveRoutingBudget::parse_max_switch_spend_pct_header_value(raw) {
            Some(pct) => return Some(pct),
            None => {
                warn!(
                    header = name,
                    value = raw,
                    "ignoring invalid max_switch_spend_pct header value"
                );
            }
        }
    }
    None
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
    use hyper::header::HeaderValue;

    #[test]
    fn parses_canonical_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            PLANO_MAX_SWITCH_SPEND_PCT_HEADER,
            HeaderValue::from_static("15"),
        );
        assert_eq!(max_switch_spend_pct_from_headers(&headers), Some(15.0));
    }

    #[test]
    fn parses_alias_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            MAX_SWITCH_SPEND_PCT_HEADER_ALIAS,
            HeaderValue::from_static("12.5"),
        );
        assert_eq!(max_switch_spend_pct_from_headers(&headers), Some(12.5));
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
            PLANO_MAX_SWITCH_SPEND_PCT_HEADER,
            HeaderValue::from_static("5"),
        );
        let resolved = routing_budget_for_request(Some(base), &headers).unwrap();
        assert_eq!(resolved.max_switch_spend_pct, 5.0);
        assert!(resolved.record_counterfactual);
    }
}
