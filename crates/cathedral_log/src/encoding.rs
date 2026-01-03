//! Canonical encoding for cross-platform reproducibility.
//!
//! Uses postcard for byte-stable encoding.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Trait for canonical serialization
pub trait CanonicalEncode: Serialize {
    /// Encode to canonical bytes
    fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("encoding failed")
    }

    /// Encode into a slice
    fn encode_to_slice(&self, slice: &mut [u8]) -> Result<usize, EncodeError> {
        postcard::to_slice(self, slice).map_err(|_| EncodeError::BufferTooSmall)?;
        Ok(self.encoded_len())
    }

    /// Get encoded length
    fn encoded_len(&self) -> usize {
        postcard::to_allocvec(self).map(|v| v.len()).unwrap_or(0)
    }
}

// Blanket impl removed - types must explicitly impl CanonicalEncode
// This allows custom implementations like Event to override behavior

/// Trait for canonical deserialization
pub trait CanonicalDecode<'de>: Deserialize<'de> {
    /// Decode from canonical bytes
    fn decode(data: &'de [u8]) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        postcard::from_bytes(data).map_err(|_| DecodeError::InvalidEncoding)
    }
}

impl<'de, T: Deserialize<'de>> CanonicalDecode<'de> for T {}

/// Encoding errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// Buffer too small for encoded data
    BufferTooSmall,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small for encoded data"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Decoding errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Invalid encoding
    InvalidEncoding,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEncoding => write!(f, "Invalid canonical encoding"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Canonical encoder for streaming
pub struct CanonicalEncoder<W> {
    writer: W,
}

impl<W: Write> CanonicalEncoder<W> {
    /// Create a new encoder
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Encode a value
    pub fn encode<T: CanonicalEncode>(&mut self, value: &T) -> Result<(), EncodeError> {
        let bytes = value.encode();
        self.writer
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .map_err(|_| EncodeError::BufferTooSmall)?;
        self.writer
            .write_all(&bytes)
            .map_err(|_| EncodeError::BufferTooSmall)?;
        Ok(())
    }

    /// Flush the writer
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Consume and return the inner writer
    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Canonical decoder for streaming
pub struct CanonicalDecoder<R> {
    reader: R,
}

impl<R: Read> CanonicalDecoder<R> {
    /// Create a new decoder
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Decode a value
    pub fn decode<T: for<'de> Deserialize<'de>>(&mut self) -> Result<Option<T>, DecodeError> {
        let mut len_bytes = [0u8; 4];
        let n = self
            .reader
            .read(&mut len_bytes)
            .map_err(|_| DecodeError::InvalidEncoding)?;

        if n == 0 {
            return Ok(None);
        }
        if n < 4 {
            return Err(DecodeError::InvalidEncoding);
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut buffer = vec![0u8; len];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|_| DecodeError::InvalidEncoding)?;

        // Use postcard directly to avoid lifetime issues
        postcard::from_bytes(&buffer).map_err(|_| DecodeError::InvalidEncoding).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Import proptest macros
    use proptest::prelude::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestStruct {
        a: u64,
        b: String,
        c: Vec<u32>,
    }

    impl CanonicalEncode for TestStruct {}

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = TestStruct {
            a: 42,
            b: "hello".to_string(),
            c: vec![1, 2, 3],
        };

        let encoded = original.encode();
        let decoded = TestStruct::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_deterministic() {
        let value = TestStruct {
            a: 42,
            b: "hello".to_string(),
            c: vec![1, 2, 3],
        };

        let enc1 = value.encode();
        let enc2 = value.encode();

        assert_eq!(enc1, enc2);
    }

    #[test]
    fn test_encode_empty_vec() {
        let value = TestStruct {
            a: 0,
            b: "".to_string(),
            c: vec![],
        };

        let encoded = value.encode();
        let decoded = TestStruct::decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_streaming_encode_decode() {
        let values = vec![
            TestStruct {
                a: 1,
                b: "first".to_string(),
                c: vec![10],
            },
            TestStruct {
                a: 2,
                b: "second".to_string(),
                c: vec![20],
            },
        ];

        let mut buffer = Vec::new();
        {
            let mut encoder = CanonicalEncoder::new(&mut buffer);
            for v in &values {
                encoder.encode(v).unwrap();
            }
        }

        let mut decoder = CanonicalDecoder::new(buffer.as_slice());
        let mut decoded = Vec::new();
        while let Some(v) = decoder.decode::<TestStruct>().unwrap() {
            decoded.push(v);
        }

        assert_eq!(values, decoded);
    }

    #[test]
    fn test_invalid_decode() {
        let invalid = &[0xFF, 0xFF, 0xFF];
        let result: Result<TestStruct, _> = TestStruct::decode(invalid);
        assert!(result.is_err());
    }

    // Property tests using proptest
    proptest::proptest! {
        #[test]
        fn prop_encode_roundtrip(
            a: u64,
            b: String,
            c: Vec<u32>
        ) {
            let value = TestStruct { a, b: b.clone(), c: c.clone() };
            let encoded = value.encode();
            let decoded = TestStruct::decode(&encoded).unwrap();
            prop_assert_eq!(value, decoded);
        }

        #[test]
        fn prop_encode_deterministic(
            a: u64,
            b: String,
            c: Vec<u32>
        ) {
            let value = TestStruct { a, b: b.clone(), c: c.clone() };
            let enc1 = value.encode();
            let enc2 = value.encode();
            prop_assert_eq!(enc1, enc2);
        }

        #[test]
        fn prop_streaming_roundtrip(values: Vec<u64>) {
            let test_values: Vec<TestStruct> = values.into_iter().map(|v| TestStruct {
                a: v,
                b: format!("test_{}", v),
                c: vec![v as u32],
            }).collect();

            let mut buffer = Vec::new();
            {
                let mut encoder = CanonicalEncoder::new(&mut buffer);
                for v in &test_values {
                    encoder.encode(v).unwrap();
                }
            }

            let mut decoder = CanonicalDecoder::new(buffer.as_slice());
            let mut decoded = Vec::new();
            while let Some(v) = decoder.decode::<TestStruct>().unwrap() {
                decoded.push(v);
            }

            prop_assert_eq!(test_values, decoded);
        }
    }
}
