//! Memory limits for WASM execution.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Memory limit configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLimit {
    /// Maximum memory in bytes
    pub max_bytes: u64,
    /// Initial memory in bytes
    pub initial_bytes: u64,
    /// Memory page size (WASM default is 64KB)
    pub page_size: u64,
    /// Maximum number of pages
    pub max_pages: u64,
}

impl MemoryLimit {
    /// Create a new memory limit
    #[must_use]
    pub fn new(max_bytes: u64) -> Self {
        let page_size = 65536; // 64KB WASM page size
        let max_pages = (max_bytes / page_size) + if max_bytes % page_size != 0 { 1 } else { 0 };

        Self {
            max_bytes,
            initial_bytes: max_bytes.min(65536), // Start with 1 page
            page_size,
            max_pages,
        }
    }

    /// Create with WASM standard 64KB pages
    #[must_use]
    pub fn with_pages(max_pages: u64) -> Self {
        let page_size = 65536;
        Self {
            max_bytes: max_pages * page_size,
            initial_bytes: page_size,
            page_size,
            max_pages,
        }
    }

    /// Get maximum pages
    #[must_use]
    pub fn max_pages(&self) -> u64 {
        self.max_pages
    }

    /// Convert bytes to pages
    #[must_use]
    pub fn bytes_to_pages(&self, bytes: u64) -> u64 {
        (bytes / self.page_size) + if bytes % self.page_size != 0 { 1 } else { 0 }
    }

    /// Convert pages to bytes
    #[must_use]
    pub fn pages_to_bytes(&self, pages: u64) -> u64 {
        pages * self.page_size
    }

    /// Check if byte count is within limit
    #[must_use]
    pub fn within_limit(&self, bytes: u64) -> bool {
        bytes <= self.max_bytes
    }

    /// Check if page count is within limit
    #[must_use]
    pub fn pages_within_limit(&self, pages: u64) -> bool {
        pages <= self.max_pages
    }
}

impl Default for MemoryLimit {
    fn default() -> Self {
        Self::new(16 * 1024 * 1024) // 16MB default
    }
}

/// A memory region with bounds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Region start address
    pub start: u64,
    /// Region end address (exclusive)
    pub end: u64,
    /// Region name for debugging
    pub name: String,
    /// Whether this region is read-only
    pub read_only: bool,
}

impl MemoryRegion {
    /// Create a new memory region
    #[must_use]
    pub fn new(start: u64, size: u64, name: String) -> Self {
        Self {
            start,
            end: start + size,
            name,
            read_only: false,
        }
    }

    /// Create a read-only region
    #[must_use]
    pub fn read_only(start: u64, size: u64, name: String) -> Self {
        Self {
            start,
            end: start + size,
            name,
            read_only: true,
        }
    }

    /// Get region size
    #[must_use]
    pub fn size(&self) -> u64 {
        self.end - self.start
    }

    /// Check if an address is within this region
    #[must_use]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Check if a range overlaps this region
    #[must_use]
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        start < self.end && end > self.start
    }

    /// Check if write is allowed to this region
    #[must_use]
    pub fn can_write(&self) -> bool {
        !self.read_only
    }
}

/// Memory region map for tracking allocations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRegionMap {
    /// All regions keyed by start address
    regions: BTreeMap<u64, MemoryRegion>,
    /// Total bytes allocated
    total_bytes: u64,
}

impl MemoryRegionMap {
    /// Create a new empty region map
    #[must_use]
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
            total_bytes: 0,
        }
    }

    /// Add a region
    ///
    /// # Errors
    ///
    /// Returns error if region overlaps existing regions
    pub fn add_region(&mut self, region: MemoryRegion) -> Result<(), MemoryError> {
        // Check for overlaps
        for existing in self.regions.values() {
            if region.overlaps(existing.start, existing.end) {
                return Err(MemoryError::RegionOverlap {
                    region: region.name.clone(),
                    existing: existing.name.clone(),
                });
            }
        }

        self.total_bytes += region.size();
        self.regions.insert(region.start, region);
        Ok(())
    }

    /// Remove a region by start address
    pub fn remove_region(&mut self, start: u64) -> Option<MemoryRegion> {
        if let Some(region) = self.regions.remove(&start) {
            self.total_bytes -= region.size();
            Some(region)
        } else {
            None
        }
    }

    /// Find region containing an address
    #[must_use]
    pub fn find_region(&self, addr: u64) -> Option<&MemoryRegion> {
        self.regions
            .values()
            .find(|r| r.contains(addr))
    }

    /// Check if write is allowed to an address
    #[must_use]
    pub fn can_write(&self, addr: u64) -> bool {
        self.find_region(addr)
            .map(|r| r.can_write())
            .unwrap_or(false)
    }

    /// Get total allocated bytes
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Get region count
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get all regions
    #[must_use]
    pub fn regions(&self) -> impl Iterator<Item = &MemoryRegion> {
        self.regions.values()
    }
}

