//! Tool schemas for input/output validation.

use cathedral_core::Capability;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Schema for a tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// Input schema
    pub input: InputSchema,
    /// Output schema
    pub output: OutputSchema,
    /// Required capabilities
    pub capabilities: BTreeSet<Capability>,
    /// Declared side effects
    pub side_effects: Vec<SideEffect>,
}

impl ToolSchema {
    /// Create a new tool schema
    #[must_use]
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            input: InputSchema::new(),
            output: OutputSchema::new(),
            capabilities: BTreeSet::new(),
            side_effects: Vec::new(),
        }
    }

    /// Add a required capability
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    /// Add a side effect
    #[must_use]
    pub fn with_side_effect(mut self, effect: SideEffect) -> Self {
        self.side_effects.push(effect);
        self
    }

    /// Set input schema
    #[must_use]
    pub fn with_input(mut self, schema: InputSchema) -> Self {
        self.input = schema;
        self
    }

    /// Set output schema
    #[must_use]
    pub fn with_output(mut self, schema: OutputSchema) -> Self {
        self.output = schema;
        self
    }
}

/// Input schema for a tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSchema {
    /// JSON Schema for input validation
    pub json_schema: Option<String>,
    /// Maximum input size in bytes
    pub max_size_bytes: Option<usize>,
    /// Required fields
    pub required_fields: Vec<String>,
    /// Content type (e.g., "application/json")
    pub content_type: Option<String>,
}

impl InputSchema {
    /// Create a new input schema
    #[must_use]
    pub fn new() -> Self {
        Self {
            json_schema: None,
            max_size_bytes: None,
            required_fields: Vec::new(),
            content_type: None,
        }
    }

    /// Set JSON schema
    #[must_use]
    pub fn with_json_schema(mut self, schema: String) -> Self {
        self.json_schema = Some(schema);
        self
    }

    /// Set maximum size
    #[must_use]
    pub fn with_max_size(mut self, bytes: usize) -> Self {
        self.max_size_bytes = Some(bytes);
        self
    }

    /// Add a required field
    #[must_use]
    pub fn with_required_field(mut self, field: String) -> Self {
        self.required_fields.push(field);
        self
    }

    /// Validate input size
    #[must_use]
    pub fn validate_size(&self, input: &[u8]) -> bool {
        if let Some(max_size) = self.max_size_bytes {
            input.len() <= max_size
        } else {
            true
        }
    }
}

impl Default for InputSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Output schema for a tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSchema {
    /// JSON Schema for output validation
    pub json_schema: Option<String>,
    /// Maximum output size in bytes
    pub max_size_bytes: Option<usize>,
    /// Content type (e.g., "application/json")
    pub content_type: Option<String>,
    /// Whether output is deterministic
    pub deterministic: bool,
}

impl OutputSchema {
    /// Create a new output schema
    #[must_use]
    pub fn new() -> Self {
        Self {
            json_schema: None,
            max_size_bytes: None,
            content_type: None,
            deterministic: true,
        }
    }

    /// Set JSON schema
    #[must_use]
    pub fn with_json_schema(mut self, schema: String) -> Self {
        self.json_schema = Some(schema);
        self
    }

    /// Set maximum size
    #[must_use]
    pub fn with_max_size(mut self, bytes: usize) -> Self {
        self.max_size_bytes = Some(bytes);
        self
    }

    /// Set whether output is deterministic
    #[must_use]
    pub fn with_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }

    /// Validate output size
    #[must_use]
    pub fn validate_size(&self, output: &[u8]) -> bool {
        if let Some(max_size) = self.max_size_bytes {
            output.len() <= max_size
        } else {
            true
        }
    }
}

impl Default for OutputSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Declared side effect of a tool
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SideEffect {
    /// File system read
    FsRead { path: String },
    /// File system write
    FsWrite { path: String },
    /// File system delete
    FsDelete { path: String },
    /// Network request
    NetRequest { url: String, method: String },
    /// Environment variable read
    EnvRead { var: String },
    /// Environment variable write
    EnvWrite { var: String },
    /// Subprocess execution
    Exec { command: String },
    /// Database query
    DbQuery { table: String, operation: String },
    /// Custom side effect
    Custom { name: String, description: String },
}

impl SideEffect {
    /// Check if this side effect is pure (no external state modification)
    #[must_use]
    pub fn is_pure(&self) -> bool {
        matches!(
            self,
            Self::FsRead { .. } | Self::EnvRead { .. } | Self::NetRequest { .. }
        )
    }

    /// Get a description of the side effect
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::FsRead { path } => format!("Read file: {}", path),
            Self::FsWrite { path } => format!("Write file: {}", path),
            Self::FsDelete { path } => format!("Delete file: {}", path),
            Self::NetRequest { url, method } => format!("{} {}", method, url),
            Self::EnvRead { var } => format!("Read env: {}", var),
            Self::EnvWrite { var } => format!("Write env: {}", var),
            Self::Exec { command } => format!("Execute: {}", command),
            Self::DbQuery { table, operation } => {
                format!("Database {} on {}", operation, table)
            }
            Self::Custom { name, description } => format!("{}: {}", name, description),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema_new() {
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());
        assert_eq!(schema.name, "test_tool");
        assert_eq!(schema.version, "1.0.0");
        assert!(schema.capabilities.is_empty());
    }

    #[test]
    fn test_tool_schema_with_capability() {
        use cathedral_core::Capability;
        let schema = ToolSchema::new("test".to_string(), "1.0.0".to_string())
            .with_capability(Capability::FsRead {
                prefixes: vec![".".to_string()],
            });
        assert_eq!(schema.capabilities.len(), 1);
    }

    #[test]
    fn test_input_schema_validate_size() {
        let schema = InputSchema::new().with_max_size(100);
        assert!(schema.validate_size(b"hello"));
        assert!(!schema.validate_size(&vec![0u8; 200]));
    }

    #[test]
    fn test_output_schema_deterministic() {
        let schema = OutputSchema::new().with_deterministic(false);
        assert!(!schema.deterministic);
    }

    #[test]
    fn test_side_effect_is_pure() {
        assert!(SideEffect::FsRead {
            path: "/tmp/test".to_string()
        }
        .is_pure());
        assert!(!SideEffect::FsWrite {
            path: "/tmp/test".to_string()
        }
        .is_pure());
    }

    #[test]
    fn test_side_effect_describe() {
        let effect = SideEffect::NetRequest {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
        };
        assert_eq!(effect.describe(), "GET https://example.com");
    }
}
