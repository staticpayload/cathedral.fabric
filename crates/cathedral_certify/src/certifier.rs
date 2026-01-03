//! Main certifier for generating determinism certificates.

use crate::certificate::{Certificate, CertificateError};
use crate::signature::{Signer, SignatureError};
use crate::validator::{DeterminismValidator, ValidationReport};
use cathedral_sim::record::SimRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the certifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifierConfig {
    /// Validator name
    pub validator_name: String,
    /// Validator version
    pub validator_version: String,
    /// Minimum number of runs required
    pub min_runs: usize,
    /// Additional metadata to include in certificates
    pub metadata: HashMap<String, String>,
}

impl Default for CertifierConfig {
    fn default() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("framework".to_string(), "cathedral.fabric".to_string());

        Self {
            validator_name: "cathedral-certifier".to_string(),
            validator_version: "0.1.0".to_string(),
            min_runs: 1,
            metadata,
        }
    }
}

/// Main certifier for deterministic execution
pub struct Certifier {
    /// Configuration
    config: CertifierConfig,
    /// Validator for checking determinism
    validator: DeterminismValidator,
    /// Signer for certificates
    signer: Signer,
}

impl Certifier {
    /// Create a new certifier
    #[must_use]
    pub fn new(config: CertifierConfig) -> Self {
        let validator = DeterminismValidator::new(
            config.validator_name.clone(),
            config.validator_version.clone(),
            // Public key will be derived from signer
            String::new(),
        );

        let signer = Signer::new();

        Self {
            config,
            validator,
            signer,
        }
    }

    /// Create a certifier with default configuration
    #[must_use]
    pub fn default() -> Self {
        Self::new(CertifierConfig::default())
    }

    /// Get the public key for this certifier
    #[must_use]
    pub fn public_key(&self) -> crate::signature::PublicKeyBytes {
        self.signer.public_key()
    }

    /// Validate multiple runs and generate a certificate
    ///
    /// # Errors
    ///
    /// Returns error if certification fails
    pub fn certify(
        &self,
        execution_id: String,
        runs: Vec<SimRecord>,
    ) -> Result<Certificate, CertifierError> {
        // Check minimum run requirement
        if runs.len() < self.config.min_runs {
            return Err(CertifierError::InsufficientRuns {
                provided: runs.len(),
                required: self.config.min_runs,
            });
        }

        // Validate runs
        let report = self.validator.validate_runs(&runs)?;

        // Get the first record for metadata
        let first_record = runs.first()
            .ok_or_else(|| CertifierError::NoRunsProvided)?;

        // Create certificate body
        let mut body = self.validator.certify(
            execution_id.clone(),
            first_record.seed.seed,
            first_record,
            &report,
        )?;

        // Add metadata from config
        for (key, value) in &self.config.metadata {
            body = body.with_metadata(key.clone(), value.clone());
        }

        // Add validation summary
        body = body.with_metadata("validation_summary".to_string(), report.summary());

        // Update validator public key
        body.validator.public_key = hex::encode(self.signer.public_key().as_bytes());

        // Sign the certificate
        let body_bytes = serde_cbor::to_vec(&body)
            .map_err(|_| CertifierError::SerializationError)?;
        let signature = self.signer.sign(&body_bytes)?;

        Ok(Certificate::new(body, signature))
    }

    /// Validate runs without creating a certificate
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn validate(&self, runs: &[SimRecord]) -> Result<ValidationReport, CertifierError> {
        Ok(self.validator.validate_runs(runs)?)
    }

    /// Verify a certificate
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify(&self, cert: &Certificate) -> Result<bool, CertifierError> {
        // Verify the certificate structure
        let body_bytes = serde_cbor::to_vec(&cert.body)
            .map_err(|_| CertifierError::SerializationError)?;

        // Check signature
        use crate::signature::{PublicKeyBytes, Verifier};
        let pub_key = PublicKeyBytes::from_hex(&cert.body.validator.public_key)
            .map_err(|_| CertifierError::InvalidPublicKey)?;
        let verifier = Verifier::new(pub_key)
            .map_err(|_| CertifierError::InvalidPublicKey)?;

        Ok(verifier.verify(&body_bytes, &cert.signature)?)
    }

    /// Export certificate to file
    ///
    /// # Errors
    ///
    /// Returns error if export fails
    pub fn export_certificate(&self, cert: &Certificate, path: &str) -> Result<(), CertifierError> {
        let json = cert.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| CertifierError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Import certificate from file
    ///
    /// # Errors
    ///
    /// Returns error if import fails
    pub fn import_certificate(&self, path: &str) -> Result<Certificate, CertifierError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| CertifierError::IoError(e.to_string()))?;
        Ok(Certificate::from_json(&json)?)
    }

    /// Create a certifier with a specific signer
    ///
    /// # Errors
    ///
    /// Returns error if signer creation fails
    pub fn with_signer(config: CertifierConfig, signer: Signer) -> Self {
        let validator = DeterminismValidator::new(
            config.validator_name.clone(),
            config.validator_version.clone(),
            String::new(),
        );

        Self {
            config,
            validator,
            signer,
        }
    }
}

impl Default for Certifier {
    fn default() -> Self {
        Self::default()
    }
}

/// Certifier-specific errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CertifierError {
    /// Insufficient runs provided
    #[error("insufficient runs: provided {provided}, required {required}")]
    InsufficientRuns { provided: usize, required: usize },
    /// No runs provided
    #[error("no runs provided")]
    NoRunsProvided,
    /// Serialization error
    #[error("serialization error")]
    SerializationError,
    /// Certificate error
    #[error("certificate error")]
    CertificateError,
    /// Signature error
    #[error("signature error")]
    SignatureError,
    /// Invalid public key
    #[error("invalid public key")]
    InvalidPublicKey,
    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<CertificateError> for CertifierError {
    fn from(_err: CertificateError) -> Self {
        Self::CertificateError
    }
}

