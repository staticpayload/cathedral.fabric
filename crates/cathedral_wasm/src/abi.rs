//! Deterministic ABI for WASM host-guest communication.

use crate::memory::MemoryLimit;
use cathedral_core::{EventId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deterministic ABI for host-guest calls
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicAbi {
    /// ABI version
    pub version: semver::Version,
    /// Supported function signatures
    pub functions: HashMap<String, AbiSignature>,
}

/// ABI function signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiSignature {
    /// Function name
    pub name: String,
    /// Parameter types
    pub params: Vec<AbiType>,
    /// Return type
    pub returns: AbiType,
    /// Whether this function is deterministic
    pub deterministic: bool,
    /// Fuel cost for calling this function
    pub fuel_cost: u64,
}

/// ABI value types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// Boolean
    Bool,
    /// String (pointer + length)
    String,
    /// Byte array (pointer + length)
    Bytes,
    /// Void/unit (no value)
    Void,
    /// Optional value
    Option(Box<AbiType>),
    /// List of values
    List(Box<AbiType>),
    /// Struct with named fields
    Struct(Vec<(String, AbiType)>),
}

/// ABI call from guest to host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiCall {
    /// Function name being called
    pub function_name: String,
    /// Arguments to the function
    pub args: Vec<AbiValue>,
    /// Caller's context
    pub context: AbiContext,
}

/// ABI context for calls
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiContext {
    /// Node ID making the call
    pub node_id: Option<NodeId>,
    /// Event ID for the call
    pub event_id: Option<EventId>,
    /// Timestamp of the call
    pub timestamp: u64,
    /// Memory limit
    pub memory_limit: MemoryLimit,
}

/// ABI values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiValue {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float (as bits for determinism)
    F32(u32),
    /// 64-bit float (as bits for determinism)
    F64(u64),
    /// Boolean
    Bool(bool),
    /// String
    String(String),
    /// Byte array
    Bytes(Vec<u8>),
    /// Void/unit
    Void,
    /// Optional value
    Option(Box<Option<AbiValue>>),
    /// List of values
    List(Vec<AbiValue>),
    /// Struct with fields
    Struct(Vec<(String, AbiValue)>),
}

/// ABI errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AbiError {
    /// Unknown function
    #[error("Unknown ABI function: {0}")]
    UnknownFunction(String),

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Invalid argument
    #[error("Invalid argument at position {position}: {reason}")]
    InvalidArgument { position: usize, reason: String },

    /// Buffer too small
    #[error("Buffer too small: {size} bytes available, {required} required")]
    BufferTooSmall { size: usize, required: usize },

    /// Invalid pointer
    #[error("Invalid pointer: 0x{pointer:X}")]
    InvalidPointer { pointer: u64 },

    /// Determinism violation
    #[error("Determinism violation: {0}")]
    DeterminismViolation(String),

    /// Out of fuel
    #[error("Out of fuel during ABI call")]
    OutOfFuel,
}

