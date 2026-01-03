//! Leader election for cluster coordination.

use crate::{consensus::Consensus, membership::Membership};
use cathedral_core::{CoreResult, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Election configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionConfig {
    /// Node ID for this instance
    pub node_id: NodeId,
    /// Election timeout in milliseconds
    pub election_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

impl ElectionConfig {
    /// Create a new election config
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            election_timeout_ms: 1000,
            heartbeat_interval_ms: 100,
        }
    }
}

impl Default for ElectionConfig {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

/// Election errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ElectionError {
    /// No quorum
    #[error("No quorum available")]
    NoQuorum,

    /// Election timeout
    #[error("Election timeout")]
    Timeout,

    /// Term changed during election
    #[error("Term changed during election")]
    TermChanged,
}

/// Leader election state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElectionState {
    /// No election in progress
    Idle,
    /// Election in progress
    InProgress,
    /// This node is leader
    Leader,
    /// Following a leader
    Follower(NodeId),
}

/// Leader election implementation
pub struct LeaderElection {
    /// Configuration
    config: ElectionConfig,
    /// Election state
    state: Arc<RwLock<ElectionState>>,
    /// Current leader
    leader: Arc<RwLock<Option<NodeId>>>,
    /// Consensus instance
    consensus: Arc<Consensus>,
    /// Membership instance
    membership: Arc<Membership>,
}

impl LeaderElection {
    /// Create a new leader election instance
    #[must_use]
    pub fn new(config: ElectionConfig, consensus: Arc<Consensus>, membership: Arc<Membership>) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ElectionState::Idle)),
            leader: Arc::new(RwLock::new(None)),
            consensus,
            membership,
        }
    }

    /// Get the current election state
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn state(&self) -> ElectionState {
        *self.state.read().await
    }

    /// Get the current leader
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn leader(&self) -> Option<NodeId> {
        *self.leader.read().await
    }

    /// Check if this node is the leader
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn is_leader(&self) -> bool {
        matches!(*self.state.read().await, ElectionState::Leader)
    }

    /// Set the election state (for testing)
    #[cfg(test)]
    pub async fn set_state(&self, state: ElectionState) {
        *self.state.write().await = state;
    }

    /// Start an election
    ///
    /// # Errors
    ///
    /// Returns error if election cannot be started
    pub async fn start_election(&self) -> CoreResult<()> {
        *self.state.write().await = ElectionState::InProgress;
        self.consensus.start_election().await?;
        Ok(())
    }

    /// Cast a vote for a candidate
    ///
    /// # Errors
    ///
    /// Returns error if vote cannot be cast
    pub async fn vote(&self, candidate_id: NodeId, term: u64) -> CoreResult<bool> {
        self.consensus.request_vote(candidate_id, term, 0, 0).await
    }

    /// Receive a vote
    ///
    /// # Errors
    ///
    /// Returns error if vote cannot be processed
    pub async fn receive_vote(&self, voter_id: NodeId, term: u64) -> CoreResult<bool> {
        let won = self.consensus.receive_vote(voter_id, term).await?;

        if won {
            *self.state.write().await = ElectionState::Leader;
            *self.leader.write().await = Some(self.config.node_id);
        }

        Ok(won)
    }

    /// Step down as leader
    ///
    /// # Errors
    ///
    /// Returns error if stepdown fails
    pub async fn step_down(&self) {
        *self.state.write().await = ElectionState::Follower(self.config.node_id);
        *self.leader.write().await = None;
        self.consensus.become_follower().await;
    }

    /// Recognize a new leader
    ///
    /// # Errors
    ///
    /// Returns error if recognition fails
    pub async fn recognize_leader(&self, leader_id: NodeId) {
        *self.state.write().await = ElectionState::Follower(leader_id);
        *self.leader.write().await = Some(leader_id);
    }

    /// Send heartbeat as leader
    ///
    /// # Errors
    ///
    /// Returns error if heartbeat fails
    pub async fn send_heartbeat(&self) -> CoreResult<()> {
        if !self.is_leader().await {
            return Ok(());
        }

        let members = self.membership.members().await;
        let current_term = self.consensus.current_term().await;

        for member in members {
            if member.node_id == self.config.node_id {
                continue;
            }

            // In a real implementation, this would send a heartbeat message
            let _ = (member.node_id, current_term);
        }

        Ok(())
    }

    /// Check if election timeout has occurred
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn check_timeout(&self, last_heartbeat: u64, current_time: u64) -> bool {
        let state = *self.state.read().await;
        if matches!(state, ElectionState::Leader) {
            return false;
        }

        let elapsed = current_time.saturating_sub(last_heartbeat);
        elapsed > self.config.election_timeout_ms
    }
}

impl Default for LeaderElection {
    fn default() -> Self {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        let consensus_config = crate::consensus::ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        Self::new(config, consensus, membership)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusConfig;

    #[tokio::test]
    async fn test_election_config_new() {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        assert_eq!(config.node_id, node_id);
        assert_eq!(config.election_timeout_ms, 1000);
    }

    #[tokio::test]
    async fn test_leader_election_new() {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        let consensus_config = ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config, consensus, membership);
        assert_eq!(election.leader().await, None);
        assert!(!election.is_leader().await);
    }

    #[tokio::test]
    async fn test_start_election() {
        let node_id = NodeId::new();
        let mut config = ElectionConfig::new(node_id);
        let mut consensus_config = ConsensusConfig::new(node_id);
        consensus_config.quorum_size = 1;

        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config.clone(), consensus, membership);
        election.start_election().await.unwrap();

        assert_eq!(election.state().await, ElectionState::InProgress);
    }

    #[tokio::test]
    async fn test_recognize_leader() {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        let consensus_config = ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config, consensus, membership);
        let leader_id = NodeId::new();
        election.recognize_leader(leader_id).await;

        assert_eq!(election.leader().await, Some(leader_id));
        assert_eq!(election.state().await, ElectionState::Follower(leader_id));
    }

    #[tokio::test]
    async fn test_step_down() {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        let consensus_config = ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config, consensus, membership);
        election.step_down().await;

        assert_eq!(election.leader().await, None);
    }

    #[tokio::test]
    async fn test_check_timeout() {
        let node_id = NodeId::new();
        let mut config = ElectionConfig::new(node_id);
        config.election_timeout_ms = 1000;

        let consensus_config = ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config, consensus, membership);
        // 2000ms > 1000ms timeout, last heartbeat at 0
        assert!(election.check_timeout(0, 2000).await);
    }

    #[tokio::test]
    async fn test_send_heartbeat_as_follower() {
        let node_id = NodeId::new();
        let config = ElectionConfig::new(node_id);
        let consensus_config = ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let membership = Arc::new(Membership::new(node_id));

        let election = LeaderElection::new(config, consensus, membership);
        // Should not fail even as follower
        election.send_heartbeat().await.unwrap();
    }

    #[test]
    fn test_election_state_equality() {
        assert_eq!(ElectionState::Idle, ElectionState::Idle);
        assert_ne!(ElectionState::Idle, ElectionState::Leader);
    }
}
