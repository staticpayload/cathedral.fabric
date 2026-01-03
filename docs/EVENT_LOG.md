# Event Log Specification

## Overview

The event log is the single source of truth for all CATHEDRAL executions. It is append-only, hash-chained, and canonically encoded for cross-platform reproducibility.

## Event Format

### Required Fields

```json
{
    "event_id": "evt_1a2b3c4d...",
    "run_id": "run_5e6f7a8b...",
    "node_id": "node_9c0d1e2f...",
    "parent_event_id": "evt_3a4b5c6d...",
    "logical_time": 42,
    "kind": "ToolInvocation",
    "payload": "<canonical encoded bytes>",
    "payload_hash": "hash_7b8c9d0e...",
    "prior_state_hash": "hash_1f2e3d4c...",
    "post_state_hash": "hash_5b6a7988...",
    "capability_check_result": {
        "allowed": true,
        "policy_decision_id": "dec_8a9b0c1d..."
    },
    "tool_request_hash": "hash_2b3c4d5e...",
    "tool_response_hash": "hash_6b7c8d9e...",
    "error_data": null
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | UUID | Unique identifier for this event |
| `run_id` | UUID | Identifies the workflow execution |
| `node_id` | UUID | DAG node this event belongs to |
| `parent_event_id` | UUID? | Causal parent event |
| `logical_time` | u64 | Monotonic counter per run |
| `kind` | EventKind | Type of event (see below) |
| `payload` | bytes | Canonical-encoded event data |
| `payload_hash` | Hash | BLAKE3 hash of payload |
| `prior_state_hash` | Hash? | Hash of state before event |
| `post_state_hash` | Hash? | Hash of state after event |
| `capability_check_result` | object | Policy decision for this event |
| `tool_request_hash` | Hash? | For tool invocations |
| `tool_response_hash` | Hash? | For tool invocations |
| `error_data` | object? | Error information if applicable |

## Event Kinds

```rust
pub enum EventKind {
    // Workflow lifecycle
    RunCreated,
    RunStarted,
    RunCompleted,
    RunFailed,

    // DAG execution
    NodeScheduled,
    NodeStarted,
    NodeCompleted,
    NodeFailed,
    NodeSkipped,

    // Tool execution
    ToolInvoked,
    ToolCompleted,
    ToolFailed,
    ToolTimedOut,

    // Capability and policy
    CapabilityCheck,
    PolicyDecision,

    // Cluster operations
    TaskAssigned,
    TaskAccepted,
    TaskRejected,

    // Storage
    SnapshotCreated,
    SnapshotRestored,
    BlobStored,

    // System
    Heartbeat,
    Error,
}
```

## Canonical Encoding

### Rules

1. **No HashMap** - Use `BTreeMap` or index-based structures
2. **Fixed integers** - Use `u64`, `i64` with explicit width
3. **Float handling** - Avoid or encode as string with fixed precision
4. **Enum encoding** - Use discriminant + index for variants
5. **Stable ordering** - Sort all collections before encoding

### Encoding Format

Default: `postcard` with COBS (CBOR alternative)

Rationale:
- No serde_json (inconsistent float encoding across platforms)
- Custom implementation if needed for absolute stability

### Validation Test

Cross-platform test verifies byte-identical encoding:

```rust
#[test]
fn test_cross_platform_encoding() {
    let event = create_test_event();
    let encoded = event.encode_canonical();

    // Verify on Linux, macOS, Windows
    assert_eq!(encoded, EXPECTED_BYTES);
}
```

## Hash Chain

### Chain Structure

```
event[N].prior_state_hash = hash(event[N-1].post_state_hash)
event[N].payload_hash = hash(canonical_encode(event[N].payload))
```

### Validation

```rust
pub struct ChainValidator {
    expected_prior_hash: Option<Hash>,
}

impl ChainValidator {
    pub fn validate(&mut self, event: &Event) -> Result<(), ChainError> {
        if let Some(expected) = self.expected_prior_hash {
            if event.prior_state_hash != expected {
                return Err(ChainError::BrokenLink);
            }
        }
        self.expected_prior_hash = Some(event.post_state_hash);
        Ok(())
    }
}
```

### Errors

- `BrokenLink`: Hash chain continuity broken
- `MissingHash`: Required hash field missing
- `InvalidHash`: Hash format invalid
- `ReorderedEvent`: Logical time non-monotonic

## Event Storage

### Write Path

```
Executor → EventBuilder → CanonicalEncoder → Hasher → LogWriter
```

### Read Path

```
LogReader → HashValidator → CanonicalDecoder → Event
```

### Persistence

- Append-only file
- Optional compaction (keeps hash chain intact)
- Content-addressed blob store for large payloads

## Streaming

```rust
pub trait EventStream {
    fn next(&mut self) -> Result<Option<Event>>;
    fn seek(&mut self, event_id: EventId) -> Result<()>;
    fn cursor(&self) -> Cursor;
}

pub struct Cursor {
    position: u64,
    direction: Direction,
}
```

## Performance

- Target: >100K events/second write throughput
- Hash: BLAKE3 (fast, cryptographic)
- Encoding: postcard (zero-copy where possible)

## Security

- All events signed in cluster mode
- Hash chain detects tampering
- Capability checks logged with proofs
- Sensitive data redaction in replay bundles
