//! Cluster membership management.

use cathedral_core::{CoreResult, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Member state in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberState {
    /// Member is joining
    Joining,
    /// Member is active
    Active,
    /// Member is leaving
    Leaving,
    /// Member has left
    Left,
    /// Member is suspected to be down
    Suspected,
}

/// Cluster member information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Member node ID
    pub node_id: NodeId,
    /// Member state
    pub state: MemberState,
    /// Member address
    pub address: String,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Member capabilities
    pub capabilities: Vec<String>,
}

impl Member {
    /// Create a new member
    #[must_use]
    pub fn new(node_id: NodeId, address: String) -> Self {
        Self {
            node_id,
            state: MemberState::Joining,
            address,
            last_heartbeat: 0,
            capabilities: Vec::new(),
        }
    }

    /// Set member state
    #[must_use]
    pub fn with_state(mut self, state: MemberState) -> Self {
        self.state = state;
        self
    }

    /// Update heartbeat
    #[must_use]
    pub fn with_heartbeat(mut self, timestamp: u64) -> Self {
        self.last_heartbeat = timestamp;
        self
    }

    /// Check if member is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.state, MemberState::Active)
    }

    /// Check if member is suspect
    #[must_use]
    pub fn is_suspect(&self) -> bool {
        matches!(self.state, MemberState::Suspected)
    }
}

/// Cluster membership
pub struct Membership {
    /// Known members
    members: Arc<RwLock<HashMap<NodeId, Member>>>,
    /// Current node ID
    node_id: NodeId,
    /// Heartbeat timeout in milliseconds
    heartbeat_timeout_ms: u64,
}

impl Membership {
    /// Create a new membership tracker
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            members: Arc::new(RwLock::new(HashMap::new())),
            node_id,
            heartbeat_timeout_ms: 5000,
        }
    }

    /// Get all members
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn members(&self) -> Vec<Member> {
        self.members.read().await.values().cloned().collect()
    }

    /// Get active members
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn active_members(&self) -> Vec<Member> {
        self.members
            .read()
            .await
            .values()
            .filter(|m| m.is_active())
            .cloned()
            .collect()
    }

    /// Add a member
    ///
    /// # Errors
    ///
    /// Returns error if add fails
    pub async fn add_member(&self, member: Member) -> CoreResult<()> {
        let mut members = self.members.write().await;
        members.insert(member.node_id, member);
        Ok(())
    }

    /// Remove a member
    ///
    /// # Errors
    ///
    /// Returns error if remove fails
    pub async fn remove_member(&self, node_id: NodeId) -> CoreResult<bool> {
        let mut members = self.members.write().await;
        Ok(members.remove(&node_id).is_some())
    }

    /// Get a member by ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn get_member(&self, node_id: NodeId) -> Option<Member> {
        self.members.read().await.get(&node_id).cloned()
    }

    /// Update member state
    ///
    /// # Errors
    ///
    /// Returns error if update fails
    pub async fn update_state(&self, node_id: NodeId, state: MemberState) -> CoreResult<bool> {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(&node_id) {
            member.state = state;
            return Ok(true);
        }
        Ok(false)
    }

    /// Update heartbeat for a member
    ///
    /// # Errors
    ///
    /// Returns error if update fails
    pub async fn update_heartbeat(&self, node_id: NodeId, timestamp: u64) -> CoreResult<bool> {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(&node_id) {
            member.last_heartbeat = timestamp;
            if member.state == MemberState::Suspected {
                member.state = MemberState::Active;
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// Check for inactive members and mark them as suspected
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn check_heartbeats(&self, current_time: u64) -> CoreResult<Vec<NodeId>> {
        let mut members = self.members.write().await;
        let mut suspected = Vec::new();

        for (node_id, member) in members.iter_mut() {
            if *node_id == self.node_id {
                continue;
            }

            if member.is_active() {
                let elapsed = current_time.saturating_sub(member.last_heartbeat);
                if elapsed > self.heartbeat_timeout_ms {
                    member.state = MemberState::Suspected;
                    suspected.push(*node_id);
                }
            }
        }

        Ok(suspected)
    }

    /// Get member count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn member_count(&self) -> usize {
        self.members.read().await.len()
    }

    /// Get active member count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn active_count(&self) -> usize {
        self.members
            .read()
            .await
            .values()
            .filter(|m| m.is_active())
            .count()
    }

    /// Check if we have quorum
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn has_quorum(&self, quorum_size: usize) -> bool {
        self.active_count().await >= quorum_size
    }

    /// Set heartbeat timeout
    pub fn set_heartbeat_timeout(&mut self, timeout_ms: u64) {
        self.heartbeat_timeout_ms = timeout_ms;
    }
}

