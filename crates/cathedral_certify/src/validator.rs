//! Validation of determinism.

use crate::certificate::{CertificateBody, DeterminismClaim, ValidatorInfo};
use crate::CertificateError;
use cathedral_sim::record::{RunComparison, SimRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of validating determinism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether validation passed
    pub passed: bool,
    /// Number of runs compared
    pub run_count: usize,
    /// Validation checks performed
    pub checks: Vec<ValidationCheck>,
    /// Additional details
    pub details: HashMap<String, String>,
}

impl ValidationReport {
    /// Create a new validation report
    #[must_use]
    pub fn new(passed: bool, run_count: usize) -> Self {
        Self {
            passed,
            run_count,
            checks: Vec::new(),
            details: HashMap::new(),
        }
    }

    /// Add a validation check
    #[must_use]
    pub fn with_check(mut self, check: ValidationCheck) -> Self {
        self.checks.push(check);
        self
    }

    /// Add detail
    #[must_use]
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    /// Get failed checks
    #[must_use]
    pub fn failed_checks(&self) -> Vec<&ValidationCheck> {
        self.checks.iter()
            .filter(|c| !c.passed)
            .collect()
    }

    /// Get summary
    #[must_use]
    pub fn summary(&self) -> String {
        let passed_count = self.checks.iter().filter(|c| c.passed).count();
        let total_count = self.checks.len();
        format!(
            "Validation {}: {}/{} checks passed, {} runs compared",
            if self.passed { "PASSED" } else { "FAILED" },
            passed_count,
            total_count,
            self.run_count
        )
    }
}

/// A single validation check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Check name
    pub name: String,
    /// Whether check passed
    pub passed: bool,
    /// Check message
    pub message: String,
}

impl ValidationCheck {
    /// Create a new validation check
    #[must_use]
    pub fn new(name: String, passed: bool, message: String) -> Self {
        Self { name, passed, message }
    }

    /// Create a passed check
    #[must_use]
    pub fn passed(name: String) -> Self {
        Self {
            name,
            passed: true,
            message: "Check passed".to_string(),
        }
    }

    /// Create a failed check
    #[must_use]
    pub fn failed(name: String, reason: String) -> Self {
        Self {
            name,
            passed: false,
            message: reason,
        }
    }
}

/// Validator for deterministic execution
pub struct DeterminismValidator {
    /// Validator name
    name: String,
    /// Validator version
    version: String,
    /// Public key for signing certificates
    public_key: String,
}

impl DeterminismValidator {
    /// Create a new validator
    #[must_use]
    pub fn new(name: String, version: String, public_key: String) -> Self {
        Self {
            name,
            version,
            public_key,
        }
    }

    /// Create a default validator
    #[must_use]
    pub fn default() -> Self {
        Self::new(
            "cathedral-validator".to_string(),
            "0.1.0".to_string(),
            "default-public-key".to_string(),
        )
    }

