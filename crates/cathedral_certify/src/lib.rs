//! Determinism certification for CATHEDRAL.FABRIC.
//!
//! This crate provides tools for certifying that executions are deterministic
//! by comparing multiple runs, verifying hash chains, and generating cryptographic
//! certificates of determinism.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod certifier;
pub mod certificate;
pub mod signature;
pub mod validator;

pub use certifier::Certifier;
pub use certificate::{Certificate, CertificateBody, CertificateError};
pub use signature::{SignatureScheme, Signer, Verifier};
pub use validator::{DeterminismValidator, ValidationReport};
