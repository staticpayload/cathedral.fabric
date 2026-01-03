//! Resource contracts for workflow nodes.

use serde::{Deserialize, Serialize};

/// Resource contract for a node or workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContract {
    /// Memory requirements
    pub memory: ResourceBounds,
    /// CPU requirements
    pub cpu: ResourceBounds,
    /// Storage requirements
    pub storage: ResourceBounds,
    /// Network requirements
    pub network: ResourceBounds,
}

impl ResourceContract {
    /// Create a new contract with default bounds
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: ResourceBounds::default(),
            cpu: ResourceBounds::default(),
            storage: ResourceBounds::default(),
            network: ResourceBounds::default(),
        }
    }

    /// Set memory bounds
    #[must_use]
    pub fn with_memory(mut self, bounds: ResourceBounds) -> Self {
        self.memory = bounds;
        self
    }

    /// Set CPU bounds
    #[must_use]
    pub fn with_cpu(mut self, bounds: ResourceBounds) -> Self {
        self.cpu = bounds;
        self
    }
}

impl Default for ResourceContract {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource bounds (min/max)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBounds {
    /// Minimum required
    pub min: Option<u64>,
    /// Maximum allowed
    pub max: Option<u64>,
    /// Default allocation
    pub default_value: Option<u64>,
}

impl ResourceBounds {
    /// Create new bounds
    #[must_use]
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            default_value: None,
        }
    }

    /// Set minimum
    #[must_use]
    pub fn with_min(mut self, min: u64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum
    #[must_use]
    pub fn with_max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set default value
    #[must_use]
    pub fn with_default(mut self, default: u64) -> Self {
        self.default_value = Some(default);
        self
    }

    /// Check if a value is within bounds
    #[must_use]
    pub fn check(&self, value: u64) -> bool {
        if let Some(min) = self.min {
            if value < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return false;
            }
        }
        true
    }
}

impl Default for ResourceBounds {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_bounds_new() {
        let bounds = ResourceBounds::new();
        assert!(bounds.min.is_none());
        assert!(bounds.max.is_none());
    }

    #[test]
    fn test_resource_bounds_with_min() {
        let bounds = ResourceBounds::new().with_min(100);
        assert_eq!(bounds.min, Some(100));
    }

    #[test]
    fn test_resource_bounds_check() {
        let bounds = ResourceBounds::new().with_min(10).with_max(100);
        assert!(bounds.check(50));
        assert!(!bounds.check(5)); // Below min
        assert!(!bounds.check(150)); // Above max
    }

    #[test]
    fn test_resource_contract_new() {
        let contract = ResourceContract::new();
        assert!(contract.memory.min.is_none());
        assert!(contract.cpu.min.is_none());
    }

    #[test]
    fn test_resource_contract_with_memory() {
        let bounds = ResourceBounds::new().with_max(1024);
        let contract = ResourceContract::new().with_memory(bounds);
        assert_eq!(contract.memory.max, Some(1024));
    }
}
