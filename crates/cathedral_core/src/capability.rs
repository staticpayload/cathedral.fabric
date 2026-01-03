//! Capability types for capability-based security.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A capability grants permission for a specific type of operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Capability {
    /// Read from network with domain allowlist
    NetRead { allowlist: Vec<String> },

    /// Write to network with domain allowlist
    NetWrite { allowlist: Vec<String> },

    /// Read from filesystem with path prefix allowlist
    FsRead { prefixes: Vec<String> },

    /// Write to filesystem with path prefix allowlist
    FsWrite { prefixes: Vec<String> },

    /// Read from database with table allowlist
    DbRead { tables: Vec<String> },

    /// Write to database with table allowlist
    DbWrite { tables: Vec<String> },

    /// Execute external process with resource limits
    Exec { cpu_limit: String, mem_limit: String },

    /// Execute WASM with fuel and memory limits
    WasmExec { fuel: u64, memory: u64 },

    /// Read logical clock
    ClockRead,

    /// Read environment variables with allowlist
    EnvRead { vars: Vec<String> },
}

impl Capability {
    /// Check if this capability matches another for the same operation type
    #[must_use]
    pub fn matches_kind(&self, other: &Capability) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Get a string representation of the capability kind
    #[must_use]
    pub fn kind_name(&self) -> &str {
        match self {
            Self::NetRead { .. } => "NetRead",
            Self::NetWrite { .. } => "NetWrite",
            Self::FsRead { .. } => "FsRead",
            Self::FsWrite { .. } => "FsWrite",
            Self::DbRead { .. } => "DbRead",
            Self::DbWrite { .. } => "DbWrite",
            Self::Exec { .. } => "Exec",
            Self::WasmExec { .. } => "WasmExec",
            Self::ClockRead => "ClockRead",
            Self::EnvRead { .. } => "EnvRead",
        }
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetRead { allowlist } => {
                write!(f, "NetRead({})", allowlist.join(","))
            }
            Self::NetWrite { allowlist } => {
                write!(f, "NetWrite({})", allowlist.join(","))
            }
            Self::FsRead { prefixes } => {
                write!(f, "FsRead({})", prefixes.join(","))
            }
            Self::FsWrite { prefixes } => {
                write!(f, "FsWrite({})", prefixes.join(","))
            }
            Self::DbRead { tables } => {
                write!(f, "DbRead({})", tables.join(","))
            }
            Self::DbWrite { tables } => {
                write!(f, "DbWrite({})", tables.join(","))
            }
            Self::Exec { cpu_limit, mem_limit } => {
                write!(f, "Exec(cpu:{},mem:{})", cpu_limit, mem_limit)
            }
            Self::WasmExec { fuel, memory } => {
                write!(f, "WasmExec(fuel:{},mem:{})", fuel, memory)
            }
            Self::ClockRead => write!(f, "ClockRead"),
            Self::EnvRead { vars } => {
                write!(f, "EnvRead({})", vars.join(","))
            }
        }
    }
}

/// A set of capabilities granted to a run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub capabilities: BTreeSet<Capability>,
}

impl CapabilitySet {
    /// Create a new empty capability set
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: BTreeSet::new(),
        }
    }

    /// Grant a capability
    pub fn grant(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    /// Grant a capability (alias for grant)
    pub fn allow(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    /// Check if a specific capability is granted
    ///
    /// This checks for exact match of the capability
    #[must_use]
    pub fn has(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if a capability is allowed (exact or kind match)
    #[must_use]
    pub fn allows(&self, capability: &Capability) -> bool {
        // First check exact match
        if self.capabilities.contains(capability) {
            return true;
        }

        // Then check for kind match with wildcard/broad permissions
        self.capabilities.iter().any(|cap| match (cap, capability) {
            // Wildcard network access
            (Capability::NetRead { allowlist: a }, Capability::NetRead { .. }) => {
                a.contains(&"*".to_string())
            }
            (Capability::NetWrite { allowlist: a }, Capability::NetWrite { .. }) => {
                a.contains(&"*".to_string())
            }
            // Wildcard filesystem access
            (Capability::FsRead { prefixes: p }, Capability::FsRead { .. }) => {
                p.contains(&".".to_string()) || p.contains(&"*".to_string())
            }
            (Capability::FsWrite { prefixes: p }, Capability::FsWrite { .. }) => {
                p.contains(&".".to_string()) || p.contains(&"*".to_string())
            }
            _ => false,
        })
    }

    /// Check if network read is allowed for a domain
    #[must_use]
    pub fn can_read_net(&self, domain: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::NetRead { allowlist } => matches_domain(allowlist, domain),
            _ => false,
        })
    }

    /// Check if network write is allowed for a domain
    #[must_use]
    pub fn can_write_net(&self, domain: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::NetWrite { allowlist } => matches_domain(allowlist, domain),
            _ => false,
        })
    }

    /// Check if filesystem read is allowed for a path
    #[must_use]
    pub fn can_read_fs(&self, path: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::FsRead { prefixes } => {
                prefixes.iter().any(|p| matches_path(p, path))
            }
            _ => false,
        })
    }

    /// Check if filesystem write is allowed for a path
    #[must_use]
    pub fn can_write_fs(&self, path: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::FsWrite { prefixes } => {
                prefixes.iter().any(|p| matches_path(p, path))
            }
            _ => false,
        })
    }

    /// Check if database read is allowed for a table
    #[must_use]
    pub fn can_read_db(&self, table: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::DbRead { tables } => tables.contains(&table.to_string()),
            _ => false,
        })
    }

    /// Check if database write is allowed for a table
    #[must_use]
    pub fn can_write_db(&self, table: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::DbWrite { tables } => tables.contains(&table.to_string()),
            _ => false,
        })
    }

    /// Check if clock read is allowed
    #[must_use]
    pub fn can_read_clock(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::ClockRead))
    }

    /// Check if env var read is allowed for a variable
    #[must_use]
    pub fn can_read_env(&self, var: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::EnvRead { vars } => vars.contains(&var.to_string()),
            _ => false,
        })
    }

    /// Get the number of capabilities
    #[must_use]
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Check if empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Iterate over capabilities
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a domain matches an allowlist pattern
fn matches_domain(allowlist: &[String], domain: &str) -> bool {
    allowlist.iter().any(|pattern| {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            return domain == suffix || domain.ends_with(&format!(".{suffix}"));
        }

        pattern == domain
    })
}

