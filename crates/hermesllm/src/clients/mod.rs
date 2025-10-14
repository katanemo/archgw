pub mod endpoints;
pub mod lib;
pub mod transformer;

// Re-export the main items for easier access
pub use endpoints::{identify_provider, SupportedAPIs};
pub use lib::*;

// Note: transformer module contains TryFrom trait implementations that are automatically available
