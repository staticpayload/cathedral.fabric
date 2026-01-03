//! Failure injection for testing fault tolerance.

use crate::seed::SimSeed;
use cathedral_core::NodeId;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Kind of failure to inject
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureKind {
    /// Node crash (stops responding)
    Crash,
    /// Network partition (isolated from cluster)
    Partition,
    /// High latency (slowed response)
    HighLatency { ms: u64 },
    /// Corrupted response
    Corrupted,
    /// Omission failure (ignores some messages)
    Omission { probability: f64 },
}

/// Failure model describing when and how failures occur
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureModel {
    /// Seed for deterministic randomness
    pub seed: u64,
    /// Probability of failure at each opportunity
    pub probability: f64,
    /// Maximum number of failures to inject
    pub max_failures: usize,
    /// Kinds of failures to inject
    pub kinds: Vec<FailureKind>,
}

impl FailureModel {
    /// Create a new failure model
    #[must_use]
    pub fn new(seed: u64, probability: f64) -> Self {
        Self {
            seed,
            probability,
            max_failures: usize::MAX,
            kinds: vec![
                FailureKind::Crash,
                FailureKind::Partition,
                FailureKind::HighLatency { ms: 1000 },
            ],
        }
    }

    /// Set max failures
    #[must_use]
    pub fn with_max_failures(mut self, max: usize) -> Self {
        self.max_failures = max;
        self
    }

    /// Set failure kinds
    #[must_use]
    pub fn with_kinds(mut self, kinds: Vec<FailureKind>) -> Self {
        self.kinds = kinds;
        self
    }

    /// Check if failure should occur
    #[must_use]
    pub fn should_fail(&self, rng: &mut ChaCha8Rng) -> bool {
        rng.r#gen::<f64>() < self.probability
    }

    /// Get random failure kind
    #[must_use]
    pub fn random_kind(&self, rng: &mut ChaCha8Rng) -> FailureKind {
        if self.kinds.is_empty() {
            return FailureKind::Crash;
        }
        self.kinds[rng.gen_range(0..self.kinds.len())].clone()
    }
}

impl Default for FailureModel {
    fn default() -> Self {
        Self::new(42, 0.0) // No failures by default
    }
}

/// Crash injector for simulating node failures
pub struct CrashInjector {
    /// RNG for deterministic randomness
    rng: Arc<std::sync::Mutex<ChaCha8Rng>>,
    /// Nodes that are crashed
    crashed: Arc<RwLock<HashSet<NodeId>>>,
    /// Failure model
    model: FailureModel,
    /// Failure count
    failures_injected: Arc<RwLock<usize>>,
}

