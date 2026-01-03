//! Distributed consensus for replicated log.

use cathedral_core::{CoreResult, CoreError, Hash, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Consensus configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Node ID for this instance
    pub node_id: NodeId,
    /// Election timeout in milliseconds
    pub election_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Maximum log entries per message
    pub max_entries_per_msg: usize,
    /// Quorum size
    pub quorum_size: usize,
}

impl ConsensusConfig {
    /// Create a new consensus config
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            election_timeout_ms: 1000,
            heartbeat_interval_ms: 100,
            max_entries_per_msg: 100,
            quorum_size: 2,
        }
    }

    /// Set election timeout
    #[must_use]
    pub fn with_election_timeout(mut self, timeout_ms: u64) -> Self {
        self.election_timeout_ms = timeout_ms;
        self
    }

    /// Set heartbeat interval
    #[must_use]
    pub fn with_heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.heartbeat_interval_ms = interval_ms;
        self
    }

    /// Set quorum size
    #[must_use]
    pub fn with_quorum_size(mut self, size: usize) -> Self {
        self.quorum_size = size;
        self
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

/// Log entry for consensus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusEntry {
    /// Entry index
    pub index: u64,
    /// Entry term
    pub term: u64,
    /// Entry data
    pub data: Vec<u8>,
    /// Entry hash
    pub hash: Hash,
}

impl ConsensusEntry {
    /// Create a new consensus entry
    #[must_use]
    pub fn new(index: u64, term: u64, data: Vec<u8>) -> Self {
        let hash = Hash::compute(&data);
        Self { index, term, data, hash }
    }
}

/// Consensus state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusState {
    /// Following a leader
    Follower,
    /// Candidate in an election
    Candidate,
    /// Leading the cluster
    Leader,
}

/// Consensus errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConsensusError {
    /// Not a leader
    #[error("Not a leader")]
    NotLeader,

    /// Term mismatch
    #[error("Term mismatch: current {current}, received {received}")]
    TermMismatch { current: u64, received: u64 },

    /// Log conflict
    #[error("Log conflict at index {index}")]
    LogConflict { index: u64 },

    /// Quorum not reached
    #[error("Quorum not reached: {have}/{needed} votes")]
    QuorumNotReached { have: usize, needed: usize },

    /// Invalid entry
    #[error("Invalid entry: {0}")]
    InvalidEntry(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),
}

/// Distributed consensus implementation
pub struct Consensus {
    /// Configuration
    config: ConsensusConfig,
    /// Current state
    state: Arc<RwLock<ConsensusState>>,
    /// Current term
    current_term: Arc<RwLock<u64>>,
    /// Voted for in this term
    voted_for: Arc<RwLock<Option<NodeId>>>,
    /// Log entries
    log: Arc<RwLock<Vec<ConsensusEntry>>>,
    /// Commit index
    commit_index: Arc<RwLock<u64>>,
    /// Last applied index
    last_applied: Arc<RwLock<u64>>,
    /// Leader ID
    leader_id: Arc<RwLock<Option<NodeId>>>,
    /// Votes received in current election
    votes_received: Arc<RwLock<HashSet<NodeId>>>,
}

