//! Simulation harness for running deterministic simulations.

use crate::{seed::SimSeed, network::NetworkSim, failure::{CrashInjector, FailureScenario}, node::{SimNode, SimNodeConfig}, record::SimRecord};
use cathedral_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simulation configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimConfig {
    /// Seed for reproducibility
    pub seed: SimSeed,
    /// Number of ticks to run
    pub max_ticks: u64,
    /// Tick delay in milliseconds (for debugging)
    pub tick_delay_ms: u64,
    /// Whether to record all events
    pub record_events: bool,
}

impl SimConfig {
    /// Create a new simulation config
    #[must_use]
    pub fn new(seed: SimSeed) -> Self {
        Self {
            seed,
            max_ticks: 1000,
            tick_delay_ms: 0,
            record_events: true,
        }
    }

    /// Set max ticks
    #[must_use]
    pub fn with_max_ticks(mut self, max: u64) -> Self {
        self.max_ticks = max;
        self
    }

    /// Set tick delay
    #[must_use]
    pub fn with_tick_delay(mut self, delay_ms: u64) -> Self {
        self.tick_delay_ms = delay_ms;
        self
    }

    /// Disable event recording
    #[must_use]
    pub fn without_recording(mut self) -> Self {
        self.record_events = false;
        self
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self::new(SimSeed::default())
    }
}

/// Simulation result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimResult {
    /// Whether simulation completed successfully
    pub success: bool,
    /// Number of ticks executed
    pub ticks_executed: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Final state of all nodes
    pub final_states: HashMap<NodeId, String>,
}

impl SimResult {
    /// Create a successful result
    #[must_use]
    pub fn success(ticks: u64) -> Self {
        Self {
            success: true,
            ticks_executed: ticks,
            error: None,
            final_states: HashMap::new(),
        }
    }

    /// Create a failed result
    #[must_use]
    pub fn failure(ticks: u64, error: String) -> Self {
        Self {
            success: false,
            ticks_executed: ticks,
            error: Some(error),
            final_states: HashMap::new(),
        }
    }
}

impl Default for SimResult {
    fn default() -> Self {
        Self::success(0)
    }
}

/// Simulation harness
pub struct SimHarness {
    /// Configuration
    config: SimConfig,
    /// Simulated nodes
    nodes: Arc<RwLock<HashMap<NodeId, SimNode>>>,
    /// Network simulator
    network: Arc<RwLock<NetworkSim>>,
    /// Crash injector
    crash_injector: Arc<CrashInjector>,
    /// Current tick
    tick: Arc<RwLock<u64>>,
    /// Event record
    record: Arc<RwLock<SimRecord>>,
    /// Failure scenario
    scenario: Option<FailureScenario>,
}

impl SimHarness {
    /// Create a new simulation harness
    #[must_use]
    pub fn new(config: SimConfig) -> Self {
        let seed = config.seed.clone();
        let network = Arc::new(RwLock::new(NetworkSim::new(seed)));
        let crash_injector = Arc::new(CrashInjector::new(config.seed.clone()));

        Self {
            config,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            network,
            crash_injector,
            tick: Arc::new(RwLock::new(0)),
            record: Arc::new(RwLock::new(SimRecord::new())),
            scenario: None,
        }
    }

    /// Add a node to the simulation
    pub async fn add_node(&self, config: SimNodeConfig) {
        let node = SimNode::new(config);
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.node_id(), node);
    }

    /// Set a failure scenario
    pub fn set_scenario(&mut self, scenario: FailureScenario) {
        self.scenario = Some(scenario);
    }

    /// Run the simulation
    pub async fn run(&self) -> SimResult {
        let max_ticks = self.config.max_ticks;

        // Initialize record
        if self.config.record_events {
            let mut record = self.record.write().await;
            record.seed = self.config.seed.clone();
            record.max_ticks = max_ticks;
        }

        // Run for max_ticks or until all nodes are dead
        for _ in 0..max_ticks {
            self.advance_tick().await;

            // Check if all nodes are dead
            let nodes = self.nodes.read().await;
            let mut all_dead = !nodes.is_empty();
            for node in nodes.values() {
                if node.is_alive().await {
                    all_dead = false;
                    break;
                }
            }
            if all_dead {
                break;
            }
        }

        let final_tick = *self.tick.read().await;

        // Collect final states
        let nodes = self.nodes.read().await;
        let mut final_states = HashMap::new();
        for (node_id, node) in nodes.iter() {
            let state = node.state().await;
            final_states.insert(*node_id, format!("{:?}", state));
        }

        SimResult {
            success: true,
            ticks_executed: final_tick,
            error: None,
            final_states,
        }
    }

    /// Advance by one tick
    pub async fn advance_tick(&self) {
        let mut tick = self.tick.write().await;
        *tick += 1;
        let current_tick = *tick;

        // Process scenario failures
        if let Some(ref scenario) = self.scenario {
            let failures = scenario.schedule.get_failures(current_tick);
            for failure in failures {
                let nodes = self.nodes.read().await;
                if let Some(node) = nodes.get(&failure.node_id) {
                    node.apply_failure(failure.kind.clone()).await;
                }
            }
        }

        // Advance all nodes
        let nodes = self.nodes.read().await;
        for node in nodes.values() {
            let events = node.advance().await;

            // Record events
            if self.config.record_events {
                let mut record = self.record.write().await;
                for event in events {
                    record.events.push((current_tick, node.node_id(), format!("{:?}", event)));
                }
            }
        }

        // Tick delay for debugging
        if self.config.tick_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.tick_delay_ms)).await;
        }
    }

    /// Get the event record
    pub async fn record(&self) -> SimRecord {
        self.record.read().await.clone()
    }

    /// Get current tick
    pub async fn current_tick(&self) -> u64 {
        *self.tick.read().await
    }

    /// Get a node by ID
    pub async fn get_node(&self, node_id: NodeId) -> Option<SimNode> {
        // We need to clone the node somehow - for simplicity, return None
        // In a real implementation, you'd use Arc<SimNode> or similar
        let nodes = self.nodes.read().await;
        if nodes.contains_key(&node_id) {
            // Return a placeholder - in production, you'd clone the Arc
            None
        } else {
            None
        }
    }

    /// Get all node IDs
    pub async fn node_ids(&self) -> Vec<NodeId> {
        let nodes = self.nodes.read().await;
        nodes.keys().copied().collect()
    }

    /// Check if simulation is finished
    pub async fn is_finished(&self) -> bool {
        let current_tick = *self.tick.read().await;
        if current_tick >= self.config.max_ticks {
            return true;
        }

        let nodes = self.nodes.read().await;
        if nodes.is_empty() {
            return true;
        }

        false
    }

    /// Reset the simulation
    pub async fn reset(&self) {
        *self.tick.write().await = 0;
        *self.record.write().await = SimRecord::new();
        self.crash_injector.reset().await;

        let nodes = self.nodes.read().await;
        for node in nodes.values() {
            node.recover().await;
        }
    }

    /// Get network simulator
    pub async fn network(&self) -> Arc<RwLock<NetworkSim>> {
        self.network.clone()
    }

    /// Get crash injector
    pub fn crash_injector(&self) -> Arc<CrashInjector> {
        self.crash_injector.clone()
    }
}