impl CrashInjector {
    /// Create a new crash injector
    #[must_use]
    pub fn new(seed: SimSeed) -> Self {
        let model = FailureModel::default();
        Self {
            rng: Arc::new(std::sync::Mutex::new(seed.into_rng())),
            crashed: Arc::new(RwLock::new(HashSet::new())),
            model,
            failures_injected: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with a specific failure model
    #[must_use]
    pub fn with_model(seed: SimSeed, model: FailureModel) -> Self {
        Self {
            rng: Arc::new(std::sync::Mutex::new(seed.into_rng())),
            crashed: Arc::new(RwLock::new(HashSet::new())),
            model,
            failures_injected: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if a node is crashed
    pub async fn is_crashed(&self, node_id: NodeId) -> bool {
        self.crashed.read().await.contains(&node_id)
    }

    /// Crash a specific node
    pub async fn crash(&self, node_id: NodeId) {
        let mut crashed = self.crashed.write().await;
        crashed.insert(node_id);
        let mut count = self.failures_injected.write().await;
        *count += 1;
    }

    /// Recover a crashed node
    pub async fn recover(&self, node_id: NodeId) {
        let mut crashed = self.crashed.write().await;
        crashed.remove(&node_id);
    }

    /// Check if failure should be injected for a node
    pub async fn maybe_fail(&self, node_id: NodeId) -> Option<FailureKind> {
        let count = *self.failures_injected.read().await;
        if count >= self.model.max_failures {
            return None;
        }

        let mut rng = self.rng.lock().unwrap();
        if self.model.should_fail(&mut *rng) {
            let kind = self.model.random_kind(&mut *rng);
            match &kind {
                FailureKind::Crash => {
                    self.crash(node_id).await;
                }
                FailureKind::Partition => {
                    self.crash(node_id).await; // Treat partition as crash for simplicity
                }
                FailureKind::HighLatency { .. } => {
                    // Latency is handled at call site
                }
                FailureKind::Corrupted => {
                    // Corrupted responses handled at call site
                }
                FailureKind::Omission { .. } => {
                    // Omission handled at call site
                }
            }
            Some(kind)
        } else {
            None
        }
    }

    /// Get list of crashed nodes
    pub async fn crashed_nodes(&self) -> Vec<NodeId> {
        self.crashed.read().await.iter().copied().collect()
    }

    /// Get failure count
    pub async fn failure_count(&self) -> usize {
        *self.failures_injected.read().await
    }

    /// Reset all failures
    pub async fn reset(&self) {
        self.crashed.write().await.clear();
        *self.failures_injected.write().await = 0;
    }
}

/// Failure schedule for planned failures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureSchedule {
    /// Scheduled failures by tick
    pub failures: HashMap<u64, Vec<ScheduledFailure>>,
}

impl FailureSchedule {
    /// Create a new empty schedule
    #[must_use]
    pub fn new() -> Self {
        Self {
            failures: HashMap::new(),
        }
    }

    /// Add a failure at a specific tick
    #[must_use]
    pub fn add_failure(mut self, tick: u64, failure: ScheduledFailure) -> Self {
        self.failures.entry(tick).or_default().push(failure);
        self
    }

    /// Get failures for a tick
    #[must_use]
    pub fn get_failures(&self, tick: u64) -> Vec<ScheduledFailure> {
        self.failures.get(&tick).cloned().unwrap_or_default()
    }

    /// Get all ticks with scheduled failures
    #[must_use]
    pub fn failure_ticks(&self) -> Vec<u64> {
        let mut ticks: Vec<_> = self.failures.keys().copied().collect();
        ticks.sort();
        ticks
    }
}

impl Default for FailureSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// A scheduled failure event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledFailure {
    /// Node to fail
    pub node_id: NodeId,
    /// Kind of failure
    pub kind: FailureKind,
    /// Duration of failure (0 = permanent)
    pub duration_ticks: u64,
}

impl ScheduledFailure {
    /// Create a new scheduled failure
    #[must_use]
    pub fn new(node_id: NodeId, kind: FailureKind) -> Self {
        Self {
            node_id,
            kind,
            duration_ticks: 0,
        }
    }

    /// Set duration
    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.duration_ticks = ticks;
        self
    }
}

/// Scenario for testing specific failure patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureScenario {
    /// Scenario name
    pub name: String,
    /// Description
    pub description: String,
    /// Scheduled failures
    pub schedule: FailureSchedule,
}

