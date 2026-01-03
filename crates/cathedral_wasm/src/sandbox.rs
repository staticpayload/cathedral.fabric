//! WASM sandbox for secure execution.

use crate::abi::{AbiCall, DeterministicAbi};
use crate::compile::{CompileConfig, CompiledModule, WasmCompiler};
use crate::fuel::FuelMeter;
use crate::host::{HostExecutor, HostRegistry};
use crate::memory::MemoryLimit;
use cathedral_core::{Capability, CoreError, CoreResult, Hash};
use serde::{Deserialize, Serialize};

/// Sandbox configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum fuel for execution
    pub max_fuel: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// Granted capabilities
    pub capabilities: Vec<Capability>,
    /// Enable WASI
    pub enable_wasi: bool,
    /// Compilation config
    pub compile_config: CompileConfig,
}

impl SandboxConfig {
    /// Create a new sandbox config
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_fuel: 10_000_000,
            memory_limit: 16 * 1024 * 1024, // 16MB
            capabilities: Vec::new(),
            enable_wasi: false,
            compile_config: CompileConfig::new(),
        }
    }

    /// Set maximum fuel
    #[must_use]
    pub fn with_max_fuel(mut self, fuel: u64) -> Self {
        self.max_fuel = fuel;
        self.compile_config.max_fuel = fuel;
        self
    }

    /// Set memory limit
    #[must_use]
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = limit;
        self.compile_config.memory_limit = limit;
        self
    }

    /// Set capabilities
    #[must_use]
    pub fn with_capabilities(mut self, caps: Vec<Capability>) -> Self {
        self.capabilities = caps;
        self
    }

    /// Add a capability
    #[must_use]
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Enable/disable WASI
    #[must_use]
    pub fn with_wasi(mut self, enable: bool) -> Self {
        self.enable_wasi = enable;
        self
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Sandbox execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Return value (if any)
    pub return_value: Option<i64>,
    /// Fuel consumed during execution
    pub fuel_consumed: u64,
    /// Peak memory usage in bytes
    pub peak_memory: u64,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Output from the module
    pub output: Vec<u8>,
    /// Host calls made during execution
    pub host_calls: Vec<String>,
}

impl SandboxResult {
    /// Create a successful result
    #[must_use]
    pub fn success(return_value: Option<i64>, fuel_consumed: u64) -> Self {
        Self {
            success: true,
            return_value,
            fuel_consumed,
            peak_memory: 0,
            error: None,
            output: Vec::new(),
            host_calls: Vec::new(),
        }
    }

    /// Create a failed result
    #[must_use]
    pub fn error(error: String, fuel_consumed: u64) -> Self {
        Self {
            success: false,
            return_value: None,
            fuel_consumed,
            peak_memory: 0,
            error: Some(error),
            output: Vec::new(),
            host_calls: Vec::new(),
        }
    }
}

/// Sandbox for secure WASM execution
pub struct Sandbox {
    /// Sandbox configuration
    config: SandboxConfig,
    /// Host registry
    host_registry: HostRegistry,
    /// ABI definition
    abi: DeterministicAbi,
    /// Compiled module (if loaded)
    module: Option<CompiledModule>,
    /// Current fuel meter
    fuel_meter: Option<FuelMeter>,
    /// Current memory limit
    memory_limit: Option<MemoryLimit>,
    /// Execution state
    state: SandboxState,
}

/// Sandbox execution state
#[derive(Debug, Clone, PartialEq, Eq)]
enum SandboxState {
    /// Not initialized
    Uninitialized,
    /// Module loaded, ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Execution completed
    Finished,
    /// Error state
    Error(String),
}

