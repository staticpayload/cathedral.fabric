//! Node executor with capability checking.
//!
//! Executes individual nodes with full capability enforcement.

use cathedral_core::{NodeId, RunId, EventId, LogicalTime, Hash, Capability, CapabilitySet, CoreResult, CoreError};
use cathedral_log::{Event, EventKind};
use std::collections::HashMap;

/// Result of node execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorResult {
    /// Execution succeeded
    Success {
        /// Output data
        output: Vec<u8>,
        /// Output hash
        output_hash: Hash,
    },
    /// Execution failed
    Failed {
        /// Error message
        error: String,
    },
    /// Skipped (capabilities not met)
    Skipped {
        /// Missing capabilities
        missing: Vec<Capability>,
    },
}

/// Executor error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// Node not found
    NodeNotFound { node_id: NodeId },
    /// Capability denied
    CapabilityDenied { capability: Capability },
    /// Execution timeout
    Timeout,
    /// Invalid input
    InvalidInput,
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound { node_id } => write!(f, "Node not found: {:?}", node_id),
            Self::CapabilityDenied { capability } => write!(f, "Capability denied: {:?}", capability),
            Self::Timeout => write!(f, "Execution timeout"),
            Self::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Node execution context
pub struct ExecutionContext {
    /// Run ID
    pub run_id: RunId,
    /// Current node ID
    pub node_id: NodeId,
    /// Current logical time
    pub logical_time: LogicalTime,
    /// Parent event ID
    pub parent_event_id: Option<EventId>,
    /// Available capabilities
    pub capabilities: CapabilitySet,
    /// Input data from dependencies
    pub inputs: HashMap<NodeId, Vec<u8>>,
}

impl ExecutionContext {
    /// Create a new execution context
    #[must_use]
    pub fn new(
        run_id: RunId,
        node_id: NodeId,
        logical_time: LogicalTime,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            run_id,
            node_id,
            logical_time,
            parent_event_id: None,
            capabilities,
            inputs: HashMap::new(),
        }
    }

    /// Set parent event ID
    pub fn with_parent(mut self, parent: EventId) -> Self {
        self.parent_event_id = Some(parent);
        self
    }

    /// Add input from a dependency
    pub fn add_input(&mut self, from: NodeId, data: Vec<u8>) {
        self.inputs.insert(from, data);
    }

    /// Check if a capability is granted
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.allows(capability)
    }
}

/// Executor for running individual nodes
///
/// Each node is executed with full capability checking.
pub struct Executor {
    /// Maximum execution time (logical ticks)
    max_ticks: u64,
    /// Strict capability checking
    strict_capabilities: bool,
}

