//! Snapshot storage for point-in-time state.

use crate::{BlobId, ContentStore};
use cathedral_core::{CoreResult, CoreError, EventId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Snapshot error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// Snapshot not found
    NotFound { id: String },
    /// Invalid snapshot data
    Invalid { reason: String },
    /// Blob missing
    MissingBlob { id: String },
    /// Version mismatch
    VersionMismatch { expected: u32, actual: u32 },
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "Snapshot not found: {}", id),
            Self::Invalid { reason } => write!(f, "Invalid snapshot: {}", reason),
            Self::MissingBlob { id } => write!(f, "Missing blob: {}", id),
            Self::VersionMismatch { expected, actual } => {
                write!(f, "Version mismatch: expected {}, got {}", expected, actual)
            }
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
    /// Timestamp when snapshot was created
    pub timestamp: u64,
    /// Parent snapshot ID (if any)
    pub parent_id: Option<String>,
    /// Event ID that triggered this snapshot
    pub event_id: Option<EventId>,
    /// Number of entries in snapshot
    pub entry_count: usize,
    /// Total size in bytes
    pub total_bytes: u64,
}

impl SnapshotMetadata {
    /// Create new metadata
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            id,
            version: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            parent_id: None,
            event_id: None,
            entry_count: 0,
            total_bytes: 0,
        }
    }

    /// With parent ID
    #[must_use]
    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// With event ID
    #[must_use]
    pub fn with_event(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
        self
    }
}

/// Snapshot entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Entry key
    pub key: String,
    /// Blob ID containing the value
    pub blob_id: BlobId,
    /// Entry size in bytes
    pub size: u64,
}

/// Snapshot containing point-in-time state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Metadata
    pub metadata: SnapshotMetadata,
    /// Snapshot entries
    pub entries: HashMap<String, SnapshotEntry>,
}

impl Snapshot {
    /// Create a new snapshot
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            metadata: SnapshotMetadata::new(id),
            entries: HashMap::new(),
        }
    }

    /// Create with parent
    #[must_use]
    pub fn with_parent(id: String, parent_id: String) -> Self {
        Self {
            metadata: SnapshotMetadata::new(id).with_parent(parent_id),
            entries: HashMap::new(),
        }
    }

    /// Add an entry to the snapshot
    pub fn add_entry(&mut self, key: String, blob_id: BlobId, size: u64) {
        let entry = SnapshotEntry { key: key.clone(), blob_id, size };
        self.entries.insert(key, entry);
        self.update_metadata();
    }

    /// Get an entry
    #[must_use]
    pub fn get_entry(&self, key: &str) -> Option<&SnapshotEntry> {
        self.entries.get(key)
    }

    /// Remove an entry
    pub fn remove_entry(&mut self, key: &str) -> Option<SnapshotEntry> {
        let entry = self.entries.remove(key);
        if entry.is_some() {
            self.update_metadata();
        }
        entry
    }

    /// Check if snapshot has a key
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Get all keys
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Get entry count
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total bytes
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.entries.values().map(|e| e.size).sum()
    }

    /// Update metadata to match current state
    fn update_metadata(&mut self) {
        self.metadata.entry_count = self.entries.len();
        self.metadata.total_bytes = self.total_bytes();
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

    /// Merge another snapshot into this one
    pub fn merge(&mut self, other: Snapshot) {
        for (key, entry) in other.entries {
            self.entries.insert(key, entry);
        }
        self.update_metadata();
    }
}

/// Builder for creating snapshots
pub struct SnapshotBuilder {
    snapshot: Snapshot,
}

impl SnapshotBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            snapshot: Snapshot::new(id),
        }
    }

    /// Set parent snapshot
    #[must_use]
    pub fn parent(mut self, parent_id: String) -> Self {
        self.snapshot.metadata.parent_id = Some(parent_id);
        self
    }

    /// Set event ID
    #[must_use]
    pub fn event(mut self, event_id: EventId) -> Self {
        self.snapshot.metadata.event_id = Some(event_id);
        self
    }

    /// Add an entry
    #[must_use]
    pub fn entry(mut self, key: String, blob_id: BlobId, size: u64) -> Self {
        self.snapshot.add_entry(key, blob_id, size);
        self
    }

    /// Add entries from an iterator
    pub fn entries<I>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = (String, BlobId, u64)>,
    {
        for (key, blob_id, size) in entries {
            self.snapshot.add_entry(key, blob_id, size);
        }
        self
    }

    /// Build the snapshot
    #[must_use]
    pub fn build(self) -> Snapshot {
        self.snapshot
    }
}

impl Default for SnapshotBuilder {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Snapshot store for managing snapshots
pub struct SnapshotStore {
    /// Content store for blob data
    content_store: Arc<ContentStore>,
    /// Snapshots indexed by ID
    snapshots: HashMap<String, Arc<Snapshot>>,
}

impl SnapshotStore {
    /// Create a new snapshot store
    #[must_use]
    pub fn new(content_store: Arc<ContentStore>) -> Self {
        Self {
            content_store,
            snapshots: HashMap::new(),
        }
    }

