//! Simulated nodes for deterministic testing.

use crate::failure::{FailureKind, FailureModel};
use cathedral_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for a simulated node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimNodeConfig {
    /// Node ID
    pub node_id: NodeId,
    /// Tick rate (ticks per second)
    pub tick_rate: u64,
    /// Failure model
    pub failure_model: Option<FailureModel>,
}

impl SimNodeConfig {
    /// Create a new config
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            tick_rate: 100,
            failure_model: None,
        }
    }

    /// Set tick rate
    #[must_use]
    pub fn with_tick_rate(mut self, rate: u64) -> Self {
        self.tick_rate = rate;
        self
    }

    /// Set failure model
    #[must_use]
    pub fn with_failure_model(mut self, model: FailureModel) -> Self {
        self.failure_model = Some(model);
        self
    }
}

impl Default for SimNodeConfig {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

/// State of a simulated node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimNodeState {
    /// Node is running
    Running,
    /// Node is crashed
    Crashed,
    /// Node is partitioned
    Partitioned,
    /// Node is recovering
    Recovering,
}

/// A simulated node
pub struct SimNode {
    /// Configuration
    config: SimNodeConfig,
    /// Current state
    state: Arc<RwLock<SimNodeState>>,
    /// Current tick
    tick: Arc<RwLock<u64>>,
    /// Pending events
    pending: Arc<RwLock<Vec<NodeEvent>>>,
    /// Injected failures
    failures: Arc<RwLock<HashMap<u64, FailureKind>>>,
}

impl SimNode {
    /// Create a new simulated node
    #[must_use]
    pub fn new(config: SimNodeConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SimNodeState::Running)),
            tick: Arc::new(RwLock::new(0)),
            pending: Arc::new(RwLock::new(Vec::new())),
            failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get node ID
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.config.node_id
    }

    /// Get current state
    pub async fn state(&self) -> SimNodeState {
        *self.state.read().await
    }

    /// Get current tick
    pub async fn tick(&self) -> u64 {
        *self.tick.read().await
    }

    /// Check if node is alive
    pub async fn is_alive(&self) -> bool {
        matches!(*self.state.read().await, SimNodeState::Running)
    }

    /// Advance the node by one tick
    pub async fn advance(&self) -> Vec<NodeEvent> {
        let mut tick = self.tick.write().await;
        *tick += 1;
        let current_tick = *tick;

        // Process scheduled failures
        let mut failures = self.failures.write().await;
        if let Some(kind) = failures.remove(&current_tick) {
            self.apply_failure(kind).await;
        }

        // Generate events based on state
        let mut events = Vec::new();
        let state = *self.state.read().await;

        match state {
            SimNodeState::Running => {
                events.push(NodeEvent::Tick { tick: current_tick });
            }
            SimNodeState::Crashed => {
                events.push(NodeEvent::Crashed { tick: current_tick });
            }
            SimNodeState::Partitioned => {
                events.push(NodeEvent::Partitioned { tick: current_tick });
            }
            SimNodeState::Recovering => {
                events.push(NodeEvent::Recovering { tick: current_tick });
                // Auto-recover after one tick
                *self.state.write().await = SimNodeState::Running;
            }
        }

        events
    }

    /// Apply a failure to the node
    pub async fn apply_failure(&self, kind: FailureKind) {
        match kind {
            FailureKind::Crash => {
                *self.state.write().await = SimNodeState::Crashed;
            }
            FailureKind::Partition => {
                *self.state.write().await = SimNodeState::Partitioned;
            }
            FailureKind::HighLatency { .. } => {
                // Latency is handled at message level
            }
            FailureKind::Corrupted => {
                // Handled at message level
            }
            FailureKind::Omission { .. } => {
                // Handled at message level
            }
        }
    }

    /// Recover from failure
    pub async fn recover(&self) {
        *self.state.write().await = SimNodeState::Recovering;
    }

    /// Schedule a failure at a future tick
    pub async fn fail_at(&self, tick: u64, kind: FailureKind) {
        let mut failures = self.failures.write().await;
        failures.insert(tick, kind);
    }

    /// Send a message to another node
    pub async fn send(&self, to: NodeId, data: Vec<u8>) -> NodeMessage {
        NodeMessage {
            from: self.config.node_id,
            to,
            data,
            tick: *self.tick.read().await,
        }
    }

    /// Receive a message
    pub async fn receive(&self, _msg: NodeMessage) -> ReceiveResult {
        if !self.is_alive().await {
            return ReceiveResult::NodeDown;
        }

        let state = *self.state.read().await;
        if state == SimNodeState::Partitioned {
            return ReceiveResult::Partitioned;
        }

        ReceiveResult::Received
    }

    /// Advance to a specific tick
    pub async fn advance_to(&self, target_tick: u64) -> Vec<NodeEvent> {
        let mut all_events = Vec::new();
        while self.tick().await < target_tick {
            all_events.extend(self.advance().await);
        }
        all_events
    }
}

