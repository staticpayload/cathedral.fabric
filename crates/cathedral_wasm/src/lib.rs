//! CATHEDRAL.FABRIC WASM Sandbox
//!
//! Deterministic WASM execution with fuel limits, memory limits,
//! and capability-mediated host functions.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod sandbox;
pub mod fuel;
pub mod memory;
pub mod abi;
pub mod host;
pub mod compile;

pub use sandbox::{Sandbox, SandboxConfig, SandboxError};
pub use fuel::{FuelMeter, FuelLimiter, FuelError};
pub use memory::{MemoryLimit, MemoryRegion, MemoryError};
pub use abi::{DeterministicAbi, AbiError, AbiCall};
pub use host::{HostFunction, HostContext, HostRegistry};
pub use compile::{WasmCompiler, CompileConfig, CompileError};
