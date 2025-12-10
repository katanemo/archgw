use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ResourceSpan {
    pub resource: Resource,
    #[serde(rename = "scopeSpans")]
    pub scope_spans: Vec<ScopeSpan>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource {
    pub attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScopeSpan {
    pub scope: Scope,
    pub spans: Vec<Span>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Scope {
    pub name: String,
    pub version: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Span {
    #[serde(rename = "traceId")]
    pub trace_id: String,
    #[serde(rename = "spanId")]
    pub span_id: String,
    #[serde(rename = "parentSpanId")]
    pub parent_span_id: Option<String>, // Optional in case there's no parent span
    pub name: String,
    #[serde(rename = "startTimeUnixNano")]
    pub start_time_unix_nano: String,
    #[serde(rename = "endTimeUnixNano")]
    pub end_time_unix_nano: String,
    pub kind: u32,
    pub attributes: Vec<Attribute>,
    pub events: Option<Vec<Event>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(rename = "timeUnixNano")]
    pub time_unix_nano: String,
    pub name: String,
    pub attributes: Vec<Attribute>,
}

impl Event {
    pub fn new(name: String, time_unix_nano: u128) -> Self {
        Event {
            time_unix_nano: format!("{}", time_unix_nano),
            name,
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, key: String, value: String) {
        self.attributes.push(Attribute {
            key,
            value: AttributeValue {
                string_value: Some(value),
            },
        });
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub key: String,
    pub value: AttributeValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttributeValue {
    #[serde(rename = "stringValue")]
    pub string_value: Option<String>, // Use Option to handle different value types
}

pub struct Traceparent {
    pub version: String,
    pub trace_id: String,
    pub parent_id: String,
    pub flags: String,
}

impl std::fmt::Display for Traceparent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}",
            self.version, self.trace_id, self.parent_id, self.flags
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TraceparentNewError {
    #[error("Invalid traceparent: \'{0}\'")]
    InvalidTraceparent(String),
}

impl TryFrom<String> for Traceparent {
    type Error = TraceparentNewError;

    fn try_from(traceparent: String) -> Result<Self, Self::Error> {
        let traceparent_tokens: Vec<&str> = traceparent.split("-").collect::<Vec<&str>>();
        if traceparent_tokens.len() != 4 {
            return Err(TraceparentNewError::InvalidTraceparent(traceparent));
        }
        Ok(Traceparent {
            version: traceparent_tokens[0].to_string(),
            trace_id: traceparent_tokens[1].to_string(),
            parent_id: traceparent_tokens[2].to_string(),
            flags: traceparent_tokens[3].to_string(),
        })
    }
}
