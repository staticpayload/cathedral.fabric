//! Storage compaction for reclaiming space.

use crate::{BlobId, ContentStore};
use cathedral_core::{CoreResult, CoreError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Compaction plan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactPlan {
    /// Blobs to keep (referenced by snapshots)
    pub keep: HashSet<BlobId>,
    /// Blobs to delete (unreferenced)
    pub delete: HashSet<BlobId>,
    /// Bytes that would be reclaimed
    pub reclaim_bytes: u64,
    /// Number of blobs to delete
    pub delete_count: usize,
}

impl CompactPlan {
    /// Create a new empty compaction plan
    #[must_use]
    pub fn new() -> Self {
        Self {
            keep: HashSet::new(),
            delete: HashSet::new(),
            reclaim_bytes: 0,
            delete_count: 0,
        }
    }

    /// Check if plan is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.delete.is_empty()
    }

    /// Get number of blobs to keep
    #[must_use]
    pub fn keep_count(&self) -> usize {
        self.keep.len()
    }

    /// Update plan statistics
    pub fn update_stats(&mut self, blob_sizes: &HashMap<BlobId, usize>) {
        self.delete_count = self.delete.len();
        self.reclaim_bytes = self
            .delete
            .iter()
            .filter_map(|id| blob_sizes.get(id))
            .map(|&size| size as u64)
            .sum();
    }
}

impl Default for CompactPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Compaction result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactResult {
    /// Number of blobs deleted
    pub deleted_count: usize,
    /// Bytes reclaimed
    pub reclaimed_bytes: u64,
    /// Number of blobs kept
    pub kept_count: usize,
    /// Number of errors during compaction
    pub error_count: usize,
}

impl CompactResult {
    /// Create a new result
    #[must_use]
    pub fn new() -> Self {
        Self {
            deleted_count: 0,
            reclaimed_bytes: 0,
            kept_count: 0,
            error_count: 0,
        }
    }

    /// Check if compaction was successful
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.error_count == 0
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: CompactResult) {
        self.deleted_count += other.deleted_count;
        self.reclaimed_bytes += other.reclaimed_bytes;
        self.kept_count += other.kept_count;
        self.error_count += other.error_count;
    }
}

impl Default for CompactResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Compactor for storage cleanup
pub struct Compactor {
    /// Content store to compact
    store: ContentStore,
}

impl Compactor {
    /// Create a new compactor
    #[must_use]
    pub fn new(store: ContentStore) -> Self {
        Self { store }
    }

    /// Analyze storage to create a compaction plan
    ///
    /// # Errors
    ///
    /// Returns error if analysis fails
    pub fn analyze(&self, referenced: &HashSet<BlobId>) -> CoreResult<CompactPlan> {
        let all_blobs: HashSet<BlobId> = self.store.list().into_iter().collect();

        let keep = referenced.clone();
        let delete: HashSet<BlobId> = all_blobs.difference(&keep).cloned().collect();

        let mut plan = CompactPlan {
            keep,
            delete,
            ..Default::default()
        };

        // Calculate sizes
        let mut blob_sizes = HashMap::new();
        for blob_id in &plan.delete {
            if let Ok(blob) = self.store.read(blob_id) {
                blob_sizes.insert(*blob_id, blob.size());
            }
        }
        plan.update_stats(&blob_sizes);

        Ok(plan)
    }

    /// Execute a compaction plan
    ///
    /// # Errors
    ///
    /// Returns error if compaction fails
    pub fn compact(&self, plan: &CompactPlan) -> CoreResult<CompactResult> {
        let mut result = CompactResult::new();
        result.kept_count = plan.keep_count();

        for blob_id in &plan.delete {
            match self.store.delete(blob_id) {
                Ok(true) => {
                    result.deleted_count += 1;
                }
                Ok(false) => {
                    // Already deleted
                }
                Err(_) => {
                    result.error_count += 1;
                }
            }
        }

        result.reclaimed_bytes = plan.reclaim_bytes;
        Ok(result)
    }

    /// Analyze and compact in one step
    ///
    /// # Errors
    ///
    /// Returns error if compaction fails
    pub fn compact_referenced(
        &self,
        referenced: &HashSet<BlobId>,
    ) -> CoreResult<CompactResult> {
        let plan = self.analyze(referenced)?;
        self.compact(&plan)
    }

    /// Compact all unreferenced blobs
    ///
    /// # Errors
    ///
    /// Returns error if compaction fails
    pub fn compact_all(&self) -> CoreResult<CompactResult> {
        self.compact_referenced(&HashSet::new())
    }

    /// Get current store statistics
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        StoreStats {
            blob_count: self.store.count(),
            total_bytes: self.store.size(),
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
}

impl StoreStats {
    /// Create new statistics
    #[must_use]
    pub fn new(blob_count: usize, total_bytes: u64) -> Self {
        Self {
            blob_count,
            total_bytes,
        }
    }

    /// Calculate potential space savings
    #[must_use]
    pub fn potential_savings(&self, plan: &CompactPlan) -> u64 {
        plan.reclaim_bytes
    }

