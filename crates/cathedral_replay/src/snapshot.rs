//! Snapshot loader for replay.

use cathedral_core::{CoreResult, CoreError};
use crate::state::ReconstructedState;
use serde::{Deserialize, Serialize};

/// Snapshot error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// Snapshot not found
    NotFound { id: String },
    /// Corrupted snapshot
    Corrupted { reason: String },
    /// Version mismatch
    VersionMismatch { expected: u32, actual: u32 },
    /// Invalid format
    InvalidFormat,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "Snapshot not found: {}", id),
            Self::Corrupted { reason } => write!(f, "Corrupted snapshot: {}", reason),
            Self::VersionMismatch { expected, actual } => {
                write!(
                    f,
                    "Version mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::InvalidFormat => write!(f, "Invalid snapshot format"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<SnapshotError> for CoreError {
    fn from(err: SnapshotError) -> Self {
        CoreError::Validation {
            field: "snapshot".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Snapshot metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Snapshot ID
    pub id: String,
    /// Snapshot version
    pub version: u32,
    /// Timestamp when snapshot was taken
    pub timestamp: u64,
    /// Number of nodes in snapshot
    pub node_count: usize,
    /// Size in bytes
    pub size_bytes: usize,
}

/// Snapshot containing state at a point in time
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Metadata
    pub metadata: SnapshotMetadata,
    /// Encoded state
    pub state: ReconstructedState,
}

impl Snapshot {
    /// Create a new snapshot
    #[must_use]
    pub fn new(id: String, state: ReconstructedState) -> Self {
        let node_count = state.total_nodes();
        let size_bytes = serde_json::to_vec(&state)
            .map(|b| b.len())
            .unwrap_or(0);

        Self {
            metadata: SnapshotMetadata {
                id,
                version: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                node_count,
                size_bytes,
            },
            state,
        }
    }

    /// Encode snapshot to bytes
    ///
    /// # Errors
    ///
    /// Returns error if encoding fails
    pub fn encode(&self) -> CoreResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CoreError::ParseError {
            message: format!("Failed to encode snapshot: {}", e),
        })
    }

    /// Decode snapshot from bytes
    ///
    /// # Errors
    ///
    /// Returns error if decoding fails
    pub fn decode(data: &[u8]) -> CoreResult<Self> {
        serde_json::from_slice(data).map_err(|e| CoreError::ParseError {
            message: format!("Failed to decode snapshot: {}", e),
        })
    }
}

/// Snapshot loader for loading snapshots during replay
pub struct SnapshotLoader {
    /// In-memory cache of snapshots
    cache: std::collections::HashMap<String, Snapshot>,
}

impl SnapshotLoader {
    /// Create a new snapshot loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Load a snapshot from bytes
    ///
    /// # Errors
    ///
    /// Returns error if loading fails
    pub fn load(&self, data: &[u8]) -> CoreResult<ReconstructedState> {
        let snapshot = Snapshot::decode(data)?;
        self.validate_version(&snapshot)?;
        Ok(snapshot.state)
    }

    /// Load a snapshot by ID
    ///
    /// # Errors
    ///
    /// Returns error if snapshot not found
    pub fn load_by_id(&self, id: &str) -> CoreResult<ReconstructedState> {
        self.cache
            .get(id)
            .map(|s| s.state.clone())
            .ok_or_else(|| SnapshotError::NotFound { id: id.to_string() }.into())
    }

    /// Register a snapshot in the cache
    pub fn register(&mut self, snapshot: Snapshot) {
        self.cache.insert(snapshot.metadata.id.clone(), snapshot);
    }

    /// Create a snapshot from state and register it
    pub fn snapshot(&mut self, id: String, state: ReconstructedState) {
        let snapshot = Snapshot::new(id, state);
        self.register(snapshot);
    }

    /// Validate snapshot version
    fn validate_version(&self, snapshot: &Snapshot) -> CoreResult<()> {
        if snapshot.metadata.version != 1 {
            return Err(SnapshotError::VersionMismatch {
                expected: 1,
                actual: snapshot.metadata.version,
            }
            .into());
        }
        Ok(())
    }

