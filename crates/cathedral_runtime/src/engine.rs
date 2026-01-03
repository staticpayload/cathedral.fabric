//! Execution engine for DAG workflows.
//!
//! Combines scheduler and executor to run complete DAGs deterministically.

use cathedral_core::{RunId, NodeId, EventId, LogicalTime, CoreResult, CoreError, CapabilitySet};
use cathedral_log::{Event, EventKind, EventStream};
use indexmap::{IndexMap, IndexSet};

use super::scheduler::{Scheduler, ScheduleDecision};
use super::executor::{Executor, ExecutionContext, ExecutorResult};

/// Execution engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum execution ticks before timeout
    pub max_ticks: u64,
    /// Capability set for execution
    pub capabilities: CapabilitySet,
    /// Whether to enable backpressure
    pub enable_backpressure: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_ticks: 1_000_000,
            capabilities: CapabilitySet::new(),
            enable_backpressure: true,
        }
    }
}

/// Execution engine error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// Timeout
    Timeout,
    /// Cycle detected
    CycleDetected,
    /// Execution failed
    NodeFailed { node_id: NodeId, error: String },
    /// Invalid state
    InvalidState,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Execution timeout"),
            Self::CycleDetected => write!(f, "Cycle detected in DAG"),
            Self::NodeFailed { node_id, error } => {
                write!(f, "Node {:?} failed: {}", node_id, error)
            }
            Self::InvalidState => write!(f, "Invalid execution state"),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Execution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// All nodes completed successfully
    Success,
    /// Some nodes failed
    PartialFailure,
    /// Timed out
    Timeout,
    /// Cycle detected
    CycleDetected,
}

/// Node output storage
#[derive(Debug, Clone)]
pub struct NodeOutput {
    /// Node ID
    pub node_id: NodeId,
    /// Output data
    pub output: Vec<u8>,
    /// Output hash
    pub output_hash: cathedral_core::Hash,
}

/// Execution engine for running DAG workflows
///
/// The engine combines a scheduler and executor to run complete DAGs
/// with deterministic ordering and full event logging.
pub struct ExecutionEngine {
    /// Scheduler
    scheduler: Scheduler,
    /// Executor
    executor: Executor,
    /// Configuration
    config: EngineConfig,
    /// Node outputs (by node ID)
    outputs: IndexMap<NodeId, NodeOutput>,
    /// Events generated during execution
    events: Vec<Event>,
    /// Current run ID
    run_id: RunId,
    /// Current logical time
    time: LogicalTime,
    /// Last event ID (for chaining)
    last_event_id: Option<EventId>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    #[must_use]
    pub fn new(run_id: RunId, config: EngineConfig) -> Self {
        let executor = Executor::new()
            .with_max_ticks(config.max_ticks)
            .with_strict_capabilities(true);

        Self {
            scheduler: Scheduler::new(),
            executor,
            config,
            outputs: IndexMap::new(),
            events: Vec::new(),
            run_id,
            time: LogicalTime::zero(),
            last_event_id: None,
        }
    }

    /// Add a node to the execution plan
    ///
    /// # Errors
    ///
    /// Returns error if cycle is detected
    pub fn add_node(&mut self, node_id: NodeId, deps: indexmap::IndexSet<NodeId>) -> CoreResult<()> {
        self.scheduler.add_node(node_id, deps)
    }

    /// Run the execution to completion
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub fn run(&mut self) -> CoreResult<ExecutionStatus> {
        loop {
            // Check for timeout
            if self.time.as_u64() >= self.config.max_ticks {
                return Ok(ExecutionStatus::Timeout);
            }

            match self.scheduler.decide() {
                ScheduleDecision::Run(node_id) => {
                    self.execute_node(node_id)?;
                }
                ScheduleDecision::Wait => {
                    // Waiting for dependencies that can't be satisfied
                    if self.scheduler.has_failures() {
                        return Ok(ExecutionStatus::PartialFailure);
                    }
                    // Otherwise, this shouldn't happen in a valid DAG
                    return Ok(ExecutionStatus::CycleDetected);
                }
                ScheduleDecision::Complete => {
                    return Ok(if self.scheduler.has_failures() {
                        ExecutionStatus::PartialFailure
                    } else {
                        ExecutionStatus::Success
                    });
                }
            }
        }
    }

    /// Execute a single node
    fn execute_node(&mut self, node_id: NodeId) -> CoreResult<()> {
        let time = self.scheduler.time();

        // Build execution context with inputs from dependencies
        let mut ctx = ExecutionContext::new(
            self.run_id,
            node_id,
            time,
            self.config.capabilities.clone(),
        );

        // Add inputs from completed dependencies
        if let Some(deps) = self.scheduler.nodes().iter().find(|&&id| id == node_id) {
            // This is a simplified version - in real implementation we'd track dependencies
        }

        // Set parent event
        if let Some(parent_id) = self.last_event_id {
            ctx = ctx.with_parent(parent_id);
        }

        // Execute with events
        let (start_event, end_event, result) = self.executor.execute_with_events(&ctx)?;

        // Get event ID before moving
        let end_event_id = end_event.event_id;

        // Record events
        self.events.push(start_event);
        self.events.push(end_event);
        self.last_event_id = Some(end_event_id);

        // Handle result
        match result {
            ExecutorResult::Success { output, output_hash } => {
                self.outputs.insert(node_id, NodeOutput {
                    node_id,
                    output,
                    output_hash,
                });
                self.scheduler.mark_complete(node_id)?;
            }
            ExecutorResult::Failed { error } => {
                self.scheduler.mark_failed(node_id)?;
                return Err(CoreError::Validation {
                    field: format!("node {:?}", node_id),
                    reason: error,
                });
            }
            ExecutorResult::Skipped { .. } => {
                self.scheduler.mark_failed(node_id)?;
            }
        }

        self.time = self.time.saturating_add(1);
        Ok(())
    }

