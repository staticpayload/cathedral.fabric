# Changelog

All notable changes to CATHEDRAL.FABRIC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Placeholder for next release

## [0.1.0] - 2025-01-03

### Stable Features

#### Core (cathedral_core)
- Pure types: IDs (EventId, RunId, NodeId), Hash (BLAKE3, SHA-256), LogicalTime
- Capability types: NetRead, NetWrite, FsRead, FsWrite, DbRead, DbWrite, Exec, WasmExec, EnvRead
- Timestamp with nanosecond precision
- Version type for semantic versioning
- Comprehensive error types with Display formatting

#### Event Log (cathedral_log)
- Canonical event encoding (byte-stable across platforms)
- EventKind with 23 event types
- Event struct with hash-chained state tracking
- HashChain with validation
- EventStream for sequential access
- StreamWriter for event collection
- Cursor for bidirectional traversal

#### Replay Engine (cathedral_replay)
- ReplayEngine with state reconstruction
- RunComparison for stable diffs between runs
- Snapshot restoration and integration
- Divergence detection with causal tracing

#### Planner (cathedral_plan)
- LALRPOP(1) grammar for workflow DSL
- AST with Workflow, Step, Resource, Capability expressions
- DAG compilation with cycle detection
- Topological sorting for execution order
- Resource and capability contract generation

#### Runtime (cathedral_runtime)
- Deterministic scheduler with priority queue
- Executor with event emission
- Backpressure handling with strategies
- Capability enforcement engine
- Monitor with metrics collection
- Time-based tick progression

#### Policy (cathedral_policy)
- Policy expression language (matches, requires, allows, denies)
- Policy compiler with validation
- Capability check expressions
- Boolean operators (and, or, not)
- Comparison operators (eq, ne, lt, le, gt, ge)

#### Tool (cathedral_tool)
- Tool trait for sandboxed execution
- ToolSchema for input/output validation
- Side effect declarations
- Capability requirements
- ToolAdapter for WASM and native tools
- Output normalization

#### WASM (cathedral_wasm)
- Wasmtime sandbox with fuel limits
- Memory limits and enforcement
- Capability-mediated host functions
- ABI definitions (read, write, random, hash, time)
- Fuel accounting with InstructionCost
- Error types for sandbox violations

#### Storage (cathedral_storage)
- Content-addressed blob storage
- Snapshot management with versioning
- BlobStore interface (in-memory: HashMap, persistent: Redb/Sled)
- Blob compaction
- State reconstruction from snapshots

#### Cluster (cathedral_cluster)
- OpenRaft-based distributed log
- Worker registration and discovery
- Task assignment with leader election
- Network message types
- Replicated state machine

#### Simulation (cathedral_sim)
- SimSeed for deterministic randomness
- SimRecord with event logging
- Deterministic network simulation
- Simulated worker execution
- Failure injection

#### Certification (cathedral_certify)
- DeterminismValidator for multi-run verification
- ValidationReport with detailed checks
- Certificate generation with Ed25519 signatures
- PublicKey handling with hex encoding

#### TUI (cathedral_tui)
- Terminal UI with ratatui
- Timeline, DAG, Worker, Provenance views
- Keyboard navigation (vim-style)
- Help screen with key bindings
- Status bar with mode display

### Experimental Features
- Workflow DSL syntax may change
- Cluster mode requires further testing
- Policy language is evolving

### Known Limitations
- No built-in tools included (must be registered)
- WASM host functions are minimal
- Network simulation is basic
- No formal verification proofs yet
- TUI does not yet load real event data

### Security Notes
- All side effects require explicit capabilities
- Tools run in WASM isolation by default
- Policy decisions are logged with full justification
- Hash chains detect log tampering

### Determinism Notes
- Event encoding is byte-stable across Linux, macOS, Windows
- Replay reconstructs exact state when using same seed
- Cross-platform certification verified
- Tool normalization ensures deterministic output handling

### Upgrade Notes
- This is the initial release
- API may change in 0.2.x releases
- No backward compatibility guarantees for 0.x.x

[Unreleased]: https://github.com/cathedral-fabric/cathedral.fabric/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/cathedral-fabric/cathedral.fabric/releases/tag/v0.1.0
