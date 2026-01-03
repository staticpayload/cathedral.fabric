//! DAG validator for workflow correctness.

use cathedral_core::{NodeId, CoreResult, CoreError};
use super::dag::Dag;
use indexmap::IndexSet;

/// Validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Cycle detected in DAG
    Cycle { nodes: Vec<NodeId> },
    /// Disconnected nodes
    Disconnected { nodes: Vec<NodeId> },
    /// Missing input
    MissingInput { node_id: NodeId },
    /// Missing output
    MissingOutput,
    /// Invalid node kind
    InvalidNodeKind { node_id: NodeId, reason: String },
    /// Resource constraint violation
    ResourceViolation { node_id: NodeId, resource: String },
    /// Capability violation
    CapabilityViolation { node_id: NodeId, capability: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cycle { nodes } => write!(f, "Cycle detected involving nodes: {:?}", nodes),
            Self::Disconnected { nodes } => write!(f, "Disconnected nodes: {:?}", nodes),
            Self::MissingInput { node_id } => write!(f, "Missing input for node {:?}", node_id),
            Self::MissingOutput => write!(f, "Missing output node"),
            Self::InvalidNodeKind { node_id, reason } => {
                write!(f, "Invalid node kind for {:?}: {}", node_id, reason)
            }
            Self::ResourceViolation { node_id, resource } => {
                write!(f, "Resource violation for {:?}: {}", node_id, resource)
            }
            Self::CapabilityViolation { node_id, capability } => {
                write!(f, "Capability violation for {:?}: {}", node_id, capability)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validator for DAG properties
pub struct Validator {
    /// Require at least one input node
    pub require_input: bool,
    /// Require at least one output node
    pub require_output: bool,
    /// Maximum allowed nodes (0 = no limit)
    pub max_nodes: usize,
}

impl Validator {
    /// Create a new validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            require_input: true,
            require_output: true,
            max_nodes: 0,
        }
    }

    /// Validate a DAG
    ///
    /// # Errors
    ///
    /// Returns error if DAG is invalid
    pub fn validate(&self, dag: &Dag) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check for cycles
        if let Err(e) = self.check_cycles(dag) {
            errors.push(e);
        }

        // Check for disconnected nodes
        if let Err(e) = self.check_connected(dag) {
            errors.push(e);
        }

        // Check for input/output
        if self.require_input && self.has_no_inputs(dag) {
            errors.push(ValidationError::MissingInput {
                node_id: *dag.entry_nodes.iter().next().unwrap_or(&NodeId::new()),
            });
        }

        if self.require_output && self.has_no_outputs(dag) {
            errors.push(ValidationError::MissingOutput);
        }

        // Check node count
        if self.max_nodes > 0 && dag.node_count() > self.max_nodes {
            errors.push(ValidationError::ResourceViolation {
                node_id: NodeId::new(),
                resource: format!("node count {} exceeds max {}", dag.node_count(), self.max_nodes),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check for cycles in the DAG
    fn check_cycles(&self, dag: &Dag) -> Result<(), ValidationError> {
        let mut visited = IndexSet::new();
        let mut rec_stack = IndexSet::new();

        for &node_id in dag.nodes.keys() {
            if self dfs_cycle(node_id, dag, &mut visited, &mut rec_stack)? {
                return Err(ValidationError::Cycle {
                    nodes: rec_stack.iter().copied().collect(),
                });
            }
        }

        Ok(())
    }

    /// DFS cycle detection
    fn dfs_cycle(
        &self,
        node_id: NodeId,
        dag: &Dag,
        visited: &mut IndexSet<NodeId>,
        rec_stack: &mut IndexSet<NodeId>,
    ) -> Result<bool, ValidationError> {
        if rec_stack.contains(&node_id) {
            return Ok(true); // Cycle found
        }
        if visited.contains(&node_id) {
            return Ok(false); // Already checked
        }

        visited.insert(node_id);
        rec_stack.insert(node_id);

        for &dep_id in &dag.dependencies(node_id) {
            if self.dfs_cycle(dep_id, dag, visited, rec_stack)? {
                return Ok(true);
            }
        }

        rec_stack.remove(&node_id);
        Ok(false)
    }

    /// Check for disconnected nodes
    fn check_connected(&self, dag: &Dag) -> Result<(), ValidationError> {
        if dag.node_count() == 0 {
            return Ok(());
        }

        let mut reachable = IndexSet::new();
        let mut stack = Vec::new();

        // Start from entry nodes
        for &entry in &dag.entry_nodes {
            stack.push(entry);
        }

        while let Some(current) = stack.pop() {
            if reachable.contains(&current) {
                continue;
            }
            reachable.insert(current);

            for &dep in &dag.dependents(current) {
                stack.push(dep);
            }
        }

        let disconnected: Vec<_> = dag.nodes.keys()
            .filter(|id| !reachable.contains(id))
            .copied()
            .collect();

        if !disconnected.is_empty() {
            return Err(ValidationError::Disconnected { nodes: disconnected });
        }

        Ok(())
    }

    /// Check if DAG has no input nodes
    fn has_no_inputs(&self, dag: &Dag) -> bool {
        dag.entry_nodes.is_empty() && !dag.nodes.is_empty()
    }

    /// Check if DAG has no output nodes
    fn has_no_outputs(&self, dag: &Dag) -> bool {
        dag.nodes.values().all(|n| !matches!(n.kind, super::dag::NodeKind::Output { .. }))
    }

    /// Set whether input is required
    #[must_use]
    pub fn with_require_input(mut self, require: bool) -> Self {
        self.require_input = require;
        self
    }

    /// Set whether output is required
    #[must_use]
    pub fn with_require_output(mut self, require: bool) -> Self {
        self.require_output = require;
        self
    }

    /// Set maximum node count
    #[must_use]
    pub fn with_max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = max;
        self
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_new() {
        let validator = Validator::new();
        assert!(validator.require_input);
        assert!(validator.require_output);
        assert_eq!(validator.max_nodes, 0);
    }

    #[test]
    fn test_validator_with_options() {
        let validator = Validator::new()
            .with_require_input(false)
            .with_require_output(false)
            .with_max_nodes(100);

        assert!(!validator.require_input);
        assert!(!validator.require_output);
        assert_eq!(validator.max_nodes, 100);
    }

    #[test]
    fn test_validate_empty_dag() {
        let validator = Validator::new().with_require_input(false).with_require_output(false);
        let dag = Dag::new();
        assert!(validator.validate(&dag).is_ok());
    }

    #[test]
    fn test_validate_dag_missing_input() {
        let validator = Validator::new();
        let dag = Dag::new();
        let result = validator.validate(&dag);
        assert!(result.is_err());
    }
}
