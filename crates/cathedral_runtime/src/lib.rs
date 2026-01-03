//! CATHEDRAL.FABRIC Runtime
//!
//! Execution engine for compiled DAGs with deterministic scheduling.
//! Handles backpressure, timeouts, and capability enforcement.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod engine;
pub mod scheduler;
pub mod executor;
pub mod backpressure;
pub mod monitor;

pub use engine::{ExecutionEngine, EngineConfig, ExecutionError};
pub use scheduler::{Scheduler, ScheduleDecision, ScheduleError};
pub use executor::{Executor, ExecutorResult, ExecutorError};
pub use backpressure::{BackpressureController, BackpressureStrategy};
pub use monitor::{ExecutionMonitor, Metrics, Telemetry};
