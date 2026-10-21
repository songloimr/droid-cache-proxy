//! Asynchronous persistence layer: SQLite writer, JSONL appender and rotation.
//!
//! The proxy pushes [`LogEvent`](crate::LogEvent) items through an in-memory
//! channel; a background worker defined here batches them into SQLite and the
//! daily JSONL files. Only the error scaffold exists for now.

use thiserror::Error;

/// Errors raised by the storage worker.
#[derive(Debug, Error)]
pub enum Error {
    /// Transparent wrapper around core errors.
    #[error(transparent)]
    Core(#[from] dcp_core::Error),
    /// Placeholder error used until real variants land with Phase 1 implementation.
    #[error("store placeholder error: {0}")]
    Placeholder(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_is_stable() {
        let err = Error::Placeholder("scaffold".to_string());
        assert_eq!(err.to_string(), "store placeholder error: scaffold");
    }
}