    /// Get cached snapshot IDs
    #[must_use]
    pub fn cached_ids(&self) -> Vec<String> {
        self.cache.keys().cloned().collect()
    }

    /// Remove a snapshot from cache
    pub fn remove(&mut self, id: &str) -> bool {
        self.cache.remove(id).is_some()
    }

    /// Clear all cached snapshots
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for SnapshotLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot writer for creating snapshots
pub struct SnapshotWriter;

impl SnapshotWriter {
    /// Create a new snapshot writer (unit struct)
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Write a snapshot to bytes
    ///
    /// # Errors
    ///
    /// Returns error if writing fails
    pub fn write(&self, id: String, state: &ReconstructedState) -> CoreResult<Vec<u8>> {
        let snapshot = Snapshot::new(id, state.clone());
        snapshot.encode()
    }

    /// Write a snapshot to a writer
    ///
    /// # Errors
    ///
    /// Returns error if writing fails
    pub fn write_to<W: std::io::Write>(
        &self,
        writer: &mut W,
        id: String,
        state: &ReconstructedState,
    ) -> CoreResult<()> {
        let bytes = self.write(id, state)?;
        writer.write_all(&bytes).map_err(|e| CoreError::ParseError {
            message: format!("Failed to write snapshot: {}", e),
        })?;
        Ok(())
    }
}

impl Default for SnapshotWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::NodeState;
    use cathedral_core::NodeId;

    #[test]
    fn test_snapshot_new() {
        let state = ReconstructedState::new();
        let snapshot = Snapshot::new("test".to_string(), state);
        assert_eq!(snapshot.metadata.id, "test");
        assert_eq!(snapshot.metadata.version, 1);
    }

    #[test]
    fn test_snapshot_encode_decode() {
        let state = ReconstructedState::new();
        let snapshot = Snapshot::new("test".to_string(), state);

        let encoded = snapshot.encode().unwrap();
        let decoded = Snapshot::decode(&encoded).unwrap();

        assert_eq!(decoded.metadata.id, snapshot.metadata.id);
    }

    #[test]
    fn test_snapshot_loader_new() {
        let loader = SnapshotLoader::new();
        assert!(loader.cached_ids().is_empty());
    }

    #[test]
    fn test_snapshot_loader_register() {
        let mut loader = SnapshotLoader::new();
        let state = ReconstructedState::new();
        let snapshot = Snapshot::new("test".to_string(), state);

        loader.register(snapshot);
        assert!(loader.cached_ids().contains(&"test".to_string()));
    }

    #[test]
    fn test_snapshot_loader_load_by_id() {
        let mut loader = SnapshotLoader::new();
        let mut state = ReconstructedState::new();
        let node_id = NodeId::new();
        state.add_node_state(node_id, NodeState::new(node_id));

        let snapshot = Snapshot::new("test".to_string(), state.clone());
        loader.register(snapshot);

        let loaded = loader.load_by_id("test").unwrap();
        assert_eq!(loaded.total_nodes(), 1);
    }

    #[test]
    fn test_snapshot_loader_load_by_id_not_found() {
        let loader = SnapshotLoader::new();
        let result = loader.load_by_id("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_loader_remove() {
        let mut loader = SnapshotLoader::new();
        let state = ReconstructedState::new();
        let snapshot = Snapshot::new("test".to_string(), state);

        loader.register(snapshot);
        assert!(loader.cached_ids().contains(&"test".to_string()));

        loader.remove("test");
        assert!(!loader.cached_ids().contains(&"test".to_string()));
    }

    #[test]
    fn test_snapshot_writer_new() {
        let writer = SnapshotWriter::new();
        // Unit struct - just verify it exists
        let _ = writer;
    }

    #[test]
    fn test_snapshot_writer_write() {
        let writer = SnapshotWriter::new();
        let state = ReconstructedState::new();

        let bytes = writer.write("test".to_string(), &state).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_snapshot_error_display() {
        let err = SnapshotError::NotFound { id: "test".to_string() };
        assert_eq!(err.to_string(), "Snapshot not found: test");
    }
}
