//! CATHEDRAL.FABRIC Core Types
//!
//! This crate contains pure types and logic with no I/O.
//! All types are serializable with stable, cross-platform encoding.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod id;
pub mod hash;
pub mod capability;
pub mod time;
pub mod version;
pub mod error;

// Re-exports
pub use id::{RunId, EventId, NodeId, WorkerId, ClusterId, TaskId, SnapshotId, DecisionId};
pub use hash::{Hash, HashChain, ContentAddress, AddressAlgorithm, HashError};
pub use capability::{Capability, CapabilitySet};
pub use time::{LogicalTime, Timestamp, Duration};
pub use version::{Version, VersionError};
pub use error::{CoreError, CoreResult};
