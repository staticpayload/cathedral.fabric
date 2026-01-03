//! CATHEDRAL.FABRIC Core Types
//!
//! This crate contains pure types and logic with no I/O.
//! All types are serializable with stable, cross-platform encoding.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod id;
pub mod hash;
pub mod capability;
pub mod time;
pub mod version;
pub mod error;

// Re-exports
pub use id::{RunId, EventId, NodeId, WorkerId, ClusterId};
pub use hash::{Hash, HashChain};
pub use capability::{Capability, CapabilitySet, CapabilityKind};
pub use time::{LogicalTime, Timestamp, Duration};
pub use version::{Version, SemVer};
pub use error::{CoreError, CoreResult};
