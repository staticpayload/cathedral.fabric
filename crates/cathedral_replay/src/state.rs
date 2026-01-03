//! Reconstructed state during replay.

use cathedral_core::{NodeId, CoreResult, CoreError};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// State reconstructed during replay
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconstructedState {
    /// Node outputs by node ID
    pub node_outputs: IndexMap<NodeId, NodeState>,
    /// Global state key-value pairs
    pub global_state: IndexMap<String, Vec<u8>>,
    /// Errors that occurred during replay
    pub errors: Vec<ReplayError>,
    /// Current logical time
    pub time: u64,
}

/// State of a single node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeState {
    /// Node ID
    pub node_id: NodeId,
    /// Whether node completed successfully
    pub completed: bool,
    /// Output data
    pub output: Option<Vec<u8>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Side effects performed
    pub side_effects: Vec<String>,
}

/// Error during replay
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayError {
    /// Node where error occurred
    pub node_id: NodeId,
    /// Error message
    pub message: String,
    /// Logical time when error occurred
    pub time: u64,
}

impl ReconstructedState {
    /// Create a new reconstructed state
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_outputs: IndexMap::new(),
            global_state: IndexMap::new(),
            errors: Vec::new(),
            time: 0,
        }
    }

    /// Add a node state
    pub fn add_node_state(&mut self, node_id: NodeId, state: NodeState) {
        self.node_outputs.insert(node_id, state);
    }

    /// Get node state
    #[must_use]
    pub fn get_node_state(&self, node_id: NodeId) -> Option<&NodeState> {
        self.node_outputs.get(&node_id)
    }

    /// Set a global state value
    pub fn set_global(&mut self, key: String, value: Vec<u8>) {
        self.global_state.insert(key, value);
    }

    /// Get a global state value
    #[must_use]
    pub fn get_global(&self, key: &str) -> Option<&[u8]> {
        self.global_state.get(key).map(|v| v.as_slice())
    }

    /// Add an error
    pub fn add_error(&mut self, error: ReplayError) {
        self.errors.push(error);
    }

    /// Increment logical time
    pub fn tick(&mut self) {
        self.time += 1;
    }

    /// Get current time
    #[must_use]
    pub fn time(&self) -> u64 {
        self.time
    }

    /// Check if replay had errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get completed node count
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.node_outputs.values().filter(|s| s.completed).count()
    }

    /// Get total node count
    #[must_use]
    pub fn total_nodes(&self) -> usize {
        self.node_outputs.len()
    }

    /// Merge another state into this one
    pub fn merge(&mut self, other: ReconstructedState) {
        for (node_id, state) in other.node_outputs {
            self.node_outputs.insert(node_id, state);
        }
        for (key, value) in other.global_state {
            self.global_state.insert(key, value);
        }
        self.errors.extend(other.errors);
        self.time = self.time.max(other.time);
    }
}

impl Default for ReconstructedState {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeState {
    /// Create a new node state
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            completed: false,
            output: None,
            error: None,
            side_effects: Vec::new(),
        }
    }

    /// Mark node as completed with output
    #[must_use]
    pub fn with_output(mut self, output: Vec<u8>) -> Self {
        self.completed = true;
        self.output = Some(output);
        self
    }

    /// Mark node as failed with error
    #[must_use]
    pub fn with_error(mut self, error: String) -> Self {
        self.completed = false;
        self.error = Some(error);
        self
    }

    /// Add a side effect
    #[must_use]
    pub fn with_side_effect(mut self, effect: String) -> Self {
        self.side_effects.push(effect);
        self
    }
}

/// Diff between two states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDiff {
    /// Nodes added
    pub added: BTreeSet<NodeId>,
    /// Nodes removed
    pub removed: BTreeSet<NodeId>,
    /// Nodes modified
    pub modified: BTreeSet<NodeId>,
    /// Global state changes
    pub global_changes: Vec<GlobalStateChange>,
}

/// Change to global state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalStateChange {
    /// Key that changed
    pub key: String,
    /// Old value (if any)
    pub old_value: Option<Vec<u8>>,
    /// New value (if any)
    pub new_value: Option<Vec<u8>>,
}

impl StateDiff {
    /// Create a new empty state diff
    #[must_use]
    pub fn new() -> Self {
        Self {
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
            modified: BTreeSet::new(),
            global_changes: Vec::new(),
        }
    }

