//! Diff engine for comparing executions.

use cathedral_core::{NodeId, CoreResult, CoreError};
use crate::state::{ReconstructedState, StateDiff};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Result of a diff operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffResult {
    /// Whether states are equivalent
    pub equivalent: bool,
    /// State diff
    pub diff: StateDiff,
    /// Divergence point (if any)
    pub divergence_point: Option<usize>,
}

/// Diff report with detailed information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReport {
    /// Overall result
    pub result: DiffResult,
    /// Summary statistics
    pub summary: DiffSummary,
    /// Detailed changes by node
    pub node_changes: Vec<NodeChange>,
}

/// Summary of diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of nodes added
    pub added_count: usize,
    /// Number of nodes removed
    pub removed_count: usize,
    /// Number of nodes modified
    pub modified_count: usize,
    /// Number of state changes
    pub state_change_count: usize,
}

/// Change to a specific node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeChange {
    /// Node ID
    pub node_id: NodeId,
    /// Type of change
    pub change_type: NodeChangeType,
    /// Output diff (if applicable)
    pub output_diff: Option<StringDiff>,
}

/// Type of node change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeChangeType {
    /// Node was added
    Added,
    /// Node was removed
    Removed,
    /// Node output changed
    OutputChanged,
    /// Node error status changed
    ErrorStatusChanged,
    /// Node side effects changed
    SideEffectsChanged,
}

/// String diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringDiff {
    /// Old value
    pub old: Vec<u8>,
    /// New value
    pub new: Vec<u8>,
    /// Line-by-line diff
    pub line_diff: Vec<LineChange>,
}

/// Line change in a diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineChange {
    /// Line was added
    Added(String),
    /// Line was removed
    Removed(String),
    /// Line was modified
    Modified { old: String, new: String },
    /// Line was unchanged
    Unchanged(String),
}

/// Engine for diffing two executions
pub struct DiffEngine;

impl DiffEngine {
    /// Create a new diff engine (unit struct)
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Diff two reconstructed states
    ///
    /// # Errors
    ///
    /// Returns error if diff fails
    pub fn diff_states(
        &self,
        before: &ReconstructedState,
        after: &ReconstructedState,
    ) -> CoreResult<DiffResult> {
        let diff = StateDiff::compute(before, after);
        let equivalent = !diff.has_changes();

        Ok(DiffResult {
            equivalent,
            diff,
            divergence_point: None,
        })
    }

