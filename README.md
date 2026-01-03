# CATHEDRAL.FABRIC

A deterministic, distributed, capability-safe execution fabric for agent workflows.

## What It Is

CATHEDRAL.FABRIC is an agent operating substrate that provides:

1. **Deterministic workflow compiler** - Compile high-level workflows into typed execution DAGs
2. **Distributed scheduler** - Execute across single machines or clusters with guaranteed ordering
3. **Replicated event log** - Hash-chained, append-only log that serves as single source of truth
4. **Replay engine** - Reconstruct full cluster state from any point in history
5. **Policy system** - Capability-based security with logged proof objects
6. **Tool sandbox** - WASM isolation with fuel limits and mediated host functions
7. **Trace UI** - Terminal interface for audit and debugging
8. **Formal verification** - TLA+ specs for core protocols

This is larger than an agent framework. It is a substrate for building verifiable, auditable, reproducible agent systems.

## Core Guarantees

CATHEDRAL guarantees:

1. **Canonical event encoding** - Byte-stable across Linux, macOS, Windows
2. **Hash chained logs** - Tamper-evident event history
3. **Deterministic replay** - Reconstruct exact state from snapshots and logs
4. **Deterministic execution** - Same inputs always produce same event sequence
5. **Capability enforcement** - Every side effect requires explicit capability
6. **Policy proofs** - All allow/deny decisions logged with justification
7. **Tool isolation** - Deny-by-default sandboxing
8. **Stable diffs** - Minimal divergence explanation between runs
9. **Portable bundles** - Reproduce without network access

## Non-Guarantees

CATHEDRAL does NOT guarantee:

1. Model truthfulness
2. Tool output truthfulness
3. Optimal plans
4. Immunity to operator misconfiguration
5. Perfect performance

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLI / TUI / API                           │
├─────────────────────────────────────────────────────────────────────┤
│                              PLAN                                   │
│  (DSL parser → DAG compiler → resource/capability contracts)        │
├─────────────────────────────────────────────────────────────────────┤
│                             RUNTIME                                 │
│  (scheduler → executor → backpressure → capability enforcement)      │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────────┐   │
│  │   LOG    │ │  REPLAY  │ │ STORAGE  │ │       CLUSTER        │   │
│  │ encoding │ │   diff   │ │ snapshot │ │ replication consensus │   │
│  │ chain    │ │  trace   │ │  blob    │ │  membership election  │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────────────────────────────┐    │
│  │  POLICY  │ │   TOOL   │ │            WASM                  │    │
│  │ language │ │ sandbox  │ │  fuel limits memory limits ABI   │    │
│  │  proof   │ │normalize │ │          isolation               │    │
│  └──────────┘ └──────────┘ └──────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────────┤
│                              CORE                                   │
│  (pure types: IDs, hashes, capabilities, time, errors)              │
├─────────────────────────────────────────────────────────────────────┤
│                              SIM                                    │
│  (deterministic network/failure simulation with recorded seeds)      │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

```bash
cargo install cathedral-fabric --cli cathedral
```

### Basic Usage

```bash
# Run a workflow
cathedral run -f workflow.cath

# Replay from logs
cathedral replay -b run-001.cath-bundle

# Diff two runs
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle

# View trace in TUI
cathedral-tui -i run-001.cath-bundle
```

### Workflow DSL Example

```
workflow "example" {
    resources cpu: "100m", memory: "64Mi"

    step "fetch" using tool:read_url {
        input: { url: "https://example.com/data" }
    }

    step "process" depends_on: ["fetch"] using tool:transform {
        input: { data: fetch.output }
    }

    step "write" depends_on: ["process"] using tool:write_file {
        input: { path: "output.txt", content: process.output }
        capabilities: [FsWrite { prefix: "./output" }]
    }
}
```

## Replay and Diff

### Replay

Replay reconstructs full execution state from snapshots and logs:

```bash
cathedral replay -b bundle.cath-bundle --from-snapshot snap-001
```

Replay guarantees:
- Byte-identical workflow compilation output
- Identical scheduling decisions from log
- State reconstruction at each event
- Divergence detection with causal tracing

### Diff

Compare two runs with minimal stable diff:

```bash
cathedral diff --left run-001.cath-bundle --right run-002.cath-bundle
```

Diff output:
- First divergent event
- Causal ancestors of divergence
- Human-readable summary
- Machine-readable JSON

## Cluster Mode

### Single-Node to Cluster

CATHEDRAL runs identically on single machines and clusters:

```bash
# Start coordinator
cathedral-server --role coordinator --bind 0.0.0.0:8080

# Start workers
cathedral-server --role worker --join coordinator:8080
```

### Deterministic Scheduling

All scheduling decisions are events in the replicated log:
- Task assignment to workers
- Resource allocation
- Capability grants
- Timeouts and retries

