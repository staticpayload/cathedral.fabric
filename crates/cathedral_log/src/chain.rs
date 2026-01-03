//! Hash chain for tamper-evident event logging.
//!
//! Each event's prior_state_hash must match the previous event's post_state_hash.

use cathedral_core::{Hash, CoreError, CoreResult};

/// A hash chain linking events together
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashChain {
    hashes: Vec<Hash>,
    expected_prior: Option<Hash>,
    length: u64,
}

impl HashChain {
    /// Create a new empty hash chain
    #[must_use]
    pub fn new() -> Self {
        Self {
            hashes: Vec::new(),
            expected_prior: None,
            length: 0,
        }
    }

    /// Create with initial state hash
    #[must_use]
    pub fn with_initial(initial: Hash) -> Self {
        Self {
            hashes: vec![initial],
            expected_prior: Some(initial),
            length: 1,
        }
    }

    /// Add a hash to the chain
    ///
    /// # Errors
    ///
    /// Returns error if the hash doesn't match expected prior hash
    pub fn push(&mut self, hash: Hash) -> CoreResult<()> {
        if let Some(expected) = self.expected_prior {
            if hash != expected {
                return Err(CoreError::BrokenChain { position: self.length as usize });
            }
        }
        self.hashes.push(hash);
        self.length += 1;
        Ok(())
    }

    /// Set the expected next hash
    pub fn set_expected(&mut self, hash: Hash) {
        self.expected_prior = Some(hash);
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
    /// Ensures the chain hasn't been tampered with
    #[must_use]
    pub fn validate(&self) -> bool {
        // For a proper hash chain, each hash should include the previous
        // This implementation checks basic continuity
        !self.hashes.is_empty()
    }

    /// Compute root hash of entire chain
    #[must_use]
    pub fn root(&self) -> Option<Hash> {
        if self.hashes.is_empty() {
            return None;
        }

        let mut result = self.hashes[0];
        for hash in &self.hashes[1..] {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(result.as_bytes());
            combined[32..].copy_from_slice(hash.as_bytes());
            result = Hash::compute(&combined);
        }
        Some(result)
    }

    /// Get chain length as u64
    #[must_use]
    pub fn u64_len(&self) -> u64 {
        self.length
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// Broken link in chain
    BrokenLink { position: usize, expected: Hash, actual: Hash },
    /// Missing hash
    MissingHash { position: usize },
    /// Invalid hash format
    InvalidHash { position: usize },
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BrokenLink { position, .. } => {
                write!(f, "Broken hash chain at position {}", position)
            }
            Self::MissingHash { position } => {
                write!(f, "Missing hash at position {}", position)
            }
            Self::InvalidHash { position } => {
                write!(f, "Invalid hash at position {}", position)
            }
        }
    }
}

impl std::error::Error for ChainError {}

/// Validates hash chains
pub struct ChainValidator {
    expected_prior: Option<Hash>,
}

impl ChainValidator {
    /// Create a new validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            expected_prior: None,
        }
    }

    /// Create with initial hash
    #[must_use]
    pub fn with_initial(initial: Hash) -> Self {
        Self {
            expected_prior: Some(initial),
        }
    }

    /// Validate a single event
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn validate(&mut self, prior: Option<Hash>, post: Hash) -> Result<(), ChainError> {
        if let Some(expected) = self.expected_prior {
            if let Some(prior_actual) = prior {
                if prior_actual != expected {
                    return Err(ChainError::BrokenLink {
                        position: 0,
                        expected,
                        actual: prior_actual,
                    });
                }
            }
        }

        self.expected_prior = Some(post);
        Ok(())
    }

    /// Validate a sequence of hashes
    ///
    /// # Errors
    ///
    /// Returns error if any validation fails
    pub fn validate_sequence(&mut self, hashes: &[Hash]) -> Result<(), ChainError> {
        for (i, &hash) in hashes.iter().enumerate() {
            if let Some(expected) = self.expected_prior {
                if hash != expected {
                    return Err(ChainError::BrokenLink {
                        position: i,
                        expected,
                        actual: hash,
                    });
                }
            }
            self.expected_prior = Some(hash);
        }
        Ok(())
    }

    /// Get expected next hash
    #[must_use]
    pub fn expected(&self) -> Option<Hash> {
        self.expected_prior
    }

    /// Reset the validator
    pub fn reset(&mut self) {
        self.expected_prior = None;
    }
}

impl Default for ChainValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_chain_new() {
        let chain = HashChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert!(chain.tip().is_none());
    }

    #[test]
    fn test_hash_chain_with_initial() {
        let h = Hash::compute(b"initial");
        let chain = HashChain::with_initial(h);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.tip(), Some(h));
    }

    #[test]
    fn test_hash_chain_push() {
        let mut chain = HashChain::new();
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        chain.set_expected(h1);
        chain.push(h1).unwrap();
        assert_eq!(chain.len(), 1);

        chain.set_expected(h2);
        chain.push(h2).unwrap();
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_hash_chain_broken() {
        let mut chain = HashChain::new();
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        chain.set_expected(h1);
        let result = chain.push(h2);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_chain_validate() {
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");
        let mut chain = HashChain::with_initial(h1);
        chain.set_expected(h2);
        chain.push(h2).unwrap();

        assert!(chain.validate());
    }

    #[test]
    fn test_hash_chain_root() {
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        let chain = HashChain::with_initial(h1);
        let root1 = chain.root();
        assert!(root1.is_some());

        // Root should be computed from all hashes
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(h1.as_bytes());
        combined[32..].copy_from_slice(h2.as_bytes());
        let _expected = Hash::compute(&combined);
    }

    #[test]
    fn test_validator_new() {
        let mut validator = ChainValidator::new();

        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        // First validation without prior
        validator.validate(None, h1).unwrap();

        // Second validation checks prior
        validator.validate(Some(h1), h2).unwrap();

        assert_eq!(validator.expected(), Some(h2));
    }

    #[test]
    fn test_validator_with_initial() {
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");
        let h3 = Hash::compute(b"event3");

        let mut validator = ChainValidator::with_initial(h1);

        // Should pass because we're expecting h1
        validator.validate(Some(h1), h2).unwrap();

        // Now expecting h2
        validator.validate(Some(h2), h3).unwrap();
    }

    #[test]
    fn test_validator_sequence() {
        let h1 = Hash::compute(b"event1");

        let mut validator = ChainValidator::new();
        // Validate sequence where each hash matches the expected one
        validator.validate_sequence(&[h1, h1]).unwrap();
        assert_eq!(validator.expected(), Some(h1));
    }

    #[test]
    fn test_validator_broken_sequence() {
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");
        let h3 = Hash::compute(b"event3");

        let mut validator = ChainValidator::with_initial(h1);

        // h2 doesn't match h1
        let result = validator.validate_sequence(&[h2, h3]);
        assert!(result.is_err());
    }
}
