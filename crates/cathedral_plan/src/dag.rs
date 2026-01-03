//! Typed DAG for workflow execution.
//!
//! The DAG is the result of compiling the planner DSL and represents
//! the executable workflow with explicit type information.

use cathedral_core::{NodeId, Capability, CoreResult, CoreError};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

/// A directed acyclic graph representing a workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dag {
    /// All nodes in the DAG
    pub nodes: IndexMap<NodeId, Node>,
    /// All edges (dependencies) in the DAG
    pub edges: Vec<Edge>,
    /// Entry nodes (no dependencies)
    pub entry_nodes: IndexSet<NodeId>,
    /// Exit nodes (no dependents)
    pub exit_nodes: IndexSet<NodeId>,
}

impl Dag {
    /// Create a new empty DAG
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            edges: Vec::new(),
            entry_nodes: IndexSet::new(),
            exit_nodes: IndexSet::new(),
        }
    }

    /// Add a node to the DAG
    ///
    /// # Errors
    ///
    /// Returns error if node already exists
    pub fn add_node(&mut self, node: Node) -> CoreResult<()> {
        let id = node.id;
        if self.nodes.contains_key(&id) {
            return Err(CoreError::AlreadyExists {
                kind: "Node".to_string(),
                id: format!("{:?}", id),
            });
        }

        // Track entry nodes (nodes with no dependencies yet)
        if node.dependencies.is_empty() {
            self.entry_nodes.insert(id);
        }

        self.nodes.insert(id, node);
        Ok(())
    }

    /// Add an edge to the DAG
    ///
    /// # Errors
    ///
    /// Returns error if it would create a cycle
    pub fn add_edge(&mut self, edge: Edge) -> CoreResult<()> {
        // Check for cycle
        if self.would_create_cycle(&edge)? {
            return Err(CoreError::Validation {
                field: "edge".to_string(),
                reason: format!("adding edge {:?} would create a cycle", edge),
            });
        }

        self.edges.push(edge);
        Ok(())
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, edge: &Edge) -> CoreResult<bool> {
        let mut visited = IndexSet::new();
        let mut stack = vec![edge.to];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                return Ok(true); // Cycle detected
            }
            visited.insert(current);

            // Follow edges from current node
            for e in &self.edges {
                if e.from == current {
                    stack.push(e.to);
                }
            }
        }

        Ok(false)
    }

    /// Validate the DAG structure
    ///
    /// # Errors
    ///
    /// Returns error if DAG is invalid
    pub fn validate(&self) -> CoreResult<()> {
        // Check all node IDs referenced in edges exist
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) {
                return Err(CoreError::NotFound {
                    kind: "Node".to_string(),
                    id: format!("{:?}", edge.from),
                });
            }
            if !self.nodes.contains_key(&edge.to) {
                return Err(CoreError::NotFound {
                    kind: "Node".to_string(),
                    id: format!("{:?}", edge.to),
                });
            }
        }

        // Check for cycles
        for edge in &self.edges {
            if self.would_create_cycle(edge)? {
                return Err(CoreError::Validation {
                    field: "dag".to_string(),
                    reason: format!("cycle detected involving edge {:?}", edge),
                });
            }
        }

        Ok(())
    }

    /// Get node by ID
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get nodes that depend on the given node
    #[must_use]
    pub fn dependents(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.from == id)
            .map(|e| e.to)
            .collect()
    }

    /// Get nodes that the given node depends on
    #[must_use]
    pub fn dependencies(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.to == id)
            .map(|e| e.from)
            .collect()
    }

    /// Get total node count
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total edge count
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if DAG is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the DAG
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// Unique node ID
    pub id: NodeId,
    /// Node kind
    pub kind: NodeKind,
    /// Node dependencies
    pub dependencies: IndexSet<NodeId>,
    /// Required capabilities
    pub capabilities: Vec<Capability>,
    /// Resource requirements
    pub resources: ResourceRequirements,
}

/// Node kind - type of operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// Input node - provides data to the workflow
    Input {
        /// Input schema
        schema: String,
    },
    /// Output node - consumes workflow result
    Output {
        /// Output schema
        schema: String,
    },
    /// Tool execution node
    Tool {
        /// Tool name
        name: String,
        /// Tool version
        version: String,
    },
    /// Map/transform operation
    Map {
        /// Transformation function
        function: String,
    },
    /// Filter operation
    Filter {
        /// Filter predicate
        predicate: String,
    },
    /// Reduce/fold operation
    Reduce {
        /// Reduction function
        function: String,
        /// Initial value
        initial: Vec<u8>,
    },
    /// Parallel execution
    Parallel {
        /// Branch count
        branches: usize,
    },
    /// Sequential composition
    Sequence {
        /// Step count
        steps: usize,
    },
    /// Condition/branch
    Condition {
        /// Condition expression
        condition: String,
    },
    /// Loop iteration
    Loop {
        /// Loop condition
        condition: String,
        /// Max iterations
        max_iterations: Option<u64>,
    },
}

