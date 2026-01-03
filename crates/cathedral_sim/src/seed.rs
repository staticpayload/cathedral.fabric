//! Seed management for reproducible simulations.

use cathedral_core::NodeId;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;

/// Source of simulation seed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeedSource {
    /// From a literal value
    Literal(u64),
    /// From a string (hashed)
    FromString(String),
    /// From node ID
    FromNode(NodeId),
    /// From timestamp
    Timestamp(u64),
    /// Random seed (non-deterministic)
    Random,
}

impl SeedSource {
    /// Generate a seed value
    #[must_use]
    pub fn to_seed(&self) -> u64 {
        match self {
            SeedSource::Literal(seed) => *seed,
            SeedSource::FromString(s) => {
                // Simple hash of string
                let mut hash = 0u64;
                for (i, b) in s.bytes().enumerate() {
                    hash = hash.wrapping_mul(31).wrapping_add(b as u64);
                    hash = hash.wrapping_add(i as u64);
                }
                hash
            }
            SeedSource::FromNode(node_id) => {
                // Use first 8 bytes of node ID
                let bytes = node_id.to_string();
                let mut hash = 0u64;
                for (i, b) in bytes.bytes().take(8).enumerate() {
                    hash = hash.wrapping_mul(31).wrapping_add(b as u64);
                    hash = hash.wrapping_add(i as u64);
                }
                hash
            }
            SeedSource::Timestamp(ts) => *ts,
            SeedSource::Random => {
                use std::time::SystemTime;
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64
            }
        }
    }
}

/// Simulation seed for reproducibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimSeed {
    /// Base seed value
    pub seed: u64,
    /// Source of the seed
    pub source: SeedSource,
    /// Namespace for derived seeds
    pub namespace: String,
}

impl SimSeed {
    /// Create a new simulation seed
    #[must_use]
    pub fn new(source: SeedSource) -> Self {
        let seed = source.to_seed();
        Self {
            seed,
            source,
            namespace: String::new(),
        }
    }

    /// Create a seed from a literal value
    #[must_use]
    pub fn from_literal(seed: u64) -> Self {
        Self::new(SeedSource::Literal(seed))
    }

    /// Create a seed from a string
    #[must_use]
    pub fn from_string(s: String) -> Self {
        Self::new(SeedSource::FromString(s))
    }

    /// Create a seed from a node ID
    #[must_use]
    pub fn from_node(node_id: NodeId) -> Self {
        Self::new(SeedSource::FromNode(node_id))
    }

    /// Set namespace
    #[must_use]
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
        self
    }

    /// Derive a seed for a specific context
    #[must_use]
    pub fn derive(&self, context: &str) -> Self {
        let mut hasher = fnv::FnvHasher::default();
        hasher.write_u64(self.seed);
        hasher.write(self.namespace.as_bytes());
        hasher.write(context.as_bytes());
        let derived_seed = hasher.finish();

        Self {
            seed: derived_seed,
            source: SeedSource::Literal(derived_seed),
            namespace: self.namespace.clone(),
        }
    }

    /// Create RNG from seed
    #[must_use]
    pub fn into_rng(self) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(self.seed)
    }

    /// Create RNG borrowing seed
    #[must_use]
    pub fn rng(&self) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(self.seed)
    }
}

impl Default for SimSeed {
    fn default() -> Self {
        Self::new(SeedSource::Literal(42))
    }
}

/// FNV hash for seed derivation
struct FnvHasher {
    hash: u64,
}

impl Default for FnvHasher {
    fn default() -> Self {
        // FNV-1a 64-bit offset basis
        Self { hash: 0xcbf29ce484222325 }
    }
}

impl FnvHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash ^= b as u64;
            self.hash = self.hash.wrapping_mul(0x100000001b3);
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.write(&n.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;

    #[test]
    fn test_seed_source_literal() {
        let source = SeedSource::Literal(123);
        assert_eq!(source.to_seed(), 123);
    }

    #[test]
    fn test_seed_source_from_string() {
        let source = SeedSource::FromString("test".to_string());
        let seed = source.to_seed();
        assert_ne!(seed, 0);
    }

    #[test]
    fn test_seed_source_from_string_reproducible() {
        let source1 = SeedSource::FromString("test".to_string());
        let source2 = SeedSource::FromString("test".to_string());
        assert_eq!(source1.to_seed(), source2.to_seed());
    }

    #[test]
    fn test_seed_source_from_string_different() {
        let source1 = SeedSource::FromString("test".to_string());
        let source2 = SeedSource::FromString("other".to_string());
        assert_ne!(source1.to_seed(), source2.to_seed());
    }

    #[test]
    fn test_sim_seed_new() {
        let seed = SimSeed::from_literal(42);
        assert_eq!(seed.seed, 42);
    }

    #[test]
    fn test_sim_seed_with_namespace() {
        let seed = SimSeed::from_literal(42).with_namespace("test".to_string());
        assert_eq!(seed.namespace, "test");
    }

    #[test]
    fn test_sim_seed_derive() {
        let base = SimSeed::from_literal(42);
        let derived1 = base.derive("context1");
        let derived2 = base.derive("context2");
        let derived1_again = base.derive("context1");

        // Different contexts give different seeds
        assert_ne!(derived1.seed, derived2.seed);
        // Same context gives same seed
        assert_eq!(derived1.seed, derived1_again.seed);
        // Derived seeds differ from base
        assert_ne!(derived1.seed, base.seed);
    }

    #[test]
    fn test_sim_seed_rng() {
        let seed = SimSeed::from_literal(42);
        let mut rng1 = seed.rng();
        let mut rng2 = seed.rng();

        let val1: u64 = rng1.r#gen();
        let val2: u64 = rng2.r#gen();
        assert_eq!(val1, val2); // Same seed produces same values
    }

    #[test]
    fn test_sim_seed_default() {
        let seed = SimSeed::default();
        assert_eq!(seed.seed, 42);
    }
}
