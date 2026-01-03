# Security Model

## Overview

CATHEDRAL is designed with a defense-in-depth approach to security. All operations are capability-gated, logged, and verifiable.

## Threat Model

### Assumed Threats

1. **Malicious tools** - Tools may attempt unauthorized operations
2. **Model deception** - LLMs may produce malicious or incorrect outputs
3. **Operator misconfiguration** - Policies may be incorrectly specified
4. **Network attacks** - Cluster communication may be intercepted
5. **Log tampering** - Attackers may attempt to modify event logs
6. **Snapshot injection** - Attackers may provide malicious snapshots

### Out of Scope

1. **Physical access** - Not protecting against physical machine access
2. **Side channels** - Timing attacks not in scope for v0.1
3. **Social engineering** - Operator training is out of scope

## Security Properties

### Capability Safety

All side effects require explicit capabilities:

```rust
// No access without capability
cap_check.require_net_read("example.com")?;

// Capability check is logged
event_log.log_capability_check("net_read", "example.com", allowed);
```

Properties:
- **Unforgeable** - Capabilities created only at run start
- **Non-transferable** - Cannot be delegated between tools
- **Explicit** - Every side effect lists required capability
- **Auditable** - All checks logged with proofs

### Log Integrity

Event logs are hash-chained:

```
event[N].prior_state_hash = hash(event[N-1].post_state_hash)
```

Detects:
- Log truncation
- Event reordering
- Event modification
- State corruption

### Replay Verifiability

Any run can be replayed and verified:

```bash
cathedral certify -b run-001.cath-bundle
```

Verifies:
- Hash chain integrity
- Snapshot signatures
- Deterministic execution
- Policy compliance

## Defense in Depth

### Layer 1: Policy Language

Policies define what is allowed:

```policy
policy "strict" {
    deny NetWrite
    grant NetRead { allowlist: ["*.trusted.com"] }
    grant FsWrite { prefixes: ["./outputs"] }
}
```

### Layer 2: Capability Gates

Runtime enforcement of capabilities:

```rust
let gate = CapabilityGate::new(capabilities);
gate.check(&Capability::NetRead { allowlist })?;
```

### Layer 3: WASM Sandbox

Tools run in isolation:

```rust
let sandbox = Sandbox::new(wasm, SandboxConfig {
    fuel: 1_000_000_000,
    memory_mb: 64,
});
```

### Layer 4: Event Logging

All actions are logged:

```
ToolInvoked → CapabilityCheck → PolicyDecision → ToolCompleted
```

### Layer 5: Hash Chaining

Tamper-evident log structure.

## Attack Scenarios

### Scenario 1: Unauthorized Network Access

**Attack**: Tool tries to connect to arbitrary host.

**Defense**:
1. Tool declares `NetRead { allowlist }` requirement
2. Policy validates allowlist
3. Capability gate checks specific domain
4. Host function validates domain at call time
5. All checks logged

**Result**: Attack blocked, logged, audit trail available.

### Scenario 2: Log Tampering

**Attack**: Attacker modifies event in log.

**Defense**:
1. Hash chain detects broken link
2. Content hashes don't match
3. Signature verification fails

**Result**: Tampering detected, log rejected.

### Scenario 3: Snapshot Injection

**Attack**: Attacker provides malicious snapshot.

**Defense**:
1. Snapshot content hash verified
2. Prior snapshot chain validated
3. All referenced blobs checked
4. Signature verification (if signed)

**Result**: Malicious snapshot rejected.

### Scenario 4: Resource Exhaustion

**Attack**: Tool attempts infinite loop or memory bomb.

**Defense**:
1. WASM fuel limit stops infinite loops
2. Memory limit prevents memory bombs
3. Timeout on tool execution
4. Backpressure prevents queue overflow

**Result**: Tool terminated, logged.

### Scenario 5: Data Exfiltration

**Attack**: Tool tries to send data to unauthorized endpoint.

**Defense**:
1. `NetWrite` capability denied by default
2. Host function checks allowlist
3. All host calls logged

**Result**: Block at capability gate, logged.

## Security Auditing

### Audit Trail

Every action produces multiple events:

1. **Capability Check** - What was requested
2. **Policy Decision** - Allow/deny with reasoning
3. **Tool Invocation** - What was executed
4. **Tool Result** - What was returned
5. **Error Events** - What went wrong

### Audit Commands

```bash
# View all capability checks for a run
cathedral audit capabilities --run run-001

# View policy decisions
cathedral audit policy --run run-001

# View tool invocations
cathedral audit tools --run run-001

# Full audit report
cathedral audit full --run run-001
```

### Redaction

Sensitive data can be redacted in logs and bundles:

```policy
rule "redact_secrets" {
    match { tool: "api_call" }
    redact {
        fields: ["api_key", "password", "token"]
    }
}
```

## Cryptographic Guarantees

### Hash Function

BLAKE3 is used for all hashing:
- Fast, cryptographically secure
- Merkle tree support for large files
- Parallelizable

### Signatures (Future)

Snapshot signing will be supported:
- Ed25519 signatures
- Key rotation support
- Multi-signature support

## Security Best Practices

### For Operators

1. **Principle of least privilege** - Grant minimal capabilities
2. **Regular audits** - Review logs and policy decisions
3. **Keep policies in VCS** - Version control your policies
4. **Test policies** - Use `cathedral policy test`
5. **Monitor for anomalies** - Set up alerts on denials

### For Tool Authors

1. **Declare all capabilities** - Be explicit about requirements
2. **Normalize output** - Make output deterministic
3. **Document side effects** - List all observable effects
4. **Test sandboxing** - Verify tool works in WASM
5. **Handle timeouts** - Respect resource limits

## Known Limitations

1. **Model output not validated** - LLM outputs are trusted as data
2. **Side channels possible** - Timing attacks not mitigated
3. **Denial of self** - Tool can refuse to use granted capabilities
4. **Collusion** - Two tools can collude if both granted capabilities

## Reporting Security Issues

See [SECURITY.md](../SECURITY.md) for the vulnerability reporting process.

## Security Checklist

Before deploying to production:

- [ ] Policies reviewed for over-permissive grants
- [ ] Capabilities minimized per tool
- [ ] Audit logging enabled
- [ ] Log retention configured
- [ ] Redaction rules configured for sensitive data
- [ ] Resource limits set appropriately
- [ ] Network access restricted to allowlists
- [ ] File access restricted to prefixes
- [ ] WASM fuel and memory limits configured
- [ ] Snapshot verification enabled
- [ ] Hash chain validation enabled
- [ ] Regular audits scheduled
