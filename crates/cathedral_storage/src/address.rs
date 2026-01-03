//! Content addressing for blob storage.

use cathedral_core::{Hash, CoreResult, CoreError};
use serde::{Deserialize, Serialize};

/// Content address combining hash and algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentAddress {
    /// Hash of the content
    pub hash: Hash,
    /// Algorithm used to compute the hash
    pub algorithm: AddressAlgorithm,
}

impl ContentAddress {
    /// Create a new content address
    #[must_use]
    pub fn new(hash: Hash, algorithm: AddressAlgorithm) -> Self {
        Self { hash, algorithm }
    }

    /// Compute content address for data using default algorithm
    #[must_use]
    pub fn compute(data: &[u8]) -> Self {
        Self {
            hash: Hash::compute(data),
            algorithm: AddressAlgorithm::Blake3,
        }
    }

    /// Parse from string representation
    ///
    /// # Errors
    ///
    /// Returns error if format is invalid
    pub fn parse(s: &str) -> CoreResult<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(CoreError::Validation {
                field: "address".to_string(),
                reason: "Invalid address format".to_string(),
            });
        }

        let algorithm = AddressAlgorithm::parse(parts[0])?;
        let hash = Hash::from_hex(parts[1]).map_err(|e| CoreError::Validation {
            field: "hash".to_string(),
            reason: e.to_string(),
        })?;

        Ok(Self { hash, algorithm })
    }

    /// Convert to string representation
    #[must_use]
    pub fn as_str(&self) -> String {
        format!("{}:{}", self.algorithm.as_str(), self.hash.to_hex())
    }

    /// Get hash bytes
    #[must_use]
    pub const fn as_hash(&self) -> &Hash {
        &self.hash
    }

    /// Get algorithm
    #[must_use]
    pub const fn algorithm(&self) -> AddressAlgorithm {
        self.algorithm
    }
}

/// Address algorithm for content hashing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AddressAlgorithm {
    /// BLAKE3 (default)
    Blake3,
    /// SHA-256
    Sha256,
    /// SHA-512
    Sha512,
}

impl AddressAlgorithm {
    /// Parse algorithm from string
    ///
    /// # Errors
    ///
    /// Returns error if algorithm is unknown
    pub fn parse(s: &str) -> CoreResult<Self> {
        match s {
            "blake3" => Ok(Self::Blake3),
            "sha256" => Ok(Self::Sha256),
            "sha512" => Ok(Self::Sha512),
            _ => Err(CoreError::Validation {
                field: "algorithm".to_string(),
                reason: format!("Unknown algorithm: {}", s),
            }),
        }
    }

    /// Get string representation
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Blake3 => "blake3",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    /// Get default algorithm
    #[must_use]
    pub const fn default() -> Self {
        Self::Blake3
    }

    /// Compute hash of data using this algorithm
    #[must_use]
    pub fn hash(&self, data: &[u8]) -> Hash {
        match self {
            Self::Blake3 => Hash::compute(data),
            Self::Sha256 => {
                use sha2::Digest;
                let hasher = sha2::Sha256::new();
                let result = hasher.chain_update(data).finalize();
                Hash::from_bytes(result.into())
            }
            Self::Sha512 => {
                use sha2::Digest;
                let hasher = sha2::Sha512::new();
                let result = hasher.chain_update(data).finalize();
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&result[..32]);
                Hash::from_bytes(bytes)
            }
        }
    }
}

impl Default for AddressAlgorithm {
    fn default() -> Self {
        Self::default()
    }
}

impl std::fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm.as_str(), self.hash.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_address_compute() {
        let data = b"hello world";
        let addr = ContentAddress::compute(data);
        assert_eq!(addr.algorithm(), AddressAlgorithm::Blake3);
    }

    #[test]
    fn test_content_address_parse() {
        let addr = ContentAddress::parse("blake3:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890");
        assert!(addr.is_ok());
    }

    #[test]
    fn test_content_address_parse_invalid() {
        let addr = ContentAddress::parse("invalid");
        assert!(addr.is_err());
    }

    #[test]
    fn test_content_address_to_string() {
        let hash = Hash::from_bytes([1u8; 32]);
        let addr = ContentAddress::new(hash, AddressAlgorithm::Blake3);
        let s = addr.as_str();
        assert!(s.starts_with("blake3:"));
    }

    #[test]
    fn test_address_algorithm_parse() {
        assert_eq!(AddressAlgorithm::parse("blake3").unwrap(), AddressAlgorithm::Blake3);
        assert_eq!(AddressAlgorithm::parse("sha256").unwrap(), AddressAlgorithm::Sha256);
        assert_eq!(AddressAlgorithm::parse("sha512").unwrap(), AddressAlgorithm::Sha512);
    }

    #[test]
    fn test_address_algorithm_parse_invalid() {
        assert!(AddressAlgorithm::parse("unknown").is_err());
    }

    #[test]
    fn test_address_algorithm_as_str() {
        assert_eq!(AddressAlgorithm::Blake3.as_str(), "blake3");
        assert_eq!(AddressAlgorithm::Sha256.as_str(), "sha256");
        assert_eq!(AddressAlgorithm::Sha512.as_str(), "sha512");
    }

    #[test]
    fn test_address_algorithm_hash() {
        let data = b"test data";
        let hash1 = AddressAlgorithm::Blake3.hash(data);
        let hash2 = AddressAlgorithm::Blake3.hash(data);
        assert_eq!(hash1, hash2);
    }
}
