# Failure Modes

## Overview

This document catalogs all failure modes in CATHEDRAL and how they are handled.

## Failure Categories

### 1. Network Failures

#### 1.1 Partition

**Description**: Network partition separates cluster nodes.

**Detection**:
- Raft election timeout expires
- Heartbeats not received
- RPC calls fail

**Handling**:
- Followers detect no leader, start new election
- Leader steps down if can't contact majority
- No new decisions until partition heals
- Existing decisions on majority side preserved

**Recovery**:
- When partition heals, nodes reconcile logs
- Follower catches up to leader's log
- No data loss if majority stayed connected

**Logging**:
- `HeartbeatFailed` event
- `ElectionStarted` event
- `LeadershipLost` event

#### 1.2 Packet Loss

**Description**: Individual packets lost in transit.

**Detection**:
- RPC timeout
- Request retransmission

**Handling**:
- Retries with exponential backoff
- Idempotent operations where possible
- Raft ensures exactly-once semantics

#### 1.3 Message Reordering

**Description**: Messages arrive out of order.

**Detection**:
- Sequence numbers in Raft
- Logical time in events

**Handling**:
- Raft provides total ordering
- Logical time ensures causal ordering
- Buffering until correct order

### 2. Node Failures

#### 2.1 Worker Crash

**Description**: Worker process crashes.

**Detection**:
- Heartbeat timeout
- RPC failure

**Handling**:
- Coordinator detects failure via heartbeat
- Tasks reassigned to other workers
- No state lost (logged)

**Recovery**:
- Worker can rejoin cluster
- Replays state from snapshot + log
- Can resume processing new tasks

**Logging**:
- `WorkerUnreachable` event
- `TaskReassigned` event

#### 2.2 Coordinator Crash

**Description**: Coordinator process crashes.

**Detection**:
- Raft leader timeout
- Followers detect no heartbeat

**Handling**:
- New leader elected via Raft
- Scheduler active on new leader
- No data loss (log replicated)

**Recovery**:
- Old coordinator can rejoin as follower
- Catches up to current log

#### 2.3 Disk Failure

**Description**: Disk becomes unavailable or corrupted.

**Detection**:
- I/O errors on read/write
- Checksum failures

**Handling**:
- Node marks itself unavailable
- Cluster continues without it
- Data recovery from replicas (if configured)

**Logging**:
- `StorageError` event
- `NodeOffline` event

### 3. Tool Failures

#### 3.1 Tool Crash

**Description**: Tool process crashes during execution.

**Detection**:
- Process exit signal
- Timeout

**Handling**:
- `ToolFailed` event logged
- Retry policy applied
- No partial state committed

**Logging**:
- `ToolFailed` event with exit code
- `TaskRetry` event if retried

#### 3.2 Tool Timeout

**Description**: Tool exceeds time limit.

**Detection**:
- Timeout timer expires

**Handling**:
- Tool process terminated
- `ToolTimedOut` event logged
- Retry if configured

**Logging**:
- `ToolTimedOut` event
- `TaskRetry` event if retried

#### 3.3 Tool Resource Exhaustion

**Description**: Tool exceeds memory or CPU limits.

**Detection**:
- WASM out of fuel
- Memory limit exceeded

**Handling**:
- Tool terminated immediately
- `ToolResourceExceeded` event logged
- Retry if configured

**Logging**:
- `ToolResourceExceeded` event
- Resource usage stats

#### 3.4 Tool Returns Invalid Output

**Description**: Tool output doesn't match schema.

**Detection**:
- Schema validation on output

**Handling**:
- `ToolOutputInvalid` event logged
- Error propagated to DAG
- Dependent tasks marked as failed

**Logging**:
- `ToolOutputInvalid` event
- Validation error details

### 4. Storage Failures

#### 4.1 Blob Corruption

**Description**: Blob file corrupted on disk.

**Detection**:
- Content hash mismatch on read
- Checksum validation

**Handling**:
- Blob rejected
- Error returned to caller
- Recovery from replica if available

**Logging**:
- `BlobCorrupted` event
- `BlobHashMismatch` event

#### 4.2 Snapshot Corruption

**Description**: Snapshot file corrupted.

**Detection**:
- Snapshot hash mismatch
- Metadata validation failure

**Handling**:
- Snapshot rejected
- Falls back to older snapshot
- Replays more events

**Logging**:
- `SnapshotCorrupted` event
- `SnapshotFallback` event

#### 4.3 Log Truncation

**Description**: Event log truncated.

**Detection**:
- Hash chain validation fails
- Missing prior hash

