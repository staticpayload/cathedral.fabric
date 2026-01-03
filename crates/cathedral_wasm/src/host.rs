//! Host functions for WASM guest execution.

use crate::abi::{AbiCall, AbiValue};
use crate::fuel::FuelMeter;
use crate::memory::MemoryLimit;
use cathedral_core::{Capability, CoreResult, EventId, NodeId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Host function that can be called from WASM
pub type HostFn = Arc<dyn Fn(&[AbiValue], &mut HostContext) -> CoreResult<AbiValue> + Send + Sync>;

/// Context for host function execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostContext {
    /// Node ID making the call
    pub node_id: Option<NodeId>,
    /// Event ID for the call
    pub event_id: Option<EventId>,
    /// Timestamp of the call
    pub timestamp: u64,
    /// Available capabilities
    pub capabilities: Vec<Capability>,
    /// Memory limit for the guest
    pub memory_limit: MemoryLimit,
    /// Fuel meter for tracking execution cost
    pub fuel_meter: Option<FuelMeter>,
}

impl HostContext {
    /// Create a new host context
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_id: None,
            event_id: None,
            timestamp: 0,
            capabilities: Vec::new(),
            memory_limit: MemoryLimit::default(),
            fuel_meter: None,
        }
    }

    /// Set node ID
    #[must_use]
    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Set event ID
    #[must_use]
    pub fn with_event(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
        self
    }

    /// Set timestamp
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Set capabilities
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set memory limit
    #[must_use]
    pub fn with_memory_limit(mut self, limit: MemoryLimit) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Set fuel meter
    #[must_use]
    pub fn with_fuel_meter(mut self, meter: FuelMeter) -> Self {
        self.fuel_meter = Some(meter);
        self
    }

    /// Check if a capability is granted
    #[must_use]
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Consume fuel if meter is present
    ///
    /// # Errors
    ///
    /// Returns error if out of fuel
    pub fn consume_fuel(&mut self, amount: u64) -> Result<(), crate::fuel::FuelError> {
        if let Some(ref mut meter) = self.fuel_meter {
            meter.consume(amount)?;
        }
        Ok(())
    }
}

impl Default for HostContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Host function definition
#[derive(Clone)]
pub struct HostFunction {
    /// Function name
    pub name: String,
    /// Whether this function is async
    pub is_async: bool,
    /// Required capabilities
    pub required_capabilities: Vec<Capability>,
    /// Fuel cost to call
    pub fuel_cost: u64,
    /// The function implementation
    implementation: HostFn,
}

impl HostFunction {
    /// Create a new host function
    #[must_use]
    pub fn new(
        name: String,
        required_capabilities: Vec<Capability>,
        fuel_cost: u64,
        implementation: HostFn,
    ) -> Self {
        Self {
            name,
            is_async: false,
            required_capabilities,
            fuel_cost,
            implementation,
        }
    }

    /// Create an async host function
    #[must_use]
    pub fn async_fn(
        name: String,
        required_capabilities: Vec<Capability>,
        fuel_cost: u64,
        implementation: HostFn,
    ) -> Self {
        Self {
            name,
            is_async: true,
            required_capabilities,
            fuel_cost,
            implementation,
        }
    }

    /// Call the host function
    ///
    /// # Errors
    ///
    /// Returns error if call fails
    pub fn call(&self, args: &[AbiValue], ctx: &mut HostContext) -> CoreResult<AbiValue> {
        // Check capabilities
        for cap in &self.required_capabilities {
            if !ctx.has_capability(cap) {
                return Err(cathedral_core::CoreError::InvalidCapability {
                    reason: format!("Missing capability: {:?}", cap),
                });
            }
        }

        // Consume fuel
        ctx.consume_fuel(self.fuel_cost)
            .map_err(|_e| cathedral_core::CoreError::CapacityExceeded {
                resource: "fuel".to_string(),
                limit: 0,
            })?;

        // Call implementation
        (self.implementation)(args, ctx)
    }
}

