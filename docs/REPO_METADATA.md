# Repository Metadata

## Short Description (GitHub)

A deterministic, distributed, capability-safe execution fabric for agent workflows with verifiable replay and certified audit trails.

## Long Description (README / Registry)

CATHEDRAL.FABRIC is a verifiable agent operating substrate that provides deterministic workflow execution with complete audit trails. It guarantees byte-identical replay across platforms, hash-chained event logs for tamper evidence, and WASM sandboxing for tool isolation.

**Core Guarantees:**
- **Canonical Encoding** — Events encode identically on Linux, macOS, Windows
- **Hash Chained Logs** — Tamper-evident history using BLAKE3
- **Deterministic Replay** — Reconstruct exact state from snapshots and event logs
- **Capability Safety** — Every side-effect requires explicit granted capability
- **Policy Proofs** — All allow/deny decisions logged with full justification
- **WASM Isolation** — Tools run in fuel-limited sandbox with mediated host functions
- **Stable Diffs** — Minimal, human-readable explanations of run divergence

**Use Cases:**
- Agent workflow orchestration with audit requirements
- Reproducible ML pipelines with provenance tracking
- Distributed task execution with consensus
- High-assurance systems requiring formal verification
- Research workflows needing precise replay for debugging

**Architecture:**
- DSL compiler from workflow definitions to typed DAGs
- Distributed scheduler with Raft consensus
- Content-addressed storage with deduplication
- TUI trace viewer for audit and debugging
- Determinism certification for cross-platform verification

## Topics Tags (25)

```
deterministic
distributed-systems
workflow-engine
agent-framework
replay
audit-trail
capability-based-security
wasm
raft
hash-chain
verifiable-computing
rust
terminal-ui
cluster
consensus
sandbox
policy-engine
event-sourcing
cqrs
content-addressable-storage
blake3
ed25519
tla
formal-verification
simulation
distributed-ledger
```

## Badges

| Badge | URL |
|-------|-----|
| **CI** | `https://github.com/cathedral-fabric/cathedral.fabric/actions/workflows/ci.yml/badge.svg` |
| **Crates.io** | `https://img.shields.io/crates/v/cathedral-core` |
| **License** | `https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue` |
| **Docs** | `https://img.shields.io/badge/docs-latest-green` |
| **Security** | `https://img.shields.io/badge/security-policy-green` |

**Markdown for README.md:**
```markdown
[![CI](https://github.com/cathedral-fabric/cathedral.fabric/actions/workflows/ci.yml/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/cathedral-core)](https://crates.io/crates/cathedral-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-latest-green)](docs)
[![Security](https://img.shields.io/badge/security-policy-green)](SECURITY.md)
```

## Social Preview Image (Optional)

For GitHub social preview, create `images/social.png`:
- Dimensions: 1280×640
- Content: Logo/name + tagline
- Colors: Dark theme, accent color #00D9FF (cyan)

**Open Graph tags would be added to GitHub automatically via repository metadata.**

## Documentation Site Plan

Documentation should be hosted at:
- Primary: `https://cathedral-fabric.github.io/fabric`
- Redirect from: `https://docs.cathedral.fabric.dev`

**Structure:**
- Getting Started (installation, quick start)
- Guides (workflows, replay, clustering, policy)
- API Reference (crate docs)
- RFCs (design proposals)
- Specs (protocol, bundle format)

**Tooling:**
- Use mdBook for static site generation
- Auto-publish from `docs/` on merge to main