impl DeterministicAbi {
    /// Create a new ABI with standard cathedral functions
    #[must_use]
    pub fn new() -> Self {
        let mut functions = HashMap::new();

        // Clock functions (deterministic in cathedral)
        functions.insert(
            "clock_read".to_string(),
            AbiSignature {
                name: "clock_read".to_string(),
                params: vec![],
                returns: AbiType::I64,
                deterministic: true,
                fuel_cost: 10,
            },
        );

        // Logging functions
        functions.insert(
            "log_write".to_string(),
            AbiSignature {
                name: "log_write".to_string(),
                params: vec![AbiType::String, AbiType::I32],
                returns: AbiType::I32,
                deterministic: true,
                fuel_cost: 50,
            },
        );

        // Capability checks
        functions.insert(
            "has_capability".to_string(),
            AbiSignature {
                name: "has_capability".to_string(),
                params: vec![AbiType::String],
                returns: AbiType::Bool,
                deterministic: true,
                fuel_cost: 20,
            },
        );

        // File system operations (deterministic with proper sandboxing)
        functions.insert(
            "fs_read".to_string(),
            AbiSignature {
                name: "fs_read".to_string(),
                params: vec![AbiType::String, AbiType::I32],
                returns: AbiType::Bytes,
                deterministic: true,
                fuel_cost: 100,
            },
        );

        functions.insert(
            "fs_write".to_string(),
            AbiSignature {
                name: "fs_write".to_string(),
                params: vec![AbiType::String, AbiType::Bytes],
                returns: AbiType::I32,
                deterministic: true,
                fuel_cost: 100,
            },
        );

        // Network operations (deterministic with mocking)
        functions.insert(
            "net_http".to_string(),
            AbiSignature {
                name: "net_http".to_string(),
                params: vec![AbiType::String, AbiType::String],
                returns: AbiType::Bytes,
                deterministic: true,
                fuel_cost: 500,
            },
        );

        Self {
            version: semver::Version::new(0, 1, 0),
            functions,
        }
    }

    /// Get function signature
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<&AbiSignature> {
        self.functions.get(name)
    }

    /// Validate a call against the ABI
    ///
    /// # Errors
    ///
    /// Returns error if call is invalid
    pub fn validate_call(&self, call: &AbiCall) -> Result<(), AbiError> {
        let sig = self
            .functions
            .get(&call.function_name)
            .ok_or_else(|| AbiError::UnknownFunction(call.function_name.clone()))?;

        if sig.params.len() != call.args.len() {
            return Err(AbiError::InvalidArgument {
                position: call.args.len(),
                reason: format!(
                    "Expected {} arguments, got {}",
                    sig.params.len(),
                    call.args.len()
                ),
            });
        }

        for (_i, (param_type, arg)) in sig.params.iter().zip(call.args.iter()).enumerate() {
            if !Self::types_compatible(param_type, arg) {
                return Err(AbiError::TypeMismatch {
                    expected: format!("{:?}", param_type),
                    actual: format!("{:?}", arg),
                });
            }
        }

        Ok(())
    }

    /// Calculate fuel cost for a call
    #[must_use]
    pub fn calculate_fuel_cost(&self, call: &AbiCall) -> u64 {
        self.functions
            .get(&call.function_name)
            .map(|sig| sig.fuel_cost)
            .unwrap_or(100)
    }

    /// Check if types are compatible for ABI
    fn types_compatible(expected: &AbiType, value: &AbiValue) -> bool {
        match (expected, value) {
            (AbiType::I32, AbiValue::I32(_)) => true,
            (AbiType::I64, AbiValue::I64(_)) => true,
            (AbiType::F32, AbiValue::F32(_)) => true,
            (AbiType::F64, AbiValue::F64(_)) => true,
            (AbiType::Bool, AbiValue::Bool(_)) => true,
            (AbiType::String, AbiValue::String(_)) => true,
            (AbiType::Bytes, AbiValue::Bytes(_)) => true,
            (AbiType::Void, AbiValue::Void) => true,
            (AbiType::Option(_), AbiValue::Option(_)) => true,
            (AbiType::List(inner), AbiValue::List(items)) => {
                items.iter().all(|item| Self::types_compatible(inner, item))
            }
            _ => false,
        }
    }
}

impl Default for DeterministicAbi {
    fn default() -> Self {
        Self::new()
    }
}

impl AbiCall {
    /// Create a new ABI call
    #[must_use]
    pub fn new(function_name: String, args: Vec<AbiValue>, context: AbiContext) -> Self {
        Self {
            function_name,
            args,
            context,
        }
    }

    /// Create a simple call with minimal context
    #[must_use]
    pub fn simple(function_name: &str, args: Vec<AbiValue>) -> Self {
        Self {
            function_name: function_name.to_string(),
            args,
            context: AbiContext {
                node_id: None,
                event_id: None,
                timestamp: 0,
                memory_limit: MemoryLimit::default(),
            },
        }
    }

