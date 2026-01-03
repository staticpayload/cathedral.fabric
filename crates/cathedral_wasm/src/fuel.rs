//! Fuel metering for deterministic WASM execution.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Fuel meter for tracking execution cost
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuelMeter {
    /// Current fuel remaining
    pub remaining: u64,
    /// Initial fuel budget
    pub initial: u64,
    /// Total fuel consumed
    pub consumed: u64,
}

impl FuelMeter {
    /// Create a new fuel meter with a budget
    #[must_use]
    pub fn new(budget: u64) -> Self {
        Self {
            remaining: budget,
            initial: budget,
            consumed: 0,
        }
    }

    /// Consume fuel for an operation
    ///
    /// # Errors
    ///
    /// Returns error if insufficient fuel
    pub fn consume(&mut self, amount: u64) -> Result<(), FuelError> {
        if amount > self.remaining {
            return Err(FuelError::OutOfFuel {
                requested: amount,
                remaining: self.remaining,
            });
        }
        self.remaining -= amount;
        self.consumed += amount;
        Ok(())
    }

    /// Check if there's enough fuel for an operation
    #[must_use]
    pub fn can_afford(&self, amount: u64) -> bool {
        amount <= self.remaining
    }

    /// Get remaining fuel
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Get fuel consumed so far
    #[must_use]
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Get initial fuel budget
    #[must_use]
    pub fn initial(&self) -> u64 {
        self.initial
    }

    /// Get fuel usage as a percentage
    #[must_use]
    pub fn usage_percent(&self) -> f64 {
        if self.initial == 0 {
            return 0.0;
        }
        (self.consumed as f64 / self.initial as f64) * 100.0
    }

    /// Check if out of fuel
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Reset the meter
    pub fn reset(&mut self) {
        self.remaining = self.initial;
        self.consumed = 0;
    }

    /// Add additional fuel (for admin operations)
    pub fn add_fuel(&mut self, amount: u64) {
        self.remaining += amount;
        self.initial += amount;
    }
}

impl Default for FuelMeter {
    fn default() -> Self {
        Self::new(1_000_000)
    }
}

/// Fuel limiter configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuelLimiter {
    /// Maximum fuel per execution
    pub max_fuel: u64,
    /// Fuel cost per instruction (base multiplier)
    pub instruction_multiplier: u64,
    /// Fuel cost for memory operations
    pub memory_multiplier: u64,
    /// Fuel cost for host calls
    pub host_call_cost: u64,
}

impl FuelLimiter {
    /// Create a new fuel limiter
    #[must_use]
    pub fn new(max_fuel: u64) -> Self {
        Self {
            max_fuel,
            instruction_multiplier: 1,
            memory_multiplier: 10,
            host_call_cost: 100,
        }
    }

    /// Create with default limits
    #[must_use]
    pub fn default_limits() -> Self {
        Self::new(10_000_000)
    }

    /// Calculate cost for instruction execution
    #[must_use]
    pub fn instruction_cost(&self, count: u64) -> u64 {
        count * self.instruction_multiplier
    }

    /// Calculate cost for memory operation
    #[must_use]
    pub fn memory_cost(&self, bytes: u64) -> u64 {
        (bytes / 1024 + 1) * self.memory_multiplier
    }

    /// Calculate cost for host call
    #[must_use]
    pub fn host_call_cost(&self) -> u64 {
        self.host_call_cost
    }

    /// Estimate execution time from fuel
    #[must_use]
    pub fn estimate_time(&self, fuel: u64) -> Duration {
        // Rough estimate: 1 billion instructions per second
        const INSNS_PER_SECOND: u64 = 1_000_000_000;
        let nanos = (fuel as f64 / INSNS_PER_SECOND as f64) * 1_000_000_000.0;
        Duration::from_nanos(nanos as u64)
    }
}

impl Default for FuelLimiter {
    fn default() -> Self {
        Self::default_limits()
    }
}

/// Fuel-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FuelError {
    /// Out of fuel
    #[error("Out of fuel: requested {requested}, remaining {remaining}")]
    OutOfFuel { requested: u64, remaining: u64 },

    /// Invalid fuel amount
    #[error("Invalid fuel amount: {0}")]
    InvalidAmount(String),

    /// Fuel limit exceeded during configuration
    #[error("Fuel limit {limit} exceeds maximum {max}")]
    LimitExceeded { limit: u64, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_meter_new() {
        let meter = FuelMeter::new(1000);
        assert_eq!(meter.remaining(), 1000);
        assert_eq!(meter.initial(), 1000);
        assert_eq!(meter.consumed(), 0);
    }

    #[test]
    fn test_fuel_meter_consume() {
        let mut meter = FuelMeter::new(1000);
        assert!(meter.consume(100).is_ok());
        assert_eq!(meter.remaining(), 900);
        assert_eq!(meter.consumed(), 100);
    }

    #[test]
    fn test_fuel_meter_out_of_fuel() {
        let mut meter = FuelMeter::new(100);
        let result = meter.consume(200);
        assert!(result.is_err());
        assert_eq!(meter.remaining(), 100);
    }

    #[test]
    fn test_fuel_meter_can_afford() {
        let meter = FuelMeter::new(100);
        assert!(meter.can_afford(50));
        assert!(meter.can_afford(100));
        assert!(!meter.can_afford(101));
    }

    #[test]
    fn test_fuel_meter_usage_percent() {
        let mut meter = FuelMeter::new(1000);
        assert_eq!(meter.usage_percent(), 0.0);
        meter.consume(500).unwrap();
        assert_eq!(meter.usage_percent(), 50.0);
        meter.consume(500).unwrap();
        assert_eq!(meter.usage_percent(), 100.0);
    }

    #[test]
    fn test_fuel_meter_is_empty() {
        let mut meter = FuelMeter::new(100);
        assert!(!meter.is_empty());
        meter.consume(100).unwrap();
        assert!(meter.is_empty());
    }

    #[test]
    fn test_fuel_meter_reset() {
        let mut meter = FuelMeter::new(1000);
        meter.consume(500).unwrap();
        meter.reset();
        assert_eq!(meter.remaining(), 1000);
        assert_eq!(meter.consumed(), 0);
    }

    #[test]
    fn test_fuel_meter_add_fuel() {
        let mut meter = FuelMeter::new(1000);
        meter.consume(500).unwrap();
        meter.add_fuel(200);
        assert_eq!(meter.remaining(), 700);
        assert_eq!(meter.initial(), 1200);
    }

    #[test]
    fn test_fuel_limiter_new() {
        let limiter = FuelLimiter::new(1000);
        assert_eq!(limiter.max_fuel, 1000);
    }

    #[test]
    fn test_fuel_limiter_instruction_cost() {
        let limiter = FuelLimiter::new(1000);
        assert_eq!(limiter.instruction_cost(100), 100);
    }

    #[test]
    fn test_fuel_limiter_memory_cost() {
        let limiter = FuelLimiter::new(1000);
        // 1 KB costs (1 + 1) * 10 = 20
        assert_eq!(limiter.memory_cost(1024), 20);
        // 2 KB costs (2 + 1) * 10 = 30
        assert_eq!(limiter.memory_cost(2048), 30);
    }

    #[test]
    fn test_fuel_limiter_default() {
        let limiter = FuelLimiter::default();
        assert_eq!(limiter.max_fuel, 10_000_000);
    }

    #[test]
    fn test_fuel_error_display() {
        let err = FuelError::OutOfFuel {
            requested: 200,
            remaining: 100,
        };
        assert!(err.to_string().contains("Out of fuel"));
    }
}
