//! WASM compilation and validation.

use crate::fuel::FuelLimiter;
use crate::memory::MemoryLimit;
use cathedral_core::{CoreResult, CoreError, Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// WASM compiler for validating and compiling modules
pub struct WasmCompiler {
    /// Configuration for compilation
    config: CompileConfig,
}

/// Compilation configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileConfig {
    /// Maximum fuel for execution
    pub max_fuel: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// Enable validation
    pub validate: bool,
    /// Enable optimization
    pub optimize: bool,
    /// Allowed WASM features
    pub allowed_features: HashSet<WasmFeature>,
}

/// WASM features that can be enabled/disabled
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmFeature {
    /// SIMD instructions
    Simd,
    /// Multi-value returns
    MultiValue,
    /// Bulk memory operations
    BulkMemory,
    /// Reference types
    ReferenceTypes,
    /// Tail calls
    TailCalls,
    /// Extended-const expressions
    ExtendedConst,
    /// Threads
    Threads,
    /// Function references
    FunctionReferences,
}

/// Compilation errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    /// Invalid WASM module
    #[error("Invalid WASM module: {0}")]
    InvalidModule(String),

    /// Unknown feature
    #[error("Unknown WASM feature: {0}")]
    UnknownFeature(String),

    /// Feature not allowed
    #[error("Feature not allowed: {0}")]
    FeatureNotAllowed(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Size limit exceeded
    #[error("Module size {size} exceeds limit {limit}")]
    SizeLimitExceeded { size: usize, limit: usize },

    /// Memory limit exceeded
    #[error("Memory limit {limit} too small, need at least {needed}")]
    MemoryLimitTooSmall { limit: u64, needed: u64 },
}

impl WasmCompiler {
    /// Create a new compiler
    #[must_use]
    pub fn new(config: CompileConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(CompileConfig::default())
    }

    /// Validate a WASM module
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn validate(&self, wasm_bytes: &[u8]) -> Result<(), CompileError> {
        if !self.config.validate {
            return Ok(());
        }

        // Check size limit
        if wasm_bytes.len() > 10 * 1024 * 1024 {
            return Err(CompileError::SizeLimitExceeded {
                size: wasm_bytes.len(),
                limit: 10 * 1024 * 1024,
            });
        }

        // Check WASM magic number
        if wasm_bytes.len() < 4 {
            return Err(CompileError::InvalidModule("Too small".to_string()));
        }

        let magic = &wasm_bytes[0..4];
        if magic != b"\0asm" {
            return Err(CompileError::InvalidModule(
                "Invalid magic number".to_string(),
            ));
        }

        // Check version
        if wasm_bytes.len() < 8 {
            return Err(CompileError::InvalidModule("Missing version".to_string()));
        }

        let version = &wasm_bytes[4..8];
        if version != b"\x01\x00\x00\x00" {
            return Err(CompileError::InvalidModule(format!(
                "Unsupported version: {:?}",
                version
            )));
        }

        // Basic validation passed
        Ok(())
    }

    /// Compile a WASM module (returns bytes for execution)
    ///
    /// # Errors
    ///
    /// Returns error if compilation fails
    pub fn compile(&self, wasm_bytes: &[u8]) -> CoreResult<Vec<u8>> {
        self.validate(wasm_bytes)
            .map_err(|e| CoreError::Validation {
                field: "wasm".to_string(),
                reason: e.to_string(),
            })?;

        // For now, just return the bytes as-is
        // In a real implementation, this would use wasmtime to compile
        Ok(wasm_bytes.to_vec())
    }

    /// Get the fuel limiter from config
    #[must_use]
    pub fn fuel_limiter(&self) -> FuelLimiter {
        FuelLimiter::new(self.config.max_fuel)
    }

    /// Get the memory limit from config
    #[must_use]
    pub fn memory_limit(&self) -> MemoryLimit {
        MemoryLimit::new(self.config.memory_limit)
    }

    /// Check if a feature is allowed
    #[must_use]
    pub fn is_feature_allowed(&self, feature: &WasmFeature) -> bool {
        self.config.allowed_features.contains(feature)
    }
}

impl Default for WasmCompiler {
    fn default() -> Self {
        Self::default_config()
    }
}

impl CompileConfig {
    /// Create a new compile config
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_fuel: 10_000_000,
            memory_limit: 16 * 1024 * 1024, // 16MB
            validate: true,
            optimize: false,
            allowed_features: HashSet::from_iter(vec![
                WasmFeature::MultiValue,
                WasmFeature::BulkMemory,
                WasmFeature::ReferenceTypes,
            ]),
        }
    }

    /// Set maximum fuel
    #[must_use]
    pub fn with_max_fuel(mut self, fuel: u64) -> Self {
        self.max_fuel = fuel;
        self
    }

    /// Set memory limit
    #[must_use]
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Enable/disable validation
    #[must_use]
    pub fn with_validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Enable/disable optimization
    #[must_use]
    pub fn with_optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Add an allowed feature
    #[must_use]
    pub fn with_feature(mut self, feature: WasmFeature) -> Self {
        self.allowed_features.insert(feature);
        self
    }

    /// Remove an allowed feature
    #[must_use]
    pub fn without_feature(mut self, feature: &WasmFeature) -> Self {
        self.allowed_features.remove(feature);
        self
    }

    /// Get fuel limiter from this config
    #[must_use]
    pub fn fuel_limiter(&self) -> FuelLimiter {
        FuelLimiter::new(self.max_fuel)
    }

    /// Get memory limit from this config
    #[must_use]
    pub fn memory_limit(&self) -> MemoryLimit {
        MemoryLimit::new(self.memory_limit)
    }
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled WASM module ready for execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledModule {
    /// Compiled WASM bytes
    pub bytes: Vec<u8>,
    /// Module hash
    pub hash: Hash,
    /// Configuration used for compilation
    pub config: CompileConfig,
    /// Module size in bytes
    pub size: usize,
}