impl Default for SimHarness {
    fn default() -> Self {
        Self::new(SimConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sim_config_new() {
        let seed = SimSeed::from_literal(42);
        let config = SimConfig::new(seed);
        assert_eq!(config.max_ticks, 1000);
        assert_eq!(config.tick_delay_ms, 0);
        assert!(config.record_events);
    }

    #[tokio::test]
    async fn test_sim_config_with_max_ticks() {
        let config = SimConfig::new(SimSeed::from_literal(42)).with_max_ticks(500);
        assert_eq!(config.max_ticks, 500);
    }

    #[test]
    fn test_sim_config_default() {
        let config = SimConfig::default();
        assert_eq!(config.max_ticks, 1000);
    }

    #[tokio::test]
    async fn test_sim_result_success() {
        let result = SimResult::success(100);
        assert!(result.success);
        assert_eq!(result.ticks_executed, 100);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_sim_result_failure() {
        let result = SimResult::failure(50, "test error".to_string());
        assert!(!result.success);
        assert_eq!(result.ticks_executed, 50);
        assert_eq!(result.error, Some("test error".to_string()));
    }

    #[test]
    fn test_sim_result_default() {
        let result = SimResult::default();
        assert!(result.success);
        assert_eq!(result.ticks_executed, 0);
    }

    #[tokio::test]
    async fn test_sim_harness_new() {
        let harness = SimHarness::new(SimConfig::default());
        assert_eq!(harness.current_tick().await, 0);
        assert!(harness.is_finished().await); // No nodes, so finished
    }

    #[tokio::test]
    async fn test_sim_harness_add_node() {
        let harness = SimHarness::new(SimConfig::default());
        let node_id = NodeId::new();
        harness.add_node(SimNodeConfig::new(node_id)).await;

        let ids = harness.node_ids().await;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], node_id);
        assert!(!harness.is_finished().await); // Has nodes
    }

    #[tokio::test]
    async fn test_sim_harness_advance_tick() {
        let harness = SimHarness::new(SimConfig::new(SimSeed::from_literal(42)).without_recording());
        let node_id = NodeId::new();
        harness.add_node(SimNodeConfig::new(node_id)).await;

        assert_eq!(harness.current_tick().await, 0);
        harness.advance_tick().await;
        assert_eq!(harness.current_tick().await, 1);
    }

    #[tokio::test]
    async fn test_sim_harness_run() {
        let config = SimConfig::new(SimSeed::from_literal(42))
            .with_max_ticks(10)
            .without_recording();
        let harness = SimHarness::new(config);
        harness.add_node(SimNodeConfig::new(NodeId::new())).await;

        let result = harness.run().await;
        assert!(result.success);
        assert_eq!(result.ticks_executed, 10);
    }

    #[tokio::test]
    async fn test_sim_harness_reset() {
        let harness = SimHarness::new(SimConfig::default());
        let node_id = NodeId::new();
        harness.add_node(SimNodeConfig::new(node_id)).await;

        harness.advance_tick().await;
        assert_eq!(harness.current_tick().await, 1);

        harness.reset().await;
        assert_eq!(harness.current_tick().await, 0);
    }

    #[tokio::test]
    async fn test_sim_harness_default() {
        let harness = SimHarness::default();
        assert_eq!(harness.current_tick().await, 0);
    }
}
