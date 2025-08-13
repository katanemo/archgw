pub mod endpoints;
pub mod lib;
pub mod transformer;

// Re-export the main items for easier access
pub use endpoints::{identify_provider, is_supported_endpoint, supported_endpoints};
pub use lib::*;

// Note: transformer module contains TryFrom trait implementations that are automatically available
