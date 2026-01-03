//! Network simulation for testing distributed systems.

use crate::seed::SimSeed;
use cathedral_core::NodeId;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network condition simulation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkCondition {
    /// Normal network (no issues)
    Normal,
    /// High latency (milliseconds)
    Latency(u64),
    /// Packet loss (0-1 probability)
    PacketLoss { probability: f64 },
    /// Partition (nodes cannot communicate)
    Partition { isolated: HashSet<NodeId> },
    /// Bandwidth limit (bytes per second)
    BandwidthLimit { bytes_per_sec: usize },
}

impl NetworkCondition {
    /// Check if packet is delivered
    #[must_use]
    pub fn is_delivered(&self, rng: &mut ChaCha8Rng) -> bool {
        match self {
            NetworkCondition::Normal => true,
            NetworkCondition::Latency(_) => true,
            NetworkCondition::PacketLoss { probability } => {
                rng.r#gen::<f64>() > *probability
            }
            NetworkCondition::Partition { .. } => false,
            NetworkCondition::BandwidthLimit { .. } => true,
        }
    }

    /// Get latency in milliseconds
    #[must_use]
    pub fn latency(&self, rng: &mut ChaCha8Rng) -> u64 {
        match self {
            NetworkCondition::Normal => 0,
            NetworkCondition::Latency(ms) => *ms,
            NetworkCondition::PacketLoss { .. } => 0,
            NetworkCondition::Partition { .. } => 0,
            NetworkCondition::BandwidthLimit { .. } => {
                // Add some jitter
                rng.gen_range(0..10)
            }
        }
    }
}

/// Packet loss model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketLoss {
    /// Base loss probability
    pub probability: f64,
    /// Burst size (0 = no burst)
    pub burst_size: usize,
}

impl PacketLoss {
    /// Create a new packet loss model
    #[must_use]
    pub fn new(probability: f64) -> Self {
        Self {
            probability,
            burst_size: 0,
        }
    }

    /// Set burst size
    #[must_use]
    pub fn with_burst(mut self, size: usize) -> Self {
        self.burst_size = size;
        self
    }
}

