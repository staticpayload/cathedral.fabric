//! Recording of simulation runs for reproducibility.

use crate::seed::SimSeed;
use cathedral_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Record of a simulation run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimRecord {
    /// Seed used for the simulation
    pub seed: SimSeed,
    /// Maximum ticks configured
    pub max_ticks: u64,
    /// Events recorded during simulation
    pub events: Vec<(u64, NodeId, String)>,
    /// Final state snapshot
    pub final_snapshot: HashMap<NodeId, String>,
    /// Timestamp when simulation started
    pub started_at: u64,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl SimRecord {
    /// Create a new simulation record
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed: SimSeed::default(),
            max_ticks: 0,
            events: Vec::new(),
            final_snapshot: HashMap::new(),
            started_at: 0,
            duration_ms: 0,
        }
    }

    /// Add an event to the record
    #[must_use]
    pub fn with_event(mut self, tick: u64, node_id: NodeId, event: String) -> Self {
        self.events.push((tick, node_id, event));
        self
    }

    /// Set the seed
    #[must_use]
    pub fn with_seed(mut self, seed: SimSeed) -> Self {
        self.seed = seed;
        self
    }

    /// Get event count
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get events for a specific tick
    #[must_use]
    pub fn events_at_tick(&self, tick: u64) -> Vec<(NodeId, String)> {
        self.events
            .iter()
            .filter(|(t, _, _)| *t == tick)
            .map(|(_, node_id, event)| (*node_id, event.clone()))
            .collect()
    }

    /// Get events for a specific node
    #[must_use]
    pub fn events_for_node(&self, node_id: NodeId) -> Vec<(u64, String)> {
        self.events
            .iter()
            .filter(|(_, id, _)| *id == node_id)
            .map(|(tick, _, event)| (*tick, event.clone()))
            .collect()
    }

    /// Serialize to JSON
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    ///
    /// # Errors
    ///
    /// Returns error if deserialization fails
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for SimRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// A recorded simulation run that can be replayed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedRun {
    /// The simulation record
    pub record: SimRecord,
    /// Run metadata
    pub metadata: RunMetadata,
}

/// Metadata about a simulation run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Run name/description
    pub name: String,
    /// Git commit hash (if available)
    pub git_commit: Option<String>,
    /// Command line arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Additional labels
    pub labels: HashMap<String, String>,
}

impl RecordedRun {
    /// Create a new recorded run
    #[must_use]
    pub fn new(record: SimRecord) -> Self {
        Self {
            record,
            metadata: RunMetadata::default(),
        }
    }

    /// Set metadata
    #[must_use]
    pub fn with_metadata(mut self, metadata: RunMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Serialize to JSON
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    ///
    /// # Errors
    ///
    /// Returns error if deserialization fails
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for RunMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            git_commit: None,
            args: Vec::new(),
            env: HashMap::new(),
            labels: HashMap::new(),
        }
    }
}

/// Comparison of two simulation runs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunComparison {
    /// Whether runs are identical
    pub identical: bool,
    /// Deltas found
    pub deltas: Vec<RunDelta>,
}

/// A difference between two runs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDelta {
    /// Tick where delta occurred
    pub tick: u64,
    /// Node where delta occurred
    pub node_id: NodeId,
    /// Expected event
    pub expected: String,
    /// Actual event
    pub actual: String,
}

impl RunComparison {
    /// Compare two simulation records
    #[must_use]
    pub fn compare(record1: &SimRecord, record2: &SimRecord) -> Self {
        if record1.events.len() != record2.events.len() {
            return Self {
                identical: false,
                deltas: vec![RunDelta {
                    tick: 0,
                    node_id: NodeId::new(),
                    expected: format!("{} events", record1.events.len()),
                    actual: format!("{} events", record2.events.len()),
                }],
            };
        }

        let mut deltas = Vec::new();
        for ((tick1, node1, event1), (tick2, node2, event2)) in record1.events.iter().zip(record2.events.iter()) {
            if tick1 != tick2 || node1 != node2 || event1 != event2 {
                deltas.push(RunDelta {
                    tick: *tick1,
                    node_id: *node1,
                    expected: event1.clone(),
                    actual: event2.clone(),
                });
            }
        }

        Self {
            identical: deltas.is_empty(),
            deltas,
        }
    }

