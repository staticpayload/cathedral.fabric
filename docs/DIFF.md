# Diff Engine Specification

## Overview

The diff engine compares two CATHEDRAL runs and produces a stable, minimal explanation of their differences.

## Goals

1. **Find first divergence** - Earliest event where runs differ
2. **Causal tracing** - Show what led to divergence
3. **Stable output** - Same comparison produces same diff
4. **Human-readable** - Clear explanations
5. **Machine-readable** - JSON for automation

## Diff Types

### Event Diff

Compare events at same logical time:

```rust
pub enum EventDiff {
    Same,
    Different {
        left: Event,
        right: Event,
        field_diffs: Vec<FieldDiff>,
    },
    MissingLeft { right: Event },
    MissingRight { left: Event },
}
```

### State Diff

Compare reconstructed states:

```rust
pub struct StateDiff {
    pub node_states: BTreeMap<NodeId, NodeStateDiff>,
    pub dag_diff: Option<DagDiff>,
    pub capability_diffs: Vec<CapabilityDiff>,
}
```

### Tool Output Diff

Compare normalized tool outputs:

```rust
pub struct OutputDiff {
    pub tool_name: String,
    pub left_output: NormalizedOutput,
    pub right_output: NormalizedOutput,
    pub semantic_diff: SemanticDiff,
}
```

## Diff Algorithm

### 1. Align Events

```rust
pub fn align_events(left: &[Event], right: &[Event]) -> Vec<(Option<Event>, Option<Event>)> {
    let mut pairs = Vec::new();
    let mut li = 0;
    let mut ri = 0;

    while li < left.len() || ri < right.len() {
        match (left.get(li), right.get(ri)) {
            (Some(l), Some(r)) if l.logical_time == r.logical_time => {
                pairs.push((Some(l.clone()), Some(r.clone())));
                li += 1;
                ri += 1;
            }
            (Some(l), None) => {
                pairs.push((Some(l.clone()), None));
                li += 1;
            }
            (None, Some(r)) => {
                pairs.push((None, Some(r.clone())));
                ri += 1;
            }
            (Some(l), Some(r)) => {
                // Logical time mismatch - divergence
                pairs.push((Some(l.clone()), Some(r.clone())));
                li += 1;
                ri += 1;
            }
        }
    }

    pairs
}
```

### 2. Find First Divergence

```rust
pub fn find_first_divergence(
    left: &[Event],
    right: &[Event],
) -> Option<usize> {
    for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
        if l != r {
            return Some(i);
        }
    }
    if left.len() != right.len() {
        return Some(left.len().min(right.len()));
    }
    None
}
```

### 3. Trace Causal Ancestors

```rust
pub fn trace_causal_ancestors(
    events: &[Event],
    event: &Event,
) -> Vec<EventId> {
    let mut ancestors = Vec::new();
    let mut current_id = event.parent_event_id;

    while let Some(id) = current_id {
        if let Some(ev) = events.iter().find(|e| e.event_id == id) {
            ancestors.push(ev.event_id);
            current_id = ev.parent_event_id;
        } else {
            break;
        }
    }

    ancestors
}
```

## Output Formats

### Human-Readable Summary

```
DIFF REPORT: run-001 vs run-002

First Divergence: Event #47 (logical_time: 47)

┌─────────────────────────────────────────────────────────────────┐
│ Left (run-001):                                                 │
│   kind: ToolCompleted                                           │
│   tool: http_fetch                                              │
│   output_hash: 7f3a8b...                                        │
│   status: success                                               │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ Right (run-002):                                                │
│   kind: ToolCompleted                                           │
│   tool: http_fetch                                              │
│   output_hash: 9e4c2d...                                        │
│   status: success                                               │
└─────────────────────────────────────────────────────────────────┘

Causal Chain:
  #46 → #45 → #44 → #1 (RunStarted)

Affected Nodes:
  - process_data (depends on fetch)
  - write_output (depends on process_data)

Summary: Tool http_fetch produced different output. External data source may have changed.
```

### Machine-Readable (JSON)

```json
{
    "left_run_id": "run-001",
    "right_run_id": "run-002",
    "first_divergence_index": 47,
    "first_divergence_logical_time": 47,
    "divergence": {
        "type": "EventMismatch",
        "left": {
            "event_id": "evt_abc123",
            "kind": "ToolCompleted",
            "tool": "http_fetch",
            "output_hash": "7f3a8b..."
        },
        "right": {
            "event_id": "evt_def456",
            "kind": "ToolCompleted",
            "tool": "http_fetch",
            "output_hash": "9e4c2d..."
        }
    },
    "causal_ancestors": ["evt_def456", "evt_ghi789", "evt_jkl012"],
    "affected_nodes": ["process_data", "write_output"],
    "likely_cause": "ExternalDataChanged"
}
```

## Semantic Diff

For structured outputs:

```rust
pub fn semantic_diff(left: &Value, right: &Value) -> SemanticDiff {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            let mut field_diffs = BTreeMap::new();

            for key in l.keys().chain(r.keys()).collect::<BTreeSet<_>>() {
                match (l.get(key), r.get(key)) {
                    (Some(lv), Some(rv)) if lv != rv => {
                        field_diffs.insert(key.clone(), semantic_diff(lv, rv));
                    }
                    (Some(_), None) => {
                        field_diffs.insert(key.clone(), SemanticDiff::Removed);
                    }
                    (None, Some(_)) => {
                        field_diffs.insert(key.clone(), SemanticDiff::Added);
                    }
                    _ => {}
                }
            }

            SemanticDiff::Object { fields: field_diffs }
        }
        (Value::Array(l), Value::Array(r)) => {
            // Array diff with index alignment
            let mut diffs = Vec::new();
            for (i, (lv, rv)) in l.iter().zip(r.iter()).enumerate() {
                if lv != rv {
                    diffs.push((i, semantic_diff(lv, rv)));
                }
            }
            SemanticDiff::Array { elements: diffs }
        }
        (l, r) => SemanticDiff::ValueChanged {
            left: l.clone(),
            right: r.clone(),
        },
    }
}
```

## CLI Usage

```bash
# Basic diff
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle

# JSON output
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle --json

# Only show causal chain
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle --causal-only

# Deep semantic diff of outputs
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle --semantic
```

## Stability Guarantees

1. **Same inputs, same output** - Diff is deterministic
2. **Ordered fields** - Object fields sorted alphabetically
3. **Stable indexes** - Array positions preserved
4. **Hash-based comparison** - Not affected by formatting

## Performance

- Diff two 10K-event runs in <100ms
- Semantic diff proportional to output size
- Causal tracing O(n) where n = event depth
