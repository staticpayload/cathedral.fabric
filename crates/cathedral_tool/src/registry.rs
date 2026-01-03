//! Tool registry for dynamic tool discovery and management.

use cathedral_core::Capability;
use indexmap::IndexMap;
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};
use crate::trait_::{Tool, ToolError};
use crate::schema::ToolSchema;

/// Error from registry operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Tool already registered
    AlreadyRegistered { name: String },
    /// Tool not found
    NotFound { name: String },
    /// Version conflict
    VersionConflict { name: String, existing: String, new: String },
    /// Schema mismatch
    SchemaMismatch { reason: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRegistered { name } => write!(f, "Tool already registered: {}", name),
            Self::NotFound { name } => write!(f, "Tool not found: {}", name),
            Self::VersionConflict { name, existing, new } => {
                write!(
                    f,
                    "Version conflict for {}: existing {}, new {}",
                    name, existing, new
                )
            }
            Self::SchemaMismatch { reason } => write!(f, "Schema mismatch: {}", reason),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Entry for a registered tool
#[derive(Clone)]
pub struct ToolEntry {
    /// Tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// The tool itself
    pub tool: Arc<dyn Tool>,
    /// Tool schema
    pub schema: ToolSchema,
    /// Required capabilities
    pub capabilities: BTreeSet<Capability>,
    /// Whether tool is enabled
    pub enabled: bool,
}

impl ToolEntry {
    /// Create a new tool entry
    #[must_use]
    pub fn new(tool: Arc<dyn Tool>, schema: ToolSchema) -> Self {
        Self {
            name: tool.name().to_string(),
            version: tool.version().to_string(),
            tool,
            schema,
            capabilities: BTreeSet::new(),
            enabled: true,
        }
    }

    /// Create with capabilities
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: BTreeSet<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Check if tool has required capabilities
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }
}

/// Registry for tools
///
/// The registry provides dynamic tool discovery and lazy loading.
/// Tools are assumed to be potentially hostile.
pub struct ToolRegistry {
    /// Registered tools by name
    tools: IndexMap<String, ToolEntry>,
}

impl ToolRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: IndexMap::new(),
        }
    }

    /// Register a tool
    ///
    /// # Errors
    ///
    /// Returns error if tool already registered or version conflict
    pub fn register(
        &mut self,
        tool: Arc<dyn Tool>,
        schema: ToolSchema,
    ) -> Result<(), RegistryError> {
        let name = tool.name().to_string();
        let version = tool.version().to_string();

        // Check for existing tool
        if let Some(existing) = self.tools.get(&name) {
            if existing.version != version {
                return Err(RegistryError::VersionConflict {
                    name,
                    existing: existing.version.clone(),
                    new: version,
                });
            }
            return Err(RegistryError::AlreadyRegistered { name });
        }

        let entry = ToolEntry::new(tool, schema);
        self.tools.insert(name, entry);
        Ok(())
    }

    /// Get a tool by name
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn get(&self, name: &str) -> Result<Arc<dyn Tool>, ToolError> {
        self.tools
            .get(name)
            .filter(|e| e.enabled)
            .map(|e| Arc::clone(&e.tool))
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_string(),
            })
    }

    /// Get tool entry by name
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn get_entry(&self, name: &str) -> Result<ToolEntry, ToolError> {
        self.tools
            .get(name)
            .filter(|e| e.enabled)
            .cloned()
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_string(),
            })
    }

    /// List all registered tool names
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter(|(_, e)| e.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List tools by capability
    #[must_use]
    pub fn list_by_capability(&self, capability: &Capability) -> Vec<String> {
        self.tools
            .iter()
            .filter(|(_, e)| e.enabled && e.has_capability(capability))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Check if a tool is registered
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .map(|e| e.enabled)
            .unwrap_or(false)
    }

    /// Enable a tool
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn enable(&mut self, name: &str) -> Result<(), ToolError> {
        self.tools
            .get_mut(name)
            .map(|e| e.enabled = true)
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_string(),
            })
    }

    /// Disable a tool
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn disable(&mut self, name: &str) -> Result<(), ToolError> {
        self.tools
            .get_mut(name)
            .map(|e| e.enabled = false)
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_string(),
            })
    }

    /// Unregister a tool
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn unregister(&mut self, name: &str) -> Result<(), ToolError> {
        self.tools
            .shift_remove(name)
            .map(|_| ())
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_string(),
            })
    }

    /// Get the count of registered tools
    #[must_use]
    pub fn count(&self) -> usize {
        self.tools.iter().filter(|(_, e)| e.enabled).count()
    }

    /// Check if registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe registry for concurrent access
pub struct SharedRegistry {
    inner: RwLock<ToolRegistry>,
}

impl SharedRegistry {
    /// Create a new shared registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(ToolRegistry::new()),
        }
    }

    /// Register a tool
    ///
    /// # Errors
    ///
    /// Returns error if tool already registered
    pub fn register(
        &self,
        tool: Arc<dyn Tool>,
        schema: ToolSchema,
    ) -> Result<(), RegistryError> {
        let mut registry = self.inner.write().unwrap();
        registry.register(tool, schema)
    }

    /// Get a tool by name
    ///
    /// # Errors
    ///
    /// Returns error if tool not found
    pub fn get(&self, name: &str) -> Result<Arc<dyn Tool>, ToolError> {
        let registry = self.inner.read().unwrap();
        registry.get(name)
    }

    /// List all registered tool names
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        let registry = self.inner.read().unwrap();
        registry.list()
    }
}

impl Default for SharedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct DummyTool {
        name: String,
    }

    impl Tool for DummyTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn execute(&self, _input: &[u8]) -> cathedral_core::CoreResult<crate::trait_::ToolOutput> {
            Ok(crate::trait_::ToolOutput::success(b"ok".to_vec()))
        }
    }

    fn make_tool(name: &str) -> Arc<dyn Tool> {
        Arc::new(DummyTool {
            name: name.to_string(),
        })
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        let result = registry.register(tool, schema);
        assert!(result.is_ok());
        assert_eq!(registry.count(), 1);
        assert!(registry.contains("test_tool"));
    }

    #[test]
    fn test_registry_register_duplicate() {
        let mut registry = ToolRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        let tool2 = make_tool("test_tool");
        registry.register(tool, schema.clone()).unwrap();
        let result = registry.register(tool2, schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        registry.register(tool, schema).unwrap();
        let result = registry.get("test_tool");
        assert!(result.is_ok());
    }

    #[test]
    fn test_registry_get_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_disable_enable() {
        let mut registry = ToolRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        registry.register(tool, schema).unwrap();
        assert!(registry.contains("test_tool"));

        registry.disable("test_tool").unwrap();
        assert!(!registry.contains("test_tool"));

        registry.enable("test_tool").unwrap();
        assert!(registry.contains("test_tool"));
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = ToolRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        registry.register(tool, schema).unwrap();
        registry.unregister("test_tool").unwrap();
        assert!(!registry.contains("test_tool"));
    }

    #[test]
    fn test_shared_registry() {
        let shared = SharedRegistry::new();
        let tool = make_tool("test_tool");
        let schema = ToolSchema::new("test_tool".to_string(), "1.0.0".to_string());

        shared.register(tool, schema).unwrap();
        let list = shared.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "test_tool");
    }
}
