//! Replay engine for deterministic reconstruction.

use cathedral_core::{CoreResult, CoreError, NodeId};
use crate::trace::{TraceReader, TraceEvent};
use crate::state::{ReconstructedState, NodeState};
use crate::snapshot::SnapshotLoader;
use serde::{Deserialize, Serialize};

/// Replay engine configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Stop on first error
    pub stop_on_error: bool,
    /// Validate hash chain during replay
    pub validate_hash_chain: bool,
    /// Maximum events to replay (0 = unlimited)
    pub max_events: usize,
    /// Enable snapshot loading
    pub enable_snapshots: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            stop_on_error: true,
            validate_hash_chain: true,
            max_events: 0,
            enable_snapshots: true,
        }
    }
}

/// Replay engine error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayEngineError {
    /// Trace is empty
    EmptyTrace,
    /// Event mismatch
    EventMismatch { expected: String, actual: String },
    /// Missing snapshot
    MissingSnapshot { id: String },
    /// Corrupted trace
    CorruptedTrace { reason: String },
    /// Validation failed
    ValidationFailed { reason: String },
}

impl std::fmt::Display for ReplayEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "Trace is empty"),
            Self::EventMismatch { expected, actual } => {
                write!(f, "Event mismatch: expected {}, got {}", expected, actual)
            }
            Self::MissingSnapshot { id } => write!(f, "Missing snapshot: {}", id),
            Self::CorruptedTrace { reason } => write!(f, "Corrupted trace: {}", reason),
            Self::ValidationFailed { reason } => write!(f, "Validation failed: {}", reason),
        }
    }
}

impl std::error::Error for ReplayEngineError {}