impl From<SignatureError> for CertifierError {
    fn from(_err: SignatureError) -> Self {
        Self::SignatureError
    }
}

/// Batch certification result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchCertificationResult {
    /// Number of certifications attempted
    pub attempted: usize,
    /// Number successful
    pub successful: usize,
    /// Number failed
    pub failed: usize,
    /// Individual results
    pub results: Vec<CertificationResult>,
}

/// Result of certifying a single execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificationResult {
    /// Execution ID
    pub execution_id: String,
    /// Whether certification succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl Certifier {
    /// Certify multiple executions in batch
    ///
    /// # Errors
    ///
    /// Returns error if batch certification fails
    pub fn certify_batch(
        &self,
        executions: Vec<(String, Vec<SimRecord>)>,
    ) -> Result<BatchCertificationResult, CertifierError> {
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for (execution_id, runs) in executions {
            match self.certify(execution_id.clone(), runs) {
                Ok(_) => {
                    successful += 1;
                    results.push(CertificationResult {
                        execution_id,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(CertificationResult {
                        execution_id,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(BatchCertificationResult {
            attempted: results.len(),
            successful,
            failed,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cathedral_sim::seed::SimSeed;
    use cathedral_core::NodeId;

    fn create_test_record(seed: u64) -> SimRecord {
        SimRecord::new()
            .with_seed(SimSeed::from_literal(seed))
            .with_event(1, NodeId::new(), "test".to_string())
    }

    #[test]
    fn test_certifier_new() {
        let certifier = Certifier::new(CertifierConfig::default());
        assert_eq!(certifier.config.validator_name, "cathedral-certifier");
    }

    #[test]
    fn test_certifier_default() {
        let certifier = Certifier::default();
        assert_eq!(certifier.config.min_runs, 1);
    }

    #[test]
    fn test_certify_single_run() {
        let certifier = Certifier::default();
        let record = create_test_record(42);
        let cert = certifier.certify("exec-1".to_string(), vec![record]).unwrap();

        assert_eq!(cert.body.execution_id, "exec-1");
        assert_eq!(cert.body.seed, 42);
        assert!(!cert.signature.bytes.is_empty());
    }

    #[test]
    fn test_certify_multiple_identical_runs() {
        let certifier = Certifier::default();
        let node_id = NodeId::new();

        let record1 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "test".to_string());

        let record2 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "test".to_string());

        let cert = certifier.certify("exec-1".to_string(), vec![record1, record2]).unwrap();
        assert_eq!(cert.body.execution_id, "exec-1");
    }

    #[test]
    fn test_certify_insufficient_runs() {
        let mut config = CertifierConfig::default();
        config.min_runs = 3;
        let certifier = Certifier::new(config);

        let record = create_test_record(42);
        let result = certifier.certify("exec-1".to_string(), vec![record]);

        assert!(matches!(result, Err(CertifierError::InsufficientRuns { .. })));
    }

    #[test]
    fn test_certify_no_runs() {
        let certifier = Certifier::default();
        let result = certifier.certify("exec-1".to_string(), vec![]);

        assert!(matches!(result, Err(CertifierError::InsufficientRuns { .. })));
    }

    #[test]
    fn test_validate() {
        let certifier = Certifier::default();
        let record = create_test_record(42);

        let report = certifier.validate(&[record]).unwrap();
        assert!(report.passed);
    }

    #[test]
    fn test_export_import_certificate() {
        let certifier = Certifier::default();
        let record = create_test_record(42);

        let cert = certifier.certify("exec-1".to_string(), vec![record]).unwrap();

        // Export to a temp file
        let tmp_dir = std::env::temp_dir();
        let cert_path = tmp_dir.join("test-cert.json");
        certifier.export_certificate(&cert, cert_path.to_str().unwrap()).unwrap();

        // Import back
        let imported = certifier.import_certificate(cert_path.to_str().unwrap()).unwrap();
        assert_eq!(cert.id(), imported.id());

        // Cleanup
        std::fs::remove_file(cert_path).ok();
    }

    #[test]
    fn test_certify_batch() {
        let certifier = Certifier::default();

        let executions = vec![
            ("exec-1".to_string(), vec![create_test_record(42)]),
            ("exec-2".to_string(), vec![create_test_record(43)]),
        ];

        let result = certifier.certify_batch(executions).unwrap();
        assert_eq!(result.attempted, 2);
        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_certifier_config_default() {
        let config = CertifierConfig::default();
        assert_eq!(config.validator_name, "cathedral-certifier");
        assert_eq!(config.validator_version, "0.1.0");
        assert_eq!(config.min_runs, 1);
    }

    #[test]
    fn test_batch_certification_result() {
        let result = BatchCertificationResult {
            attempted: 5,
            successful: 4,
            failed: 1,
            results: vec![],
        };
        assert_eq!(result.attempted, 5);
        assert_eq!(result.successful, 4);
        assert_eq!(result.failed, 1);
    }

    #[test]
    fn test_certification_result() {
        let result = CertificationResult {
            execution_id: "exec-1".to_string(),
            success: true,
            error: None,
        };
        assert!(result.success);
    }

    #[test]
    fn test_certification_result_failed() {
        let result = CertificationResult {
            execution_id: "exec-1".to_string(),
            success: false,
            error: Some("test error".to_string()),
        };
        assert!(!result.success);
        assert_eq!(result.error, Some("test error".to_string()));
    }
}
