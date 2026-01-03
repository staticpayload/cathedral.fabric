//! CATHEDRAL.FABRIC Core Types
//!
//! This crate contains pure types and logic with no I/O.
//! All types are serializable with stable, cross-platform encoding.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod capability;
pub mod error;
pub mod hash;
pub mod id;
pub mod time;
pub mod version;

// Re-exports
pub use capability::{Capability, CapabilitySet};
pub use error::{CoreError, CoreResult};
pub use hash::{AddressAlgorithm, ContentAddress, Hash, HashChain, HashError};
pub use id::{ClusterId, DecisionId, EventId, NodeId, RunId, SnapshotId, TaskId, WorkerId};
pub use time::{Duration, LogicalTime, Timestamp};
pub use version::{Version, VersionError};