### Consensus

Uses Raft for log replication:
- Leader election
- Log commitment
- Snapshot transfer
- Membership changes

## Tool Sandbox

### Tool Interface

Tools declare:
```
name: "web_fetch"
version: "1.0.0"
input_schema: {...}
output_schema: {...}
capabilities: [NetRead { allowlist: ["*.example.com"] }]
side_effects: []
determinism: "maybe"
timeout: "30s"
```

### WASM Sandbox

Tools run in WASM with:
- Fuel limits (instruction count)
- Memory limits
- Capability-mediated host calls
- No ambient authority

### Normalization

Tool output is normalized:
- Raw output stored
- Normalized output computed
- Hash of normalized output in log
- Never trust raw output

## Policy Language

### Example Policy

```
policy "default" {
    allow tools: ["read_url", "write_file", "transform"]
    deny capabilities: [NetWrite]

    rule "data_access" {
        match { tool: "read_url" }
        require { capability: NetRead }
        allow { domains: ["*.trusted.com"] }
    }

    rule "file_write" {
        match { tool: "write_file" }
        require { capability: FsWrite }
        allow { prefix: "./outputs" }
        redact { field: "api_key" }
    }
}
```

### Decision Proofs

Each policy decision produces a proof:
```json
{
    "decision_id": "...",
    "allow": true,
    "matched_rule": "data_access",
    "reasoning": {...},
    "timestamp": "..."
}
```

## Determinism Certification

### Certify a Run

```bash
cathedral certify -b run-001.cath-bundle
```

Certification validates:
- Hash chain integrity
- Snapshot signatures
- Cross-platform reproducibility
- Event ordering correctness

### Cross-Platform

Certification runs on:
- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

Bundles certified on one platform reproduce on all platforms.

## Security Model

### Threat Model

Assumes:
- Tools may be malicious
- Models may hallucinate
- Operators may misconfigure
- Network may be partitioned

Defends against:
- Unauthorized capability use
- Log tampering (detects via hash chain)
- Snapshot injection
- Undocumented side effects

### Capability Safety

Capabilities are:
- Immutable per run
- Explicitly granted
- Logged on every check
- Non-transferable

### Audit Trail

Every action is logged:
- Capability checks
- Policy decisions
- Tool invocations
- I/O operations
- Scheduling decisions

## Failure Modes

### Handled Failures

1. **Network partition** - Consensus pauses, resumes on reconnect
2. **Tool crash** - Logged, retry policy applies
3. **Worker crash** - State replayed on another worker
4. **Snapshot corruption** - Detected, rejected
5. **Resource exhaustion** - Backpressure, queue limits
6. **Malicious tool** - Isolated, logged, can be analyzed

### Unhandled Failures

1. **Model deception** - Cannot detect model lying
2. **Tool data lies** - Treated as data, validated separately
3. **Operator error** - Policy cannot prevent all misconfigurations

## Repo Layout

```
cathedral.fabric/
├── Cargo.toml              # Workspace config
├── crates/
│   ├── cathedral_core/     # Pure types, no I/O
│   ├── cathedral_log/      # Event log, encoding, hash chain
│   ├── cathedral_replay/   # Replay engine, diff
│   ├── cathedral_plan/     # DSL parser, DAG compiler
│   ├── cathedral_runtime/  # Execution engine, scheduler
│   ├── cathedral_policy/   # Policy language, proofs
│   ├── cathedral_tool/     # Tool interface, sandbox
│   ├── cathedral_wasm/     # WASM runtime
│   ├── cathedral_storage/  # Content-addressed storage
│   ├── cathedral_cluster/  # Distributed execution
│   ├── cathedral_sim/      # Deterministic simulation
│   ├── cathedral_cli/      # Command-line interface
│   ├── cathedral_server/   # HTTP API
│   └── cathedral_tui/      # Terminal UI
├── docs/                   # Documentation
├── rfcs/                   # RFC process
├── examples/               # Example programs
├── fuzz/                   # Fuzz targets
└── .github/workflows/      # CI/CD
```

## Development Workflow

### Prerequisites

- Rust 1.85+
- Nightly for fuzzing

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test --workspace
```

### Fuzz

```bash
cargo install cargo-fuzz
cargo fuzz run cathedral_log_parser
```

### Simulate

```bash
cargo test --package cathedral_sim sim_long
```

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy --all-targets --all-features
```

## Release Process

See [CHANGELOG.md](CHANGELOG.md) for version history.

Release template:
- Stable features
- Experimental features
- Known limitations
- Security notes
- Determinism notes
- Upgrade notes

## License

MIT OR Apache-2.0

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment.

## Security Policy

See [SECURITY.md](docs/SECURITY.md) for reporting vulnerabilities.