/// Registry of host functions
#[derive(Clone)]
pub struct HostRegistry {
    /// Registered functions
    functions: Arc<RwLock<HashMap<String, HostFunction>>>,
}

impl HostRegistry {
    /// Create a new host registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            functions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a host function
    pub async fn register(&self, func: HostFunction) {
        let mut functions = self.functions.write().await;
        functions.insert(func.name.clone(), func);
    }

    /// Get a function by name
    pub async fn get(&self, name: &str) -> Option<HostFunction> {
        let functions = self.functions.read().await;
        functions.get(name).cloned()
    }

    /// Check if a function exists
    pub async fn has(&self, name: &str) -> bool {
        let functions = self.functions.read().await;
        functions.contains_key(name)
    }

    /// List all registered function names
    pub async fn list(&self) -> Vec<String> {
        let functions = self.functions.read().await;
        functions.keys().cloned().collect()
    }

    /// Create a registry with standard cathedral host functions
    pub async fn with_standard_functions() -> Self {
        let registry = Self::new();

        // Clock read function
        registry
            .register(HostFunction::new(
                "clock_read".to_string(),
                vec![Capability::ClockRead],
                10,
                Arc::new(|_args, _ctx| Ok(AbiValue::I64(0))),
            ))
            .await;

        // Log write function
        registry
            .register(HostFunction::new(
                "log_write".to_string(),
                vec![],
                50,
                Arc::new(|args, _ctx| {
                    if args.len() >= 1 {
                        if let AbiValue::String(msg) = &args[0] {
                            tracing::debug!("WASM log: {}", msg);
                        }
                    }
                    Ok(AbiValue::I32(0))
                }),
            ))
            .await;

        // Has capability check
        registry
            .register(HostFunction::new(
                "has_capability".to_string(),
                vec![],
                20,
                Arc::new(|args, ctx| {
                    if args.len() >= 1 {
                        if let AbiValue::String(cap_str) = &args[0] {
                            // Simple check for common capabilities
                            let has = ctx.capabilities.iter().any(|c| {
                                format!("{:?}", c).contains(cap_str)
                                    || cap_str == "ClockRead"
                            });
                            return Ok(AbiValue::Bool(has));
                        }
                    }
                    Ok(AbiValue::Bool(false))
                }),
            ))
            .await;

        registry
    }
}

impl Default for HostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Async host function trait
#[async_trait]
pub trait AsyncHostFunction: Send + Sync {
    /// Call the host function asynchronously
    ///
    /// # Errors
    ///
    /// Returns error if call fails
    async fn call_async(
        &self,
        args: Vec<AbiValue>,
        ctx: HostContext,
    ) -> CoreResult<AbiValue>;
}

/// Host executor for managing host function calls
pub struct HostExecutor {
    /// Registry of functions
    registry: HostRegistry,
    /// Default context for calls
    default_context: HostContext,
}

impl HostExecutor {
    /// Create a new host executor
    #[must_use]
    pub fn new(registry: HostRegistry) -> Self {
        Self {
            registry,
            default_context: HostContext::new(),
        }
    }

    /// Create with standard functions
    pub async fn with_standard() -> Self {
        Self::new(HostRegistry::with_standard_functions().await)
    }

    /// Set default context
    #[must_use]
    pub fn with_context(mut self, ctx: HostContext) -> Self {
        self.default_context = ctx;
        self
    }