/// Check if a path matches a prefix
fn matches_path(prefix: &str, path: &str) -> bool {
    let normalized_prefix = if let Some(stripped) = prefix.strip_suffix('/') {
        stripped
    } else {
        prefix
    };

    let normalized_path = if let Some(stripped) = path.strip_suffix('/') {
        stripped
    } else {
        path
    };

    if normalized_prefix == "." || normalized_prefix == "./" {
        return true;
    }

    if normalized_path == normalized_prefix {
        return true;
    }

    if normalized_path.starts_with(&format!("{normalized_prefix}/")) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set_grant() {
        let mut caps = CapabilitySet::new();
        assert!(caps.is_empty());

        caps.grant(Capability::ClockRead);
        assert_eq!(caps.len(), 1);
        assert!(caps.has(&Capability::ClockRead));
        assert!(caps.can_read_clock());
    }

    #[test]
    fn test_net_read_domain_matching() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::NetRead {
            allowlist: vec!["*.example.com".to_string(), "api.service.com".to_string()],
        });

        assert!(caps.can_read_net("example.com"));
        assert!(caps.can_read_net("sub.example.com"));
        assert!(caps.can_read_net("api.service.com"));
        assert!(!caps.can_read_net("other.com"));
    }

    #[test]
    fn test_net_wildcard() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::NetRead {
            allowlist: vec!["*".to_string()],
        });

        assert!(caps.can_read_net("any.domain.com"));
        assert!(caps.can_read_net("example.com"));
    }

    #[test]
    fn test_fs_path_matching() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::FsWrite {
            prefixes: vec!["./outputs".to_string(), "./cache".to_string()],
        });

        assert!(caps.can_write_fs("./outputs/data.json"));
        assert!(caps.can_write_fs("./cache/tmp"));
        assert!(caps.can_write_fs("./outputs"));
        assert!(!caps.can_write_fs("./inputs"));
    }

    #[test]
    fn test_fs_current_directory() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::FsRead {
            prefixes: vec![".".to_string()],
        });

        assert!(caps.can_read_fs("./file.txt"));
        assert!(caps.can_read_fs("file.txt"));
        assert!(caps.can_read_fs("./sub/dir/file.txt"));
    }

    #[test]
    fn test_db_table_access() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::DbRead {
            tables: vec!["users".to_string(), "posts".to_string()],
        });

        assert!(caps.can_read_db("users"));
        assert!(caps.can_read_db("posts"));
        assert!(!caps.can_read_db("admin"));
    }

    #[test]
    fn test_env_var_access() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::EnvRead {
            vars: vec!["PATH".to_string(), "HOME".to_string()],
        });

        assert!(caps.can_read_env("PATH"));
        assert!(caps.can_read_env("HOME"));
        assert!(!caps.can_read_env("SECRET"));
    }

    #[test]
    fn test_capability_equality() {
        let cap1 = Capability::ClockRead;
        let cap2 = Capability::ClockRead;
        assert_eq!(cap1, cap2);

        let cap3 = Capability::NetRead {
            allowlist: vec!["*".to_string()],
        };
        let cap4 = Capability::NetRead {
            allowlist: vec!["example.com".to_string()],
        };
        assert_ne!(cap3, cap4);
        assert!(cap3.matches_kind(&cap4));
    }

    #[test]
    fn test_wasm_exec_limits() {
        let mut caps = CapabilitySet::new();
        caps.grant(Capability::WasmExec {
            fuel: 1_000_000,
            memory: 64 * 1024 * 1024,
        });

        assert!(caps.has(&Capability::WasmExec {
            fuel: 1_000_000,
            memory: 64 * 1024 * 1024
        }));
    }

    #[test]
    fn test_capability_ord() {
        // Capabilities should be comparable for deterministic ordering
        let mut set = BTreeSet::new();
        set.insert(Capability::ClockRead);
        set.insert(Capability::NetRead {
            allowlist: vec!["*".to_string()],
        });
        set.insert(Capability::FsWrite {
            prefixes: vec![".".to_string()],
        });

        assert_eq!(set.len(), 3);
    }
}
