//! CATHEDRAL.FABRIC Storage
//!
//! Content-addressed blob store with snapshot support.
//! All operations are logged and verifiable.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blob;
pub mod store;
pub mod snapshot;
pub mod compact;
pub mod address;

pub use blob::{Blob, BlobData, BlobId};
pub use store::{ContentStore, StoreError, StoreConfig};
pub use snapshot::{Snapshot, SnapshotBuilder, SnapshotError};
pub use compact::{Compactor, CompactPlan, CompactResult};
pub use address::{ContentAddress, AddressAlgorithm};
