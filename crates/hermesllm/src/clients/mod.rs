pub mod lib;
pub mod transformer;
pub mod endpoints;

// Re-export the main items for easier access
pub use lib::*;
pub use endpoints::{SupportedAPIs, identify_provider};

// Note: transformer module contains TryFrom trait implementations that are automatically available
