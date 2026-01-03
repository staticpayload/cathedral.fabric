//! Content-addressed blob store.

use crate::{Blob, BlobData, BlobId, address::ContentAddress};
use cathedral_core::{CoreResult, CoreError, EventId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Store configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Maximum blob size in bytes (0 = unlimited)
    pub max_blob_size: usize,
    /// Maximum total storage in bytes (0 = unlimited)
    pub max_storage: usize,
    /// Enable compression
    pub compression: bool,
    /// Storage directory
    pub storage_dir: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_blob_size: 100 * 1024 * 1024, // 100 MB
            max_storage: 10 * 1024 * 1024 * 1024, // 10 GB
            compression: true,
            storage_dir: ".cathedral/storage".to_string(),
        }
    }
}

/// Store error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Blob not found
    NotFound { id: String },
    /// Blob too large
    BlobTooLarge { size: usize, limit: usize },
    /// Storage full
    StorageFull,
    /// Invalid blob
    InvalidBlob { reason: String },
    /// IO error
    Io { reason: String },
    /// Serialization error
    Serialization { reason: String },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "Blob not found: {}", id),
            Self::BlobTooLarge { size, limit } => {
                write!(f, "Blob too large: {} bytes (limit: {})", size, limit)
            }
            Self::StorageFull => write!(f, "Storage full"),
            Self::InvalidBlob { reason } => write!(f, "Invalid blob: {}", reason),
            Self::Io { reason } => write!(f, "IO error: {}", reason),
            Self::Serialization { reason } => write!(f, "Serialization error: {}", reason),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<StoreError> for CoreError {
    fn from(err: StoreError) -> Self {
        CoreError::Validation {
            field: "store".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Store statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreStats {
    /// Total number of blobs
    pub blob_count: usize,
    /// Total bytes stored
    pub total_bytes: u64,
    /// Number of reads
    pub read_count: u64,
    /// Number of writes
    pub write_count: u64,
}

impl Default for StoreStats {
    fn default() -> Self {
        Self {
            blob_count: 0,
            total_bytes: 0,
            read_count: 0,
            write_count: 0,
        }
    }
}

/// In-memory content store
pub struct ContentStore {
    /// Store configuration
    config: StoreConfig,
    /// Blob storage indexed by content address
    blobs: RwLock<HashMap<BlobId, Arc<Blob>>>,
    /// Store statistics
    stats: RwLock<StoreStats>,
}

impl ContentStore {
    /// Create a new content store
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(StoreConfig::default())
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(config: StoreConfig) -> Self {
        Self {
            config,
            blobs: RwLock::new(HashMap::new()),
            stats: RwLock::new(StoreStats::default()),
        }
    }

    /// Write a blob to the store
    ///
    /// # Errors
    ///
    /// Returns error if write fails
    pub fn write(&self, data: Vec<u8>) -> CoreResult<BlobId> {
        self.write_with_type(data, None)
    }

    /// Write a blob with content type
    ///
    /// # Errors
    ///
    /// Returns error if write fails
    pub fn write_with_type(&self, data: Vec<u8>, content_type: Option<String>) -> CoreResult<BlobId> {
        // Store size before moving data
        let data_size = data.len();

        // Check blob size
        if self.config.max_blob_size > 0 && data_size > self.config.max_blob_size {
            return Err(StoreError::BlobTooLarge {
                size: data_size,
                limit: self.config.max_blob_size,
            }
            .into());
        }

        // Create blob
        let blob = if let Some(ct) = content_type {
            Blob::with_type(data, ct)
        } else {
            Blob::new(data)
        };

        let id = blob.id();

        // Check storage limit
        if self.config.max_storage > 0 {
            let stats = self.stats.read().unwrap();
            let new_size = stats.total_bytes + blob.size() as u64;
            if new_size > self.config.max_storage as u64 {
                return Err(StoreError::StorageFull.into());
            }
        }

        // Insert blob
        {
            let mut blobs = self.blobs.write().unwrap();
            // Only increment stats if this is a new blob
            let is_new = !blobs.contains_key(&id);
            blobs.insert(id, Arc::new(blob));

            if is_new {
                let mut stats = self.stats.write().unwrap();
                stats.blob_count += 1;
                stats.total_bytes += data_size as u64;
                stats.write_count += 1;
            }
        }

        Ok(id)
    }

    /// Read a blob from the store
    ///
    /// # Errors
    ///
    /// Returns error if blob not found
    pub fn read(&self, id: &BlobId) -> CoreResult<Arc<Blob>> {
        let blobs = self.blobs.read().unwrap();
        let blob = blobs
            .get(id)
            .ok_or_else(|| StoreError::NotFound {
                id: id.to_string(),
            })
            .map_err(|e| CoreError::Validation {
                field: "blob".to_string(),
                reason: e.to_string(),
            })?;

        // Update read stats
        drop(blobs);
        let mut stats = self.stats.write().unwrap();
        stats.read_count += 1;

        // Return the blob
        let blobs = self.blobs.read().unwrap();
        Ok(blobs.get(id).cloned().unwrap())
    }

    /// Check if a blob exists
    #[must_use]
    pub fn contains(&self, id: &BlobId) -> bool {
        self.blobs.read().unwrap().contains_key(id)
    }

    /// Delete a blob from the store
    ///
    /// # Errors
    ///
    /// Returns error if deletion fails
    pub fn delete(&self, id: &BlobId) -> CoreResult<bool> {
        let mut blobs = self.blobs.write().unwrap();
        if let Some(blob) = blobs.remove(id) {
            let mut stats = self.stats.write().unwrap();
            stats.blob_count -= 1;
            stats.total_bytes -= blob.size() as u64;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get store statistics
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        self.stats.read().unwrap().clone()
    }

    /// List all blob IDs
    #[must_use]
    pub fn list(&self) -> Vec<BlobId> {
        self.blobs
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// Clear all blobs from the store
    pub fn clear(&self) {
        self.blobs.write().unwrap().clear();
        *self.stats.write().unwrap() = StoreStats::default();
    }

    /// Get total blob count
    #[must_use]
    pub fn count(&self) -> usize {
        self.stats.read().unwrap().blob_count
    }

    /// Get total storage used
    #[must_use]
    pub fn size(&self) -> u64 {
        self.stats.read().unwrap().total_bytes
    }
}

impl Default for ContentStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistent content store backed by filesystem
pub struct FsContentStore {
    /// In-memory store
    memory: ContentStore,
    /// Storage directory
    dir: String,
}

impl FsContentStore {
    /// Create a new filesystem-backed store
    ///
    /// # Errors
    ///
    /// Returns error if directory creation fails
    pub fn new(dir: String) -> CoreResult<Self> {
        std::fs::create_dir_all(&dir).map_err(|e| CoreError::Validation {
            field: "storage_dir".to_string(),
            reason: format!("Failed to create storage directory: {}", e),
        })?;

        Ok(Self {
            memory: ContentStore::new(),
            dir,
        })
    }

    /// Write a blob to persistent storage
    ///
    /// # Errors
    ///
    /// Returns error if write fails
    pub fn write(&self, data: Vec<u8>) -> CoreResult<BlobId> {
        let id = self.memory.write(data.clone())?;
        let path = self.blob_path(&id);

        std::fs::write(&path, data).map_err(|e| CoreError::Validation {
            field: "write".to_string(),
            reason: format!("Failed to write blob: {}", e),
        })?;

        Ok(id)
    }

    /// Read a blob from persistent storage
    ///
    /// # Errors
    ///
    /// Returns error if read fails
    pub fn read(&self, id: &BlobId) -> CoreResult<Arc<Blob>> {
        // Check memory first
        if self.memory.contains(id) {
            return self.memory.read(id);
        }

        // Load from disk
        let path = self.blob_path(id);
        let data = std::fs::read(&path).map_err(|e| CoreError::Validation {
            field: "read".to_string(),
            reason: format!("Failed to read blob: {}", e),
        })?;

        // Insert into memory and return
        self.memory.write(data)?;
        self.memory.read(id)
    }

    /// Get blob file path
    fn blob_path(&self, id: &BlobId) -> String {
        let hex = id.hash.to_hex();
        format!("{}/{}.blob", self.dir, hex)
    }

    /// Get store statistics
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        self.memory.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_config_default() {
        let config = StoreConfig::default();
        assert_eq!(config.max_blob_size, 100 * 1024 * 1024);
        assert!(config.compression);
    }

    #[test]
    fn test_store_new() {
        let store = ContentStore::new();
        assert_eq!(store.count(), 0);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn test_store_write_read() {
        let store = ContentStore::new();
        let data = b"hello world".to_vec();
        let id = store.write(data.clone()).unwrap();

        let blob = store.read(&id).unwrap();
        assert_eq!(blob.as_bytes(), &data[..]);
    }

    #[test]
    fn test_store_contains() {
        let store = ContentStore::new();
        let data = b"test".to_vec();
        let id = store.write(data).unwrap();

        assert!(store.contains(&id));
        assert!(!store.contains(&ContentAddress::compute(b"other")));
    }

    #[test]
    fn test_store_delete() {
        let store = ContentStore::new();
        let data = b"test".to_vec();
        let id = store.write(data).unwrap();

        assert!(store.delete(&id).unwrap());
        assert!(!store.contains(&id));
        assert!(!store.delete(&id).unwrap());
    }

    #[test]
    fn test_store_stats() {
        let store = ContentStore::new();
        store.write(b"data".to_vec()).unwrap();

        let stats = store.stats();
        assert_eq!(stats.blob_count, 1);
        assert_eq!(stats.total_bytes, 4);
        assert_eq!(stats.write_count, 1);
    }

    #[test]
    fn test_store_blob_too_large() {
        let config = StoreConfig {
            max_blob_size: 10,
            ..Default::default()
        };
        let store = ContentStore::with_config(config);
        let result = store.write(vec![0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_clear() {
        let store = ContentStore::new();
        store.write(b"test".to_vec()).unwrap();
        store.clear();

        assert_eq!(store.count(), 0);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn test_store_list() {
        let store = ContentStore::new();
        let id1 = store.write(b"test1".to_vec()).unwrap();
        let id2 = store.write(b"test2".to_vec()).unwrap();

        let ids = store.list();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_store_error_display() {
        let err = StoreError::NotFound {
            id: "test".to_string(),
        };
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_store_write_duplicate() {
        let store = ContentStore::new();
        let data = b"duplicate".to_vec();

        let id1 = store.write(data.clone()).unwrap();
        let id2 = store.write(data).unwrap();

        assert_eq!(id1, id2);
        // Stats should only count unique blobs
        assert_eq!(store.stats().blob_count, 1);
    }
}