impl FailureScenario {
    /// Create a new scenario
    #[must_use]
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            schedule: FailureSchedule::new(),
        }
    }

    /// Add a crash to the scenario
    #[must_use]
    pub fn crash_at(mut self, tick: u64, node_id: NodeId) -> Self {
        self.schedule.failures.entry(tick).or_default()
            .push(ScheduledFailure::new(node_id, FailureKind::Crash));
        self
    }

    /// Add a partition to the scenario
    #[must_use]
    pub fn partition_at(mut self, tick: u64, node_id: NodeId) -> Self {
        self.schedule.failures.entry(tick).or_default()
            .push(ScheduledFailure::new(node_id, FailureKind::Partition));
        self
    }

    /// Add high latency to the scenario
    #[must_use]
    pub fn latency_at(mut self, tick: u64, node_id: NodeId, ms: u64) -> Self {
        self.schedule.failures.entry(tick).or_default()
            .push(ScheduledFailure::new(node_id, FailureKind::HighLatency { ms }));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_model_new() {
        let model = FailureModel::new(42, 0.1);
        assert_eq!(model.seed, 42);
        assert_eq!(model.probability, 0.1);
        assert_eq!(model.max_failures, usize::MAX);
        assert_eq!(model.kinds.len(), 3);
    }

    #[test]
    fn test_failure_model_with_max_failures() {
        let model = FailureModel::new(42, 0.1).with_max_failures(5);
        assert_eq!(model.max_failures, 5);
    }

    #[test]
    fn test_failure_model_default() {
        let model = FailureModel::default();
        assert_eq!(model.seed, 42);
        assert_eq!(model.probability, 0.0);
    }

    #[test]
    fn test_failure_model_should_fail() {
        let model = FailureModel::new(42, 1.0); // 100% failure rate
        let seed = SimSeed::from_literal(42);
        let mut rng = seed.into_rng();
        assert!(model.should_fail(&mut rng));
    }

    #[test]
    fn test_failure_model_should_not_fail() {
        let model = FailureModel::new(42, 0.0); // 0% failure rate
        let seed = SimSeed::from_literal(42);
        let mut rng = seed.into_rng();
        assert!(!model.should_fail(&mut rng));
    }

    #[tokio::test]
    async fn test_crash_injector_new() {
        let injector = CrashInjector::new(SimSeed::from_literal(42));
        assert_eq!(injector.crashed_nodes().await.len(), 0);
        assert_eq!(injector.failure_count().await, 0);
    }

    #[tokio::test]
    async fn test_crash_injector_crash() {
        let injector = CrashInjector::new(SimSeed::from_literal(42));
        let node_id = NodeId::new();

        injector.crash(node_id).await;
        assert!(injector.is_crashed(node_id).await);
        assert_eq!(injector.crashed_nodes().await.len(), 1);
        assert_eq!(injector.failure_count().await, 1);
    }

    #[tokio::test]
    async fn test_crash_injector_recover() {
        let injector = CrashInjector::new(SimSeed::from_literal(42));
        let node_id = NodeId::new();

        injector.crash(node_id).await;
        assert!(injector.is_crashed(node_id).await);

        injector.recover(node_id).await;
        assert!(!injector.is_crashed(node_id).await);
    }

    #[tokio::test]
    async fn test_crash_injector_reset() {
        let injector = CrashInjector::new(SimSeed::from_literal(42));
        let node_id = NodeId::new();

        injector.crash(node_id).await;
        injector.reset().await;

        assert!(!injector.is_crashed(node_id).await);
        assert_eq!(injector.failure_count().await, 0);
    }

    #[test]
    fn test_failure_schedule_new() {
        let schedule = FailureSchedule::new();
        assert_eq!(schedule.get_failures(0).len(), 0);
        assert_eq!(schedule.failure_ticks().len(), 0);
    }

    #[test]
    fn test_failure_schedule_add() {
        let schedule = FailureSchedule::new()
            .add_failure(10, ScheduledFailure::new(NodeId::new(), FailureKind::Crash));

        assert_eq!(schedule.get_failures(10).len(), 1);
        assert_eq!(schedule.failure_ticks(), vec![10]);
    }

    #[test]
    fn test_failure_schedule_default() {
        let schedule = FailureSchedule::default();
        assert_eq!(schedule.get_failures(0).len(), 0);
    }

    #[test]
    fn test_scheduled_failure_new() {
        let node_id = NodeId::new();
        let failure = ScheduledFailure::new(node_id, FailureKind::Crash);
        assert_eq!(failure.node_id, node_id);
        assert_eq!(failure.duration_ticks, 0);
    }

    #[test]
    fn test_scheduled_failure_with_duration() {
        let node_id = NodeId::new();
        let failure = ScheduledFailure::new(node_id, FailureKind::Crash)
            .with_duration(5);
        assert_eq!(failure.duration_ticks, 5);
    }

    #[test]
    fn test_failure_scenario_new() {
        let scenario = FailureScenario::new(
            "test".to_string(),
            "Test scenario".to_string(),
        );
        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.description, "Test scenario");
    }

    #[test]
    fn test_failure_scenario_crash_at() {
        let node_id = NodeId::new();
        let scenario = FailureScenario::new(
            "test".to_string(),
            "Test scenario".to_string(),
        ).crash_at(10, node_id);

        assert_eq!(scenario.schedule.get_failures(10).len(), 1);
    }

    #[test]
    fn test_failure_kind_equality() {
        assert_eq!(FailureKind::Crash, FailureKind::Crash);
        assert_ne!(FailureKind::Crash, FailureKind::Partition);
        assert_eq!(
            FailureKind::HighLatency { ms: 100 },
            FailureKind::HighLatency { ms: 100 }
        );
    }
}
