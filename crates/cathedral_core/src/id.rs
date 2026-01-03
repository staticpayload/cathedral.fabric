//! Unique identifiers for CATHEDRAL entities.
//!
//! All IDs are UUIDs for uniqueness and are serialized in canonical format.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Run identifier - identifies a single workflow execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Create a new random RunId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "run_{}", self.0)
    }
}

/// Event identifier - identifies a single event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventId(Uuid);

impl EventId {
    /// Create a new random EventId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evt_{}", self.0)
    }
}

/// Node identifier - identifies a DAG node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new random NodeId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Create from name (for named nodes)
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        // Use name-based UUID (v5) with DNS namespace
        Self(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node_{}", self.0)
    }
}

/// Worker identifier - identifies a worker node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkerId(Uuid);

impl WorkerId {
    /// Create a new random WorkerId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "worker_{}", self.0)
    }
}

/// Cluster identifier - identifies a cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClusterId(Uuid);

impl ClusterId {
    /// Create a new random ClusterId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for ClusterId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClusterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cluster_{}", self.0)
    }
}

/// Task identifier - identifies a task assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Create a new random TaskId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task_{}", self.0)
    }
}

/// Snapshot identifier - identifies a snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SnapshotId(Uuid);

impl SnapshotId {
    /// Create a new random SnapshotId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "snap_{}", self.0)
    }
}

/// Decision identifier - identifies a policy decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DecisionId(Uuid);

impl DecisionId {
    /// Create a new random DecisionId
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get as UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for DecisionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DecisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dec_{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let run_id = RunId::new();
        let event_id = EventId::new();
        let node_id = NodeId::new();

        assert_ne!(run_id, RunId::new());
        assert_ne!(event_id, EventId::new());
        assert_ne!(node_id, NodeId::new());
    }

    #[test]
    fn test_id_from_bytes() {
        let bytes = [1u8; 16];
        let id = RunId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_id_display() {
        let id = RunId::new();
        let s = format!("{}", id);
        assert!(s.starts_with("run_"));
    }

    #[test]
    fn test_node_id_from_name() {
        let id1 = NodeId::from_name("test_node");
        let id2 = NodeId::from_name("test_node");
        assert_eq!(id1, id2);

        let id3 = NodeId::from_name("other_node");
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_id_ord() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        // IDs are comparable for deterministic ordering
        let _ = id1.cmp(&id2);
    }
}
