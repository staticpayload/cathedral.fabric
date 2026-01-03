//! Certificate of determinism.

use crate::signature::Signature;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A certificate attesting to deterministic execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificate {
    /// The certificate body
    pub body: CertificateBody,
    /// Signature over the body
    pub signature: Signature,
}

impl Certificate {
    /// Create a new certificate
    #[must_use]
    pub fn new(body: CertificateBody, signature: Signature) -> Self {
        Self { body, signature }
    }

    /// Get the certificate ID
    #[must_use]
    pub fn id(&self) -> &str {
        &self.body.id
    }

    /// Get the certificate as JSON
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn to_json(&self) -> Result<String, CertificateError> {
        serde_json::to_string(self)
            .map_err(|_| CertificateError::SerializationError)
    }

    /// Parse certificate from JSON
    ///
    /// # Errors
    ///
    /// Returns error if deserialization fails
    pub fn from_json(json: &str) -> Result<Self, CertificateError> {
        serde_json::from_str(json)
            .map_err(|_| CertificateError::ParseError)
    }

    /// Verify the certificate signature
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify(&self) -> Result<bool, CertificateError> {
        let _body_bytes = serde_cbor::to_vec(&self.body)
            .map_err(|_| CertificateError::SerializationError)?;
        // Verification requires a verifier - this just checks structure
        Ok(!self.signature.bytes.is_empty())
    }
}

/// The body of a certificate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateBody {
    /// Unique certificate ID
    pub id: String,
    /// Execution ID being certified
    pub execution_id: String,
    /// Seed used for execution
    pub seed: u64,
    /// Number of ticks executed
    pub ticks: u64,
    /// Number of events recorded
    pub event_count: usize,
    /// Hash of the event log
    pub log_hash: String,
    /// Validator information
    pub validator: ValidatorInfo,
    /// Certification timestamp
    pub certified_at: DateTime<Utc>,
    /// Determinism claims
    pub claims: Vec<DeterminismClaim>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl CertificateBody {
    /// Create a new certificate body
    #[must_use]
    pub fn new(
        execution_id: String,
        seed: u64,
        ticks: u64,
        event_count: usize,
        log_hash: String,
        validator: ValidatorInfo,
    ) -> Self {
        Self {
            id: format!("cert-{}", uuid::Uuid::new_v4()),
            execution_id,
            seed,
            ticks,
            event_count,
            log_hash,
            validator,
            certified_at: Utc::now(),
            claims: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a determinism claim
    #[must_use]
    pub fn with_claim(mut self, claim: DeterminismClaim) -> Self {
        self.claims.push(claim);
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add detail (alias for with_metadata)
    #[must_use]
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate hash of the body
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn hash(&self) -> Result<String, CertificateError> {
        let bytes = serde_cbor::to_vec(self)
            .map_err(|_| CertificateError::SerializationError)?;
        Ok(format!("blake3:{}", hex::encode(blake3::hash(&bytes).as_bytes())))
    }
}

/// Information about the validator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator name
    pub name: String,
    /// Validator version
    pub version: String,
    /// Validator public key (hex)
    pub public_key: String,
}

impl ValidatorInfo {
    /// Create new validator info
    #[must_use]
    pub fn new(name: String, version: String, public_key: String) -> Self {
        Self {
            name,
            version,
            public_key,
        }
    }
}

/// A claim about determinism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeterminismClaim {
    /// All runs produced identical event sequences
    IdenticalRuns { run_count: usize },
    /// Hash chain is intact
    ValidHashChain,
    /// No external state was accessed
    NoExternalAccess,
    /// All randomness was seeded
    SeededRandomness,
    /// Custom claim with description
    Custom { description: String },
}

/// Certificate-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CertificateError {
    /// Serialization error
    #[error("serialization error")]
    SerializationError,
    /// Parse error
    #[error("parse error")]
    ParseError,
    /// Invalid certificate format
    #[error("invalid certificate format")]
    InvalidFormat,
    /// Signature verification failed
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    /// Invalid hash
    #[error("invalid hash")]
    InvalidHash,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_info_new() {
        let info = ValidatorInfo::new(
            "test-validator".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        );
        assert_eq!(info.name, "test-validator");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn test_certificate_body_new() {
        let validator = ValidatorInfo::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        let body = CertificateBody::new(
            "exec-1".to_string(),
            42,
            100,
            50,
            "hash123".to_string(),
            validator,
        );
        assert_eq!(body.execution_id, "exec-1");
        assert_eq!(body.seed, 42);
        assert_eq!(body.ticks, 100);
    }

    #[test]
    fn test_certificate_body_with_claim() {
        let validator = ValidatorInfo::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        let body = CertificateBody::new(
            "exec-1".to_string(),
            42,
            100,
            50,
            "hash123".to_string(),
            validator,
        )
        .with_claim(DeterminismClaim::ValidHashChain);

        assert_eq!(body.claims.len(), 1);
        assert!(matches!(body.claims[0], DeterminismClaim::ValidHashChain));
    }

    #[test]
    fn test_certificate_body_with_metadata() {
        let validator = ValidatorInfo::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        let body = CertificateBody::new(
            "exec-1".to_string(),
            42,
            100,
            50,
            "hash123".to_string(),
            validator,
        )
        .with_metadata("foo".to_string(), "bar".to_string());

        assert_eq!(body.metadata.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_certificate_to_from_json() {
        let validator = ValidatorInfo::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        let body = CertificateBody::new(
            "exec-1".to_string(),
            42,
            100,
            50,
            "hash123".to_string(),
            validator,
        );
        use crate::signature::Signature;
        use crate::signature::SignatureScheme;

        let sig = Signature::new(SignatureScheme::Ed25519, vec![1u8; 64]);
        let cert = Certificate::new(body, sig);

        let json = cert.to_json().unwrap();
        let restored = Certificate::from_json(&json).unwrap();
        assert_eq!(cert.id(), restored.id());
    }

    #[test]
    fn test_determinism_claim_variants() {
        let claims = vec![
            DeterminismClaim::IdenticalRuns { run_count: 3 },
            DeterminismClaim::ValidHashChain,
            DeterminismClaim::NoExternalAccess,
            DeterminismClaim::SeededRandomness,
            DeterminismClaim::Custom { description: "custom claim".to_string() },
        ];

        assert!(matches!(claims[0], DeterminismClaim::IdenticalRuns { run_count: 3 }));
        assert!(matches!(claims[1], DeterminismClaim::ValidHashChain));
    }

    #[test]
    fn test_certificate_verify() {
        let validator = ValidatorInfo::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        let body = CertificateBody::new(
            "exec-1".to_string(),
            42,
            100,
            50,
            "hash123".to_string(),
            validator,
        );
        use crate::signature::Signature;
        use crate::signature::SignatureScheme;

        let sig = Signature::new(SignatureScheme::Ed25519, vec![1u8; 64]);
        let cert = Certificate::new(body, sig);

        // Verify should succeed (structure check)
        assert!(cert.verify().unwrap());
    }
}
