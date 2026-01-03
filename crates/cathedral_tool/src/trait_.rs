//! Tool trait for deterministic tool execution.

use cathedral_core::{CoreResult, CoreError};
use serde::{Deserialize, Serialize};

/// Output from a tool execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Raw output bytes
    pub data: Vec<u8>,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output captured
    pub stdout: Vec<u8>,
    /// Standard error captured
    pub stderr: Vec<u8>,
    /// Side effects that occurred
    pub side_effects: Vec<String>,
}

impl ToolOutput {
    /// Create a new successful output
    #[must_use]
    pub fn success(data: Vec<u8>) -> Self {
        Self {
            data,
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            side_effects: Vec::new(),
        }
    }

    /// Create a new failed output
    #[must_use]
    pub fn failure(exit_code: i32, stderr: Vec<u8>) -> Self {
        Self {
            data: Vec::new(),
            exit_code,
            stdout: Vec::new(),
            stderr,
            side_effects: Vec::new(),
        }
    }

    /// Check if the execution was successful
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Error from tool execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Tool not found
    NotFound { name: String },
    /// Invalid input
    InvalidInput { reason: String },
    /// Execution failed
    ExecutionFailed { reason: String },
    /// Timeout
    Timeout,
    /// Capability denied
    CapabilityDenied { capability: String },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "Tool not found: {}", name),
            Self::InvalidInput { reason } => write!(f, "Invalid input: {}", reason),
            Self::ExecutionFailed { reason } => write!(f, "Execution failed: {}", reason),
            Self::Timeout => write!(f, "Tool execution timed out"),
            Self::CapabilityDenied { capability } => {
                write!(f, "Capability denied: {}", capability)
            }
        }
    }
}

impl std::error::Error for ToolError {}

impl From<ToolError> for CoreError {
    fn from(err: ToolError) -> Self {
        CoreError::Validation {
            field: "tool".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Tool trait for deterministic execution
///
/// All tools must be deterministic - same input always produces same output.
/// Tools are assumed to be potentially hostile and must be sandboxed.
pub trait Tool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's version
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Execute the tool with the given input
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput>;

    /// Get the tool's input schema (JSON Schema)
    fn input_schema(&self) -> Option<String> {
        None
    }

    /// Get the tool's output schema (JSON Schema)
    fn output_schema(&self) -> Option<String> {
        None
    }

    /// Check if the tool is deterministic
    fn is_deterministic(&self) -> bool {
        true
    }

    /// Get the tool's timeout in logical ticks (0 = no limit)
    fn timeout_ticks(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success(b"hello".to_vec());
        assert!(output.is_success());
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.data, b"hello");
    }

    #[test]
    fn test_tool_output_failure() {
        let output = ToolOutput::failure(1, b"error".to_vec());
        assert!(!output.is_success());
        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stderr, b"error");
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::NotFound {
            name: "test_tool".to_string(),
        };
        assert_eq!(err.to_string(), "Tool not found: test_tool");
    }

    #[test]
    fn test_tool_error_capability_denied() {
        let err = ToolError::CapabilityDenied {
            capability: "fs_write".to_string(),
        };
        assert_eq!(err.to_string(), "Capability denied: fs_write");
    }
}