**Handling**:
- Log rejected
- Error returned
- Recovery from snapshot if available

**Logging**:
- `LogTruncated` event
- `ChainBroken` event

### 5. Cluster Failures

#### 5.1 Split Brain

**Description**: Two nodes both believe they are leader.

**Prevention**:
- Raft ensures at most one leader
- Quorum requirement for commits

**Detection**:
- Duplicate leader detection
- Term number conflict

**Handling**:
- Lower-term leader steps down
- Higher-term leader prevails

**Logging**:
- `DuplicateLeaderDetected` event
- `LeadershipConflict` event

#### 5.2 Loss of Quorum

**Description**: Not enough nodes to form majority.

**Detection**:
- Raft can't commit entries
- Election fails repeatedly

**Handling**:
- Cluster stops accepting writes
- Reads continue from available nodes
- Manual intervention may be needed

**Logging**:
- `QuorumLost` event
- `ClusterReadOnly` event

### 6. Replay Failures

#### 6.1 Replay Divergence

**Description**: Replay produces different state.

**Detection**:
- State hash mismatch
- Different output for same input

**Handling**:
- Divergence logged
- Diff engine produces explanation
- Non-determinism investigated

**Logging**:
- `ReplayDiverged` event
- Detailed diff report

#### 6.2 Bundle Corruption

**Description**: Replay bundle file corrupted.

**Detection**:
- Bundle hash mismatch
- Manifest validation failure

**Handling**:
- Bundle rejected
- Error returned
- Attempt recovery from backup

**Logging**:
- `BundleCorrupted` event
- `BundleValidationFailed` event

### 7. Policy Failures

#### 7.1 Policy Parse Error

**Description**: Policy file has syntax error.

**Detection**:
- Parser error during compilation

**Handling**:
- Policy rejected
- Error message with location
- No execution without valid policy

**Logging**:
- `PolicyParseError` event
- Error location and message

#### 7.2 Policy Conflict

**Description**: Policy has conflicting rules.

**Detection**:
- Conflict detection during compilation

**Handling**:
- Policy rejected
- Conflict details provided
- Must be resolved before use

**Logging**:
- `PolicyConflict` event
- Conflict details

#### 7.3 Policy Decision Timeout

**Description**: Policy evaluation takes too long.

**Detection**:
- Timeout on policy decision

**Handling**:
- Request denied (fail closed)
- `PolicyTimeout` event logged
- Policy needs optimization

**Logging**:
- `PolicyTimeout` event

### 8. WASM Sandbox Failures

#### 8.1 Out of Fuel

**Description**: WASM execution exceeds fuel limit.

**Detection**:
- Wasmtime fuel exhaustion

**Handling**:
- Execution terminated
- `ToolOutOfFuel` event logged
- No partial state committed

**Logging**:
- `ToolOutOfFuel` event
- Fuel consumed vs limit

#### 8.2 Memory Limit Exceeded

**Description**: WASM tries to allocate beyond limit.

**Detection**:
- Memory limiter catches allocation

**Handling**:
- Execution terminated
- `ToolMemoryExceeded` event logged

**Logging**:
- `ToolMemoryExceeded` event
- Memory requested vs limit

#### 8.3 Host Function Error

**Description**: Host function fails during WASM call.

**Detection**:
- Host function returns error

**Handling**:
- Error propagated to WASM
- Tool can handle or fail
- Logged in event log

**Logging**:
- `HostFunctionError` event
- Function name and error

## Error Recovery Strategies

### Retry Policies

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub retry_on: Vec<ErrorKind>,
}

pub enum BackoffStrategy {
    Fixed { duration: Duration },
    Exponential { base: Duration, max: Duration },
}
```

### Fallback Strategies

1. **Retry** - For transient failures
2. **Skip** - For optional steps
3. **Fail** - For critical failures
4. **Fallback** - Use alternative implementation

### Circuit Breaking

```rust
pub struct CircuitBreaker {
    failure_threshold: u32,
    timeout: Duration,
    state: CircuitState,
}

pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}
```

## Testing Failure Modes

### Chaos Engineering

The simulation framework injects failures:

```rust
let sim = SimHarness::new();
sim.inject_partition(vec![worker1, worker2], Duration::from_secs(5));
sim.inject_crash(worker3);
sim.run().await;

assert!(sim.recovered());
```

### Property-Based Testing

```rust
#[proptest]
fn test_crash_recovery(steps: Vec<SimStep>) {
    let mut sim = SimHarness::new();
    for step in steps {
        sim.step(step).unwrap();
    }
    assert!(sim.verify_consistency());
}
```
