//! Tool adapter for sandboxed execution.

use cathedral_core::{Capability, CapabilitySet, CoreResult, CoreError};
use crate::trait_::{Tool, ToolOutput};
use crate::registry::SharedRegistry;
use std::sync::Arc;

/// Error from adapter operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// Tool execution failed
    ExecutionFailed { reason: String },
    /// Capability check failed
    CapabilityDenied { capability: String },
    /// Timeout exceeded
    Timeout,
    /// Invalid input
    InvalidInput { reason: String },
    /// Invalid output
    InvalidOutput { reason: String },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionFailed { reason } => write!(f, "Execution failed: {}", reason),
            Self::CapabilityDenied { capability } => {
                write!(f, "Capability denied: {}", capability)
            }
            Self::Timeout => write!(f, "Execution timeout"),
            Self::InvalidInput { reason } => write!(f, "Invalid input: {}", reason),
            Self::InvalidOutput { reason } => write!(f, "Invalid output: {}", reason),
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<AdapterError> for CoreError {
    fn from(err: AdapterError) -> Self {
        CoreError::Validation {
            field: "adapter".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Adapter for executing tools with capability checking
pub struct ToolAdapter {
    /// The underlying tool
    tool: Arc<dyn Tool>,
    /// Allowed capabilities
    capabilities: CapabilitySet,
    /// Timeout in logical ticks (0 = no limit)
    timeout_ticks: u64,
}

impl ToolAdapter {
    /// Create a new adapter for a tool
    #[must_use]
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self {
            tool,
            capabilities: CapabilitySet::new(),
            timeout_ticks: 0,
        }
    }

    /// Set allowed capabilities
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: CapabilitySet) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set timeout in logical ticks
    #[must_use]
    pub fn with_timeout(mut self, ticks: u64) -> Self {
        self.timeout_ticks = ticks;
        self
    }

    /// Check if tool has required capabilities
    fn check_capabilities(&self, required: &[Capability]) -> Result<(), AdapterError> {
        for cap in required {
            if !self.capabilities.allows(cap) {
                return Err(AdapterError::CapabilityDenied {
                    capability: format!("{:?}", cap),
                });
            }
        }
        Ok(())
    }

    /// Execute the tool with capability checking
    ///
    /// # Errors
    ///
    /// Returns error if capability check fails or execution fails
    pub fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput> {
        // For now, we execute without specific capability requirements
        // In a full implementation, the tool would declare its required capabilities
        self.tool.execute(input)
    }
}

/// Host adapter for running tools in a sandboxed environment
pub struct HostAdapter {
    /// Available tools
    tools: Arc<SharedRegistry>,
    /// Global capability set
    capabilities: CapabilitySet,
}

impl HostAdapter {
    /// Create a new host adapter
    #[must_use]
    pub fn new(tools: Arc<SharedRegistry>) -> Self {
        Self {
            tools,
            capabilities: CapabilitySet::new(),
        }
    }

    /// Set global capabilities
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: CapabilitySet) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Execute a tool by name
    ///
    /// # Errors
    ///
    /// Returns error if tool not found or execution fails
    pub fn execute_tool(&self, name: &str, input: &[u8]) -> CoreResult<ToolOutput> {
        let tool = self.tools.get(name)?;
        let adapter = ToolAdapter::new(tool).with_capabilities(self.capabilities.clone());
        adapter.execute(input)
    }

    /// List available tools
    #[must_use]
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.list()
    }

    /// Check if a tool is available
    #[must_use]
    pub fn has_tool(&self, name: &str) -> bool {
        let list = self.tools.list();
        list.contains(&name.to_string())
    }
}

/// Builtin tools for common operations
pub mod builtin {
    use super::*;
    use crate::trait_::ToolOutput;

    /// Echo tool - returns input as output
    pub struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput> {
            Ok(ToolOutput::success(input.to_vec()))
        }
    }

    /// Length tool - returns length of input
    pub struct LengthTool;

    impl Tool for LengthTool {
        fn name(&self) -> &str {
            "length"
        }

        fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput> {
            let len = input.len();
            let data = serde_json::to_vec(&len).map_err(|_| CoreError::Validation {
                field: "output".to_string(),
                reason: "Failed to serialize output".to_string(),
            })?;
            Ok(ToolOutput::success(data))
        }
    }

    /// Concat tool - concatenates input parts
    pub struct ConcatTool;

    impl Tool for ConcatTool {
        fn name(&self) -> &str {
            "concat"
        }

        fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput> {
            // Input is expected to be a JSON array of strings
            let parts: Vec<String> = serde_json::from_slice(input).map_err(|_| CoreError::Validation {
                field: "input".to_string(),
                reason: "Input must be a JSON array of strings".to_string(),
            })?;
            let result = parts.concat();
            Ok(ToolOutput::success(result.into_bytes()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::builtin::*;
    use std::sync::Arc as StdArc;

    fn make_arc_tool<T: Tool + 'static>(tool: T) -> StdArc<dyn Tool> {
        StdArc::new(tool)
    }

    #[test]
    fn test_tool_adapter_new() {
        let tool = make_arc_tool(EchoTool);
        let adapter = ToolAdapter::new(tool);
        assert_eq!(adapter.tool.name(), "echo");
    }

    #[test]
    fn test_tool_adapter_execute() {
        let tool = make_arc_tool(EchoTool);
        let adapter = ToolAdapter::new(tool);
        let result = adapter.execute(b"hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data, b"hello");
    }

    #[test]
    fn test_echo_tool() {
        let tool = EchoTool;
        let result = tool.execute(b"test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data, b"test");
    }

    #[test]
    fn test_length_tool() {
        let tool = LengthTool;
        let result = tool.execute(b"hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data, br#"5"#);
    }

    #[test]
    fn test_concat_tool() {
        let tool = ConcatTool;
        let input = br#"["hello", " ", "world"]"#;
        let result = tool.execute(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data, b"hello world");
    }

    #[test]
    fn test_adapter_error_display() {
        let err = AdapterError::CapabilityDenied {
            capability: "fs_write".to_string(),
        };
        assert_eq!(err.to_string(), "Capability denied: fs_write");
    }

    #[test]
    fn test_concat_tool_invalid_input() {
        let tool = ConcatTool;
        let result = tool.execute(b"not an array");
        assert!(result.is_err());
    }
}
