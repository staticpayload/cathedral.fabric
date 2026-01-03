//! CATHEDRAL.FABRIC Replay Engine
//!
//! Deterministic reconstruction of execution from logs.
//! Divergence detection and stable diff generation.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod engine;
pub mod diff;
pub mod state;
pub mod trace;
pub mod snapshot;

pub use engine::{ReplayEngine, ReplayConfig, ReplayError};
pub use diff::{DiffEngine, DiffResult, DiffReport};
pub use state::{ReconstructedState, StateDiff};
pub use trace::{TraceReader, TraceEvent};
pub use snapshot::{SnapshotLoader, SnapshotError};