    /// Get delta count
    #[must_use]
    pub fn delta_count(&self) -> usize {
        self.deltas.len()
    }

    /// Get formatted delta report
    #[must_use]
    pub fn report(&self) -> String {
        if self.identical {
            return "Runs are identical".to_string();
        }

        let mut report = format!("Found {} deltas:\n", self.deltas.len());
        for delta in &self.deltas {
            report.push_str(&format!(
                "  Tick {}: Node {:?} - expected '{}', got '{}'\n",
                delta.tick, delta.node_id, delta.expected, delta.actual
            ));
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_record_new() {
        let record = SimRecord::new();
        assert_eq!(record.event_count(), 0);
        assert_eq!(record.max_ticks, 0);
    }

    #[test]
    fn test_sim_record_with_event() {
        let record = SimRecord::new()
            .with_event(1, NodeId::new(), "test".to_string());
        assert_eq!(record.event_count(), 1);
    }

    #[test]
    fn test_sim_record_events_at_tick() {
        let node_id = NodeId::new();
        let record = SimRecord::new()
            .with_event(1, node_id, "event1".to_string())
            .with_event(1, NodeId::new(), "event2".to_string())
            .with_event(2, node_id, "event3".to_string());

        let tick1_events = record.events_at_tick(1);
        assert_eq!(tick1_events.len(), 2);

        let tick2_events = record.events_at_tick(2);
        assert_eq!(tick2_events.len(), 1);
    }

    #[test]
    fn test_sim_record_events_for_node() {
        let node_id = NodeId::new();
        let record = SimRecord::new()
            .with_event(1, node_id, "event1".to_string())
            .with_event(2, node_id, "event2".to_string());

        let node_events = record.events_for_node(node_id);
        assert_eq!(node_events.len(), 2);
    }

    #[test]
    fn test_sim_record_default() {
        let record = SimRecord::default();
        assert_eq!(record.event_count(), 0);
    }

    #[test]
    fn test_sim_record_to_from_json() {
        let record = SimRecord::new()
            .with_event(1, NodeId::new(), "test".to_string());
        let json = record.to_json();
        let restored = SimRecord::from_json(&json).unwrap();
        assert_eq!(restored.event_count(), 1);
    }

    #[test]
    fn test_run_metadata_default() {
        let metadata = RunMetadata::default();
        assert_eq!(metadata.name, "");
        assert!(metadata.git_commit.is_none());
    }

    #[test]
    fn test_recorded_run_new() {
        let record = SimRecord::new();
        let run = RecordedRun::new(record);
        assert_eq!(run.metadata.name, "");
    }

    #[test]
    fn test_recorded_run_with_metadata() {
        let record = SimRecord::new();
        let metadata = RunMetadata {
            name: "test".to_string(),
            ..Default::default()
        };
        let run = RecordedRun::new(record).with_metadata(metadata);
        assert_eq!(run.metadata.name, "test");
    }

    #[test]
    fn test_run_comparison_identical() {
        let record = SimRecord::new();
        let comparison = RunComparison::compare(&record, &record);
        assert!(comparison.identical);
        assert_eq!(comparison.report(), "Runs are identical");
    }

    #[test]
    fn test_run_comparison_different() {
        let node_id = NodeId::new();
        let record1 = SimRecord::new().with_event(1, node_id, "event1".to_string());
        let record2 = SimRecord::new().with_event(1, node_id, "event2".to_string());

        let comparison = RunComparison::compare(&record1, &record2);
        assert!(!comparison.identical);
        assert_eq!(comparison.delta_count(), 1);
    }
}
