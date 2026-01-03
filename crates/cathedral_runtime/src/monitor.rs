//! Execution monitor for metrics and telemetry.
//!
//! Tracks execution metrics and provides telemetry for observability.

use cathedral_core::{NodeId, LogicalTime};
use std::time::Duration;

/// Execution metrics
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    /// Total nodes executed
    pub nodes_executed: u64,
    /// Nodes completed successfully
    pub nodes_completed: u64,
    /// Nodes failed
    pub nodes_failed: u64,
    /// Nodes skipped
    pub nodes_skipped: u64,
    /// Total execution time (logical ticks)
    pub total_ticks: u64,
    /// Events generated
    pub events_generated: u64,
}

impl Metrics {
    /// Create new metrics
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a node execution
    pub fn record_execution(&mut self) {
        self.nodes_executed += 1;
    }

    /// Record a node completion
    pub fn record_completion(&mut self) {
        self.nodes_completed += 1;
    }

    /// Record a node failure
    pub fn record_failure(&mut self) {
        self.nodes_failed += 1;
    }

    /// Record a node skip
    pub fn record_skip(&mut self) {
        self.nodes_skipped += 1;
    }

    /// Record a tick
    pub fn record_tick(&mut self) {
        self.total_ticks += 1;
    }

    /// Record an event
    pub fn record_event(&mut self) {
        self.events_generated += 1;
    }

    /// Get success rate (0.0 - 1.0)
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.nodes_executed == 0 {
            return 1.0;
        }
        self.nodes_completed as f64 / self.nodes_executed as f64
    }

    /// Get failure rate (0.0 - 1.0)
    #[must_use]
    pub fn failure_rate(&self) -> f64 {
        if self.nodes_executed == 0 {
            return 0.0;
        }
        self.nodes_failed as f64 / self.nodes_executed as f64
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Telemetry data point
#[derive(Debug, Clone)]
pub struct Telemetry {
    /// Timestamp when telemetry was captured
    pub timestamp: Duration,
    /// Current logical time
    pub logical_time: LogicalTime,
    /// Current metrics snapshot
    pub metrics: Metrics,
    /// Nodes currently executing
    pub executing_nodes: Vec<NodeId>,
    /// Backpressure active
    pub backpressure_active: bool,
}

impl Telemetry {
    /// Create new telemetry
    #[must_use]
    pub fn new(
        timestamp: Duration,
        logical_time: LogicalTime,
        metrics: Metrics,
        executing_nodes: Vec<NodeId>,
        backpressure_active: bool,
    ) -> Self {
        Self {
            timestamp,
            logical_time,
            metrics,
            executing_nodes,
            backpressure_active,
        }
    }
}

/// Execution monitor
///
/// Tracks execution metrics and provides telemetry snapshots.
pub struct ExecutionMonitor {
    /// Current metrics
    metrics: Metrics,
    /// Start time
    start_time: std::time::Instant,
    /// Telemetry history
    telemetry_history: Vec<Telemetry>,
    /// Max history size
    max_history: usize,
}

impl ExecutionMonitor {
    /// Create a new monitor
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        Self {
            metrics: Metrics::new(),
            start_time: std::time::Instant::now(),
            telemetry_history: Vec::new(),
            max_history,
        }
    }

    /// Get current metrics
    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Get mutable metrics
    pub fn metrics_mut(&mut self) -> &mut Metrics {
        &mut self.metrics
    }

    /// Capture a telemetry snapshot
    pub fn capture_telemetry(&mut self, logical_time: LogicalTime, executing_nodes: Vec<NodeId>, backpressure: bool) -> Telemetry {
        let telemetry = Telemetry::new(
            self.start_time.elapsed(),
            logical_time,
            self.metrics.clone(),
            executing_nodes,
            backpressure,
        );

        self.add_telemetry(telemetry.clone());
        telemetry
    }

    /// Add telemetry to history
    fn add_telemetry(&mut self, telemetry: Telemetry) {
        self.telemetry_history.push(telemetry);
        if self.telemetry_history.len() > self.max_history {
            self.telemetry_history.remove(0);
        }
    }

    /// Get telemetry history
    #[must_use]
    pub fn history(&self) -> &[Telemetry] {
        &self.telemetry_history
    }

    /// Reset the monitor
    pub fn reset(&mut self) {
        self.metrics.reset();
        self.start_time = std::time::Instant::now();
        self.telemetry_history.clear();
    }
}

impl Default for ExecutionMonitor {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        assert_eq!(metrics.nodes_executed, 0);
        assert_eq!(metrics.nodes_completed, 0);
        assert_eq!(metrics.success_rate(), 1.0);
    }

    #[test]
    fn test_metrics_record() {
        let mut metrics = Metrics::new();
        metrics.record_execution();
        metrics.record_completion();

        assert_eq!(metrics.nodes_executed, 1);
        assert_eq!(metrics.nodes_completed, 1);
        assert_eq!(metrics.success_rate(), 1.0);
    }

    #[test]
    fn test_metrics_failure_rate() {
        let mut metrics = Metrics::new();
        metrics.record_execution();
        metrics.record_completion();
        metrics.record_execution();
        metrics.record_failure();

        assert_eq!(metrics.nodes_executed, 2);
        assert_eq!(metrics.nodes_failed, 1);
        assert_eq!(metrics.failure_rate(), 0.5);
    }

    #[test]
    fn test_metrics_reset() {
        let mut metrics = Metrics::new();
        metrics.record_execution();
        metrics.record_completion();
        metrics.reset();

        assert_eq!(metrics.nodes_executed, 0);
    }

    #[test]
    fn test_monitor_new() {
        let monitor = ExecutionMonitor::new(100);
        assert_eq!(monitor.max_history, 100);
        assert_eq!(monitor.metrics().nodes_executed, 0);
    }

    #[test]
    fn test_monitor_capture_telemetry() {
        let mut monitor = ExecutionMonitor::new(10);
        monitor.metrics_mut().record_execution();
        monitor.metrics_mut().record_completion();

        let test_node = NodeId::new();
        let telemetry = monitor.capture_telemetry(
            LogicalTime::from_raw(5),
            vec![test_node],
            false,
        );

        assert_eq!(telemetry.logical_time.as_u64(), 5);
        assert_eq!(telemetry.metrics.nodes_executed, 1);
        assert_eq!(telemetry.executing_nodes.len(), 1);
        assert!(!telemetry.backpressure_active);
    }

    #[test]
    fn test_monitor_history() {
        let mut monitor = ExecutionMonitor::new(3);

        monitor.capture_telemetry(LogicalTime::zero(), vec![], false);
        monitor.capture_telemetry(LogicalTime::from_raw(1), vec![], false);
        monitor.capture_telemetry(LogicalTime::from_raw(2), vec![], false);
        monitor.capture_telemetry(LogicalTime::from_raw(3), vec![], false);

        // Should only keep last 3 due to max_history
        assert_eq!(monitor.history().len(), 3);
    }

    #[test]
    fn test_monitor_reset() {
        let mut monitor = ExecutionMonitor::new(10);
        monitor.metrics_mut().record_execution();
        monitor.capture_telemetry(LogicalTime::zero(), vec![], false);

        monitor.reset();

        assert_eq!(monitor.metrics().nodes_executed, 0);
        assert_eq!(monitor.history().len(), 0);
    }

    #[test]
    fn test_monitor_default() {
        let monitor = ExecutionMonitor::default();
        assert_eq!(monitor.max_history, 1000);
    }
}
