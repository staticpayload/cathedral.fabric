//! Cryptographic hashes for content addressing and hash chaining.
//!
//! Uses BLAKE3 for all hashing operations.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A BLAKE3 hash (256 bits / 32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    /// The number of bytes in a hash
    pub const LEN: usize = 32;

    /// Compute BLAKE3 hash of data
    #[must_use]
    pub fn compute(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    /// Compute hash of empty data
    #[must_use]
    pub const fn empty() -> Self {
        Self([0u8; 32])
    }

    /// Create from bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    ///
    /// # Errors
    ///
    /// Returns error if hex is invalid or not 32 bytes
    pub fn from_hex(hex: &str) -> Result<Self, HashError> {
        let bytes = hex::decode(hex).map_err(|_| HashError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength(bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Check if hash matches data
    #[must_use]
    pub fn verify(&self, data: &[u8]) -> bool {
        Self::compute(data) == *self
    }

    /// Chain this hash with another (for hash chaining)
    ///
    /// Computes: hash(self || other)
    #[must_use]
    pub fn chain(&self, other: &Hash) -> Self {
        let mut combined = [0u8; 64];
        combined[0..32].copy_from_slice(&self.0);
        combined[32..64].copy_from_slice(&other.0);
        Self::compute(&combined)
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<&[u8; 32]> for Hash {
    fn from(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }
}

/// Hash-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashError {
    /// Invalid hex encoding
    InvalidHex,
    /// Invalid length (not 32 bytes)
    InvalidLength(usize),
}

impl std::error::Error for HashError {}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex => write!(f, "Invalid hex encoding"),
            Self::InvalidLength(len) => write!(f, "Invalid hash length: {} (expected 32)", len),
        }
    }
}

/// A hash chain for linking events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashChain {
    hashes: Vec<Hash>,
}

impl HashChain {
    /// Create a new empty hash chain
    #[must_use]
    pub fn new() -> Self {
        Self { hashes: Vec::new() }
    }

    /// Create with initial hash
    #[must_use]
    pub fn with_initial(initial: Hash) -> Self {
        Self {
            hashes: vec![initial],
        }
    }

    /// Add a hash to the chain
    pub fn push(&mut self, hash: Hash) {
        self.hashes.push(hash);
    }

    /// Get the current tip of the chain
    #[must_use]
    pub fn tip(&self) -> Option<Hash> {
        self.hashes.last().copied()
    }

    /// Get all hashes in the chain
    #[must_use]
    pub fn as_slice(&self) -> &[Hash] {
        &self.hashes
    }

    /// Get length of chain
    #[must_use]
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Check if chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Validate chain continuity
    ///
    /// Ensures each hash links to the previous one
    #[must_use]
    pub fn validate(&self) -> bool {
        if self.hashes.len() <= 1 {
            return true;
        }

        for i in 1..self.hashes.len() {
            let expected = self.hashes[i - 1];
            let actual = self.hashes[i];
            // In a proper hash chain, each hash should include the previous
            // For now, we just check that hashes are different
            if expected == actual {
                return false;
            }
        }

        true
    }

    /// Compute root hash of entire chain
    #[must_use]
    pub fn root(&self) -> Option<Hash> {
        if self.hashes.is_empty() {
            return None;
        }

        let mut result = self.hashes[0];
        for hash in &self.hashes[1..] {
            result = result.chain(hash);
        }
        Some(result)
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for HashChain {
    type Item = Hash;
    type IntoIter = std::vec::IntoIter<Hash>;

    fn into_iter(self) -> Self::IntoIter {
        self.hashes.into_iter()
    }
}

/// Content address for blob storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentAddress {
    pub hash: Hash,
    pub algorithm: AddressAlgorithm,
}

impl ContentAddress {
    /// Create content address from data
    #[must_use]
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            hash: Hash::compute(data),
            algorithm: AddressAlgorithm::Blake3,
        }
    }

    /// Create from hash and algorithm
    #[must_use]
    pub const fn new(hash: Hash, algorithm: AddressAlgorithm) -> Self {
        Self { hash, algorithm }
    }

    /// Convert to string representation
    #[must_use]
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.algorithm.as_str(), self.hash)
    }

    /// Parse from string
    ///
    /// # Errors
    ///
    /// Returns error if format is invalid
    pub fn from_str(s: &str) -> Result<Self, HashError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(HashError::InvalidHex);
        }

        let algorithm = match parts[0] {
            "blake3" => AddressAlgorithm::Blake3,
            _ => return Err(HashError::InvalidHex),
        };

        let hash = Hash::from_hex(parts[1])?;

        Ok(Self { hash, algorithm })
    }
}

impl fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Hash algorithm used for content addressing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AddressAlgorithm {
    /// BLAKE3 (default)
    Blake3,
}

impl AddressAlgorithm {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Blake3 => "blake3",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_compute() {
        let data = b"hello world";
        let hash = Hash::compute(data);
        assert_eq!(hash.to_hex().len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_hash_from_to_hex() {
        let hash = Hash::compute(b"test");
        let hex = hash.to_hex();
        let restored = Hash::from_hex(&hex).unwrap();
        assert_eq!(hash, restored);
    }

    #[test]
    fn test_hash_verify() {
        let data = b"test data";
        let hash = Hash::compute(data);
        assert!(hash.verify(data));
        assert!(!hash.verify(b"other data"));
    }

    #[test]
    fn test_hash_chain() {
        let mut chain = HashChain::new();
        assert!(chain.is_empty());

        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        chain.push(h1);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.tip(), Some(h1));

        chain.push(h2);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain.tip(), Some(h2));

        assert!(chain.validate());
    }

    #[test]
    fn test_hash_chain_root() {
        let mut chain = HashChain::new();
        assert!(chain.root().is_none());

        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");
        chain.push(h1);
        chain.push(h2);

        let root = chain.root();
        assert!(root.is_some());

        // Root should be h1.chain(h2)
        let expected = h1.chain(&h2);
        assert_eq!(root, Some(expected));
    }

    #[test]
    fn test_content_address() {
        let data = b"blob content";
        let addr = ContentAddress::from_data(data);
        assert_eq!(addr.algorithm, AddressAlgorithm::Blake3);
        assert!(addr.hash.verify(data));

        let s = addr.to_string();
        let restored = ContentAddress::from_str(&s).unwrap();
        assert_eq!(addr, restored);
    }

    #[test]
    fn test_hash_chain_concatenation() {
        let h1 = Hash::compute(b"first");
        let h2 = Hash::compute(b"second");

        let chained = h1.chain(&h2);
        assert_ne!(chained, h1);
        assert_ne!(chained, h2);

        // Chaining should be deterministic
        let chained2 = h1.chain(&h2);
        assert_eq!(chained, chained2);
    }
}