    /// Create a snapshot
    ///
    /// # Errors
    ///
    /// Returns error if snapshot creation fails
    pub fn create(&mut self, snapshot: Snapshot) -> CoreResult<String> {
        let id = snapshot.metadata.id.clone();

        // Verify all blobs exist
        for entry in snapshot.entries.values() {
            if !self.content_store.contains(&entry.blob_id) {
                return Err(SnapshotError::MissingBlob {
                    id: entry.blob_id.to_string(),
                }
                .into());
            }
        }

        self.snapshots.insert(id.clone(), Arc::new(snapshot));
        Ok(id)
    }

    /// Get a snapshot
    ///
    /// # Errors
    ///
    /// Returns error if snapshot not found
    pub fn get(&self, id: &str) -> CoreResult<Arc<Snapshot>> {
        self.snapshots
            .get(id)
            .cloned()
            .ok_or_else(|| SnapshotError::NotFound { id: id.to_string() }.into())
    }

    /// Delete a snapshot
    pub fn delete(&mut self, id: &str) -> bool {
        self.snapshots.remove(id).is_some()
    }

    /// List all snapshot IDs
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        self.snapshots.keys().cloned().collect()
    }

    /// Get snapshot count
    #[must_use]
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// Create a snapshot from state entries
    ///
    /// # Errors
    ///
    /// Returns error if snapshot creation fails
    pub fn snapshot_from(
        &mut self,
        id: String,
        entries: HashMap<String, Vec<u8>>,
    ) -> CoreResult<String> {
        let mut snapshot = Snapshot::new(id.clone());

        for (key, data) in entries {
            let blob_id = self.content_store.write(data)?;
            let size = blob_id.hash.as_bytes().len() as u64;
            snapshot.add_entry(key, blob_id, size);
        }

        self.create(snapshot)
    }

    /// Restore state from snapshot
    ///
    /// # Errors
    ///
    /// Returns error if restoration fails
    pub fn restore(&self, id: &str) -> CoreResult<HashMap<String, Vec<u8>>> {
        let snapshot = self.get(id)?;
        let mut state = HashMap::new();

        for (key, entry) in &snapshot.entries {
            let blob = self.content_store.read(&entry.blob_id)?;
            state.insert(key.clone(), blob.as_bytes().to_vec());
        }

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::ContentAddress;

    #[test]
    fn test_snapshot_new() {
        let snapshot = Snapshot::new("test".to_string());
        assert_eq!(snapshot.metadata.id, "test");
        assert_eq!(snapshot.entry_count(), 0);
    }

    #[test]
    fn test_snapshot_with_parent() {
        let snapshot = Snapshot::with_parent("child".to_string(), "parent".to_string());
        assert_eq!(snapshot.metadata.parent_id, Some("parent".to_string()));
    }

    #[test]
    fn test_snapshot_add_entry() {
        let mut snapshot = Snapshot::new("test".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot.add_entry("key".to_string(), blob_id, 4);

        assert_eq!(snapshot.entry_count(), 1);
        assert!(snapshot.contains_key("key"));
    }

    #[test]
    fn test_snapshot_get_entry() {
        let mut snapshot = Snapshot::new("test".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot.add_entry("key".to_string(), blob_id, 4);

        let entry = snapshot.get_entry("key").unwrap();
        assert_eq!(entry.key, "key");
    }

    #[test]
    fn test_snapshot_remove_entry() {
        let mut snapshot = Snapshot::new("test".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot.add_entry("key".to_string(), blob_id, 4);

        let removed = snapshot.remove_entry("key");
        assert!(removed.is_some());
        assert!(!snapshot.contains_key("key"));
    }

    #[test]
    fn test_snapshot_keys() {
        let mut snapshot = Snapshot::new("test".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot.add_entry("key1".to_string(), blob_id, 4);
        snapshot.add_entry("key2".to_string(), blob_id, 4);

        let keys = snapshot.keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_snapshot_encode_decode() {
        let mut snapshot = Snapshot::new("test".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot.add_entry("key".to_string(), blob_id, 4);

        let encoded = snapshot.encode().unwrap();
        let decoded = Snapshot::decode(&encoded).unwrap();

        assert_eq!(decoded.metadata.id, snapshot.metadata.id);
        assert_eq!(decoded.entry_count(), snapshot.entry_count());
    }

    #[test]
    fn test_snapshot_merge() {
        let mut snapshot1 = Snapshot::new("snap1".to_string());
        let blob_id = ContentAddress::compute(b"data");
        snapshot1.add_entry("key1".to_string(), blob_id, 4);

        let mut snapshot2 = Snapshot::new("snap2".to_string());
        snapshot2.add_entry("key2".to_string(), blob_id, 4);

        snapshot1.merge(snapshot2);
        assert_eq!(snapshot1.entry_count(), 2);
    }

    #[test]
    fn test_snapshot_builder() {
        let blob_id = ContentAddress::compute(b"data");
        let snapshot = SnapshotBuilder::new("test".to_string())
            .parent("parent".to_string())
            .entry("key".to_string(), blob_id, 4)
            .build();

        assert_eq!(snapshot.metadata.id, "test");
        assert_eq!(snapshot.metadata.parent_id, Some("parent".to_string()));
        assert_eq!(snapshot.entry_count(), 1);
    }

    #[test]
    fn test_snapshot_metadata_new() {
        let metadata = SnapshotMetadata::new("test".to_string());
        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.version, 1);
    }

    #[test]
    fn test_snapshot_error_display() {
        let err = SnapshotError::NotFound { id: "test".to_string() };
        assert!(err.to_string().contains("not found"));
    }
}