    /// Get estimated space after compaction
    #[must_use]
    pub fn after_compaction(&self, plan: &CompactPlan) -> u64 {
        self.total_bytes.saturating_sub(plan.reclaim_bytes)
    }
}

impl Default for StoreStats {
    fn default() -> Self {
        Self {
            blob_count: 0,
            total_bytes: 0,
        }
    }
}

/// Reference tracker for tracking blob references
pub struct ReferenceTracker {
    /// References indexed by blob ID
    references: HashMap<BlobId, usize>,
}

impl ReferenceTracker {
    /// Create a new reference tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
        }
    }

    /// Add a reference to a blob
    pub fn add_reference(&mut self, blob_id: BlobId) {
        *self.references.entry(blob_id).or_insert(0) += 1;
    }

    /// Remove a reference to a blob
    pub fn remove_reference(&mut self, blob_id: &BlobId) {
        if let Some(count) = self.references.get_mut(blob_id) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.references.remove(blob_id);
            }
        }
    }

    /// Get reference count for a blob
    #[must_use]
    pub fn ref_count(&self, blob_id: &BlobId) -> usize {
        self.references.get(blob_id).copied().unwrap_or(0)
    }

    /// Check if a blob is referenced
    #[must_use]
    pub fn is_referenced(&self, blob_id: &BlobId) -> bool {
        self.references.contains_key(blob_id)
    }

    /// Get all referenced blob IDs
    #[must_use]
    pub fn referenced_blobs(&self) -> HashSet<BlobId> {
        self.references.keys().cloned().collect()
    }

    /// Clear all references
    pub fn clear(&mut self) {
        self.references.clear();
    }

    /// Get total reference count
    #[must_use]
    pub fn total_references(&self) -> usize {
        self.references.values().sum()
    }
}

impl Default for ReferenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::ContentAddress;

    #[test]
    fn test_compact_plan_new() {
        let plan = CompactPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.keep_count(), 0);
        assert_eq!(plan.delete_count, 0);
    }

    #[test]
    fn test_compact_result_new() {
        let result = CompactResult::new();
        assert!(result.is_success());
        assert_eq!(result.deleted_count, 0);
    }

    #[test]
    fn test_compact_result_merge() {
        let mut result1 = CompactResult::new();
        let mut result2 = CompactResult::new();
        result1.deleted_count = 5;
        result2.deleted_count = 3;

        result1.merge(result2);
        assert_eq!(result1.deleted_count, 8);
    }

    #[test]
    fn test_compactor_analyze() {
        let store = ContentStore::new();
        let blob_id1 = store.write(b"data1".to_vec()).unwrap();
        let blob_id2 = store.write(b"data2".to_vec()).unwrap();

        let compactor = Compactor::new(store);
        let mut referenced = HashSet::new();
        referenced.insert(blob_id1);

        let plan = compactor.analyze(&referenced).unwrap();
        assert!(plan.keep.contains(&blob_id1));
        assert!(!plan.delete.contains(&blob_id1));
    }

    #[test]
    fn test_compactor_compact() {
        let store = ContentStore::new();
        let blob_id1 = store.write(b"data1".to_vec()).unwrap();
        let blob_id2 = store.write(b"data2".to_vec()).unwrap();

        let compactor = Compactor::new(store);
        let mut referenced = HashSet::new();
        referenced.insert(blob_id1);

        let plan = compactor.analyze(&referenced).unwrap();
        let result = compactor.compact(&plan).unwrap();

        assert_eq!(result.kept_count, 1);
        assert_eq!(result.deleted_count, 1);
        assert!(result.is_success());
    }

    #[test]
    fn test_compactor_stats() {
        let store = ContentStore::new();
        store.write(b"data".to_vec()).unwrap();

        let compactor = Compactor::new(store);
        let stats = compactor.stats();

        assert_eq!(stats.blob_count, 1);
        assert_eq!(stats.total_bytes, 4);
    }

    #[test]
    fn test_store_stats() {
        let stats = StoreStats::new(10, 1000);
        assert_eq!(stats.blob_count, 10);
        assert_eq!(stats.total_bytes, 1000);

        let mut plan = CompactPlan::new();
        plan.reclaim_bytes = 500;

        assert_eq!(stats.potential_savings(&plan), 500);
        assert_eq!(stats.after_compaction(&plan), 500);
    }

    #[test]
    fn test_reference_tracker() {
        let mut tracker = ReferenceTracker::new();
        let blob_id = ContentAddress::compute(b"data");

        assert_eq!(tracker.ref_count(&blob_id), 0);

        tracker.add_reference(blob_id);
        assert_eq!(tracker.ref_count(&blob_id), 1);
        assert!(tracker.is_referenced(&blob_id));

        tracker.add_reference(blob_id);
        assert_eq!(tracker.ref_count(&blob_id), 2);

        tracker.remove_reference(&blob_id);
        assert_eq!(tracker.ref_count(&blob_id), 1);

        tracker.remove_reference(&blob_id);
        assert_eq!(tracker.ref_count(&blob_id), 0);
        assert!(!tracker.is_referenced(&blob_id));
    }

    #[test]
    fn test_reference_tracker_referenced_blobs() {
        let mut tracker = ReferenceTracker::new();
        let blob_id1 = ContentAddress::compute(b"data1");
        let blob_id2 = ContentAddress::compute(b"data2");

        tracker.add_reference(blob_id1);
        tracker.add_reference(blob_id2);

        let referenced = tracker.referenced_blobs();
        assert_eq!(referenced.len(), 2);
        assert!(referenced.contains(&blob_id1));
        assert!(referenced.contains(&blob_id2));
    }

    #[test]
    fn test_reference_tracker_clear() {
        let mut tracker = ReferenceTracker::new();
        let blob_id = ContentAddress::compute(b"data");

        tracker.add_reference(blob_id);
        tracker.clear();

        assert_eq!(tracker.ref_count(&blob_id), 0);
        assert_eq!(tracker.total_references(), 0);
    }
}