impl Consensus {
    /// Create a new consensus instance
    #[must_use]
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConsensusState::Follower)),
            current_term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(Vec::new())),
            commit_index: Arc::new(RwLock::new(0)),
            last_applied: Arc::new(RwLock::new(0)),
            leader_id: Arc::new(RwLock::new(None)),
            votes_received: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Get the current state
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn state(&self) -> ConsensusState {
        *self.state.read().await
    }

    /// Get the current term
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn current_term(&self) -> u64 {
        *self.current_term.read().await
    }

    /// Get the leader ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn leader_id(&self) -> Option<NodeId> {
        *self.leader_id.read().await
    }

    /// Append an entry to the log
    ///
    /// # Errors
    ///
    /// Returns error if not leader
    pub async fn append(&self, data: Vec<u8>) -> CoreResult<u64> {
        let state = *self.state.read().await;
        if state != ConsensusState::Leader {
            return Err(CoreError::Validation {
                field: "state".to_string(),
                reason: "Not a leader".to_string(),
            });
        }

        let mut log = self.log.write().await;
        let index = log.len() as u64;
        let term = *self.current_term.read().await;
        let entry = ConsensusEntry::new(index, term, data);
        log.push(entry);
        Ok(index)
    }

    /// Request a vote from this node
    ///
    /// # Errors
    ///
    /// Returns error if vote cannot be granted
    pub async fn request_vote(
        &self,
        candidate_id: NodeId,
        term: u64,
        _last_log_index: u64,
        _last_log_term: u64,
    ) -> CoreResult<bool> {
        let mut current_term = self.current_term.write().await;

        if term < *current_term {
            return Ok(false);
        }

        if term > *current_term {
            *current_term = term;
            *self.state.write().await = ConsensusState::Follower;
            *self.leader_id.write().await = None;
            *self.voted_for.write().await = None;
        }

        let mut voted_for = self.voted_for.write().await;
        if voted_for.is_none() || *voted_for == Some(candidate_id) {
            *voted_for = Some(candidate_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Append entries to the log (leader -> follower)
    ///
    /// # Errors
    ///
    /// Returns error if replication fails
    pub async fn append_entries(
        &self,
        term: u64,
        _prev_log_index: u64,
        _prev_log_term: u64,
        entries: Vec<ConsensusEntry>,
        _leader_commit: u64,
    ) -> CoreResult<bool> {
        let mut current_term = self.current_term.write().await;

        if term < *current_term {
            return Ok(false);
        }

        if term > *current_term {
            *current_term = term;
            *self.state.write().await = ConsensusState::Follower;
        }

        *self.leader_id.write().await = Some(self.config.node_id);

        if !entries.is_empty() {
            let mut log = self.log.write().await;
            log.extend(entries);
        }

        Ok(true)
    }

    /// Start an election
    ///
    /// # Errors
    ///
    /// Returns error if election cannot be started
    pub async fn start_election(&self) -> CoreResult<()> {
        let mut state = self.state.write().await;
        let mut term = self.current_term.write().await;

        *term += 1;
        *state = ConsensusState::Candidate;
        *self.leader_id.write().await = None;
        *self.voted_for.write().await = Some(self.config.node_id);

        self.votes_received.write().await.clear();
        self.votes_received.write().await.insert(self.config.node_id);

        Ok(())
    }

    /// Receive a vote
    ///
    /// # Errors
    ///
    /// Returns error if vote cannot be processed
    pub async fn receive_vote(&self, voter_id: NodeId, term: u64) -> CoreResult<bool> {
        let current_term = *self.current_term.read().await;

        if term != current_term {
            return Ok(false);
        }

        let state = *self.state.read().await;
        if state != ConsensusState::Candidate {
            return Ok(false);
        }

        let mut votes = self.votes_received.write().await;
        votes.insert(voter_id);

        if votes.len() >= self.config.quorum_size {
            *self.state.write().await = ConsensusState::Leader;
            *self.leader_id.write().await = Some(self.config.node_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get commit index
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn commit_index(&self) -> u64 {
        *self.commit_index.read().await
    }

    /// Commit entries up to an index
    ///
    /// # Errors
    ///
    /// Returns error if commit fails
    pub async fn commit_to(&self, index: u64) -> CoreResult<()> {
        let mut commit_index = self.commit_index.write().await;
        if index > *commit_index {
            *commit_index = index;
        }
        Ok(())
    }

    /// Get the log length
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn log_len(&self) -> usize {
        self.log.read().await.len()
    }

    /// Become a follower
    pub async fn become_follower(&self) {
        *self.state.write().await = ConsensusState::Follower;
        *self.leader_id.write().await = None;
    }
}

impl Default for Consensus {
    fn default() -> Self {
        Self::new(ConsensusConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_new() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);
        assert_eq!(consensus.state().await, ConsensusState::Follower);
        assert_eq!(consensus.current_term().await, 0);
    }

    #[tokio::test]
    async fn test_consensus_config_new() {
        let node_id = NodeId::new();
        let config = ConsensusConfig::new(node_id);
        assert_eq!(config.node_id, node_id);
        assert_eq!(config.election_timeout_ms, 1000);
    }

    #[tokio::test]
    async fn test_consensus_config_with_election_timeout() {
        let config = ConsensusConfig::new(NodeId::new()).with_election_timeout(2000);
        assert_eq!(config.election_timeout_ms, 2000);
    }

    #[tokio::test]
    async fn test_consensus_config_with_quorum_size() {
        let config = ConsensusConfig::new(NodeId::new()).with_quorum_size(3);
        assert_eq!(config.quorum_size, 3);
    }

    #[tokio::test]
    async fn test_consensus_entry_new() {
        let entry = ConsensusEntry::new(0, 1, b"data".to_vec());
        assert_eq!(entry.index, 0);
        assert_eq!(entry.term, 1);
        assert_eq!(entry.data, b"data");
    }

    #[tokio::test]
    async fn test_request_vote_new_term() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        let candidate_id = NodeId::new();
        let granted = consensus.request_vote(candidate_id, 5, 0, 0).await.unwrap();
        assert!(granted);
        assert_eq!(consensus.current_term().await, 5);
    }

    #[tokio::test]
    async fn test_request_vote_old_term() {
        let node_id = NodeId::new();
        let mut config = ConsensusConfig::new(node_id);
        config.node_id = node_id;
        let consensus = Consensus::new(config);

        // Set current term to 1 first
        *consensus.current_term.write().await = 1;

        let candidate_id = NodeId::new();
        // Request with term 0 when current term is 1 should be denied
        let granted = consensus.request_vote(candidate_id, 0, 0, 0).await.unwrap();
        assert!(!granted);
    }

    #[tokio::test]
    async fn test_start_election() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        consensus.start_election().await.unwrap();
        assert_eq!(consensus.state().await, ConsensusState::Candidate);
        assert_eq!(consensus.current_term().await, 1);
    }

    #[tokio::test]
    async fn test_receive_vote() {
        let node_id = NodeId::new();
        let mut config = ConsensusConfig::new(node_id);
        config.quorum_size = 2;

        let consensus = Consensus::new(config);
        consensus.start_election().await.unwrap();

        let voter_id = NodeId::new();
        let won = consensus.receive_vote(voter_id, 1).await.unwrap();
        assert!(won);
        assert_eq!(consensus.state().await, ConsensusState::Leader);
    }

    #[tokio::test]
    async fn test_append_as_leader() {
        let node_id = NodeId::new();
        let config = ConsensusConfig::new(node_id);
        let consensus = Consensus::new(config);

        // Make this node the leader
        *consensus.state.write().await = ConsensusState::Leader;
        consensus.append(b"test data".to_vec()).await.unwrap();

        assert_eq!(consensus.log_len().await, 1);
    }

    #[tokio::test]
    async fn test_append_as_follower() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        let result = consensus.append(b"test data".to_vec()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append_entries() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        let entries = vec![ConsensusEntry::new(0, 1, b"data".to_vec())];
        let success = consensus.append_entries(1, 0, 0, entries, 0).await.unwrap();
        assert!(success);
        assert_eq!(consensus.log_len().await, 1);
    }

    #[tokio::test]
    async fn test_commit_to() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        consensus.commit_to(5).await.unwrap();
        assert_eq!(consensus.commit_index().await, 5);
    }

    #[tokio::test]
    async fn test_become_follower() {
        let config = ConsensusConfig::new(NodeId::new());
        let consensus = Consensus::new(config);

        *consensus.state.write().await = ConsensusState::Leader;
        consensus.become_follower().await;
        assert_eq!(consensus.state().await, ConsensusState::Follower);
    }

    #[tokio::test]
    async fn test_consensus_default() {
        let consensus = Consensus::default();
        assert_eq!(consensus.state().await, ConsensusState::Follower);
    }

    #[test]
    fn test_consensus_state_equality() {
        assert_eq!(ConsensusState::Follower, ConsensusState::Follower);
        assert_ne!(ConsensusState::Follower, ConsensusState::Leader);
    }
}