impl Default for MemoryRegionMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MemoryError {
    /// Out of memory
    #[error("Out of memory: requested {requested} bytes, {available} bytes available")]
    OutOfMemory { requested: u64, available: u64 },

    /// Invalid address
    #[error("Invalid memory address: 0x{address:X}")]
    InvalidAddress { address: u64 },

    /// Access violation
    #[error("Access violation at 0x{address:X}: {reason}")]
    AccessViolation { address: u64, reason: String },

    /// Region overlap
    #[error("Memory region '{region}' overlaps '{existing}'")]
    RegionOverlap { region: String, existing: String },

    /// Size limit exceeded
    #[error("Memory size {size} exceeds limit {limit}")]
    SizeLimitExceeded { size: u64, limit: u64 },

    /// Invalid allocation
    #[error("Invalid allocation: {0}")]
    InvalidAllocation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_limit_new() {
        let limit = MemoryLimit::new(1024 * 1024);
        assert_eq!(limit.max_bytes, 1024 * 1024);
        assert_eq!(limit.page_size, 65536);
    }

    #[test]
    fn test_memory_limit_with_pages() {
        let limit = MemoryLimit::with_pages(10);
        assert_eq!(limit.max_pages, 10);
        assert_eq!(limit.max_bytes, 10 * 65536);
    }

    #[test]
    fn test_memory_limit_bytes_to_pages() {
        let limit = MemoryLimit::new(65536 * 10);
        assert_eq!(limit.bytes_to_pages(65536), 1);
        assert_eq!(limit.bytes_to_pages(65536 * 2), 2);
        assert_eq!(limit.bytes_to_pages(65536 + 1), 2);
    }

    #[test]
    fn test_memory_limit_within_limit() {
        let limit = MemoryLimit::new(1024);
        assert!(limit.within_limit(1024));
        assert!(limit.within_limit(512));
        assert!(!limit.within_limit(1025));
    }

    #[test]
    fn test_memory_region_new() {
        let region = MemoryRegion::new(0, 1024, "test".to_string());
        assert_eq!(region.start, 0);
        assert_eq!(region.end, 1024);
        assert_eq!(region.size(), 1024);
        assert!(!region.read_only);
    }

    #[test]
    fn test_memory_region_read_only() {
        let region = MemoryRegion::read_only(0, 1024, "ro".to_string());
        assert!(region.read_only);
        assert!(!region.can_write());
    }

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion::new(1000, 100, "test".to_string());
        assert!(region.contains(1000));
        assert!(region.contains(1050));
        assert!(region.contains(1099));
        assert!(!region.contains(999));
        assert!(!region.contains(1100));
    }

    #[test]
    fn test_memory_region_overlaps() {
        let region = MemoryRegion::new(1000, 100, "test".to_string());
        assert!(region.overlaps(900, 1001));
        assert!(region.overlaps(1050, 1150));
        assert!(region.overlaps(1000, 1100));
        assert!(!region.overlaps(800, 900));
        assert!(!region.overlaps(1100, 1200));
    }

    #[test]
    fn test_memory_region_map_add() {
        let mut map = MemoryRegionMap::new();
        let region = MemoryRegion::new(0, 1024, "test".to_string());
        assert!(map.add_region(region).is_ok());
        assert_eq!(map.region_count(), 1);
        assert_eq!(map.total_bytes(), 1024);
    }

    #[test]
    fn test_memory_region_map_overlap() {
        let mut map = MemoryRegionMap::new();
        let r1 = MemoryRegion::new(0, 1024, "first".to_string());
        let r2 = MemoryRegion::new(512, 1024, "second".to_string());
        assert!(map.add_region(r1).is_ok());
        assert!(map.add_region(r2).is_err());
    }

    #[test]
    fn test_memory_region_map_find() {
        let mut map = MemoryRegionMap::new();
        let region = MemoryRegion::new(1000, 100, "test".to_string());
        map.add_region(region).unwrap();
        assert!(map.find_region(1050).is_some());
        assert!(map.find_region(900).is_none());
    }

    #[test]
    fn test_memory_region_map_can_write() {
        let mut map = MemoryRegionMap::new();
        let rw = MemoryRegion::new(0, 1024, "rw".to_string());
        let ro = MemoryRegion::read_only(1024, 1024, "ro".to_string());
        map.add_region(rw).unwrap();
        map.add_region(ro).unwrap();
        assert!(map.can_write(512));
        assert!(!map.can_write(1536));
    }

    #[test]
    fn test_memory_limit_default() {
        let limit = MemoryLimit::default();
        assert_eq!(limit.max_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_memory_error_display() {
        let err = MemoryError::OutOfMemory {
            requested: 2048,
            available: 1024,
        };
        assert!(err.to_string().contains("Out of memory"));
    }
}
