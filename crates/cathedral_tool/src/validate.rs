//! Tool validation for safety and correctness.

use crate::schema::{ToolSchema, SideEffect};
use crate::trait_::Tool;
use std::sync::Arc;

/// Validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Invalid tool name
    InvalidName { name: String },
    /// Missing required capability
    MissingCapability { capability: String },
    /// Undeclared side effect
    UndeclaredSideEffect { effect: String },
    /// Non-deterministic tool marked as deterministic
    DeterminismViolation { reason: String },
    /// Schema validation failed
    SchemaError { field: String, reason: String },
    /// Resource limit exceeded
    ResourceLimit { resource: String, limit: u64 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName { name } => write!(f, "Invalid tool name: {}", name),
            Self::MissingCapability { capability } => {
                write!(f, "Missing required capability: {}", capability)
            }
            Self::UndeclaredSideEffect { effect } => {
                write!(f, "Undeclared side effect: {}", effect)
            }
            Self::DeterminismViolation { reason } => {
                write!(f, "Determinism violation: {}", reason)
            }
            Self::SchemaError { field, reason } => {
                write!(f, "Schema error in {}: {}", field, reason)
            }
            Self::ResourceLimit { resource, limit } => {
                write!(f, "Resource limit: {} exceeds {}", resource, limit)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validation rule
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationRule {
    /// Tool name must be lowercase alphanumeric with underscores
    NameConvention,
    /// Tool must declare all required capabilities
    CapabilityDeclaration,
    /// Tool must declare all side effects
    SideEffectDeclaration,
    /// Deterministic tools must produce consistent output
    DeterminismCheck,
    /// Input/output schemas must be valid
    SchemaValidation,
    /// Resource limits must be respected
    ResourceLimits,
}

/// Tool validator for safety and correctness checks
pub struct ToolValidator {
    /// Enabled validation rules
    rules: Vec<ValidationRule>,
    /// Maximum allowed output size in bytes
    max_output_size: usize,
    /// Maximum allowed timeout in ticks
    max_timeout: u64,
}

impl ToolValidator {
    /// Create a new validator with default rules
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: vec![
                ValidationRule::NameConvention,
                ValidationRule::CapabilityDeclaration,
                ValidationRule::SideEffectDeclaration,
                ValidationRule::DeterminismCheck,
                ValidationRule::SchemaValidation,
                ValidationRule::ResourceLimits,
            ],
            max_output_size: 10 * 1024 * 1024, // 10 MB default
            max_timeout: 1_000_000, // 1M ticks default
        }
    }

    /// Create a validator with only specific rules
    #[must_use]
    pub fn with_rules(mut self, rules: Vec<ValidationRule>) -> Self {
        self.rules = rules;
        self
    }

    /// Set maximum output size
    #[must_use]
    pub fn with_max_output_size(mut self, bytes: usize) -> Self {
        self.max_output_size = bytes;
        self
    }

    /// Set maximum timeout
    #[must_use]
    pub fn with_max_timeout(mut self, ticks: u64) -> Self {
        self.max_timeout = ticks;
        self
    }

    /// Validate a tool against its schema
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn validate(&self, tool: &Arc<dyn Tool>, schema: &ToolSchema) -> Result<(), ValidationError> {
        // Validate tool name
        if self.rules.contains(&ValidationRule::NameConvention) {
            self.validate_name(tool.name())?;
        }

        // Validate capabilities
        if self.rules.contains(&ValidationRule::CapabilityDeclaration) {
            self.validate_capabilities(tool, schema)?;
        }

        // Validate side effects
        if self.rules.contains(&ValidationRule::SideEffectDeclaration) {
            self.validate_side_effects(tool, schema)?;
        }

        // Validate determinism
        if self.rules.contains(&ValidationRule::DeterminismCheck) {
            self.validate_determinism(tool)?;
        }

        // Validate schemas
        if self.rules.contains(&ValidationRule::SchemaValidation) {
            self.validate_schemas(schema)?;
        }

        // Validate resource limits
        if self.rules.contains(&ValidationRule::ResourceLimits) {
            self.validate_resources(tool)?;
        }

        Ok(())
    }

    /// Validate tool name follows convention
    fn validate_name(&self, name: &str) -> Result<(), ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
            });
        }

        // Name must be lowercase alphanumeric with underscores
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
            });
        }

        // Must not start or end with underscore
        if name.starts_with('_') || name.ends_with('_') {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
            });
        }

        // Must not have consecutive underscores
        if name.contains("__") {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
            });
        }

        Ok(())
    }

    /// Validate capabilities are declared
    fn validate_capabilities(
        &self,
        _tool: &Arc<dyn Tool>,
        schema: &ToolSchema,
    ) -> Result<(), ValidationError> {
        // Check that schema has capabilities if tool performs I/O
        // For now, this is a placeholder
        let _ = schema;
        Ok(())
    }

    /// Validate side effects are declared
    fn validate_side_effects(
        &self,
        _tool: &Arc<dyn Tool>,
        schema: &ToolSchema,
    ) -> Result<(), ValidationError> {
        // Check that schema declares side effects
        // For now, this is a placeholder
        let _ = schema;
        Ok(())
    }

    /// Validate determinism
    fn validate_determinism(&self, tool: &Arc<dyn Tool>) -> Result<(), ValidationError> {
        // If tool claims to be deterministic, we need to verify
        // For now, we trust the tool's declaration
        let _ = tool;
        Ok(())
    }

    /// Validate schemas
    fn validate_schemas(&self, schema: &ToolSchema) -> Result<(), ValidationError> {
        // Check schema name matches
        if schema.name.is_empty() {
            return Err(ValidationError::SchemaError {
                field: "name".to_string(),
                reason: "Schema name is empty".to_string(),
            });
        }

        // Check version
        if schema.version.is_empty() {
            return Err(ValidationError::SchemaError {
                field: "version".to_string(),
                reason: "Schema version is empty".to_string(),
            });
        }

        Ok(())
    }

    /// Validate resource limits
    fn validate_resources(&self, tool: &Arc<dyn Tool>) -> Result<(), ValidationError> {
        let timeout = tool.timeout_ticks();
        if timeout > 0 && timeout > self.max_timeout {
            return Err(ValidationError::ResourceLimit {
                resource: "timeout".to_string(),
                limit: self.max_timeout,
            });
        }

        Ok(())
    }

    /// Validate tool output against schema
    ///
    /// # Errors
    ///
    /// Returns error if output doesn't match schema
    pub fn validate_output(
        &self,
        output: &[u8],
        schema: &ToolSchema,
    ) -> Result<(), ValidationError> {
        // Check output size
        if output.len() > self.max_output_size {
            return Err(ValidationError::ResourceLimit {
                resource: "output_size".to_string(),
                limit: self.max_output_size as u64,
            });
        }

        // Validate against schema
        if let Some(ref json_schema) = schema.output.json_schema {
            // For now, just check it's valid JSON
            if serde_json::from_slice::<serde_json::Value>(output).is_err() {
                return Err(ValidationError::SchemaError {
                    field: "output".to_string(),
                    reason: "Output is not valid JSON".to_string(),
                });
            }
            let _ = json_schema;
        }

        Ok(())
    }

    /// Validate tool input against schema
    ///
    /// # Errors
    ///
    /// Returns error if input doesn't match schema
    pub fn validate_input(
        &self,
        input: &[u8],
        schema: &ToolSchema,
    ) -> Result<(), ValidationError> {
        // Check input size
        if let Some(max_size) = schema.input.max_size_bytes {
            if input.len() > max_size {
                return Err(ValidationError::ResourceLimit {
                    resource: "input_size".to_string(),
                    limit: max_size as u64,
                });
            }
        }

        // Validate against schema
        if let Some(ref json_schema) = schema.input.json_schema {
            // For now, just check it's valid JSON
            if serde_json::from_slice::<serde_json::Value>(input).is_err() {
                return Err(ValidationError::SchemaError {
                    field: "input".to_string(),
                    reason: "Input is not valid JSON".to_string(),
                });
            }
            let _ = json_schema;
        }

        Ok(())
    }
}

