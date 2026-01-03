# CATHEDRAL.FABRIC Architecture

## Overview

CATHEDRAL.FABRIC is a distributed, event-sourced execution engine for agent workflows with determinism guarantees.

## Core Principles

1. **Determinism is mandatory** - Same inputs produce identical event sequences
2. **Every side effect is explicit** - No silent mutations
3. **Event log is append-only** - Hash-chained, canonical encoding
4. **No ambient authority** - Capability-based security
5. **Replay correctness beats performance** - Correctness first
6. **Security beats convenience** - Secure by default

## Component Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                         User Interface                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │   CLI    │  │   TUI    │  │   API    │  │    Language SDK  │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘  │
└───────┼─────────────┼─────────────┼─────────────────┼────────────┘
        │             │             │                 │
        └─────────────┴─────────────┴─────────────────┘
                              │
┌────────────────────────────────────────────────────────────────────┐
│                        Planner Layer                               │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  DSL Parser → AST → Type Checker → Resource Analyzer       │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐  │
│  │  DAG Compiler: typed nodes, edges, capability contracts    │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
└──────────────────────────────┼─────────────────────────────────────┘
                               │
┌────────────────────────────────────────────────────────────────────┐
│                       Runtime Layer                                │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  Scheduler: priority, resources, dependencies               │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐  │
│  │  Executor: capability checks, tool invocation, I/O logging  │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
└──────────────────────────────┼─────────────────────────────────────┘
                               │
┌────────────────────────────────────────────────────────────────────┐
│                      Policy & Tool Layer                           │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────┐   │
│  │  Policy Engine   │  │   Tool Registry  │  │  WASM Sandbox  │   │
│  │  - Language      │  │   - Schema       │  │  - Fuel limit  │   │
│  │  - Compiler      │  │   - Normalize    │  │  - Memory      │   │
│  │  - Proofs        │  │   - Validate     │  │  - Host calls  │   │
│  └──────────────────┘  └──────────────────┘  └────────────────┘   │
└────────────────────────────────────────────────────────────────────┘
                               │
┌────────────────────────────────────────────────────────────────────┐
│                    Storage & Cluster Layer                         │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  Replicated Log: Raft consensus, hash chain, append-only   │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐  │
│  │  Content Store: blobs, snapshots, compaction               │  │
│  └───────────────────────────┬─────────────────────────────────┘  │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐  │
│  │  Cluster: membership, leader election, remote execution    │  │
│  └─────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
                               │
┌────────────────────────────────────────────────────────────────────┐
│                         Core Types                                 │
│  IDs, Hashes, Capabilities, Time, Errors, Versioning              │
└────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Workflow Submission

```
User → CLI/Server → Plan Parser → DAG Compiler → Runtime Scheduler
```

### 2. Execution

```
Scheduler → Executor → Policy Check → Tool Invocation → Event Log
                      ↓                      ↓
                 Capability Gate       WASM Sandbox
                      ↓                      ↓
                 Policy Proof          Normalized Output
```

### 3. Cluster Execution

```
Coordinator: assigns tasks via consensus log
    ↓
Worker: receives assignment, executes, publishes result
    ↓
Coordinator: commits result, updates DAG state
```

### 4. Replay

```
Snapshot + Event Log → Replay Engine → State Reconstruction → Diff
```

## Determinism Guarantees

### Canonical Encoding

All events are encoded using a stable format:
- BTreeMap instead of HashMap (ordering)
- Fixed-width integers
- No platform-specific serialization
- Tests verify byte-identical cross-platform

### Hash Chain

Each event contains:
- `prior_state_hash`: hash of previous state
- `post_state_hash`: hash of resulting state
- `payload_hash`: hash of canonical payload

Chain validation detects:
- Log truncation
- Event reordering
- Event modification
- State corruption

### Logical Time

All events use logical time:
- Monotonic per run
- Assigned at scheduling time
- Used for causal ordering
- Independent of wall clock

## Capability System

### Capability Types

```rust
pub enum Capability {
    NetRead { allowlist: Vec<String> },
    NetWrite { allowlist: Vec<String> },
    FsRead { prefixes: Vec<PathBuf> },
    FsWrite { prefixes: Vec<PathBuf> },
    DbRead { tables: Vec<String> },
    DbWrite { tables: Vec<String> },
    Exec { cpu_limit: String, mem_limit: String },
    WasmExec { fuel: u64, memory: u64 },
    ClockRead,
    EnvRead { vars: Vec<String> },
}
```

### Capability Flow

```
Policy → CapabilitySet → Executor → CapabilityGate → Tool
                                    ↓
                            Event (capability_check_result)
```

## Failure Handling

### Network Failures

- Partition detected via Raft
- Consensus pauses until quorum
- No silent divergent state

### Worker Failures

- Timeout detected by coordinator
- Task reassigned to different worker
- Original worker state discarded

### Tool Failures

- Crash logged as error event
- Retry policy applied
- No mutation without event

### Snapshot Failures

- Corruption detected via hash
- Rejected, falls back to older snapshot
- Logged for investigation

## Security Model

### Threats Addressed

1. **Malicious tools** - WASM sandbox, capability gating
2. **Log tampering** - Hash chain detection
3. **Unauthorized access** - Capability enforcement
4. **Resource exhaustion** - Fuel/memory limits, backpressure

### Threats Not Addressed

1. **Model deception** - Models can lie in outputs
2. **Tool data lies** - Output is data, validated separately
3. **Side-channel attacks** - Not in scope for v0.1

## Performance Considerations

Determinism takes priority over performance:
- Replay may be slower than original execution
- Encoding overhead for canonical representation
- Hash computation for every event
- Policy evaluation on every capability check

Optimization is done only when it doesn't compromise determinism.
