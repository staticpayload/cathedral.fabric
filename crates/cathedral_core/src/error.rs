//! Core error types for CATHEDRAL.

use std::fmt;

/// Core result type
pub type CoreResult<T> = Result<T, CoreError>;

/// Core error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Invalid encoding
    InvalidEncoding,

    /// Encoding buffer overflow
    EncodingOverflow,

    /// Hash mismatch
    HashMismatch { expected: String, actual: String },

    /// Invalid hash format
    InvalidHash { reason: String },

    /// Broken hash chain
    BrokenChain { position: usize },

    /// Invalid ID format
    InvalidId { reason: String },

    /// Invalid timestamp
    InvalidTimestamp { reason: String },

    /// Invalid capability
    InvalidCapability { reason: String },

    /// Invalid version
    InvalidVersion { reason: String },

    /// Parse error
    ParseError { message: String },

    /// Validation error
    Validation { field: String, reason: String },

    /// Not found
    NotFound { kind: String, id: String },

    /// Already exists
    AlreadyExists { kind: String, id: String },

    /// Capacity exceeded
    CapacityExceeded { resource: String, limit: u64 },

    /// Timeout
    Timeout {
        /// Operation that timed out
        operation: String,
    },

    /// Cancelled
    Cancelled,

    /// Permission denied
    PermissionDenied {
        /// Operation that was denied
        operation: String,
    },

    /// Internal error (for unexpected errors)
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEncoding => write!(f, "Invalid encoding"),
            Self::EncodingOverflow => write!(f, "Encoding buffer overflow"),
            Self::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch: expected {}, got {}", expected, actual)
            }
            Self::InvalidHash { reason } => write!(f, "Invalid hash: {}", reason),
            Self::BrokenChain { position } => {
                write!(f, "Broken hash chain at position {}", position)
            }
            Self::InvalidId { reason } => write!(f, "Invalid ID: {}", reason),
            Self::InvalidTimestamp { reason } => write!(f, "Invalid timestamp: {}", reason),
            Self::InvalidCapability { reason } => write!(f, "Invalid capability: {}", reason),
            Self::InvalidVersion { reason } => write!(f, "Invalid version: {}", reason),
            Self::ParseError { message } => write!(f, "Parse error: {}", message),
            Self::Validation { field, reason } => {
                write!(f, "Validation failed for {}: {}", field, reason)
            }
            Self::NotFound { kind, id } => write!(f, "{} not found: {}", kind, id),
            Self::AlreadyExists { kind, id } => write!(f, "{} already exists: {}", kind, id),
            Self::CapacityExceeded { resource, limit } => {
                write!(f, "Capacity exceeded for {}: {}", resource, limit)
            }
            Self::Timeout { operation } => write!(f, "Timeout: {}", operation),
            Self::Cancelled => write!(f, "Operation cancelled"),
            Self::PermissionDenied { operation } => {
                write!(f, "Permission denied: {}", operation)
            }
            Self::Internal { message } => write!(f, "Internal error: {}", message),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<serde_json::Error> for CoreError {
    fn from(_err: serde_json::Error) -> Self {
        Self::InvalidEncoding
    }
}

impl From<postcard::Error> for CoreError {
    fn from(_: postcard::Error) -> Self {
        Self::InvalidEncoding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CoreError::InvalidEncoding;
        assert_eq!(format!("{}", err), "Invalid encoding");

        let err = CoreError::NotFound {
            kind: "Event".to_string(),
            id: "evt_123".to_string(),
        };
        assert_eq!(format!("{}", err), "Event not found: evt_123");
    }

    #[test]
    fn test_hash_mismatch_error() {
        let err = CoreError::HashMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let s = format!("{}", err);
        assert!(s.contains("abc123"));
        assert!(s.contains("def456"));
    }

    #[test]
    fn test_broken_chain_error() {
        let err = CoreError::BrokenChain { position: 42 };
        let s = format!("{}", err);
        assert!(s.contains("42"));
    }

    #[test]
    fn test_error_equality() {
        let err1 = CoreError::InvalidEncoding;
        let err2 = CoreError::InvalidEncoding;
        assert_eq!(err1, err2);

        let err3 = CoreError::Cancelled;
        assert_ne!(err1, err3);
    }
}