impl From<ReplayEngineError> for CoreError {
    fn from(err: ReplayEngineError) -> Self {
        CoreError::Validation {
            field: "replay".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Replay engine for reconstructing execution from traces
pub struct ReplayEngine {
    config: ReplayConfig,
    snapshot_loader: Option<SnapshotLoader>,
}

impl ReplayEngine {
    /// Create a new replay engine
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ReplayConfig::default(),
            snapshot_loader: None,
        }
    }

    /// Create with custom config
    #[must_use]
    pub fn with_config(mut self, config: ReplayConfig) -> Self {
        self.config = config;
        self
    }

    /// Set snapshot loader
    #[must_use]
    pub fn with_snapshot_loader(mut self, loader: SnapshotLoader) -> Self {
        self.snapshot_loader = Some(loader);
        self
    }

    /// Replay a trace reader to reconstruct state
    ///
    /// # Errors
    ///
    /// Returns error if replay fails
    pub fn replay(&mut self, reader: &mut TraceReader) -> CoreResult<ReconstructedState> {
        let mut state = ReconstructedState::new();

        if !reader.has_more() {
            return Err(ReplayEngineError::EmptyTrace.into());
        }

        let event_count = reader.total();
        let max_events = if self.config.max_events > 0 {
            self.config.max_events
        } else {
            event_count
        };

        for i in 0..max_events {
            if !reader.has_more() {
                break;
            }

            let event = reader.next_event()?;
            self.process_event(&mut state, &event)?;

            if self.config.stop_on_error && state.has_errors() {
                break;
            }

            // Check progress
            if i % 1000 == 0 && i > 0 {
                // Progress reporting could go here
                let _ = i;
            }
        }

        Ok(state)
    }

    /// Process a single trace event
    fn process_event(
        &mut self,
        state: &mut ReconstructedState,
        event: &TraceEvent,
    ) -> CoreResult<()> {
        state.tick();

        match &event.kind {
            crate::trace::TraceEventKind::NodeStarted => {
                // Initialize node state
                let node_state = NodeState::new(event.node_id);
                state.add_node_state(event.node_id, node_state);
            }
            crate::trace::TraceEventKind::NodeCompleted => {
                // Mark node as completed
                if let Some(node_state) = state.node_outputs.get_mut(&event.node_id) {
                    node_state.completed = true;
                    node_state.output = Some(event.data.clone());
                }
            }
            crate::trace::TraceEventKind::NodeFailed { exit_code } => {
                // Mark node as failed
                let error = crate::state::ReplayError {
                    node_id: event.node_id,
                    message: format!("Node failed with exit code {}", exit_code),
                    time: state.time(),
                };
                state.add_error(error);

                if let Some(node_state) = state.node_outputs.get_mut(&event.node_id) {
                    node_state.completed = false;
                    node_state.error = Some(format!("Exit code {}", exit_code));
                }
            }
            crate::trace::TraceEventKind::OutputProduced => {
                // Update node output
                if let Some(node_state) = state.node_outputs.get_mut(&event.node_id) {
                    node_state.output = Some(event.data.clone());
                }
            }
            crate::trace::TraceEventKind::SideEffect { effect } => {
                // Record side effect
                if let Some(node_state) = state.node_outputs.get_mut(&event.node_id) {
                    node_state.side_effects.push(effect.clone());
                }
            }
            crate::trace::TraceEventKind::CapabilityCheck { capability, allowed } => {
                // Track capability checks
                if !*allowed {
                    let error = crate::state::ReplayError {
                        node_id: event.node_id,
                        message: format!("Capability denied: {}", capability),
                        time: state.time(),
                    };
                    state.add_error(error);
                }
            }
            crate::trace::TraceEventKind::Snapshot => {
                // Handle snapshot event
                if let Some(loader) = &self.snapshot_loader {
                    match loader.load(&event.data) {
                        Ok(snapshot_state) => {
                            state.merge(snapshot_state);
                        }
                        Err(_) => {
                            // Continue without snapshot
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Replay with step-by-step callbacks
    ///
    /// # Errors
    ///
    /// Returns error if replay fails
    pub fn replay_with_callback<F>(
        &mut self,
        reader: &mut TraceReader,
        mut callback: F,
    ) -> CoreResult<ReconstructedState>
    where
        F: FnMut(&TraceEvent, &ReconstructedState),
    {
        let mut state = ReconstructedState::new();

        while reader.has_more() {
            let event = reader.next_event()?;
            self.process_event(&mut state, &event)?;
            callback(&event, &state);

            if self.config.stop_on_error && state.has_errors() {
                break;
            }
        }

        Ok(state)
    }

    /// Verify that two traces produce the same state
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify_equivalence(
        &mut self,
        reader1: &mut TraceReader,
        reader2: &mut TraceReader,
    ) -> CoreResult<bool> {
        let state1 = self.replay(reader1)?;
        let state2 = self.replay(reader2)?;

        Ok(state1 == state2)
    }
}

impl Default for ReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::trace::{TraceEvent, TraceEventKind};
    use cathedral_core::{EventId, LogicalTime};

    #[test]
    fn test_replay_engine_new() {
        let engine = ReplayEngine::new();
        assert!(engine.config.stop_on_error);
    }

    #[test]
    fn test_replay_config_default() {
        let config = ReplayConfig::default();
        assert!(config.stop_on_error);
        assert!(config.validate_hash_chain);
        assert_eq!(config.max_events, 0);
    }

    #[test]
    fn test_replay_empty_trace() {
        let mut engine = ReplayEngine::new();
        let mut reader = TraceReader::new();
        let result = engine.replay(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_replay_single_event() {
        let mut engine = ReplayEngine::new();
        let event = TraceEvent {
            id: EventId::new(),
            time: LogicalTime::zero(),
            node_id: NodeId::new(),
            kind: TraceEventKind::NodeStarted,
            data: Vec::new(),
            parent_id: None,
        };

        let mut reader = TraceReader::from_events(vec![event]);
        let state = engine.replay(&mut reader).unwrap();
        assert_eq!(state.time(), 1);
        assert_eq!(state.total_nodes(), 1);
    }

    #[test]
    fn test_replay_node_completed() {
        let mut engine = ReplayEngine::new();
        let node_id = NodeId::new();

        let events = vec![
            TraceEvent {
                id: EventId::new(),
                time: LogicalTime::zero(),
                node_id,
                kind: TraceEventKind::NodeStarted,
                data: Vec::new(),
                parent_id: None,
            },
            TraceEvent {
                id: EventId::new(),
                time: LogicalTime::from_raw(1),
                node_id,
                kind: TraceEventKind::NodeCompleted,
                data: b"output".to_vec(),
                parent_id: None,
            },
        ];

        let mut reader = TraceReader::from_events(events);
        let state = engine.replay(&mut reader).unwrap();
        assert_eq!(state.total_nodes(), 1);
        assert_eq!(state.completed_count(), 1);

        let node_state = state.get_node_state(node_id).unwrap();
        assert!(node_state.completed);
        assert_eq!(node_state.output, Some(b"output".to_vec()));
    }

    #[test]
    fn test_replay_node_failed() {
        let config = ReplayConfig {
            stop_on_error: false,
            ..Default::default()
        };
        let mut engine = ReplayEngine::new().with_config(config);
        let node_id = NodeId::new();

        let events = vec![
            TraceEvent {
                id: EventId::new(),
                time: LogicalTime::zero(),
                node_id,
                kind: TraceEventKind::NodeFailed { exit_code: 1 },
                data: Vec::new(),
                parent_id: None,
            },
        ];

        let mut reader = TraceReader::from_events(events);
        let state = engine.replay(&mut reader).unwrap();
        assert!(state.has_errors());
    }

    #[test]
    fn test_replay_error_display() {
        let err = ReplayEngineError::EmptyTrace;
        assert_eq!(err.to_string(), "Trace is empty");
    }

    #[test]
    fn test_replay_max_events() {
        let config = ReplayConfig {
            max_events: 1,
            ..Default::default()
        };
        let mut engine = ReplayEngine::new().with_config(config);

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
                kind: TraceEventKind::NodeStarted,
                data: Vec::new(),
                parent_id: None,
            },
        ];

        let mut reader = TraceReader::from_events(events);
        let state = engine.replay(&mut reader).unwrap();
        assert_eq!(state.total_nodes(), 1); // Only first event processed
    }
}
