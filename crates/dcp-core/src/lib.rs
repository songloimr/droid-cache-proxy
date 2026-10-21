//! Core data models, configuration types and shared error definitions.
//!
//! This crate owns the OpenAI-compatible protocol structs, configuration
//! representation and the base error type used by the other workspace crates.
//! Business logic lives in the downstream crates; `dcp-core` stays dependency-light.

use thiserror::Error;

/// Base error type for core model and configuration operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Placeholder error used until real variants land with Phase 1 implementation.
    #[error("core placeholder error: {0}")]
    Placeholder(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_is_stable() {
        let err = Error::Placeholder("scaffold".to_string());
        assert_eq!(err.to_string(), "core placeholder error: scaffold");
    }
}
