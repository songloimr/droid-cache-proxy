//! HTTP gateway server, upstream forwarder and SSE relay loop.
//!
//! Owns route handling for `/v1/*`, header filtering, `include_usage`
//! injection and latency tracking. Only the error scaffold exists for now.

use thiserror::Error;

/// Errors raised by the proxy gateway.
#[derive(Debug, Error)]
pub enum Error {
    /// Transparent wrapper around core errors.
    #[error(transparent)]
    Core(#[from] dcp_core::Error),
    /// Transparent wrapper around store errors.
    #[error(transparent)]
    Store(#[from] dcp_store::Error),
    /// Transparent wrapper around transform errors.
    #[error(transparent)]
    Transform(#[from] dcp_transform::Error),
    /// Placeholder error used until real variants land with Phase 1 implementation.
    #[error("proxy placeholder error: {0}")]
    Placeholder(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_is_stable() {
        let err = Error::Placeholder("scaffold".to_string());
        assert_eq!(err.to_string(), "proxy placeholder error: scaffold");
    }
}
