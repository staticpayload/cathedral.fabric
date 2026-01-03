//! CATHEDRAL.FABRIC Tool System
//!
//! Tool interface with schemas, normalization, and capability gating.
//! All tools are treated as potentially hostile.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod trait_;
pub mod schema;
pub mod normalize;
pub mod registry;
pub mod adapter;
pub mod validate;

pub use trait_::{Tool, ToolOutput, ToolError};
pub use schema::{ToolSchema, InputSchema, OutputSchema, SideEffect};
pub use normalize::{Normalizer, NormalizedOutput, NormalizationError};
pub use registry::{ToolRegistry, RegistryError, ToolEntry};
pub use adapter::{ToolAdapter, HostAdapter, AdapterError};
pub use validate::{ToolValidator, ValidationError};