    /// Validate that multiple runs are deterministic
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn validate_runs(&self, runs: &[SimRecord]) -> Result<ValidationReport, CertificateError> {
        if runs.is_empty() {
            return Ok(ValidationReport::new(false, 0)
                .with_check(ValidationCheck::failed(
                    "has_runs".to_string(),
                    "No runs provided".to_string(),
                )));
        }

        let mut report = ValidationReport::new(true, runs.len());

        // Check 1: All runs have the same seed
        let first_seed = &runs[0].seed;
        let seed_match = runs.iter().all(|r| r.seed == *first_seed);
        report = report.with_check(ValidationCheck::new(
            "seed_consistency".to_string(),
            seed_match,
            if seed_match {
                format!("All runs use seed: {:?}", first_seed)
            } else {
                "Seeds differ between runs".to_string()
            },
        ));

        // Check 2: All runs have the same tick count
        let first_ticks = runs[0].max_ticks;
        let ticks_match = runs.iter().all(|r| r.max_ticks == first_ticks);
        report = report.with_check(ValidationCheck::new(
            "tick_consistency".to_string(),
            ticks_match,
            if ticks_match {
                format!("All runs: {} ticks", first_ticks)
            } else {
                "Tick counts differ".to_string()
            },
        ));

        // Check 3: Event count consistency
        let first_event_count = runs[0].events.len();
        let event_counts: Vec<_> = runs.iter().map(|r| r.events.len()).collect();
        let events_match = runs.iter().all(|r| r.events.len() == first_event_count);
        report = report.with_check(ValidationCheck::new(
            "event_count_consistency".to_string(),
            events_match,
            if events_match {
                format!("All runs: {} events", first_event_count)
            } else {
                format!("Event counts differ: {:?}", event_counts)
            },
        ));

        // Check 4: Event sequence consistency (pairwise comparison)
        let mut all_match = true;
        let mut mismatches = Vec::new();
        for (i, run_a) in runs.iter().enumerate() {
            for run_b in runs.iter().skip(i + 1) {
                let comparison = RunComparison::compare(run_a, run_b);
                if !comparison.identical {
                    all_match = false;
                    mismatches.push(format!("Runs {} and {} differ", i, i + 1));
                }
            }
        }
        report = report.with_check(ValidationCheck::new(
            "event_sequence_consistency".to_string(),
            all_match,
            if all_match {
                "All event sequences match".to_string()
            } else {
                format!("Sequence mismatches: {}", mismatches.join(", "))
            },
        ));

        // Update overall passed status
        report.passed = report.checks.iter().all(|c| c.passed);

        Ok(report)
    }

    /// Generate a certificate from validation
    ///
    /// # Errors
    ///
    /// Returns error if certificate generation fails
    pub fn certify(
        &self,
        execution_id: String,
        seed: u64,
        record: &SimRecord,
        report: &ValidationReport,
    ) -> Result<CertificateBody, CertificateError> {
        let validator_info = ValidatorInfo::new(
            self.name.clone(),
            self.version.clone(),
            self.public_key.clone(),
        );

        // Compute log hash
        let log_hash = self.compute_log_hash(record)?;

        let mut body = CertificateBody::new(
            execution_id,
            seed,
            record.max_ticks,
            record.events.len(),
            log_hash,
            validator_info,
        );

        // Add claims based on validation report
        if report.checks.iter().any(|c| c.name == "event_sequence_consistency" && c.passed) {
            body = body.with_claim(DeterminismClaim::IdenticalRuns {
                run_count: report.run_count,
            });
        }

        if report.checks.iter().any(|c| c.name == "seed_consistency" && c.passed) {
            body = body.with_claim(DeterminismClaim::SeededRandomness);
        }

        if report.checks.iter().all(|c| c.passed) {
            body = body.with_claim(DeterminismClaim::ValidHashChain);
        }

        // Add validation metadata
        body = body.with_detail("validation_report".to_string(), report.summary());

        Ok(body)
    }

    /// Compute hash of event log
    ///
    /// # Errors
    ///
    /// Returns error if hashing fails
    fn compute_log_hash(&self, record: &SimRecord) -> Result<String, CertificateError> {
        let serialized = serde_cbor::to_vec(&record.events)
            .map_err(|_| CertificateError::InvalidHash)?;
        Ok(format!("blake3:{}", hex::encode(blake3::hash(&serialized).as_bytes())))
    }
}

impl Default for DeterminismValidator {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cathedral_sim::seed::SimSeed;
    use cathedral_core::NodeId;

    #[test]
    fn test_validation_check_passed() {
        let check = ValidationCheck::passed("test_check".to_string());
        assert!(check.passed);
        assert_eq!(check.name, "test_check");
    }

