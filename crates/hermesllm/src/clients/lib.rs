//! Helper functions and utilities for API transformations
//! Contains error types and shared utilities

use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Unsupported content type: {0}")]
    UnsupportedContent(String),
    #[error("Invalid tool input format")]
    InvalidToolInput,
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Unsupported conversion: {0}")]
    UnsupportedConversion(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let error = TransformError::MissingField("test".to_string());
        assert!(matches!(error, TransformError::MissingField(_)));
    }
}
