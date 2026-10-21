//! Offline analysis engine: prefix divergence detection and reporting.
//!
//! Reads SQLite metadata and JSONL payloads, groups requests by session and
//! classifies cache-miss causes (M1-M6). Only the error scaffold exists for now.

use thiserror::Error;

/// Errors raised by the offline analyzer.
#[derive(Debug, Error)]
pub enum Error {
    /// Transparent wrapper around core errors.
    #[error(transparent)]
    Core(#[from] dcp_core::Error),
    /// Transparent wrapper around store errors.
    #[error(transparent)]
    Store(#[from] dcp_store::Error),
    /// Placeholder error used until real variants land with Phase 2 implementation.
    #[error("analyze placeholder error: {0}")]
    Placeholder(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_is_stable() {
        let err = Error::Placeholder("scaffold".to_string());
        assert_eq!(err.to_string(), "analyze placeholder error: scaffold");
    }
}