    #[test]
    fn test_validation_check_failed() {
        let check = ValidationCheck::failed("test_check".to_string(), "test reason".to_string());
        assert!(!check.passed);
        assert_eq!(check.message, "test reason");
    }

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new(true, 3);
        assert!(report.passed);
        assert_eq!(report.run_count, 3);
        assert!(report.checks.is_empty());
    }

    #[test]
    fn test_validation_report_with_check() {
        let check = ValidationCheck::passed("test".to_string());
        let report = ValidationReport::new(true, 1).with_check(check);
        assert_eq!(report.checks.len(), 1);
    }

    #[test]
    fn test_validation_report_with_detail() {
        let report = ValidationReport::new(true, 1)
            .with_detail("key".to_string(), "value".to_string());
        assert_eq!(report.details.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_validation_report_summary() {
        let report = ValidationReport::new(true, 3)
            .with_check(ValidationCheck::passed("check1".to_string()))
            .with_check(ValidationCheck::passed("check2".to_string()));
        let summary = report.summary();
        assert!(summary.contains("PASSED"));
        assert!(summary.contains("2/2"));
    }

    #[test]
    fn test_determinism_validator_new() {
        let validator = DeterminismValidator::new(
            "test".to_string(),
            "1.0".to_string(),
            "key".to_string(),
        );
        assert_eq!(validator.name, "test");
        assert_eq!(validator.version, "1.0");
    }

    #[test]
    fn test_determinism_validator_default() {
        let validator = DeterminismValidator::default();
        assert_eq!(validator.name, "cathedral-validator");
    }

    #[test]
    fn test_validate_runs_empty() {
        let validator = DeterminismValidator::default();
        let report = validator.validate_runs(&[]).unwrap();
        assert!(!report.passed);
        assert_eq!(report.run_count, 0);
    }

    #[test]
    fn test_validate_runs_single() {
        let validator = DeterminismValidator::default();
        let record = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, NodeId::new(), "test".to_string());

        let report = validator.validate_runs(&[record]).unwrap();
        assert!(report.passed);
        assert_eq!(report.run_count, 1);
    }

    #[test]
    fn test_validate_runs_identical() {
        let validator = DeterminismValidator::default();
        let node_id = NodeId::new();

        let record1 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "test".to_string());

        let record2 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "test".to_string());

        let report = validator.validate_runs(&[record1, record2]).unwrap();
        assert!(report.passed);
        assert_eq!(report.run_count, 2);
    }

    #[test]
    fn test_validate_runs_different_seeds() {
        let validator = DeterminismValidator::default();
        let node_id = NodeId::new();

        let record1 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "test".to_string());

        let record2 = SimRecord::new()
            .with_seed(SimSeed::from_literal(43))
            .with_event(1, node_id, "test".to_string());

        let report = validator.validate_runs(&[record1, record2]).unwrap();
        // Validation fails because seeds differ
        assert!(!report.passed);
    }

    #[test]
    fn test_validate_runs_different_events() {
        let validator = DeterminismValidator::default();
        let node_id = NodeId::new();

        let record1 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "event1".to_string());

        let record2 = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, node_id, "event2".to_string());

        let report = validator.validate_runs(&[record1, record2]).unwrap();
        assert!(!report.passed);
    }

    #[test]
    fn test_certify() {
        let validator = DeterminismValidator::default();
        let record = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, NodeId::new(), "test".to_string());

        let report = ValidationReport::new(true, 1)
            .with_check(ValidationCheck::passed("event_sequence_consistency".to_string()))
            .with_check(ValidationCheck::passed("seed_consistency".to_string()));

        let body = validator.certify("exec-1".to_string(), 42, &record, &report).unwrap();
        assert_eq!(body.execution_id, "exec-1");
        assert_eq!(body.seed, 42);
        assert!(!body.claims.is_empty());
    }

    #[test]
    fn test_compute_log_hash() {
        let validator = DeterminismValidator::default();
        let record = SimRecord::new()
            .with_seed(SimSeed::from_literal(42))
            .with_event(1, NodeId::new(), "test".to_string());

        let hash = validator.compute_log_hash(&record).unwrap();
        assert!(hash.starts_with("blake3:"));
    }
}
