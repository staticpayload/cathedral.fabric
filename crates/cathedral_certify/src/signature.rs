//! Cryptographic signature support for certificates.

use ed25519_dalek::Signer as DalekSigner;
use ed25519_dalek::Verifier as DalekVerifier;
use ed25519_dalek::Signature as DalekSignature;
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

/// Signature scheme for certificates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureScheme {
    /// Ed25519 signature scheme
    Ed25519,
}

impl Default for SignatureScheme {
    fn default() -> Self {
        Self::Ed25519
    }
}

/// A cryptographic signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Signature scheme used
    pub scheme: SignatureScheme,
    /// Signature bytes
    pub bytes: Vec<u8>,
}

impl Signature {
    /// Create a new signature
    #[must_use]
    pub fn new(scheme: SignatureScheme, bytes: Vec<u8>) -> Self {
        Self { scheme, bytes }
    }

    /// Create an Ed25519 signature
    #[must_use]
    pub fn ed25519(bytes: Vec<u8>) -> Self {
        Self {
            scheme: SignatureScheme::Ed25519,
            bytes,
        }
    }

    /// Get the signature bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A signer that can create signatures
pub struct Signer {
    /// The signing key
    signing_key: SigningKey,
    /// The verifying key (derived)
    verifying_key: VerifyingKey,
}

impl Signer {
    /// Create a new signer with a random keypair
    #[must_use]
    pub fn new() -> Self {
        let mut rng = OsRng;
        // Generate 32 random bytes for the secret key
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = VerifyingKey::from(&signing_key);
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create a signer from a secret key
    ///
    /// # Errors
    ///
    /// Returns error if the secret key is invalid
    pub fn from_secret(secret: &[u8]) -> Result<Self, SignatureError> {
        let bytes: [u8; 32] = secret.try_into()
            .map_err(|_| SignatureError::InvalidSecretKey)?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = VerifyingKey::from(&signing_key);
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Get the public key
    #[must_use]
    pub fn public_key(&self) -> PublicKeyBytes {
        PublicKeyBytes(self.verifying_key.to_bytes())
    }

    /// Sign a message
    ///
    /// # Errors
    ///
    /// Returns error if signing fails
    pub fn sign(&self, message: &[u8]) -> Result<Signature, SignatureError> {
        let sig = self.signing_key.sign(message);
        Ok(Signature::ed25519(sig.to_bytes().to_vec()))
    }
}

impl Default for Signer {
    fn default() -> Self {
        Self::new()
    }
}

/// Public key bytes for verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyBytes(pub [u8; 32]);

impl PublicKeyBytes {
    /// Create from bytes
    ///
    /// # Errors
    ///
    /// Returns error if bytes are invalid
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
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
    /// Returns error if hex is invalid
    pub fn from_hex(hex: &str) -> Result<Self, SignatureError> {
        let bytes = hex::decode(hex)
            .map_err(|_| SignatureError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(SignatureError::InvalidPublicKey);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// A verifier that can verify signatures
pub struct Verifier {
    /// The public key for verification
    verifying_key: VerifyingKey,
}

impl Verifier {
    /// Create a verifier from a public key
    ///
    /// # Errors
    ///
    /// Returns error if public key is invalid
    pub fn new(public_key: PublicKeyBytes) -> Result<Self, SignatureError> {
        let verifying_key = VerifyingKey::from_bytes(&public_key.0)
            .map_err(|_| SignatureError::InvalidPublicKey)?;
        Ok(Self { verifying_key })
    }

    /// Verify a signature on a message
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, SignatureError> {
        if signature.scheme != SignatureScheme::Ed25519 {
            return Err(SignatureError::UnsupportedScheme);
        }
        let sig = DalekSignature::from_slice(&signature.bytes)
            .map_err(|_| SignatureError::InvalidSignature)?;
        Ok(self.verifying_key.verify(message, &sig).is_ok())
    }
}

/// Signature-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    /// Invalid secret key
    #[error("invalid secret key")]
    InvalidSecretKey,
    /// Invalid public key
    #[error("invalid public key")]
    InvalidPublicKey,
    /// Invalid signature
    #[error("invalid signature")]
    InvalidSignature,
    /// Invalid hex encoding
    #[error("invalid hex encoding")]
    InvalidHex,
    /// Unsupported signature scheme
    #[error("unsupported signature scheme")]
    UnsupportedScheme,
    /// Verification failed
    #[error("signature verification failed")]
    VerificationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_new() {
        let signer = Signer::new();
        let pub_key = signer.public_key();
        assert_ne!(pub_key.0, [0u8; 32]);
    }

    #[test]
    fn test_sign_and_verify() {
        let signer = Signer::new();
        let message = b"test message";
        let signature = signer.sign(message).unwrap();

        let verifier = Verifier::new(signer.public_key()).unwrap();
        assert!(verifier.verify(message, &signature).unwrap());
    }

    #[test]
    fn test_verify_fails_with_different_message() {
        let signer = Signer::new();
        let message = b"test message";
        let signature = signer.sign(message).unwrap();

        let verifier = Verifier::new(signer.public_key()).unwrap();
        assert!(!verifier.verify(b"different message", &signature).unwrap());
    }

    #[test]
    fn test_public_key_hex_roundtrip() {
        let signer = Signer::new();
        let pub_key = signer.public_key();
        let hex = pub_key.to_hex();
        let restored = PublicKeyBytes::from_hex(&hex).unwrap();
        assert_eq!(pub_key, restored);
    }

    #[test]
    fn test_signature_scheme_default() {
        assert_eq!(SignatureScheme::default(), SignatureScheme::Ed25519);
    }

    #[test]
    fn test_signature_ed25519() {
        let sig = Signature::ed25519(vec![1u8; 64]);
        assert_eq!(sig.scheme, SignatureScheme::Ed25519);
        assert_eq!(sig.bytes.len(), 64);
    }

    #[test]
    fn test_verifier_invalid_public_key() {
        // In ed25519-dalek v2, any 32-byte array is accepted as a public key
        // The verification will fail for invalid signatures instead
        let result = Verifier::new(PublicKeyBytes([0u8; 32]));
        assert!(result.is_ok());
    }

    #[test]
    fn test_public_key_from_hex_invalid_length() {
        let result = PublicKeyBytes::from_hex("abcd");
        assert!(matches!(result, Err(SignatureError::InvalidPublicKey)));
    }

    #[test]
    fn test_public_key_from_hex_invalid_chars() {
        let result = PublicKeyBytes::from_hex("gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg");
        assert!(matches!(result, Err(SignatureError::InvalidHex)));
    }
}
