//! Request transform pipeline: canonicalize, prefix align, volatile extract.
//!
//! Transforms must stay pure, idempotent and semantics-preserving with a
//! fail-open fallback to the original request body. Only the error scaffold
//! exists for now.

use thiserror::Error;

/// Errors raised by the transform pipeline.
#[derive(Debug, Error)]
pub enum Error {
    /// Transparent wrapper around core errors.
    #[error(transparent)]
    Core(#[from] dcp_core::Error),
    /// Placeholder error used until real variants land with Phase 2 implementation.
    #[error("transform placeholder error: {0}")]
    Placeholder(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_is_stable() {
        let err = Error::Placeholder("scaffold".to_string());
        assert_eq!(err.to_string(), "transform placeholder error: scaffold");
    }
}