    /// Generate a detailed diff report
    ///
    /// # Errors
    ///
    /// Returns error if report generation fails
    pub fn generate_report(
        &self,
        before: &ReconstructedState,
        after: &ReconstructedState,
    ) -> CoreResult<DiffReport> {
        let result = self.diff_states(before, after)?;
        let summary = DiffSummary {
            added_count: result.diff.added.len(),
            removed_count: result.diff.removed.len(),
            modified_count: result.diff.modified.len(),
            state_change_count: result.diff.global_changes.len(),
        };

        let mut node_changes = Vec::new();

        // Process added nodes
        for node_id in &result.diff.added {
            node_changes.push(NodeChange {
                node_id: *node_id,
                change_type: NodeChangeType::Added,
                output_diff: None,
            });
        }

        // Process removed nodes
        for node_id in &result.diff.removed {
            node_changes.push(NodeChange {
                node_id: *node_id,
                change_type: NodeChangeType::Removed,
                output_diff: None,
            });
        }

        // Process modified nodes
        for node_id in &result.diff.modified {
            let before_state = before.get_node_state(*node_id);
            let after_state = after.get_node_state(*node_id);

            let change_type = match (before_state, after_state) {
                (Some(b), Some(a)) => {
                    if b.output != a.output {
                        NodeChangeType::OutputChanged
                    } else if b.error != a.error {
                        NodeChangeType::ErrorStatusChanged
                    } else if b.side_effects != a.side_effects {
                        NodeChangeType::SideEffectsChanged
                    } else {
                        NodeChangeType::OutputChanged // Default
                    }
                }
                _ => NodeChangeType::OutputChanged,
            };

            let output_diff = match (before_state, after_state) {
                (Some(b), Some(a)) => {
                    if b.output != a.output {
                        Some(Self::diff_bytes(
                            b.output.as_deref().unwrap_or(&[]),
                            a.output.as_deref().unwrap_or(&[]),
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            node_changes.push(NodeChange {
                node_id: *node_id,
                change_type,
                output_diff,
            });
        }

        Ok(DiffReport {
            result,
            summary,
            node_changes,
        })
    }

    /// Diff two byte arrays
    fn diff_bytes(old: &[u8], new: &[u8]) -> StringDiff {
        let old_str = String::from_utf8_lossy(old).to_string();
        let new_str = String::from_utf8_lossy(new).to_string();

        let old_lines: Vec<&str> = old_str.lines().collect();
        let new_lines: Vec<&str> = new_str.lines().collect();

        let mut line_diff = Vec::new();
        let max_len = old_lines.len().max(new_lines.len());

        for i in 0..max_len {
            let old_line = old_lines.get(i).map(|s| s.to_string());
            let new_line = new_lines.get(i).map(|s| s.to_string());

            match (old_line, new_line) {
                (Some(o), Some(n)) if o == n => {
                    line_diff.push(LineChange::Unchanged(o));
                }
                (Some(o), Some(n)) => {
                    line_diff.push(LineChange::Modified { old: o, new: n });
                }
                (Some(o), None) => {
                    line_diff.push(LineChange::Removed(o));
                }
                (None, Some(n)) => {
                    line_diff.push(LineChange::Added(n));
                }
                (None, None) => {
                    // Both empty - skip
                }
            }
        }

        StringDiff {
            old: old.to_vec(),
            new: new.to_vec(),
            line_diff,
        }
    }

    /// Find divergence point between two traces
    ///
    /// # Errors
    ///
    /// Returns error if finding divergence fails
    pub fn find_divergence(
        &self,
        state1: &ReconstructedState,
        state2: &ReconstructedState,
    ) -> CoreResult<Option<usize>> {
        let mut divergence_time = None;

        // Check for divergence in node states
        for node_id in state1
            .node_outputs
            .keys()
            .chain(state2.node_outputs.keys())
            .collect::<BTreeSet<_>>()
        {
            let s1 = state1.get_node_state(*node_id);
            let s2 = state2.get_node_state(*node_id);

            if s1 != s2 {
                divergence_time = Some(0); // Simplified - in real implementation, find exact time
                break;
            }
        }

        Ok(divergence_time)
    }

    /// Check if two states are semantically equivalent
    ///
    /// # Errors
    ///
    /// Returns error if comparison fails
    pub fn is_semantically_equivalent(
        &self,
        state1: &ReconstructedState,
        state2: &ReconstructedState,
    ) -> CoreResult<bool> {
        // States are semantically equivalent if:
        // - They have the same nodes
        // - All nodes have the same completion status
        // - Errors are the same (ignoring exact timing)

        if state1.total_nodes() != state2.total_nodes() {
            return Ok(false);
        }

        if state1.completed_count() != state2.completed_count() {
            return Ok(false);
        }

        // Check node states
        for node_id in state1.node_outputs.keys() {
            let s1 = state1.get_node_state(*node_id);
            let s2 = state2.get_node_state(*node_id);

            match (s1, s2) {
                (Some(n1), Some(n2)) => {
                    // Check completion status matches
                    if n1.completed != n2.completed {
                        return Ok(false);
                    }
                    // Check error status matches
                    if n1.error.is_some() != n2.error.is_some() {
                        return Ok(false);
                    }
                }
                (None, None) => {}
                _ => return Ok(false),
            }
        }

        Ok(true)
    }
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::NodeState;

    #[test]
    fn test_diff_engine_new() {
        let engine = DiffEngine::new();
        // Unit struct - just verify it exists
        let _ = engine;
    }

    #[test]
    fn test_diff_states_identical() {
        let engine = DiffEngine::new();
        let state1 = ReconstructedState::new();
        let state2 = ReconstructedState::new();

        let result = engine.diff_states(&state1, &state2).unwrap();
        assert!(result.equivalent);
        assert!(!result.diff.has_changes());
    }

    #[test]
    fn test_diff_states_different() {
        let engine = DiffEngine::new();
        let mut state1 = ReconstructedState::new();
        let mut state2 = ReconstructedState::new();

        let node_id = NodeId::new();
        state2.add_node_state(node_id, NodeState::new(node_id));

        let result = engine.diff_states(&state1, &state2).unwrap();
        assert!(!result.equivalent);
        assert!(result.diff.has_changes());
    }

    #[test]
    fn test_generate_report() {
        let engine = DiffEngine::new();
        let mut state1 = ReconstructedState::new();
        let mut state2 = ReconstructedState::new();

        let node_id = NodeId::new();
        state2.add_node_state(node_id, NodeState::new(node_id));

        let report = engine.generate_report(&state1, &state2).unwrap();
        assert!(!report.result.equivalent);
        assert_eq!(report.summary.added_count, 1);
    }

    #[test]
    fn test_diff_bytes() {
        let old = b"line1\nline2";
        let new = b"line1\nmodified";

        let diff = DiffEngine::diff_bytes(old, new);
        assert_eq!(diff.line_diff.len(), 2);
    }

    #[test]
    fn test_is_semantically_equivalent() {
        let engine = DiffEngine::new();
        let state1 = ReconstructedState::new();
        let state2 = ReconstructedState::new();

        assert!(engine
            .is_semantically_equivalent(&state1, &state2)
            .unwrap());
    }
}
