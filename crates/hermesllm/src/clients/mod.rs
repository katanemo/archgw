pub mod lib;
pub mod transformer;
pub mod endpoints;

// Re-export the main items for easier access
pub use lib::*;
pub use endpoints::{is_supported_endpoint, supported_endpoints, identify_provider};

// Note: transformer module contains TryFrom trait implementations that are automatically available