/// An edge between nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// Source node
    pub from: NodeId,
    /// Target node
    pub to: NodeId,
    /// Output port from source
    pub from_port: Option<String>,
    /// Input port at target
    pub to_port: Option<String>,
}

impl Edge {
    /// Create a new edge
    #[must_use]
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self {
            from,
            to,
            from_port: None,
            to_port: None,
        }
    }

    /// Create a new edge with ports
    #[must_use]
    pub fn with_ports(from: NodeId, to: NodeId, from_port: String, to_port: String) -> Self {
        Self {
            from,
            to,
            from_port: Some(from_port),
            to_port: Some(to_port),
        }
    }
}

/// Resource requirements for a node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Maximum memory in bytes
    pub max_memory: Option<u64>,
    /// Maximum execution time (logical ticks)
    pub max_ticks: Option<u64>,
    /// Required CPU shares
    pub cpu_shares: Option<u32>,
    /// Disk space requirements
    pub disk_space: Option<u64>,
    /// Network bandwidth requirements
    pub network_bandwidth: Option<u64>,
}

impl ResourceRequirements {
    /// Create new empty requirements
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_memory: None,
            max_ticks: None,
            cpu_shares: None,
            disk_space: None,
            network_bandwidth: None,
        }
    }

    /// Set max memory
    #[must_use]
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.max_memory = Some(bytes);
        self
    }

    /// Set max ticks
    #[must_use]
    pub fn with_max_ticks(mut self, ticks: u64) -> Self {
        self.max_ticks = Some(ticks);
        self
    }

    /// Set CPU shares
    #[must_use]
    pub fn with_cpu_shares(mut self, shares: u32) -> Self {
        self.cpu_shares = Some(shares);
        self
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_node(id: NodeId) -> Node {
        Node {
            id,
            kind: NodeKind::Tool {
                name: "test_tool".to_string(),
                version: "1.0.0".to_string(),
            },
            dependencies: IndexSet::new(),
            capabilities: Vec::new(),
            resources: ResourceRequirements::new(),
        }
    }

    #[test]
    fn test_dag_new() {
        let dag = Dag::new();
        assert!(dag.is_empty());
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_dag_add_node() {
        let mut dag = Dag::new();
        let node = make_test_node(NodeId::new());

        let result = dag.add_node(node.clone());
        assert!(result.is_ok());
        assert_eq!(dag.node_count(), 1);
        assert!(dag.entry_nodes.contains(&node.id));
    }

    #[test]
    fn test_dag_add_node_duplicate() {
        let mut dag = Dag::new();
        let id = NodeId::new();
        let node = make_test_node(id);

        dag.add_node(node.clone()).unwrap();
        let result = dag.add_node(node);

        assert!(result.is_err());
    }

    #[test]
    fn test_dag_add_edge() {
        let mut dag = Dag::new();
        let id1 = NodeId::new();
        let id2 = NodeId::new();

        dag.add_node(make_test_node(id1)).unwrap();
        dag.add_node(make_test_node(id2)).unwrap();

        let edge = Edge::new(id1, id2);
        let result = dag.add_edge(edge);

        assert!(result.is_ok());
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn test_dag_validate_empty() {
        let dag = Dag::new();
        assert!(dag.validate().is_ok());
    }

    #[test]
    fn test_dag_validate_with_nodes() {
        let mut dag = Dag::new();
        let id = NodeId::new();
        dag.add_node(make_test_node(id)).unwrap();

        assert!(dag.validate().is_ok());
    }

    #[test]
    fn test_dag_validate_missing_node() {
        let mut dag = Dag::new();
        let id = NodeId::new();
        dag.add_node(make_test_node(id)).unwrap();

        let edge = Edge::new(id, NodeId::new());
        dag.edges.push(edge);

        let result = dag.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_dag_dependents() {
        let mut dag = Dag::new();
        let id1 = NodeId::new();
        let id2 = NodeId::new();

        dag.add_node(make_test_node(id1)).unwrap();
        dag.add_node(make_test_node(id2)).unwrap();

        dag.edges.push(Edge::new(id1, id2));

        let deps = dag.dependencies(id2);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], id1);
    }

    #[test]
    fn test_edge_new() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let edge = Edge::new(id1, id2);

        assert_eq!(edge.from, id1);
        assert_eq!(edge.to, id2);
        assert!(edge.from_port.is_none());
        assert!(edge.to_port.is_none());
    }

    #[test]
    fn test_edge_with_ports() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let edge = Edge::with_ports(id1, id2, "output".to_string(), "input".to_string());

        assert_eq!(edge.from_port, Some("output".to_string()));
        assert_eq!(edge.to_port, Some("input".to_string()));
    }

    #[test]
    fn test_resource_requirements() {
        let reqs = ResourceRequirements::new()
            .with_max_memory(1024)
            .with_max_ticks(100)
            .with_cpu_shares(4);

        assert_eq!(reqs.max_memory, Some(1024));
        assert_eq!(reqs.max_ticks, Some(100));
        assert_eq!(reqs.cpu_shares, Some(4));
    }
}