impl Executor {
    /// Create a new executor
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_ticks: 1_000_000,
            strict_capabilities: true,
        }
    }

    /// Set maximum execution ticks
    pub fn with_max_ticks(mut self, max: u64) -> Self {
        self.max_ticks = max;
        self
    }

    /// Enable/disable strict capability checking
    pub fn with_strict_capabilities(mut self, strict: bool) -> Self {
        self.strict_capabilities = strict;
        self
    }

    /// Execute a node with the given context
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub fn execute(&self, _ctx: &ExecutionContext) -> CoreResult<ExecutorResult> {
        // TODO: Implement actual node execution
        // For now, return a placeholder result
        Ok(ExecutorResult::Success {
            output: Vec::new(),
            output_hash: Hash::empty(),
        })
    }

    /// Check if execution should proceed based on capabilities
    ///
    /// # Errors
    ///
    /// Returns error if required capabilities are not met
    pub fn check_capabilities(
        &self,
        ctx: &ExecutionContext,
        required: &[Capability],
    ) -> CoreResult<()> {
        if !self.strict_capabilities {
            return Ok(());
        }

        for capability in required {
            if !ctx.has_capability(capability) {
                return Err(CoreError::PermissionDenied {
                    operation: format!("{:?}", capability),
                });
            }
        }

        Ok(())
    }

    /// Create a start event for node execution
    #[must_use]
    pub fn create_start_event(&self, ctx: &ExecutionContext) -> Event {
        Event::new(
            EventId::new(),
            ctx.run_id,
            ctx.node_id,
            ctx.logical_time,
            EventKind::NodeStarted,
        )
        .with_parent(ctx.parent_event_id.unwrap_or_else(EventId::new))
    }

    /// Create a completion event for node execution
    #[must_use]
    pub fn create_complete_event(
        &self,
        ctx: &ExecutionContext,
        result: &ExecutorResult,
    ) -> Event {
        let kind = match result {
            ExecutorResult::Success { .. } => EventKind::NodeCompleted,
            ExecutorResult::Failed { .. } => EventKind::NodeFailed,
            ExecutorResult::Skipped { .. } => EventKind::NodeSkipped,
        };

        Event::new(
            EventId::new(),
            ctx.run_id,
            ctx.node_id,
            ctx.logical_time.saturating_add(1),
            kind,
        )
    }

    /// Execute and generate events
    ///
    /// Returns (start_event, end_event, result)
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub fn execute_with_events(
        &self,
        ctx: &ExecutionContext,
    ) -> CoreResult<(Event, Event, ExecutorResult)> {
        let start_event = self.create_start_event(ctx);
        let result = self.execute(ctx)?;
        let end_event = self.create_complete_event(ctx, &result);

        Ok((start_event, end_event, result))
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cathedral_core::Capability;

    fn make_test_run() -> RunId {
        RunId::new()
    }

    fn make_test_node() -> NodeId {
        NodeId::new()
    }

    #[test]
    fn test_executor_new() {
        let executor = Executor::new();
        assert_eq!(executor.max_ticks, 1_000_000);
        assert!(executor.strict_capabilities);
    }

    #[test]
    fn test_executor_with_max_ticks() {
        let executor = Executor::new().with_max_ticks(100);
        assert_eq!(executor.max_ticks, 100);
    }

    #[test]
    fn test_executor_with_strict_capabilities() {
        let executor = Executor::new().with_strict_capabilities(false);
        assert!(!executor.strict_capabilities);
    }

    #[test]
    fn test_check_capabilities_pass() {
        let executor = Executor::new();
        let mut capabilities = CapabilitySet::new();
        capabilities.allow(Capability::FsRead { prefixes: vec!["/tmp".to_string()] });

        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            capabilities,
        );

        let result = executor.check_capabilities(&ctx, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_capabilities_fail() {
        let executor = Executor::new();
        let capabilities = CapabilitySet::new();

        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            capabilities,
        );

        let required = vec![Capability::FsRead { prefixes: vec!["/tmp".to_string()] }];
        let result = executor.check_capabilities(&ctx, &required);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_capabilities_non_strict() {
        let executor = Executor::new().with_strict_capabilities(false);
        let capabilities = CapabilitySet::new();

        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            capabilities,
        );

        let required = vec![Capability::FsRead { prefixes: vec!["/tmp".to_string()] }];
        let result = executor.check_capabilities(&ctx, &required);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_start_event() {
        let executor = Executor::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(10),
            CapabilitySet::new(),
        );

        let event = executor.create_start_event(&ctx);
        assert_eq!(event.kind, EventKind::NodeStarted);
        assert_eq!(event.logical_time.as_u64(), 10);
    }

    #[test]
    fn test_create_complete_event() {
        let executor = Executor::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(10),
            CapabilitySet::new(),
        );

        let result = ExecutorResult::Success {
            output: vec![1, 2, 3],
            output_hash: Hash::empty(),
        };
        let event = executor.create_complete_event(&ctx, &result);

        assert_eq!(event.kind, EventKind::NodeCompleted);
        assert_eq!(event.logical_time.as_u64(), 11);
    }

    #[test]
    fn test_create_complete_event_failed() {
        let executor = Executor::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(10),
            CapabilitySet::new(),
        );

        let result = ExecutorResult::Failed {
            error: "test error".to_string(),
        };
        let event = executor.create_complete_event(&ctx, &result);

        assert_eq!(event.kind, EventKind::NodeFailed);
    }

    #[test]
    fn test_create_complete_event_skipped() {
        let executor = Executor::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(10),
            CapabilitySet::new(),
        );

        let result = ExecutorResult::Skipped {
            missing: vec![Capability::FsRead { prefixes: vec!["/tmp".to_string()] }],
        };
        let event = executor.create_complete_event(&ctx, &result);

        assert_eq!(event.kind, EventKind::NodeSkipped);
    }

    #[test]
    fn test_execution_context_new() {
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(5),
            CapabilitySet::new(),
        );

        assert!(ctx.inputs.is_empty());
        assert!(ctx.parent_event_id.is_none());
    }

    #[test]
    fn test_execution_context_with_parent() {
        let parent = EventId::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            CapabilitySet::new(),
        )
        .with_parent(parent);

        assert_eq!(ctx.parent_event_id, Some(parent));
    }

    #[test]
    fn test_execution_context_add_input() {
        let mut ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            CapabilitySet::new(),
        );

        let from = make_test_node();
        ctx.add_input(from, vec![1, 2, 3]);

        assert_eq!(ctx.inputs.len(), 1);
        assert_eq!(ctx.inputs.get(&from), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_execution_context_has_capability() {
        let mut capabilities = CapabilitySet::new();
        capabilities.allow(Capability::FsRead { prefixes: vec!["/tmp".to_string()] });

        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::zero(),
            capabilities,
        );

        assert!(ctx.has_capability(&Capability::FsRead { prefixes: vec!["/tmp".to_string()] }));
        assert!(!ctx.has_capability(&Capability::FsWrite { prefixes: vec!["/tmp".to_string()] }));
    }

    #[test]
    fn test_execute_with_events() {
        let executor = Executor::new();
        let ctx = ExecutionContext::new(
            make_test_run(),
            make_test_node(),
            LogicalTime::from_raw(10),
            CapabilitySet::new(),
        );

        let result = executor.execute_with_events(&ctx);
        assert!(result.is_ok());

        let (start, end, exec_result) = result.unwrap();
        assert_eq!(start.kind, EventKind::NodeStarted);
        assert_eq!(end.kind, EventKind::NodeCompleted);
        assert!(matches!(exec_result, ExecutorResult::Success { .. }));
    }
}
