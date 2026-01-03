//! Blob storage primitives.

use crate::address::ContentAddress;
use cathedral_core::{Hash, CoreResult, CoreError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unique identifier for a blob
pub type BlobId = ContentAddress;

/// Raw blob data with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobData {
    /// Content address
    pub address: ContentAddress,
    /// Raw data bytes
    pub data: Vec<u8>,
    /// Size in bytes
    pub size: usize,
    /// Content type hint
    pub content_type: Option<String>,
}

impl BlobData {
    /// Create new blob data
    #[must_use]
    pub fn new(data: Vec<u8>, content_type: Option<String>) -> Self {
        let size = data.len();
        let address = ContentAddress::compute(&data);
        Self {
            address,
            data,
            size,
            content_type,
        }
    }

    /// Create blob data with explicit content type
    #[must_use]
    pub fn with_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Verify the content address matches the data
    ///
    /// # Errors
    ///
    /// Returns error if address doesn't match data
    pub fn verify(&self) -> CoreResult<()> {
        let computed = ContentAddress::compute(&self.data);
        if computed != self.address {
            return Err(CoreError::Validation {
                field: "address".to_string(),
                reason: "Content address mismatch".to_string(),
            });
        }
        Ok(())
    }

    /// Get blob ID
    #[must_use]
    pub const fn id(&self) -> &BlobId {
        &self.address
    }

    /// Get data slice
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Check if blob is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// A shared blob reference
#[derive(Debug, Clone)]
pub struct Blob {
    /// Blob data
    inner: Arc<BlobData>,
}

impl Serialize for Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Blob {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = BlobData::deserialize(deserializer)?;
        Ok(Self {
            inner: Arc::new(data),
        })
    }
}

impl Blob {
    /// Create a new blob from data
    #[must_use]
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: Arc::new(BlobData::new(data, None)),
        }
    }

    /// Create a new blob with content type
    #[must_use]
    pub fn with_type(data: Vec<u8>, content_type: String) -> Self {
        Self {
            inner: Arc::new(BlobData::new(data, Some(content_type))),
        }
    }

    /// Create from existing blob data
    #[must_use]
    pub fn from_data(data: BlobData) -> Self {
        Self {
            inner: Arc::new(data),
        }
    }

    /// Get blob ID
    #[must_use]
    pub fn id(&self) -> BlobId {
        self.inner.address
    }

    /// Get content address
    #[must_use]
    pub fn address(&self) -> ContentAddress {
        self.inner.address
    }

    /// Get data bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner.data
    }

    /// Get data size
    #[must_use]
    pub fn size(&self) -> usize {
        self.inner.size
    }

    /// Get content type
    #[must_use]
    pub fn content_type(&self) -> Option<&String> {
        self.inner.content_type.as_ref()
    }

    /// Clone the inner data
    #[must_use]
    pub fn to_data(&self) -> BlobData {
        (*self.inner).clone()
    }

    /// Verify the blob
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify(&self) -> CoreResult<()> {
        self.inner.verify()
    }

    /// Get reference count
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }
}

impl PartialEq for Blob {
    fn eq(&self, other: &Self) -> bool {
        self.inner.address == other.inner.address
    }
}

impl Eq for Blob {}

impl std::hash::Hash for Blob {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.address.hash(state);
    }
}

impl From<Vec<u8>> for Blob {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for Blob {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl AsRef<[u8]> for Blob {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_new() {
        let blob = Blob::new(b"hello".to_vec());
        assert_eq!(blob.size(), 5);
        assert_eq!(blob.as_bytes(), b"hello");
    }

    #[test]
    fn test_blob_with_type() {
        let blob = Blob::with_type(b"data".to_vec(), "text/plain".to_string());
        assert_eq!(blob.content_type(), Some(&"text/plain".to_string()));
    }

    #[test]
    fn test_blob_id() {
        let blob = Blob::new(b"test".to_vec());
        let id = blob.id();
        assert_eq!(id, blob.address());
    }

    #[test]
    fn test_blob_verify() {
        let blob = Blob::new(b"verify me".to_vec());
        assert!(blob.verify().is_ok());
    }

    #[test]
    fn test_blob_from_slice() {
        let data: &[u8] = b"slice";
        let blob = Blob::from(data);
        assert_eq!(blob.as_bytes(), b"slice");
    }

    #[test]
    fn test_blob_equality() {
        let data = b"same data";
        let blob1 = Blob::new(data.to_vec());
        let blob2 = Blob::new(data.to_vec());
        assert_eq!(blob1, blob2);
    }

    #[test]
    fn test_blob_data_new() {
        let blob_data = BlobData::new(b"test".to_vec(), Some("application/json".to_string()));
        assert_eq!(blob_data.size, 4);
        assert_eq!(blob_data.content_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_blob_data_verify() {
        let blob_data = BlobData::new(b"verify".to_vec(), None);
        assert!(blob_data.verify().is_ok());
    }

    #[test]
    fn test_blob_data_is_empty() {
        let empty = BlobData::new(vec![], None);
        assert!(empty.is_empty());

        let non_empty = BlobData::new(b"x".to_vec(), None);
        assert!(!non_empty.is_empty());
    }
}
