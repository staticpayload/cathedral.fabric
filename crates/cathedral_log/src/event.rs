//! Event types for the event log.
//!
//! All events are canonically encoded and part of the hash chain.

use crate::encoding::CanonicalEncode;
use cathedral_core::{EventId, RunId, NodeId, Hash, LogicalTime};
use serde::{Deserialize, Serialize};

/// Event kind - type of event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    RunCreated,
    RunStarted,
    RunCompleted,
    RunFailed,
    NodeScheduled,
    NodeStarted,
    NodeCompleted,
    NodeFailed,
    NodeSkipped,
    ToolInvoked,
    ToolCompleted,
    ToolFailed,
    ToolTimedOut,
    CapabilityCheck,
    PolicyDecision,
    TaskAssigned,
    TaskAccepted,
    TaskRejected,
    SnapshotCreated,
    SnapshotRestored,
    BlobStored,
    Heartbeat,
    Error,
}

impl EventKind {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::RunCompleted | Self::RunFailed | Self::NodeCompleted |
            Self::NodeFailed | Self::NodeSkipped | Self::ToolCompleted |
            Self::ToolFailed | Self::ToolTimedOut
        )
    }

    pub const fn is_error(self) -> bool {
        matches!(self, Self::RunFailed | Self::NodeFailed | Self::ToolFailed | Self::Error)
    }
}

/// A CATHEDRAL event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: EventId,
    pub run_id: RunId,
    pub node_id: NodeId,
    pub parent_event_id: Option<EventId>,
    pub logical_time: LogicalTime,
    pub kind: EventKind,
    pub payload: Vec<u8>,
    pub payload_hash: Hash,
    pub prior_state_hash: Option<Hash>,
    pub post_state_hash: Option<Hash>,
}

impl Event {
    pub fn new(
        event_id: EventId,
        run_id: RunId,
        node_id: NodeId,
        logical_time: LogicalTime,
        kind: EventKind,
    ) -> Self {
        Self {
            event_id,
            run_id,
            node_id,
            parent_event_id: None,
            logical_time,
            kind,
            payload: Vec::new(),
            payload_hash: Hash::empty(),
            prior_state_hash: None,
            post_state_hash: None,
        }
    }

    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload_hash = Hash::compute(&payload);
        self.payload = payload;
        self
    }

    pub fn with_state_hashes(mut self, prior: Hash, post: Hash) -> Self {
        self.prior_state_hash = Some(prior);
        self.post_state_hash = Some(post);
        self
    }

    pub fn with_parent(mut self, parent: EventId) -> Self {
        self.parent_event_id = Some(parent);
        self
    }

    pub fn is_terminal(&self) -> bool {
        self.kind.is_terminal()
    }

    pub fn is_error(&self) -> bool {
        self.kind.is_error()
    }
}

impl CanonicalEncode for Event {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            EventId::new(),
            RunId::new(),
            NodeId::new(),
            LogicalTime::zero(),
            EventKind::RunStarted,
        );
        assert!(!event.is_terminal());
        assert!(!event.is_error());
    }

    #[test]
    fn test_event_with_payload() {
        let event = Event::new(
            EventId::new(),
            RunId::new(),
            NodeId::new(),
            LogicalTime::zero(),
            EventKind::ToolCompleted,
        ).with_payload(b"data".to_vec());
        assert_eq!(event.payload, b"data");
        assert!(event.is_terminal());
    }

    #[test]
    fn test_event_encode() {
        let event = Event::new(
            EventId::new(),
            RunId::new(),
            NodeId::new(),
            LogicalTime::zero(),
            EventKind::RunStarted,
        );
        let _encoded = event.encode();
    }
}
