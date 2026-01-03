//! Trace reader for replaying execution logs.

use cathedral_core::{CoreResult, CoreError, EventId, NodeId, LogicalTime};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Event from a trace during replay
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event ID
    pub id: EventId,
    /// Logical time when event occurred
    pub time: LogicalTime,
    /// Node that generated this event
    pub node_id: NodeId,
    /// Event type
    pub kind: TraceEventKind,
    /// Event data
    pub data: Vec<u8>,
    /// Parent event ID
    pub parent_id: Option<EventId>,
}

/// Kind of trace event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventKind {
    /// Node started execution
    NodeStarted,
    /// Node completed successfully
    NodeCompleted,
    /// Node failed
    NodeFailed { exit_code: i32 },
    /// Output produced
    OutputProduced,
    /// Side effect occurred
    SideEffect { effect: String },
    /// Capability check
    CapabilityCheck { capability: String, allowed: bool },
    /// Snapshot taken
    Snapshot,
}

/// Trace reader for reading execution logs
pub struct TraceReader {
    /// Buffered events
    buffer: VecDeque<TraceEvent>,
    /// Current position in trace
    position: usize,
    /// Total events in trace
    total: usize,
    /// Current logical time
    time: LogicalTime,
}

impl TraceReader {
    /// Create a new trace reader
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            position: 0,
            total: 0,
            time: LogicalTime::zero(),
        }
    }

    /// Create trace reader from events
    #[must_use]
    pub fn from_events(events: Vec<TraceEvent>) -> Self {
        let total = events.len();
        Self {
            buffer: events.into(),
            position: 0,
            total,
            time: LogicalTime::zero(),
        }
    }

    /// Read the next event
    ///
    /// # Errors
    ///
    /// Returns error if no more events
    pub fn next_event(&mut self) -> CoreResult<TraceEvent> {
        self.buffer
            .pop_front()
            .ok_or_else(|| CoreError::Validation {
                field: "trace".to_string(),
                reason: "No more events in trace".to_string(),
            })
    }

    /// Peek at the next event without consuming it
    ///
    /// # Errors
    ///
    /// Returns error if no more events
    pub fn peek_event(&self) -> CoreResult<&TraceEvent> {
        self.buffer.front().ok_or_else(|| CoreError::Validation {
            field: "trace".to_string(),
            reason: "No more events in trace".to_string(),
        })
    }

    /// Check if there are more events
    #[must_use]
    pub fn has_more(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get remaining event count
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.buffer.len()
    }

    /// Get current position
    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get total event count
    #[must_use]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.position = 0;
        self.time = LogicalTime::zero();
    }

    /// Seek to a specific position
    ///
    /// # Errors
    ///
    /// Returns error if position is out of bounds
    pub fn seek(&mut self, pos: usize) -> CoreResult<()> {
        if pos > self.total {
            return Err(CoreError::Validation {
                field: "position".to_string(),
                reason: format!("Position {} exceeds total {}", pos, self.total),
            });
        }
        self.position = pos;
        Ok(())
    }
}

impl Default for TraceReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over trace events
impl Iterator for TraceReader {
    type Item = CoreResult<TraceEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_more() {
            self.position += 1;
            Some(self.next_event())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_reader_new() {
        let reader = TraceReader::new();
        assert!(!reader.has_more());
        assert_eq!(reader.remaining(), 0);
        assert_eq!(reader.total(), 0);
    }

    #[test]
    fn test_trace_reader_from_events() {
        let events = vec![
            TraceEvent {
                id: EventId::new(),
                time: LogicalTime::zero(),
                node_id: NodeId::new(),
                kind: TraceEventKind::NodeStarted,
                data: Vec::new(),
                parent_id: None,
            },
            TraceEvent {
                id: EventId::new(),
                time: LogicalTime::from_raw(1),
                node_id: NodeId::new(),
                kind: TraceEventKind::NodeCompleted,
                data: Vec::new(),
                parent_id: None,
            },
        ];

        let reader = TraceReader::from_events(events);
        assert_eq!(reader.total(), 2);
        assert_eq!(reader.remaining(), 2);
        assert!(reader.has_more());
    }

    #[test]
    fn test_trace_reader_next_event() {
        let event = TraceEvent {
            id: EventId::new(),
            time: LogicalTime::zero(),
            node_id: NodeId::new(),
            kind: TraceEventKind::NodeStarted,
            data: vec![1, 2, 3],
            parent_id: None,
        };

        let mut reader = TraceReader::from_events(vec![event.clone()]);
        let next = reader.next_event().unwrap();
        assert_eq!(next.kind, TraceEventKind::NodeStarted);
        assert_eq!(next.data, vec![1, 2, 3]);
        assert!(!reader.has_more());
    }

    #[test]
    fn test_trace_reader_next_event_empty() {
        let mut reader = TraceReader::new();
        let result = reader.next_event();
        assert!(result.is_err());
    }

    #[test]
    fn test_trace_reader_peek() {
        let event = TraceEvent {
            id: EventId::new(),
            time: LogicalTime::zero(),
            node_id: NodeId::new(),
            kind: TraceEventKind::NodeStarted,
            data: Vec::new(),
            parent_id: None,
        };

        let reader = TraceReader::from_events(vec![event]);
        assert!(reader.peek_event().is_ok());
        assert_eq!(reader.remaining(), 1); // peek doesn't consume
    }

    #[test]
    fn test_trace_event_kind_serialization() {
        let kind = TraceEventKind::NodeFailed { exit_code: 42 };
        let serialized = serde_json::to_vec(&kind).unwrap();
        let deserialized: TraceEventKind = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(kind, deserialized);
    }
}