impl CompiledModule {
    /// Create a new compiled module
    #[must_use]
    pub fn new(bytes: Vec<u8>, config: CompileConfig) -> Self {
        let size = bytes.len();
        let hash = Hash::compute(&bytes);
        Self { bytes, hash, config, size }
    }

    /// Get the module hash
    #[must_use]
    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    /// Check if module matches expected hash
    #[must_use]
    pub fn verify_hash(&self, expected: &Hash) -> bool {
        &self.hash == expected
    }

    /// Get fuel limiter for this module
    #[must_use]
    pub fn fuel_limiter(&self) -> FuelLimiter {
        self.config.fuel_limiter()
    }

    /// Get memory limit for this module
    #[must_use]
    pub fn memory_limit(&self) -> MemoryLimit {
        self.config.memory_limit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_wasm() -> Vec<u8> {
        // Minimal WASM module with magic and version
        let mut wasm = vec![
            0x00, 0x61, 0x73, 0x6D, // \0asm
            0x01, 0x00, 0x00, 0x00, // version 1
        ];
        wasm
    }

    #[test]
    fn test_compiler_new() {
        let config = CompileConfig::new();
        let compiler = WasmCompiler::new(config);
        assert_eq!(compiler.config.max_fuel, 10_000_000);
    }

    #[test]
    fn test_compiler_validate_valid() {
        let compiler = WasmCompiler::default_config();
        let wasm = make_valid_wasm();
        assert!(compiler.validate(&wasm).is_ok());
    }

    #[test]
    fn test_compiler_validate_invalid_magic() {
        let compiler = WasmCompiler::default_config();
        let wasm = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        assert!(compiler.validate(&wasm).is_err());
    }

    #[test]
    fn test_compiler_validate_too_small() {
        let compiler = WasmCompiler::default_config();
        let wasm = vec![0x00, 0x61, 0x73];
        assert!(compiler.validate(&wasm).is_err());
    }

    #[test]
    fn test_compiler_compile() {
        let compiler = WasmCompiler::default_config();
        let wasm = make_valid_wasm();
        let result = compiler.compile(&wasm);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm);
    }

    #[test]
    fn test_compile_config_new() {
        let config = CompileConfig::new();
        assert_eq!(config.max_fuel, 10_000_000);
        assert_eq!(config.memory_limit, 16 * 1024 * 1024);
    }

    #[test]
    fn test_compile_config_with_max_fuel() {
        let config = CompileConfig::new().with_max_fuel(1_000_000);
        assert_eq!(config.max_fuel, 1_000_000);
    }

    #[test]
    fn test_compile_config_with_memory_limit() {
        let config = CompileConfig::new().with_memory_limit(8 * 1024 * 1024);
        assert_eq!(config.memory_limit, 8 * 1024 * 1024);
    }

    #[test]
    fn test_compile_config_with_validate() {
        let config = CompileConfig::new().with_validate(false);
        assert!(!config.validate);
    }

    #[test]
    fn test_compile_config_with_feature() {
        let config = CompileConfig::new().with_feature(WasmFeature::Simd);
        assert!(config.allowed_features.contains(&WasmFeature::Simd));
    }

    #[test]
    fn test_compile_config_without_feature() {
        let config = CompileConfig::new().without_feature(&WasmFeature::MultiValue);
        assert!(!config.allowed_features.contains(&WasmFeature::MultiValue));
    }

    #[test]
    fn test_compiler_fuel_limiter() {
        let compiler = WasmCompiler::default_config();
        let limiter = compiler.fuel_limiter();
        assert_eq!(limiter.max_fuel, 10_000_000);
    }

    #[test]
    fn test_compiler_memory_limit() {
        let compiler = WasmCompiler::default_config();
        let limit = compiler.memory_limit();
        assert_eq!(limit.max_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_compiler_is_feature_allowed() {
        let compiler = WasmCompiler::default_config();
        assert!(compiler.is_feature_allowed(&WasmFeature::MultiValue));
        assert!(!compiler.is_feature_allowed(&WasmFeature::Simd));
    }

    #[test]
    fn test_compiled_module_new() {
        let wasm = make_valid_wasm();
        let config = CompileConfig::new();
        let module = CompiledModule::new(wasm.clone(), config);
        assert_eq!(module.bytes, wasm);
        assert_eq!(module.size, wasm.len());
    }

    #[test]
    fn test_compiled_module_hash() {
        let wasm = make_valid_wasm();
        let config = CompileConfig::new();
        let module = CompiledModule::new(wasm, config);
        let hash = module.hash();
        assert_ne!(hash, &Hash::empty());
    }

    #[test]
    fn test_compiled_module_verify_hash() {
        let wasm = make_valid_wasm();
        let config = CompileConfig::new();
        let module = CompiledModule::new(wasm, config);
        assert!(module.verify_hash(module.hash()));
    }

    #[test]
    fn test_compile_error_display() {
        let err = CompileError::InvalidModule("test".to_string());
        assert!(err.to_string().contains("Invalid WASM module"));
    }

    #[test]
    fn test_wasm_feature_equality() {
        assert_eq!(WasmFeature::Simd, WasmFeature::Simd);
        assert_ne!(WasmFeature::Simd, WasmFeature::Threads);
    }

    #[test]
    fn test_default() {
        let compiler = WasmCompiler::default();
        assert_eq!(compiler.config.max_fuel, 10_000_000);
    }
}