impl Default for Membership {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_membership_new() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);
        assert_eq!(membership.member_count().await, 0);
    }

    #[tokio::test]
    async fn test_member_new() {
        let node_id = NodeId::new();
        let member = Member::new(node_id, "addr".to_string());
        assert_eq!(member.node_id, node_id);
        assert_eq!(member.state, MemberState::Joining);
        assert!(member.is_active() == false);
    }

    #[tokio::test]
    async fn test_member_with_state() {
        let node_id = NodeId::new();
        let member = Member::new(node_id, "addr".to_string())
            .with_state(MemberState::Active);
        assert!(member.is_active());
    }

    #[tokio::test]
    async fn test_member_with_heartbeat() {
        let node_id = NodeId::new();
        let member = Member::new(node_id, "addr".to_string())
            .with_heartbeat(12345);
        assert_eq!(member.last_heartbeat, 12345);
    }

    #[tokio::test]
    async fn test_add_member() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        let member = Member::new(NodeId::new(), "addr".to_string())
            .with_state(MemberState::Active);
        membership.add_member(member).await.unwrap();

        assert_eq!(membership.member_count().await, 1);
        assert_eq!(membership.active_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_member() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        let member = Member::new(NodeId::new(), "addr".to_string());
        let id = member.node_id;
        membership.add_member(member).await.unwrap();

        let removed = membership.remove_member(id).await.unwrap();
        assert!(removed);
        assert_eq!(membership.member_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_member() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        let member = Member::new(NodeId::new(), "addr".to_string());
        let id = member.node_id;
        membership.add_member(member.clone()).await.unwrap();

        let retrieved = membership.get_member(id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().address, "addr");
    }

    #[tokio::test]
    async fn test_update_state() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        let member = Member::new(NodeId::new(), "addr".to_string());
        let id = member.node_id;
        membership.add_member(member).await.unwrap();

        let updated = membership.update_state(id, MemberState::Active).await.unwrap();
        assert!(updated);

        let retrieved = membership.get_member(id).await.unwrap();
        assert!(retrieved.is_active());
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        let member = Member::new(NodeId::new(), "addr".to_string())
            .with_state(MemberState::Active)
            .with_heartbeat(1000);
        let id = member.node_id;
        membership.add_member(member).await.unwrap();

        let updated = membership.update_heartbeat(id, 2000).await.unwrap();
        assert!(updated);

        let retrieved = membership.get_member(id).await.unwrap();
        assert_eq!(retrieved.last_heartbeat, 2000);
    }

    #[tokio::test]
    async fn test_check_heartbeats() {
        let node_id = NodeId::new();
        let mut membership = Membership::new(node_id);
        membership.set_heartbeat_timeout(1000);

        let member = Member::new(NodeId::new(), "addr".to_string())
            .with_state(MemberState::Active)
            .with_heartbeat(1000);
        membership.add_member(member).await.unwrap();

        let suspected = membership.check_heartbeats(2500).await.unwrap();
        assert_eq!(suspected.len(), 1);

        let active_count = membership.active_count().await;
        assert_eq!(active_count, 0);
    }

    #[tokio::test]
    async fn test_has_quorum() {
        let node_id = NodeId::new();
        let membership = Membership::new(node_id);

        // Add 2 active members
        for _ in 0..2 {
            let member = Member::new(NodeId::new(), "addr".to_string())
                .with_state(MemberState::Active);
            membership.add_member(member).await.unwrap();
        }

        assert!(membership.has_quorum(2).await);
        assert!(!membership.has_quorum(3).await);
    }

    #[test]
    fn test_member_state_equality() {
        assert_eq!(MemberState::Active, MemberState::Active);
        assert_ne!(MemberState::Active, MemberState::Suspected);
    }
}
