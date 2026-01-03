# Capability System Specification

## Overview

CATHEDRAL uses a capability-based security model where all side effects require explicit, immutable capabilities granted at run start.

## Principles

1. **No ambient authority** - No default permissions
2. **Explicit grants** - All capabilities declared
3. **Immutable per run** - Capabilities don't change
4. **Logged checks** - Every capability check is an event
5. **Deny by default** - Absent capability = denied

## Capability Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read from network with domain allowlist
    NetRead {
        allowlist: Vec<String>,
    },

    /// Write to network with domain allowlist
    NetWrite {
        allowlist: Vec<String>,
    },

    /// Read from filesystem with path prefix allowlist
    FsRead {
        prefixes: Vec<PathBuf>,
    },

    /// Write to filesystem with path prefix allowlist
    FsWrite {
        prefixes: Vec<PathBuf>,
    },

    /// Read from database with table allowlist
    DbRead {
        tables: Vec<String>,
    },

    /// Write to database with table allowlist
    DbWrite {
        tables: Vec<String>,
    },

    /// Execute external process with resource limits
    Exec {
        cpu_limit: String,
        mem_limit: String,
    },

    /// Execute WASM with fuel and memory limits
    WasmExec {
        fuel: u64,
        memory: u64,
    },

    /// Read logical clock (no wall clock)
    ClockRead,

    /// Read environment variables with allowlist
    EnvRead {
        vars: Vec<String>,
    },
}
```

## Capability Set

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub capabilities: BTreeSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: BTreeSet::new(),
        }
    }

    pub fn grant(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    pub fn has(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn check_net_read(&self, domain: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::NetRead { allowlist } => {
                allowlist.iter().any(|pattern| matches_domain(pattern, domain))
            }
            _ => false,
        })
    }

    // Similar methods for other capability types...
}
```

## Domain Matching

```rust
pub fn matches_domain(pattern: &str, domain: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
        return domain == suffix || domain.ends_with(&format!(".{}", suffix));
    }

    pattern == domain
}
```

## Path Matching

```rust
pub fn matches_path(allowed_prefix: &Path, target: &Path) -> bool {
    let normalized_allowed = allowed_prefix.normalize();
    let normalized_target = target.normalize();

    normalized_target.starts_with(&normalized_allowed)
}
```

## Capability Flow

```
┌─────────────┐
│   Policy    │
│   Engine    │
└──────┬──────┘
       │
       │ Creates CapabilitySet
       ▼
┌─────────────┐
│   Runtime   │
│   Config    │
└──────┬──────┘
       │
       │ Grants to Executor
       ▼
┌─────────────┐
│  Executor   │
└──────┬──────┘
       │
       │ Requests from CapabilityGate
       ▼
┌─────────────┐
│ Capability │
│    Gate     │
└──────┬──────┘
       │
       │ Logs result as event
       ▼
┌─────────────┐
│ Event Log   │
└─────────────┘
```

## Capability Gate

```rust
pub struct CapabilityGate {
    capabilities: CapabilitySet,
}

impl CapabilityGate {
    pub fn check(&self, requested: &Capability) -> CapabilityCheck {
        let allowed = self.capabilities.has(requested);

        CapabilityCheck {
            allowed,
            requested: requested.clone(),
            granted: if allowed { Some(requested.clone()) } else { None },
        }
    }

    pub fn require_net_read(&self, domain: &str) -> Result<(), CapabilityError> {
        if self.capabilities.check_net_read(domain) {
            Ok(())
        } else {
            Err(CapabilityError::Denied {
                capability: format!("NetRead({})", domain),
            })
        }
    }

    // Similar methods for other capabilities...
}
```

## Capability Check Event

Every capability check produces an event:

```json
{
    "event_id": "evt_...",
    "run_id": "run_...",
    "kind": "CapabilityCheck",
    "payload": {
        "capability": "NetRead",
        "target": "example.com",
        "allowed": true,
        "policy_decision_id": "dec_..."
    }
}
```

## Policy Integration

Capabilities are granted through policy:

```policy
policy "example" {
    # Grant network read to specific domains
    grant NetRead {
        allowlist: ["*.api.example.com", "cdn.example.com"]
    }

    # Grant file write to output directory
    grant FsWrite {
        prefixes: ["./outputs", "./cache"]
    }

    # Deny all network write
    deny NetWrite
}
```

## Tool Capability Declaration

Tools declare required capabilities:

```rust
pub struct ToolCapability {
    pub capability: Capability,
    pub required: bool,  // true = tool cannot work without it
}

pub struct ToolDef {
    pub name: String,
    pub required_capabilities: Vec<ToolCapability>,
}
```

## Examples

### Example 1: Web Fetcher

```rust
let capabilities = CapabilitySet::new();
capabilities.grant(Capability::NetRead {
    allowlist: vec!["*.api.example.com".to_string()],
});
```

### Example 2: File Processor

```rust
let capabilities = CapabilitySet::new();
capabilities.grant(Capability::FsRead {
    prefixes: vec![PathBuf::from("./inputs")],
});
capabilities.grant(Capability::FsWrite {
    prefixes: vec![PathBuf::from("./outputs")],
});
```

### Example 3: WASM Tool

```rust
let capabilities = CapabilitySet::new();
capabilities.grant(Capability::WasmExec {
    fuel: 10_000_000,
    memory: 64 * 1024 * 1024,  // 64 MiB
});
```

## Security Properties

1. **Unforgeable** - Capabilities cannot be created after run starts
2. **Non-transferable** - Cannot be delegated between tools
3. **Explicit** - Every side effect lists required capability
4. **Auditable** - All checks logged with proofs

## Attack Prevention

### Prevented Attacks

1. **Unauthorized network access** - Domain allowlist enforced
2. **Unauthorized file access** - Path prefix enforced
3. **Resource exhaustion** - Fuel/memory limits
4. **Data exfiltration** - No NetWrite without explicit grant

### Not Prevented

1. **Data injection** - Valid domains may return malicious data
2. **Side channels** - Timing attacks through capabilities
3. **Denial of self** - Tool can refuse to use granted capabilities