    /// Execute a host function call
    ///
    /// # Errors
    ///
    /// Returns error if call fails
    pub async fn execute(&self, call: &AbiCall) -> CoreResult<AbiValue> {
        let func = self
            .registry
            .get(&call.function_name)
            .await
            .ok_or_else(|| {
                cathedral_core::CoreError::Validation {
                    field: "function_name".to_string(),
                    reason: format!("Unknown host function: {}", call.function_name),
                }
            })?;

        let mut ctx = self.default_context.clone();
        ctx.node_id = call.context.node_id.clone();
        ctx.event_id = call.context.event_id.clone();
        ctx.timestamp = call.context.timestamp;
        ctx.memory_limit = call.context.memory_limit.clone();

        func.call(&call.args, &mut ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_context_new() {
        let ctx = HostContext::new();
        assert!(ctx.node_id.is_none());
        assert!(ctx.event_id.is_none());
        assert_eq!(ctx.capabilities.len(), 0);
    }

    #[test]
    fn test_host_context_with_node() {
        let node_id = NodeId::new();
        let ctx = HostContext::new().with_node(node_id);
        assert_eq!(ctx.node_id, Some(node_id));
    }

    #[test]
    fn test_host_context_with_capabilities() {
        let caps = vec![Capability::ClockRead];
        let ctx = HostContext::new().with_capabilities(caps.clone());
        assert!(ctx.has_capability(&Capability::ClockRead));
    }

    #[test]
    fn test_host_context_consume_fuel() {
        let mut ctx = HostContext::new().with_fuel_meter(FuelMeter::new(100));
        assert!(ctx.consume_fuel(50).is_ok());
        assert!(ctx.consume_fuel(30).is_ok());
        assert!(ctx.consume_fuel(25).is_err());
    }

    #[tokio::test]
    async fn test_host_registry_register() {
        let registry = HostRegistry::new();
        let func = HostFunction::new(
            "test_func".to_string(),
            vec![],
            10,
            Arc::new(|_args, _ctx| Ok(AbiValue::I32(42))),
        );
        registry.register(func).await;
        assert!(registry.has("test_func").await);
    }

    #[tokio::test]
    async fn test_host_registry_get() {
        let registry = HostRegistry::new();
        let func = HostFunction::new(
            "test_func".to_string(),
            vec![],
            10,
            Arc::new(|_args, _ctx| Ok(AbiValue::I32(42))),
        );
        registry.register(func).await;
        let retrieved = registry.get("test_func").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_host_registry_list() {
        let registry = HostRegistry::with_standard_functions().await;
        let names = registry.list().await;
        assert!(names.contains(&"clock_read".to_string()));
        assert!(names.contains(&"log_write".to_string()));
    }

    #[tokio::test]
    async fn test_host_function_call() {
        let func = HostFunction::new(
            "test_func".to_string(),
            vec![],
            10,
            Arc::new(|_args, _ctx| Ok(AbiValue::I32(42))),
        );
        let mut ctx = HostContext::new();
        let result = func.call(&[], &mut ctx).unwrap();
        assert_eq!(result, AbiValue::I32(42));
    }

    #[tokio::test]
    async fn test_host_function_capability_check() {
        let func = HostFunction::new(
            "test_func".to_string(),
            vec![Capability::ClockRead],
            10,
            Arc::new(|_args, _ctx| Ok(AbiValue::I32(42))),
        );
        let mut ctx = HostContext::new();
        // Should fail without capability
        assert!(func.call(&[], &mut ctx).is_err());
        // Should succeed with capability
        ctx.capabilities.push(Capability::ClockRead);
        assert!(func.call(&[], &mut ctx).is_ok());
    }

    #[tokio::test]
    async fn test_host_executor_execute() {
        let executor = HostExecutor::with_standard()
            .await
            .with_context(HostContext::new().with_capabilities(vec![
                Capability::ClockRead,
            ]));
        let call = AbiCall::clock_read();
        let result = executor.execute(&call).await.unwrap();
        assert!(matches!(result, AbiValue::I64(_)));
    }

    #[test]
    fn test_host_context_default() {
        let ctx = HostContext::default();
        assert!(ctx.node_id.is_none());
    }

    #[tokio::test]
    async fn test_host_registry_default() {
        let registry = HostRegistry::default();
        assert!(!registry.has("test").await);
    }
}