impl Default for PacketLoss {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Network simulator
pub struct NetworkSim {
    /// RNG for deterministic randomness
    rng: ChaCha8Rng,
    /// Current network conditions
    conditions: Arc<RwLock<HashMap<(NodeId, NodeId), NetworkCondition>>>,
    /// Default condition
    default: NetworkCondition,
    /// Partition state
    partitions: Arc<RwLock<HashSet<Vec<NodeId>>>>,
}

impl NetworkSim {
    /// Create a new network simulator
    #[must_use]
    pub fn new(seed: SimSeed) -> Self {
        Self {
            rng: seed.into_rng(),
            conditions: Arc::new(RwLock::new(HashMap::new())),
            default: NetworkCondition::Normal,
            partitions: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Set condition for a pair of nodes
    pub async fn set_condition(&self, from: NodeId, to: NodeId, condition: NetworkCondition) {
        let mut conditions = self.conditions.write().await;
        conditions.insert((from, to), condition);
    }

    /// Get condition for a pair of nodes
    pub async fn get_condition(&self, from: NodeId, to: NodeId) -> NetworkCondition {
        let conditions = self.conditions.read().await;
        conditions
            .get(&(from, to))
            .cloned()
            .unwrap_or_else(|| self.default.clone())
    }

    /// Create a partition (isolated groups)
    pub async fn partition(&self, groups: Vec<Vec<NodeId>>) {
        let mut partitions = self.partitions.write().await;
        partitions.clear();
        for group in groups {
            partitions.insert(group);
        }
    }

    /// Heal all partitions
    pub async fn heal_partitions(&self) {
        let mut partitions = self.partitions.write().await;
        partitions.clear();
    }

    /// Check if two nodes can communicate
    pub async fn can_communicate(&self, from: NodeId, to: NodeId) -> bool {
        let partitions = self.partitions.read().await;

        // Check if nodes are in different partitions
        for partition in partitions.iter() {
            let from_in_partition = partition.contains(&from);
            let to_in_partition = partition.contains(&to);

            if from_in_partition || to_in_partition {
                // Both in same partition = can communicate
                // One in partition, one not = cannot communicate
                return from_in_partition && to_in_partition;
            }
        }

        // No partitions affecting these nodes
        true
    }

    /// Simulate sending a message
    pub async fn send(&mut self, from: NodeId, to: NodeId, _data: &[u8]) -> SendResult {
        if !self.can_communicate(from, to).await {
            return SendResult::Partitioned;
        }

        let condition = self.get_condition(from, to).await;
        let delivered = condition.is_delivered(&mut self.rng);
        let latency = condition.latency(&mut self.rng);

        if delivered {
            SendResult::Delivered { latency }
        } else {
            SendResult::Dropped
        }
    }

    /// Set default condition
    pub fn set_default(&mut self, condition: NetworkCondition) {
        self.default = condition;
    }

    /// Add latency to all communications
    pub async fn add_latency(&mut self, ms: u64) {
        self.default = NetworkCondition::Latency(ms);
        let mut conditions = self.conditions.write().await;
        conditions.clear();
    }

    /// Add packet loss to all communications
    pub async fn add_packet_loss(&mut self, probability: f64) {
        self.default = NetworkCondition::PacketLoss { probability };
        let mut conditions = self.conditions.write().await;
        conditions.clear();
    }
}

/// Result of sending a message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendResult {
    /// Message was delivered
    Delivered { latency: u64 },
    /// Message was dropped (packet loss)
    Dropped,
    /// Message blocked by partition
    Partitioned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_packet_loss_new() {
        let loss = PacketLoss::new(0.1);
        assert_eq!(loss.probability, 0.1);
        assert_eq!(loss.burst_size, 0);
    }

    #[tokio::test]
    async fn test_packet_loss_with_burst() {
        let loss = PacketLoss::new(0.1).with_burst(5);
        assert_eq!(loss.probability, 0.1);
        assert_eq!(loss.burst_size, 5);
    }

    #[tokio::test]
    async fn test_packet_loss_default() {
        let loss = PacketLoss::default();
        assert_eq!(loss.probability, 0.0);
    }

    #[tokio::test]
    async fn test_network_condition_normal_delivered() {
        let condition = NetworkCondition::Normal;
        let seed = SimSeed::from_literal(42);
        let mut rng = seed.into_rng();
        assert!(condition.is_delivered(&mut rng));
        assert_eq!(condition.latency(&mut rng), 0);
    }

    #[tokio::test]
    async fn test_network_condition_latency() {
        let condition = NetworkCondition::Latency(100);
        let seed = SimSeed::from_literal(42);
        let mut rng = seed.into_rng();
        assert!(condition.is_delivered(&mut rng));
        assert_eq!(condition.latency(&mut rng), 100);
    }

    #[tokio::test]
    async fn test_network_sim_new() {
        let sim = NetworkSim::new(SimSeed::from_literal(42));
        // Can communicate when no partitions
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        assert!(sim.can_communicate(node1, node2).await);
    }

    #[tokio::test]
    async fn test_network_sim_partition() {
        let sim = NetworkSim::new(SimSeed::from_literal(42));
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        let node3 = NodeId::new();

        // Partition: {node1, node2} vs {node3}
        sim.partition(vec![vec![node1, node2], vec![node3]]).await;

        // node1 and node2 can communicate
        assert!(sim.can_communicate(node1, node2).await);
        // node2 and node3 cannot
        assert!(!sim.can_communicate(node2, node3).await);
        // node1 and node3 cannot
        assert!(!sim.can_communicate(node1, node3).await);
    }

    #[tokio::test]
    async fn test_network_sim_heal_partitions() {
        let sim = NetworkSim::new(SimSeed::from_literal(42));
        let node1 = NodeId::new();
        let node2 = NodeId::new();

        sim.partition(vec![vec![node1], vec![node2]]).await;
        assert!(!sim.can_communicate(node1, node2).await);

        sim.heal_partitions().await;
        assert!(sim.can_communicate(node1, node2).await);
    }

    #[tokio::test]
    async fn test_network_sim_send_normal() {
        let mut sim = NetworkSim::new(SimSeed::from_literal(42));
        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let result = sim.send(node1, node2, b"test").await;
        assert!(matches!(result, SendResult::Delivered { latency: 0 }));
    }

    #[tokio::test]
    async fn test_network_sim_send_partitioned() {
        let mut sim = NetworkSim::new(SimSeed::from_literal(42));
        let node1 = NodeId::new();
        let node2 = NodeId::new();

        sim.partition(vec![vec![node1], vec![node2]]).await;
        let result = sim.send(node1, node2, b"test").await;
        assert_eq!(result, SendResult::Partitioned);
    }

    #[test]
    fn test_send_result_equality() {
        assert_eq!(
            SendResult::Delivered { latency: 10 },
            SendResult::Delivered { latency: 10 }
        );
        assert_ne!(
            SendResult::Delivered { latency: 10 },
            SendResult::Delivered { latency: 20 }
        );
        assert_ne!(
            SendResult::Delivered { latency: 0 },
            SendResult::Dropped
        );
    }
}