    /// Check if there are any differences
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty()
            || !self.removed.is_empty()
            || !self.modified.is_empty()
            || !self.global_changes.is_empty()
    }

    /// Compute diff between two states
    #[must_use]
    pub fn compute(before: &ReconstructedState, after: &ReconstructedState) -> Self {
        let mut diff = Self::new();

        // Find added nodes
        for node_id in after.node_outputs.keys() {
            if !before.node_outputs.contains_key(node_id) {
                diff.added.insert(*node_id);
            }
        }

        // Find removed nodes
        for node_id in before.node_outputs.keys() {
            if !after.node_outputs.contains_key(node_id) {
                diff.removed.insert(*node_id);
            }
        }

        // Find modified nodes
        for node_id in after.node_outputs.keys() {
            if let Some(before_state) = before.get_node_state(*node_id) {
                let after_state = after.get_node_state(*node_id).unwrap();
                if before_state != after_state {
                    diff.modified.insert(*node_id);
                }
            }
        }

        // Find global state changes
        for key in before.global_state.keys().chain(after.global_state.keys()) {
            let before_val = before.get_global(key);
            let after_val = after.get_global(key);

            if before_val != after_val {
                diff.global_changes.push(GlobalStateChange {
                    key: key.clone(),
                    old_value: before_val.map(|v| v.to_vec()),
                    new_value: after_val.map(|v| v.to_vec()),
                });
            }
        }

        diff
    }

    /// Merge another diff into this one
    pub fn merge(&mut self, other: StateDiff) {
        self.added.extend(other.added);
        self.removed.extend(other.removed);
        self.modified.extend(other.modified);
        self.global_changes.extend(other.global_changes);
    }
}

impl Default for StateDiff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstructed_state_new() {
        let state = ReconstructedState::new();
        assert_eq!(state.time(), 0);
        assert!(!state.has_errors());
        assert_eq!(state.total_nodes(), 0);
    }

    #[test]
    fn test_reconstructed_state_add_node() {
        let mut state = ReconstructedState::new();
        let node_id = NodeId::new();
        let node_state = NodeState::new(node_id).with_output(b"result".to_vec());

        state.add_node_state(node_id, node_state);
        assert_eq!(state.total_nodes(), 1);
        assert_eq!(state.completed_count(), 1);
    }

    #[test]
    fn test_reconstructed_state_global() {
        let mut state = ReconstructedState::new();
        state.set_global("key".to_string(), b"value".to_vec());
        assert_eq!(state.get_global("key"), Some(&b"value"[..]));
        assert_eq!(state.get_global("missing"), None);
    }

    #[test]
    fn test_reconstructed_state_tick() {
        let mut state = ReconstructedState::new();
        assert_eq!(state.time(), 0);
        state.tick();
        assert_eq!(state.time(), 1);
    }

    #[test]
    fn test_node_state_new() {
        let node_id = NodeId::new();
        let state = NodeState::new(node_id);
        assert!(!state.completed);
        assert!(state.output.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_node_state_with_output() {
        let node_id = NodeId::new();
        let state = NodeState::new(node_id).with_output(b"data".to_vec());
        assert!(state.completed);
        assert_eq!(state.output, Some(b"data".to_vec()));
    }

    #[test]
    fn test_node_state_with_error() {
        let node_id = NodeId::new();
        let state = NodeState::new(node_id).with_error("failed".to_string());
        assert!(!state.completed);
        assert_eq!(state.error, Some("failed".to_string()));
    }

    #[test]
    fn test_state_diff_empty() {
        let diff = StateDiff::new();
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_state_diff_compute_no_changes() {
        let state1 = ReconstructedState::new();
        let state2 = ReconstructedState::new();
        let diff = StateDiff::compute(&state1, &state2);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_state_diff_compute_with_changes() {
        let mut state1 = ReconstructedState::new();
        let mut state2 = ReconstructedState::new();

        let node_id = NodeId::new();
        state2.add_node_state(node_id, NodeState::new(node_id));

        let diff = StateDiff::compute(&state1, &state2);
        assert!(diff.has_changes());
        assert!(diff.added.contains(&node_id));
    }

    #[test]
    fn test_state_diff_global_changes() {
        let mut state1 = ReconstructedState::new();
        let mut state2 = ReconstructedState::new();

        state1.set_global("key".to_string(), b"old".to_vec());
        state2.set_global("key".to_string(), b"new".to_vec());

        let diff = StateDiff::compute(&state1, &state2);
        assert!(diff.global_changes.iter().any(|c| c.key == "key"));
    }
}