    /// Create a clock read call
    #[must_use]
    pub fn clock_read() -> Self {
        Self::simple("clock_read", vec![])
    }

    /// Create a log write call
    #[must_use]
    pub fn log_write(message: String, level: i32) -> Self {
        Self::simple("log_write", vec![AbiValue::String(message), AbiValue::I32(level)])
    }

    /// Create a capability check call
    #[must_use]
    pub fn has_capability(capability: String) -> Self {
        Self::simple("has_capability", vec![AbiValue::String(capability)])
    }
}

impl AbiContext {
    /// Create a new context
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_id: None,
            event_id: None,
            timestamp: 0,
            memory_limit: MemoryLimit::default(),
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
}

impl Default for AbiContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_new() {
        let abi = DeterministicAbi::new();
        assert!(abi.functions.contains_key("clock_read"));
        assert!(abi.functions.contains_key("log_write"));
    }

    #[test]
    fn test_abi_get_function() {
        let abi = DeterministicAbi::new();
        let func = abi.get_function("clock_read");
        assert!(func.is_some());
        assert_eq!(func.unwrap().returns, AbiType::I64);
    }

    #[test]
    fn test_abi_validate_call_valid() {
        let abi = DeterministicAbi::new();
        let call = AbiCall::clock_read();
        assert!(abi.validate_call(&call).is_ok());
    }

    #[test]
    fn test_abi_validate_call_unknown() {
        let abi = DeterministicAbi::new();
        let call = AbiCall::simple("unknown_func", vec![]);
        assert!(abi.validate_call(&call).is_err());
    }

    #[test]
    fn test_abi_validate_call_wrong_args() {
        let abi = DeterministicAbi::new();
        let call = AbiCall::simple("clock_read", vec![AbiValue::I32(42)]);
        assert!(abi.validate_call(&call).is_err());
    }

    #[test]
    fn test_abi_calculate_fuel_cost() {
        let abi = DeterministicAbi::new();
        let call = AbiCall::clock_read();
        assert_eq!(abi.calculate_fuel_cost(&call), 10);
    }

    #[test]
    fn test_abi_call_simple() {
        let call = AbiCall::simple("clock_read", vec![]);
        assert_eq!(call.function_name, "clock_read");
        assert_eq!(call.args.len(), 0);
    }

    #[test]
    fn test_abi_call_log_write() {
        let call = AbiCall::log_write("test".to_string(), 1);
        assert_eq!(call.function_name, "log_write");
        assert_eq!(call.args.len(), 2);
    }

    #[test]
    fn test_abi_call_has_capability() {
        let call = AbiCall::has_capability("fs_read".to_string());
        assert_eq!(call.function_name, "has_capability");
        assert_eq!(call.args.len(), 1);
    }

    #[test]
    fn test_abi_context_new() {
        let ctx = AbiContext::new();
        assert!(ctx.node_id.is_none());
        assert!(ctx.event_id.is_none());
    }

    #[test]
    fn test_abi_context_with_node() {
        let node_id = NodeId::new();
        let ctx = AbiContext::new().with_node(node_id);
        assert_eq!(ctx.node_id, Some(node_id));
    }

    #[test]
    fn test_abi_error_display() {
        let err = AbiError::UnknownFunction("foo".to_string());
        assert!(err.to_string().contains("Unknown ABI function"));
    }

    #[test]
    fn test_abi_type_compat_i32() {
        assert!(DeterministicAbi::types_compatible(
            &AbiType::I32,
            &AbiValue::I32(42)
        ));
    }

    #[test]
    fn test_abi_type_compat_string() {
        assert!(DeterministicAbi::types_compatible(
            &AbiType::String,
            &AbiValue::String("test".to_string())
        ));
    }

    #[test]
    fn test_abi_default() {
        let abi = DeterministicAbi::default();
        assert_eq!(abi.version.major, 0);
        assert_eq!(abi.version.minor, 1);
    }
}
