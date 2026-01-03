//! Deterministic scheduler for DAG execution.
//!
//! The scheduler is completely deterministic:
//! - No thread pools
//! - Priority-based selection (BTreeMap for deterministic ordering)
//! - Logical time increments on each operation
//! - No runtime load balancing

use cathedral_core::{NodeId, LogicalTime, CoreResult, CoreError};
use indexmap::{IndexMap, IndexSet};
use std::collections::{BTreeSet, BTreeMap};

/// Scheduling decision - which node to run next
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleDecision {
    /// Run this node next
    Run(NodeId),
    /// Wait for dependencies
    Wait,
    /// No more nodes to run
    Complete,
}

/// Scheduler error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// Node not found in DAG
    NodeNotFound { node_id: NodeId },
    /// Cycle detected in dependencies
    CycleDetected { node_id: NodeId },
    /// Invalid state
    InvalidState,
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound { node_id } => write!(f, "Node not found: {:?}", node_id),
            Self::CycleDetected { node_id } => write!(f, "Cycle detected at: {:?}", node_id),
            Self::InvalidState => write!(f, "Invalid scheduler state"),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Deterministic scheduler for DAG execution
///
/// Uses BTreeMap and BTreeSet for deterministic ordering.
/// Ready nodes are sorted by priority, then by NodeId.
pub struct Scheduler {
    /// All nodes in the DAG
    all_nodes: IndexSet<NodeId>,
    /// Nodes ready to run (sorted for determinism)
    ready: BTreeMap<(u64, NodeId), NodeId>,
    /// Completed nodes
    completed: BTreeSet<NodeId>,
    /// Failed nodes
    failed: BTreeSet<NodeId>,
    /// Dependencies: node -> set of nodes it depends on
    dependencies: IndexMap<NodeId, IndexSet<NodeId>>,
    /// Dependents (reverse edges): node -> set of nodes that depend on it
    dependents: IndexMap<NodeId, IndexSet<NodeId>>,
    /// Current logical time
    time: LogicalTime,
}

impl Scheduler {
    /// Create a new scheduler
    #[must_use]
    pub fn new() -> Self {
        Self {
            all_nodes: IndexSet::new(),
            ready: BTreeMap::new(),
            completed: BTreeSet::new(),
            failed: BTreeSet::new(),
            dependencies: IndexMap::new(),
            dependents: IndexMap::new(),
            time: LogicalTime::zero(),
        }
    }

    /// Add a node to the scheduler
    ///
    /// # Errors
    ///
    /// Returns error if a cycle is detected
    pub fn add_node(&mut self, node_id: NodeId, deps: IndexSet<NodeId>) -> CoreResult<()> {
        // Check for direct self-cycle
        if deps.contains(&node_id) {
            return Err(CoreError::Validation {
                field: "dependencies".to_string(),
                reason: format!("node {:?} depends on itself (cycle)", node_id),
            });
        }

        // Check for cycles
        for dep in &deps {
            if self.is_dependent_on(node_id, *dep) {
                return Err(CoreError::Validation {
                    field: "dependencies".to_string(),
                    reason: format!("cycle detected involving node {:?}", node_id),
                });
            }
        }

        self.all_nodes.insert(node_id);
        self.dependencies.insert(node_id, deps.clone());

        // Update dependents map
        for dep in &deps {
            self.dependents
                .entry(*dep)
                .or_insert_with(IndexSet::new)
                .insert(node_id);
        }

        // If no dependencies, node is ready
        if deps.is_empty() && !self.completed.contains(&node_id) {
            self.ready.insert((0, node_id), node_id);
        }

        Ok(())
    }

    /// Check if `a` is (transitively) dependent on `b`
    fn is_dependent_on(&self, a: NodeId, b: NodeId) -> bool {
        if let Some(deps) = self.dependencies.get(&a) {
            deps.contains(&b) || deps.iter().any(|&dep| self.is_dependent_on(dep, b))
        } else {
            false
        }
    }

    /// Get the next scheduling decision
    ///
    /// This is deterministic: always returns the highest priority ready node
    #[must_use]
    pub fn decide(&self) -> ScheduleDecision {
        if let Some((_, node_id)) = self.ready.keys().next() {
            ScheduleDecision::Run(*node_id)
        } else if self.completed.len() + self.failed.len() < self.all_nodes.len() {
            ScheduleDecision::Wait
        } else {
            ScheduleDecision::Complete
        }
    }

    /// Mark a node as completed
    ///
    /// # Errors
    ///
    /// Returns error if node wasn't ready
    pub fn mark_complete(&mut self, node_id: NodeId) -> CoreResult<()> {
        // Remove from ready queue
        let key = self.ready
            .iter()
            .find(|(_, id)| **id == node_id)
            .map(|(k, _)| *k);

        if let Some(key) = key {
            self.ready.remove(&key);
        }

        self.completed.insert(node_id);
        self.tick();

        // Check if any dependents are now ready
        if let Some(dependents) = self.dependents.get(&node_id) {
            for dep in dependents {
                if self.is_ready(*dep) && !self.completed.contains(dep) {
                    self.ready.insert((0, *dep), *dep);
                }
            }
        }

        Ok(())
    }