impl Default for ToolValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Side effect tracker for monitoring tool execution
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SideEffectTracker {
    /// Declared side effects
    declared: Vec<SideEffect>,
    /// Actual side effects observed
    actual: Vec<String>,
}

impl SideEffectTracker {
    /// Create a new tracker with declared effects
    #[must_use]
    pub fn new(declared: Vec<SideEffect>) -> Self {
        Self {
            declared,
            actual: Vec::new(),
        }
    }

    /// Record an actual side effect
    pub fn record(&mut self, effect: String) {
        self.actual.push(effect);
    }

    /// Check if all actual effects were declared
    ///
    /// # Errors
    ///
    /// Returns error if undeclared effects were found
    pub fn check(&self) -> Result<(), ValidationError> {
        let declared_descriptions: Vec<String> =
            self.declared.iter().map(|e| e.describe()).collect();

        for actual in &self.actual {
            if !declared_descriptions.contains(actual) {
                return Err(ValidationError::UndeclaredSideEffect {
                    effect: actual.clone(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::adapter::builtin::EchoTool;
    use std::sync::Arc;

    fn make_tool<T: Tool + 'static>(tool: T) -> Arc<dyn Tool> {
        Arc::new(tool)
    }

    #[test]
    fn test_validator_new() {
        let validator = ToolValidator::new();
        assert!(!validator.rules.is_empty());
    }

    #[test]
    fn test_validate_name_valid() {
        let validator = ToolValidator::new();
        assert!(validator.validate_name("valid_name").is_ok());
        assert!(validator.validate_name("lowercase123").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        let validator = ToolValidator::new();
        assert!(validator.validate_name("").is_err());
        assert!(validator.validate_name("InvalidName").is_err());
        assert!(validator.validate_name("_invalid").is_err());
        assert!(validator.validate_name("invalid_").is_err());
        assert!(validator.validate_name("double__underscore").is_err());
    }

    #[test]
    fn test_validate_tool() {
        let validator = ToolValidator::new();
        let tool = make_tool(EchoTool);
        let schema = ToolSchema::new("echo".to_string(), "1.0.0".to_string());
        assert!(validator.validate(&tool, &schema).is_ok());
    }

    #[test]
    fn test_validate_output_size() {
        let validator = ToolValidator::new().with_max_output_size(10);
        let schema = ToolSchema::new("test".to_string(), "1.0.0".to_string());

        // Small output should pass
        assert!(validator.validate_output(b"hello", &schema).is_ok());

        // Large output should fail
        assert!(validator.validate_output(&vec![0u8; 100], &schema).is_err());
    }

    #[test]
    fn test_side_effect_tracker() {
        let declared = vec![SideEffect::FsRead {
            path: "/tmp/test".to_string(),
        }];
        let mut tracker = SideEffectTracker::new(declared);

        tracker.record("Read file: /tmp/test".to_string());
        assert!(tracker.check().is_ok());

        tracker.record("Write file: /tmp/other".to_string());
        assert!(tracker.check().is_err());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::InvalidName {
            name: "BadName".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid tool name: BadName");
    }
}
