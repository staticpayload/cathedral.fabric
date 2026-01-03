//! CATHEDRAL.FABRIC Cluster
//!
//! Distributed execution with replicated log and deterministic scheduling.
//! Leader election, membership, and remote execution.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod consensus;
pub mod membership;
pub mod leader;
pub mod remote;
pub mod coordinator;
pub mod worker;

pub use consensus::{Consensus, ConsensusConfig, ConsensusError};
pub use membership::{Membership, Member, MemberState};
pub use leader::{LeaderElection, ElectionConfig, ElectionError};
pub use remote::{RemoteExecutor, RemoteClient, TransportError};
pub use coordinator::{Coordinator, CoordinatorConfig, CoordinatorError};
pub use worker::{Worker, WorkerConfig, WorkerError};
