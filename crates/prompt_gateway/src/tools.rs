use common::configuration::{HttpMethod, Parameter};
use std::collections::HashMap;

use serde_yaml::Value;

// only add params that are of string, number and bool type
pub fn filter_tool_params(tool_params: &Option<HashMap<String, Value>>) -> HashMap<String, String> {
    if tool_params.is_none() {
        return HashMap::new();
    }
    tool_params
        .as_ref()
        .unwrap()
        .iter()
        .filter(|(_, value)| value.is_number() || value.is_string() || value.is_bool())
        .map(|(key, value)| match value {
            Value::Number(n) => (key.clone(), n.to_string()),
            Value::String(s) => (key.clone(), s.clone()),
            Value::Bool(b) => (key.clone(), b.to_string()),
            Value::Null => todo!(),
            Value::Sequence(_) => todo!(),
            Value::Mapping(_) => todo!(),
            Value::Tagged(_) => todo!(),
        })
        .collect::<HashMap<String, String>>()
}

pub fn compute_request_path_body(
    endpoint_path: &str,
    tool_params: &Option<HashMap<String, Value>>,
    prompt_target_params: &[Parameter],
    http_method: &HttpMethod,
) -> Result<(String, Option<String>), String> {
    let tool_url_params = filter_tool_params(tool_params);
    let (path_with_params, query_string, additional_params) = common::path::replace_params_in_path(
        endpoint_path,
        &tool_url_params,
        prompt_target_params,
    )?;

    let (path, body) = match http_method {
        HttpMethod::Get => (format!("{}?{}", path_with_params, query_string), None),
        HttpMethod::Post => {
            let mut additional_params = additional_params;
            if !query_string.is_empty() {
                query_string.split("&").for_each(|param| {
                    let mut parts = param.split("=");
                    let key = parts.next().unwrap();
                    let value = parts.next().unwrap();
                    additional_params.insert(key.to_string(), value.to_string());
                });
            }
            let body = serde_json::to_string(&additional_params).unwrap();
            (path_with_params, Some(body))
        }
    };

    Ok((path, body))
}

#[cfg(test)]
mod test {
    use common::configuration::{HttpMethod, Parameter};

    #[test]
    fn test_compute_request_path_body() {
        let endpoint_path = "/cluster.open-cluster-management.io/v1/managedclusters/{cluster_name}";
        let tool_params = serde_yaml::from_str(
            r#"
      cluster_name: test1
      hello: hello world
      "#,
        )
        .unwrap();
        let prompt_target_params = vec![Parameter {
            name: "country".to_string(),
            parameter_type: None,
            description: "test target".to_string(),
            required: None,
            enum_values: None,
            default: Some("US".to_string()),
            in_path: None,
            format: None,
        }];
        let http_method = HttpMethod::Get;
        let (path, body) = super::compute_request_path_body(
            endpoint_path,
            &tool_params,
            &prompt_target_params,
            &http_method,
        )
        .unwrap();
        assert_eq!(
            path,
            "/cluster.open-cluster-management.io/v1/managedclusters/test1?hello=hello%20world&country=US"
        );
        assert_eq!(body, None);
    }

    #[test]
    fn test_compute_request_path_body_empty_params() {
        let endpoint_path = "/cluster.open-cluster-management.io/v1/managedclusters/";
        let tool_params = serde_yaml::from_str(r#"{}"#).unwrap();
        let prompt_target_params = vec![Parameter {
            name: "country".to_string(),
            parameter_type: None,
            description: "test target".to_string(),
            required: None,
            enum_values: None,
            default: Some("US".to_string()),
            in_path: None,
            format: None,
        }];
        let http_method = HttpMethod::Get;
        let (path, body) = super::compute_request_path_body(
            endpoint_path,
            &tool_params,
            &prompt_target_params,
            &http_method,
        )
        .unwrap();
        assert_eq!(
            path,
            "/cluster.open-cluster-management.io/v1/managedclusters/?country=US"
        );
        assert_eq!(body, None);
    }

    #[test]
    fn test_compute_request_path_body_override_default_val() {
        let endpoint_path = "/cluster.open-cluster-management.io/v1/managedclusters/";
        let tool_params = serde_yaml::from_str(
            r#"
      country: UK
      "#,
        )
        .unwrap();
        let prompt_target_params = vec![Parameter {
            name: "country".to_string(),
            parameter_type: None,
            description: "test target".to_string(),
            required: None,
            enum_values: None,
            default: Some("US".to_string()),
            in_path: None,
            format: None,
        }];
        let http_method = HttpMethod::Get;
        let (path, body) = super::compute_request_path_body(
            endpoint_path,
            &tool_params,
            &prompt_target_params,
            &http_method,
        )
        .unwrap();
        assert_eq!(
            path,
            "/cluster.open-cluster-management.io/v1/managedclusters/?country=UK"
        );
        assert_eq!(body, None);
    }
}
