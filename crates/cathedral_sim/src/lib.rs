//! CATHEDRAL.FABRIC Deterministic Simulation
//!
//! Deterministic simulation of network and failure conditions.
//! All simulations are reproducible from a seed.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod network;
pub mod failure;
pub mod node;
pub mod seed;
pub mod harness;
pub mod record;

pub use network::{NetworkSim, NetworkCondition, PacketLoss};
pub use failure::{FailureModel, FailureKind, CrashInjector};
pub use node::{SimNode, SimNodeConfig};
pub use seed::{SimSeed, SeedSource};
pub use harness::{SimHarness, SimConfig, SimResult};
pub use record::{SimRecord, RecordedRun};
