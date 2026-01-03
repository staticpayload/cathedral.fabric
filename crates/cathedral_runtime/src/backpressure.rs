//! Backpressure control for execution.
//!
//! Monitors resource usage and applies backpressure when needed.

/// Backpressure strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureStrategy {
    /// No backpressure
    None,
    /// Drop events when buffer is full
    Drop,
    /// Block when buffer is full
    Block,
    /// Signal producer to slow down
    Signal,
}

/// Backpressure controller
///
/// Monitors queue depths and applies backpressure when thresholds are exceeded.
pub struct BackpressureController {
    /// Maximum buffer size
    max_buffer_size: usize,
    /// Current buffer size
    current_buffer_size: usize,
    /// Backpressure threshold (0.0 - 1.0)
    threshold: f64,
    /// Strategy
    strategy: BackpressureStrategy,
}

impl BackpressureController {
    /// Create a new backpressure controller
    #[must_use]
    pub fn new(max_buffer_size: usize, threshold: f64, strategy: BackpressureStrategy) -> Self {
        Self {
            max_buffer_size,
            current_buffer_size: 0,
            threshold: threshold.clamp(0.0, 1.0),
            strategy,
        }
    }

    /// Update current buffer size
    pub fn update_buffer_size(&mut self, size: usize) {
        self.current_buffer_size = size;
    }

    /// Check if backpressure should be applied
    #[must_use]
    pub fn should_apply(&self) -> bool {
        if self.max_buffer_size == 0 {
            return false;
        }
        let ratio = self.current_buffer_size as f64 / self.max_buffer_size as f64;
        ratio >= self.threshold
    }

    /// Get current backpressure status
    #[must_use]
    pub fn status(&self) -> BackpressureStatus {
        if !self.should_apply() {
            BackpressureStatus::Ok
        } else {
            match self.strategy {
                BackpressureStrategy::None => BackpressureStatus::Ok,
                BackpressureStrategy::Drop => BackpressureStatus::Drop,
                BackpressureStrategy::Block => BackpressureStatus::Block,
                BackpressureStrategy::Signal => BackpressureStatus::Signal,
            }
        }
    }

    /// Get buffer fill ratio (0.0 - 1.0)
    #[must_use]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_buffer_size == 0 {
            return 0.0;
        }
        (self.current_buffer_size as f64 / self.max_buffer_size as f64).min(1.0)
    }

    /// Get strategy
    #[must_use]
    pub const fn strategy(&self) -> BackpressureStrategy {
        self.strategy
    }
}

impl Default for BackpressureController {
    fn default() -> Self {
        Self {
            max_buffer_size: 1000,
            current_buffer_size: 0,
            threshold: 0.8,
            strategy: BackpressureStrategy::Signal,
        }
    }
}

/// Backpressure status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureStatus {
    /// No backpressure needed
    Ok,
    /// Should drop incoming events
    Drop,
    /// Should block until space available
    Block,
    /// Should signal producer to slow down
    Signal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_controller_new() {
        let controller = BackpressureController::new(100, 0.8, BackpressureStrategy::Block);
        assert_eq!(controller.max_buffer_size, 100);
        assert_eq!(controller.threshold, 0.8);
        assert_eq!(controller.strategy(), BackpressureStrategy::Block);
    }

    #[test]
    fn test_backpressure_should_apply() {
        let mut controller = BackpressureController::new(100, 0.8, BackpressureStrategy::Block);

        controller.update_buffer_size(50);
        assert!(!controller.should_apply());

        controller.update_buffer_size(85);
        assert!(controller.should_apply());
    }

    #[test]
    fn test_backpressure_status() {
        let mut controller = BackpressureController::new(100, 0.8, BackpressureStrategy::Drop);

        controller.update_buffer_size(50);
        assert_eq!(controller.status(), BackpressureStatus::Ok);

        controller.update_buffer_size(85);
        assert_eq!(controller.status(), BackpressureStatus::Drop);
    }

    #[test]
    fn test_backpressure_fill_ratio() {
        let mut controller = BackpressureController::new(100, 0.8, BackpressureStrategy::Block);

        controller.update_buffer_size(50);
        assert_eq!(controller.fill_ratio(), 0.5);

        controller.update_buffer_size(100);
        assert_eq!(controller.fill_ratio(), 1.0);
    }

    #[test]
    fn test_backpressure_default() {
        let controller = BackpressureController::default();
        assert_eq!(controller.max_buffer_size, 1000);
        assert_eq!(controller.threshold, 0.8);
        assert_eq!(controller.strategy(), BackpressureStrategy::Signal);
    }

    #[test]
    fn test_backpressure_strategies() {
        let mut controller = BackpressureController::new(10, 0.5, BackpressureStrategy::None);
        controller.update_buffer_size(10);
        assert_eq!(controller.status(), BackpressureStatus::Ok);

        controller.strategy = BackpressureStrategy::Drop;
        assert_eq!(controller.status(), BackpressureStatus::Drop);

        controller.strategy = BackpressureStrategy::Block;
        assert_eq!(controller.status(), BackpressureStatus::Block);

        controller.strategy = BackpressureStrategy::Signal;
        assert_eq!(controller.status(), BackpressureStatus::Signal);
    }
}
