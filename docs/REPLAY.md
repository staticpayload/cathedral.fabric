# Replay Engine Specification

## Overview

The replay engine reconstructs execution state from snapshots and event logs. It validates determinism by reproducing the exact state at each event.

## Capabilities

1. **Full state reconstruction** - Rebuild coordinator and all worker states
2. **Snapshot recovery** - Start from any snapshot boundary
3. **Divergence detection** - Identify where runs differ
4. **Diff generation** - Minimal, stable diff between runs
5. **Portable bundles** - Replay without network access

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         Replay Engine                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Loader     │  │  Validator   │  │    State Builder     │  │
│  │  bundles     │  │  hash chain  │  │  reconstruction      │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
└─────────┼─────────────────┼─────────────────────┼──────────────┘
          │                 │                     │
          ▼                 ▼                     ▼
┌────────────────────────────────────────────────────────────────┐
│                      Event Processor                            │
│  - Replay events in order                                      │
│  - Validate each step                                          │
│  - Track state transitions                                     │
└────────────────────────────────────────────────────────────────┘
          │
          ▼
┌────────────────────────────────────────────────────────────────┐
│                      Diff Engine                                │
│  - Compare two runs                                            │
│  - Find first divergence                                       │
│  - Trace causal ancestors                                      │
└────────────────────────────────────────────────────────────────┘
```

## Replay Process

### 1. Load Bundle

```rust
pub struct ReplayBundle {
    pub metadata: BundleMetadata,
    pub snapshot: Option<Snapshot>,
    pub events: Vec<Event>,
    pub blobs: ContentStore,
}
```

### 2. Validate

- Hash chain integrity
- Snapshot signatures
- Event ordering (logical time monotonic)
- Required fields present

### 3. Reconstruct State

```rust
pub struct ReplayEngine {
    run_id: RunId,
    state: ReconstructedState,
    events: Vec<Event>,
}

impl ReplayEngine {
    pub fn replay(&mut self) -> Result<ReplayResult> {
        for event in &self.events {
            self.validate_event(event)?;
            self.apply_event(event)?;
        }
        Ok(ReplayResult::from(self.state.clone()))
    }
}
```

### 4. Produce Output

- Final state
- Event trace
- Metrics
- Divergence info (if comparing)

## State Reconstruction

### Coordinator State

```rust
pub struct CoordinatorState {
    pub dag: Dag,
    pub node_states: BTreeMap<NodeId, NodeState>,
    pub pending_tasks: Vec<Task>,
    pub completed_nodes: BTreeSet<NodeId>,
    pub logical_time: LogicalTime,
}
```

### Worker State

```rust
pub struct WorkerState {
    pub worker_id: WorkerId,
    pub active_tasks: BTreeMap<TaskId, Task>,
    pub completed_tasks: BTreeSet<TaskId>,
    pub capabilities: CapabilitySet,
}
```

## Replay Verification

The replay engine verifies:

1. **Byte-identical compilation** - DAG compiles to same bytes
2. **Identical scheduling** - Same decisions from log
3. **State hash matches** - Each state hash validates
4. **Tool output hashes** - Normalized output hashes match

```rust
pub struct ReplayVerification {
    pub dag_match: bool,
    pub schedule_match: bool,
    pub state_hashes_match: bool,
    pub output_hashes_match: bool,
}
```

## Divergence Detection

### Detection

When comparing two runs:

```rust
pub fn find_divergence(left: &Run, right: &Run) -> Option<Divergence> {
    for (i, (l, r)) in left.events.iter().zip(right.events.iter()).enumerate() {
        if l != r {
            return Some(Divergence {
                event_index: i,
                left_event: l.clone(),
                right_event: r.clone(),
            });
        }
    }
    None
}
```

### Causal Tracing

For each divergence, trace causal ancestors:

```rust
pub fn trace_ancestors(run: &Run, event: &Event) -> Vec<Event> {
    let mut ancestors = Vec::new();
    let mut current = event.parent_event_id;

    while let Some(id) = current {
        let ev = run.find_event(id)?;
        ancestors.push(ev.clone());
        current = ev.parent_event_id;
    }

    ancestors
}
```

## Diff Output

### Human-Readable

```
Divergence at event #42

Left (run-001):
  kind: ToolCompleted
  tool: web_fetch
  output_hash: abc123...

Right (run-002):
  kind: ToolCompleted
  tool: web_fetch
  output_hash: def456...

Causal chain:
  - #41: ToolInvoked (web_fetch)
  - #40: NodeStarted (fetch_data)
  - #1: RunStarted
```

### Machine-Readable (JSON)

```json
{
    "divergence_event": 42,
    "left": {
        "event_id": "evt_abc...",
        "kind": "ToolCompleted",
        "output_hash": "abc123..."
    },
    "right": {
        "event_id": "evt_def...",
        "kind": "ToolCompleted",
        "output_hash": "def456..."
    },
    "causal_ancestors": ["evt_41", "evt_40", "evt_1"]
}
```

## Partial Replay

Start from a snapshot:

```rust
impl ReplayEngine {
    pub fn replay_from_snapshot(
        &mut self,
        snapshot_id: SnapshotId,
    ) -> Result<ReplayResult> {
        let snapshot = self.load_snapshot(snapshot_id)?;
        self.state = snapshot.into_state();
        self.replay_events_after(snapshot.logical_time)
    }
}
```

## Bundle Format

```
bundle.cath-bundle/
├── metadata.json        # Run metadata
├── snapshot.cath-snap   # Optional starting snapshot
├── events.cath-log      # Event log
├── blobs/              # Content-addressed storage
│   ├── abc123...
│   └── def456...
└── manifest.json       # Hash of all files
```

## Performance

- Replay at ~50K events/second
- Validation adds ~20% overhead
- Bundle compression: 5-10x

## Security

- All bundle files content-addressed
- Manifest hash verifies integrity
- Snapshot validation prevents injection
- Redaction rules apply on export