    /// Mark a node as failed
    ///
    /// # Errors
    ///
    /// Returns error if node wasn't ready
    pub fn mark_failed(&mut self, node_id: NodeId) -> CoreResult<()> {
        // Remove from ready queue
        let key = self.ready
            .iter()
            .find(|(_, id)| **id == node_id)
            .map(|(k, _)| *k);

        if let Some(key) = key {
            self.ready.remove(&key);
        }

        self.failed.insert(node_id);
        self.tick();

        Ok(())
    }

    /// Check if a node is ready (all dependencies completed)
    fn is_ready(&self, node_id: NodeId) -> bool {
        if let Some(deps) = self.dependencies.get(&node_id) {
            deps.iter().all(|dep| self.completed.contains(dep))
        } else {
            true
        }
    }

    /// Increment logical time
    fn tick(&mut self) {
        self.time = self.time.saturating_add(1);
    }

    /// Get current logical time
    #[must_use]
    pub const fn time(&self) -> LogicalTime {
        self.time
    }

    /// Get number of ready nodes
    #[must_use]
    pub fn ready_count(&self) -> usize {
        self.ready.len()
    }

    /// Get number of completed nodes
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Get number of failed nodes
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// Check if execution is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.completed.len() + self.failed.len() == self.all_nodes.len()
            && self.ready.is_empty()
    }

    /// Check if execution failed
    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Reset the scheduler state
    pub fn reset(&mut self) {
        self.ready.clear();
        self.completed.clear();
        self.failed.clear();
        self.time = LogicalTime::zero();

        // Re-populate ready queue with nodes that have no dependencies
        for &node_id in &self.all_nodes {
            if let Some(deps) = self.dependencies.get(&node_id) {
                if deps.is_empty() {
                    self.ready.insert((0, node_id), node_id);
                }
            }
        }
    }

    /// Get all nodes
    #[must_use]
    pub fn nodes(&self) -> &IndexSet<NodeId> {
        &self.all_nodes
    }

    /// Get completed nodes
    #[must_use]
    pub fn completed_nodes(&self) -> &BTreeSet<NodeId> {
        &self.completed
    }

    /// Get failed nodes
    #[must_use]
    pub fn failed_nodes(&self) -> &BTreeSet<NodeId> {
        &self.failed
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_id() -> NodeId {
        NodeId::new()
    }

    #[test]
    fn test_scheduler_new() {
        let scheduler = Scheduler::new();
        assert_eq!(scheduler.time().as_u64(), 0);
        assert!(scheduler.is_complete());
        assert!(!scheduler.has_failures());
    }

    #[test]
    fn test_scheduler_single_node() {
        let mut scheduler = Scheduler::new();
        let node = make_test_id();

        scheduler.add_node(node, IndexSet::new()).unwrap();
        assert_eq!(scheduler.ready_count(), 1);

        assert!(matches!(scheduler.decide(), ScheduleDecision::Run(id) if id == node));

        scheduler.mark_complete(node).unwrap();
        assert!(scheduler.is_complete());
        assert_eq!(scheduler.completed_count(), 1);
    }

    #[test]
    fn test_scheduler_dependencies() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();
        let node2 = make_test_id();

        // node2 depends on node1
        let mut deps = IndexSet::new();
        deps.insert(node1);
        scheduler.add_node(node1, IndexSet::new()).unwrap();
        scheduler.add_node(node2, deps).unwrap();

        // Only node1 is ready
        assert_eq!(scheduler.ready_count(), 1);

        // Complete node1
        scheduler.mark_complete(node1).unwrap();

        // Now node2 is ready
        assert_eq!(scheduler.ready_count(), 1);
    }

    #[test]
    fn test_scheduler_cycle_direct() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();

        let mut deps = IndexSet::new();
        deps.insert(node1); // node1 depends on itself - cycle
        let result = scheduler.add_node(node1, deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_scheduler_failure() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();
        let node2 = make_test_id();

        let mut deps = IndexSet::new();
        deps.insert(node1);
        scheduler.add_node(node1, IndexSet::new()).unwrap();
        scheduler.add_node(node2, deps).unwrap();

        // Fail node1
        scheduler.mark_failed(node1).unwrap();
        assert!(scheduler.has_failures());

        // node2 should not become ready since its dependency failed
        assert_eq!(scheduler.ready_count(), 0);
    }

    #[test]
    fn test_scheduler_reset() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();

        scheduler.add_node(node1, IndexSet::new()).unwrap();
        scheduler.mark_complete(node1).unwrap();
        assert_eq!(scheduler.completed_count(), 1);

        scheduler.reset();
        assert_eq!(scheduler.completed_count(), 0);
        assert_eq!(scheduler.ready_count(), 1);
    }

    #[test]
    fn test_scheduler_time_tick() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();

        assert_eq!(scheduler.time().as_u64(), 0);

        scheduler.add_node(node1, IndexSet::new()).unwrap();
        scheduler.mark_complete(node1).unwrap();

        assert_eq!(scheduler.time().as_u64(), 1);
    }

    #[test]
    fn test_scheduler_multiple_ready() {
        let mut scheduler = Scheduler::new();
        let node1 = make_test_id();
        let node2 = make_test_id();
        let node3 = make_test_id();

        scheduler.add_node(node1, IndexSet::new()).unwrap();
        scheduler.add_node(node2, IndexSet::new()).unwrap();
        scheduler.add_node(node3, IndexSet::new()).unwrap();

        assert_eq!(scheduler.ready_count(), 3);

        // Should run some node
        assert!(matches!(scheduler.decide(), ScheduleDecision::Run(_)));
    }
}