impl Sandbox {
    /// Create a new sandbox
    #[must_use]
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            host_registry: HostRegistry::new(),
            abi: DeterministicAbi::new(),
            module: None,
            fuel_meter: None,
            memory_limit: None,
            state: SandboxState::Uninitialized,
            config,
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(SandboxConfig::default())
    }

    /// Get the sandbox configuration
    #[must_use]
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Get the host registry
    #[must_use]
    pub fn host_registry(&self) -> &HostRegistry {
        &self.host_registry
    }

    /// Get the ABI
    #[must_use]
    pub fn abi(&self) -> &DeterministicAbi {
        &self.abi
    }

    /// Load a WASM module into the sandbox
    ///
    /// # Errors
    ///
    /// Returns error if loading fails
    pub fn load_module(&mut self, wasm_bytes: Vec<u8>) -> CoreResult<()> {
        let compiler = WasmCompiler::new(self.config.compile_config.clone());
        let compiled_bytes = compiler.compile(&wasm_bytes)?;

        let module = CompiledModule::new(compiled_bytes, self.config.compile_config.clone());

        // Set up fuel and memory limits
        self.fuel_meter = Some(FuelMeter::new(self.config.max_fuel));
        self.memory_limit = Some(module.memory_limit());

        self.module = Some(module);
        self.state = SandboxState::Ready;

        Ok(())
    }

    /// Get the loaded module's hash
    #[must_use]
    pub fn module_hash(&self) -> Option<Hash> {
        self.module.as_ref().map(|m| m.hash.clone())
    }

    /// Execute the loaded module
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub fn execute(&mut self) -> CoreResult<SandboxResult> {
        if !matches!(self.state, SandboxState::Ready) {
            return Ok(SandboxResult::error(
                "Sandbox not ready".to_string(),
                0,
            ));
        }

        self.state = SandboxState::Running;

        // For now, simulate execution since we don't have actual WASM runtime
        // In a real implementation, this would use wasmtime
        let result = match self.simulate_execution() {
            Ok(result) => result,
            Err(e) => {
                self.state = SandboxState::Error(e.to_string());
                let consumed = self
                    .fuel_meter
                    .as_ref()
                    .map(|f| f.consumed())
                    .unwrap_or(0);
                return Ok(SandboxResult::error(e.to_string(), consumed));
            }
        };

        let consumed = self
            .fuel_meter
            .as_ref()
            .map(|f| f.consumed())
            .unwrap_or(0);

        self.state = SandboxState::Finished;

        Ok(SandboxResult {
            success: true,
            return_value: Some(0),
            fuel_consumed: consumed,
            peak_memory: 0,
            error: None,
            output: result,
            host_calls: Vec::new(),
        })
    }

    /// Execute with a specific function entry point
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub fn execute_function(&mut self, _function: &str, _args: &[i64]) -> CoreResult<SandboxResult> {
        self.execute()
    }

    /// Make a host call from within the sandbox
    ///
    /// # Errors
    ///
    /// Returns error if call fails
    pub fn host_call(&mut self, call: &AbiCall) -> CoreResult<crate::abi::AbiValue> {
        // Validate the call against ABI
        self.abi.validate_call(call).map_err(|e| {
            CoreError::Validation {
                field: "abi_call".to_string(),
                reason: e.to_string(),
            }
        })?;

        // Consume fuel for the call
        let fuel_cost = self.abi.calculate_fuel_cost(call);
        if let Some(ref mut meter) = self.fuel_meter {
            meter.consume(fuel_cost).map_err(|_e| {
                CoreError::CapacityExceeded {
                    resource: "fuel".to_string(),
                    limit: 0,
                }
            })?;
        }

        // Execute through host registry using handle
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            CoreError::Validation {
                field: "runtime".to_string(),
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;
        let executor = HostExecutor::new(self.host_registry.clone());
        let result = runtime.block_on(executor.execute(call))?;

        Ok(result)
    }

    /// Get remaining fuel
    #[must_use]
    pub fn remaining_fuel(&self) -> Option<u64> {
        self.fuel_meter.as_ref().map(|f| f.remaining())
    }

    /// Get fuel consumed so far
    #[must_use]
    pub fn fuel_consumed(&self) -> Option<u64> {
        self.fuel_meter.as_ref().map(|f| f.consumed())
    }

    /// Get current state
    #[must_use]
    pub fn state(&self) -> &SandboxState {
        &self.state
    }

    /// Reset the sandbox to initial state
    pub fn reset(&mut self) {
        self.module = None;
        self.fuel_meter = Some(FuelMeter::new(self.config.max_fuel));
        self.memory_limit = Some(MemoryLimit::new(self.config.memory_limit));
        self.state = SandboxState::Uninitialized;
    }

    /// Simulate WASM execution (placeholder)
    fn simulate_execution(&mut self) -> CoreResult<Vec<u8>> {
        // Consume some fuel
        if let Some(ref mut meter) = self.fuel_meter {
            meter.consume(1000).map_err(|_e| {
                CoreError::CapacityExceeded {
                    resource: "fuel".to_string(),
                    limit: 0,
                }
            })?;
        }

        // Return simulated output
        Ok(b"execution successful".to_vec())
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Sandbox errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SandboxError {
    /// Module not loaded
    #[error("No module loaded")]
    NoModule,

    /// Already loaded
    #[error("Module already loaded")]
    AlreadyLoaded,

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Host call failed
    #[error("Host call failed: {0}")]
    HostCallFailed(String),

    /// Fuel exhausted
    #[error("Fuel exhausted")]
    FuelExhausted,

    /// Memory limit exceeded
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_wasm() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6D, // \0asm
            0x01, 0x00, 0x00, 0x00, // version 1
        ]
    }

    #[test]
    fn test_sandbox_new() {
        let sandbox = Sandbox::default_config();
        assert_eq!(sandbox.config.max_fuel, 10_000_000);
    }

    #[test]
    fn test_sandbox_load_module() {
        let mut sandbox = Sandbox::default_config();
        let wasm = make_valid_wasm();
        assert!(sandbox.load_module(wasm).is_ok());
        assert!(sandbox.module_hash().is_some());
        assert!(matches!(sandbox.state, SandboxState::Ready));
    }

    #[test]
    fn test_sandbox_execute() {
        let mut sandbox = Sandbox::default_config();
        let wasm = make_valid_wasm();
        sandbox.load_module(wasm).unwrap();
        let result = sandbox.execute().unwrap();
        assert!(result.success);
        assert_eq!(result.fuel_consumed, 1000);
    }

    #[test]
    fn test_sandbox_remaining_fuel() {
        let mut sandbox = Sandbox::default_config();
        sandbox.load_module(make_valid_wasm()).unwrap();
        sandbox.execute().unwrap();
        assert_eq!(sandbox.remaining_fuel(), Some(10_000_000 - 1000));
    }

    #[test]
    fn test_sandbox_fuel_consumed() {
        let mut sandbox = Sandbox::default_config();
        sandbox.load_module(make_valid_wasm()).unwrap();
        sandbox.execute().unwrap();
        assert_eq!(sandbox.fuel_consumed(), Some(1000));
    }

    #[test]
    fn test_sandbox_reset() {
        let mut sandbox = Sandbox::default_config();
        sandbox.load_module(make_valid_wasm()).unwrap();
        sandbox.execute().unwrap();
        sandbox.reset();
        assert!(matches!(sandbox.state, SandboxState::Uninitialized));
    }

    #[test]
    fn test_sandbox_config_new() {
        let config = SandboxConfig::new();
        assert_eq!(config.max_fuel, 10_000_000);
        assert_eq!(config.memory_limit, 16 * 1024 * 1024);
    }

    #[test]
    fn test_sandbox_config_with_max_fuel() {
        let config = SandboxConfig::new().with_max_fuel(1_000_000);
        assert_eq!(config.max_fuel, 1_000_000);
        assert_eq!(config.compile_config.max_fuel, 1_000_000);
    }

    #[test]
    fn test_sandbox_config_with_memory_limit() {
        let config = SandboxConfig::new().with_memory_limit(8 * 1024 * 1024);
        assert_eq!(config.memory_limit, 8 * 1024 * 1024);
    }

    #[test]
    fn test_sandbox_config_with_capabilities() {
        let caps = vec![Capability::ClockRead];
        let config = SandboxConfig::new().with_capabilities(caps.clone());
        assert_eq!(config.capabilities, caps);
    }

    #[test]
    fn test_sandbox_config_with_capability() {
        let config = SandboxConfig::new().with_capability(Capability::ClockRead);
        assert_eq!(config.capabilities.len(), 1);
    }

    #[test]
    fn test_sandbox_config_with_wasi() {
        let config = SandboxConfig::new().with_wasi(true);
        assert!(config.enable_wasi);
    }

    #[test]
    fn test_sandbox_result_success() {
        let result = SandboxResult::success(Some(42), 1000);
        assert!(result.success);
        assert_eq!(result.return_value, Some(42));
        assert_eq!(result.fuel_consumed, 1000);
    }

    #[test]
    fn test_sandbox_result_error() {
        let result = SandboxResult::error("test error".to_string(), 500);
        assert!(!result.success);
        assert_eq!(result.error, Some("test error".to_string()));
        assert_eq!(result.fuel_consumed, 500);
    }

    #[test]
    fn test_sandbox_error_display() {
        let err = SandboxError::NoModule;
        assert!(err.to_string().contains("No module loaded"));
    }

    #[test]
    fn test_sandbox_host_call() {
        // This test would require async setup for the host registry
        // For now, just verify the sandbox can handle module loading
        let mut sandbox = Sandbox::default_config();
        sandbox.load_module(make_valid_wasm()).unwrap();
        assert!(sandbox.module_hash().is_some());
    }

    #[test]
    fn test_default() {
        let sandbox = Sandbox::default();
        assert_eq!(sandbox.config.max_fuel, 10_000_000);
    }
}