/// Event from a simulated node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeEvent {
    /// Tick advanced
    Tick { tick: u64 },
    /// Node crashed
    Crashed { tick: u64 },
    /// Node partitioned
    Partitioned { tick: u64 },
    /// Node recovering
    Recovering { tick: u64 },
    /// Node recovered
    Recovered { tick: u64 },
}

/// Message between simulated nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMessage {
    /// Sender
    pub from: NodeId,
    /// Receiver
    pub to: NodeId,
    /// Message data
    pub data: Vec<u8>,
    /// Tick when sent
    pub tick: u64,
}

/// Result of receiving a message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiveResult {
    /// Message received
    Received,
    /// Node is down
    NodeDown,
    /// Node is partitioned
    Partitioned,
    /// Message omitted
    Omitted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sim_node_config_new() {
        let node_id = NodeId::new();
        let config = SimNodeConfig::new(node_id);
        assert_eq!(config.node_id, node_id);
        assert_eq!(config.tick_rate, 100);
        assert!(config.failure_model.is_none());
    }

    #[tokio::test]
    async fn test_sim_node_config_with_tick_rate() {
        let config = SimNodeConfig::new(NodeId::new()).with_tick_rate(50);
        assert_eq!(config.tick_rate, 50);
    }

    #[test]
    fn test_sim_node_config_default() {
        let config = SimNodeConfig::default();
        assert_eq!(config.tick_rate, 100);
    }

    #[tokio::test]
    async fn test_sim_node_new() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);
        assert_eq!(node.tick().await, 0);
        assert!(node.is_alive().await);
    }

    #[tokio::test]
    async fn test_sim_node_advance() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        let events = node.advance().await;
        assert_eq!(node.tick().await, 1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NodeEvent::Tick { tick: 1 }));
    }

    #[tokio::test]
    async fn test_sim_node_apply_failure_crash() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        node.apply_failure(FailureKind::Crash).await;
        assert!(!node.is_alive().await);
        assert_eq!(node.state().await, SimNodeState::Crashed);
    }

    #[tokio::test]
    async fn test_sim_node_apply_failure_partition() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        node.apply_failure(FailureKind::Partition).await;
        assert!(!node.is_alive().await);
        assert_eq!(node.state().await, SimNodeState::Partitioned);
    }

    #[tokio::test]
    async fn test_sim_node_recover() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        node.apply_failure(FailureKind::Crash).await;
        assert!(!node.is_alive().await);

        node.recover().await;
        let events = node.advance().await;
        assert!(node.is_alive().await);
        assert!(events.iter().any(|e| matches!(e, NodeEvent::Recovering { .. })));
    }

    #[tokio::test]
    async fn test_sim_node_fail_at() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        node.fail_at(5, FailureKind::Crash).await;
        assert!(node.is_alive().await);

        node.advance_to(5).await;
        assert!(!node.is_alive().await);
    }

    #[tokio::test]
    async fn test_sim_node_send_receive() {
        let node_id = NodeId::new();
        let config = SimNodeConfig::new(node_id);
        let node = SimNode::new(config);

        let msg = node.send(NodeId::new(), b"test".to_vec()).await;
        assert_eq!(msg.from, node_id);
        assert_eq!(msg.tick, 0);
    }

    #[tokio::test]
    async fn test_sim_node_receive_while_running() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        let msg = NodeMessage {
            from: NodeId::new(),
            to: node.node_id(),
            data: Vec::new(),
            tick: 0,
        };

        let result = node.receive(msg).await;
        assert_eq!(result, ReceiveResult::Received);
    }

    #[tokio::test]
    async fn test_sim_node_receive_while_crashed() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        node.apply_failure(FailureKind::Crash).await;

        let msg = NodeMessage {
            from: NodeId::new(),
            to: node.node_id(),
            data: Vec::new(),
            tick: 0,
        };

        let result = node.receive(msg).await;
        assert_eq!(result, ReceiveResult::NodeDown);
    }

    #[tokio::test]
    async fn test_sim_node_advance_to() {
        let config = SimNodeConfig::new(NodeId::new());
        let node = SimNode::new(config);

        let events = node.advance_to(10).await;
        assert_eq!(node.tick().await, 10);
        assert_eq!(events.len(), 10);
    }

    #[test]
    fn test_node_event_equality() {
        assert_eq!(NodeEvent::Tick { tick: 1 }, NodeEvent::Tick { tick: 1 });
        assert_ne!(NodeEvent::Tick { tick: 1 }, NodeEvent::Tick { tick: 2 });
        assert_ne!(NodeEvent::Tick { tick: 1 }, NodeEvent::Crashed { tick: 1 });
    }

    #[test]
    fn test_receive_result_equality() {
        assert_eq!(ReceiveResult::Received, ReceiveResult::Received);
        assert_ne!(ReceiveResult::Received, ReceiveResult::NodeDown);
    }

    #[test]
    fn test_sim_node_state_equality() {
        assert_eq!(SimNodeState::Running, SimNodeState::Running);
        assert_ne!(SimNodeState::Running, SimNodeState::Crashed);
        assert_ne!(SimNodeState::Partitioned, SimNodeState::Crashed);
    }
}