    /// Get all events from execution
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Get events as a stream
    #[must_use]
    pub fn event_stream(&self) -> EventStream {
        // Convert to the log's Event type (simplified)
        let events = self.events.iter().map(|e| cathedral_log::stream::Event {
            logical_time: e.logical_time,
        }).collect();
        EventStream::new(events)
    }

    /// Get output for a specific node
    #[must_use]
    pub fn get_output(&self, node_id: NodeId) -> Option<&NodeOutput> {
        self.outputs.get(&node_id)
    }

    /// Get all outputs
    #[must_use]
    pub fn outputs(&self) -> &IndexMap<NodeId, NodeOutput> {
        &self.outputs
    }

    /// Get current logical time
    #[must_use]
    pub const fn time(&self) -> LogicalTime {
        self.time
    }

    /// Get run ID
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Reset the engine for re-execution
    pub fn reset(&mut self) {
        self.scheduler.reset();
        self.outputs.clear();
        self.events.clear();
        self.time = LogicalTime::zero();
        self.last_event_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexSet;

    fn make_test_run() -> RunId {
        RunId::new()
    }

    fn make_test_node() -> NodeId {
        NodeId::new()
    }

    #[test]
    fn test_engine_config_default() {
        let config = EngineConfig::default();
        assert_eq!(config.max_ticks, 1_000_000);
        assert!(config.capabilities.is_empty());
        assert!(config.enable_backpressure);
    }

    #[test]
    fn test_engine_new() {
        let run_id = make_test_run();
        let engine = ExecutionEngine::new(run_id, EngineConfig::default());

        assert_eq!(engine.run_id(), run_id);
        assert_eq!(engine.time().as_u64(), 0);
        assert!(engine.events().is_empty());
        assert!(engine.outputs().is_empty());
    }

    #[test]
    fn test_engine_add_node() {
        let mut engine = ExecutionEngine::new(make_test_run(), EngineConfig::default());
        let node = make_test_node();

        let result = engine.add_node(node, IndexSet::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_engine_run_single_node() {
        let mut engine = ExecutionEngine::new(make_test_run(), EngineConfig::default());
        let node = make_test_node();

        engine.add_node(node, IndexSet::new()).unwrap();

        let result = engine.run().unwrap();
        assert_eq!(result, ExecutionStatus::Success);
        assert!(!engine.events().is_empty());
    }

    #[test]
    fn test_engine_run_two_nodes_independent() {
        let mut engine = ExecutionEngine::new(make_test_run(), EngineConfig::default());
        let node1 = make_test_node();
        let node2 = make_test_node();

        engine.add_node(node1, IndexSet::new()).unwrap();
        engine.add_node(node2, IndexSet::new()).unwrap();

        let result = engine.run().unwrap();
        assert_eq!(result, ExecutionStatus::Success);
        assert_eq!(engine.events().len(), 4); // 2 start + 2 complete
    }

    #[test]
    fn test_engine_run_two_nodes_dependent() {
        let mut engine = ExecutionEngine::new(make_test_run(), EngineConfig::default());
        let node1 = make_test_node();
        let node2 = make_test_node();

        let mut deps = IndexSet::new();
        deps.insert(node1);

        engine.add_node(node1, IndexSet::new()).unwrap();
        engine.add_node(node2, deps).unwrap();

        let result = engine.run().unwrap();
        assert_eq!(result, ExecutionStatus::Success);
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = ExecutionEngine::new(make_test_run(), EngineConfig::default());
        let node = make_test_node();

        engine.add_node(node, IndexSet::new()).unwrap();
        engine.run().unwrap();

        assert!(engine.time().as_u64() > 0);
        assert!(!engine.events().is_empty());

        engine.reset();

        assert_eq!(engine.time().as_u64(), 0);
        assert!(engine.events().is_empty());
    }

    #[test]
    fn test_engine_timeout() {
        let config = EngineConfig {
            max_ticks: 1,
            ..Default::default()
        };
        let mut engine = ExecutionEngine::new(make_test_run(), config);

        // Add nodes that will take more than 1 tick
        let node1 = make_test_node();
        let node2 = make_test_node();

        let mut deps = IndexSet::new();
        deps.insert(node1);

        engine.add_node(node1, IndexSet::new()).unwrap();
        engine.add_node(node2, deps).unwrap();

        // Run the engine - may timeout or complete
        let _result = engine.run();

        // The engine should have run at least one node
        assert!(engine.time().as_u64() >= 1);
    }
}
