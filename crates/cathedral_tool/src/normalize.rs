//! Output normalization for deterministic tool results.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Normalization error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    /// Invalid JSON
    InvalidJson { reason: String },
    /// Schema mismatch
    SchemaMismatch { field: String },
    /// Cannot normalize type
    UnsupportedType { type_name: String },
    /// Circular reference
    CircularReference,
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson { reason } => write!(f, "Invalid JSON: {}", reason),
            Self::SchemaMismatch { field } => write!(f, "Schema mismatch: {}", field),
            Self::UnsupportedType { type_name } => {
                write!(f, "Unsupported type: {}", type_name)
            }
            Self::CircularReference => write!(f, "Circular reference detected"),
        }
    }
}

impl std::error::Error for NormalizationError {}

/// Normalization configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeConfig {
    /// Sort object keys
    pub sort_keys: bool,
    /// Normalize whitespace in strings
    pub normalize_whitespace: bool,
    /// Trim string values
    pub trim_strings: bool,
    /// Normalize floating point precision
    pub float_precision: Option<usize>,
    /// Normalize timestamps to UTC
    pub normalize_timestamps: bool,
    /// Remove null values
    pub remove_nulls: bool,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            sort_keys: true,
            normalize_whitespace: false,
            trim_strings: false,
            float_precision: None,
            normalize_timestamps: true,
            remove_nulls: false,
        }
    }
}

/// Normalized output from a tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedOutput {
    /// Normalized data as JSON
    pub data: serde_json::Value,
    /// Original size before normalization
    pub original_size: usize,
    /// Normalized size
    pub normalized_size: usize,
    /// List of transformations applied
    pub transformations: Vec<String>,
}

impl NormalizedOutput {
    /// Create normalized output from raw bytes
    ///
    /// # Errors
    ///
    /// Returns error if input is not valid JSON
    pub fn from_bytes(input: &[u8]) -> Result<Self, NormalizationError> {
        let original_size = input.len();
        let mut data: serde_json::Value = serde_json::from_slice(input)
            .map_err(|e| NormalizationError::InvalidJson {
                reason: e.to_string(),
            })?;

        let mut transformations = Vec::new();
        let config = NormalizeConfig::default();

        // Apply normalization
        if config.sort_keys {
            data = Self::sort_keys(data);
            transformations.push("sort_keys".to_string());
        }

        if config.remove_nulls {
            data = Self::remove_nulls(data);
            transformations.push("remove_nulls".to_string());
        }

        let normalized_size = serde_json::to_vec(&data)
            .map_err(|e| NormalizationError::InvalidJson {
                reason: e.to_string(),
            })?
            .len();

        Ok(Self {
            data,
            original_size,
            normalized_size,
            transformations,
        })
    }

    /// Get the normalized output as bytes
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn to_bytes(&self) -> Result<Vec<u8>, NormalizationError> {
        serde_json::to_vec(&self.data).map_err(|e| NormalizationError::InvalidJson {
            reason: e.to_string(),
        })
    }

    /// Get the normalized output as a pretty string
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn to_string_pretty(&self) -> Result<String, NormalizationError> {
        serde_json::to_string_pretty(&self.data).map_err(|e| NormalizationError::InvalidJson {
            reason: e.to_string(),
        })
    }

    /// Sort object keys recursively
    fn sort_keys(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                // Sort keys and process values recursively
                let mut sorted = BTreeMap::new();
                for (k, v) in map {
                    sorted.insert(k, Self::sort_keys(v));
                }
                serde_json::Value::Object(sorted.into_iter().collect())
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::sort_keys).collect())
            }
            _ => value,
        }
    }

    /// Remove null values recursively
    fn remove_nulls(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(mut map) => {
                map.retain(|_, v| !v.is_null());
                let processed: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .map(|(k, v)| (k, Self::remove_nulls(v)))
                    .collect();
                serde_json::Value::Object(processed)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::remove_nulls).collect())
            }
            _ => value,
        }
    }
}

/// Normalizer for tool outputs
pub struct Normalizer {
    config: NormalizeConfig,
}

impl Normalizer {
    /// Create a new normalizer with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: NormalizeConfig::default(),
        }
    }

    /// Create a normalizer with custom config
    #[must_use]
    pub fn with_config(config: NormalizeConfig) -> Self {
        Self { config }
    }

    /// Normalize tool output
    ///
    /// # Errors
    ///
    /// Returns error if normalization fails
    pub fn normalize(&self, input: &[u8]) -> Result<NormalizedOutput, NormalizationError> {
        NormalizedOutput::from_bytes(input)
    }

    /// Normalize a JSON value
    #[must_use]
    pub fn normalize_value(&self, value: serde_json::Value) -> serde_json::Value {
        let mut result = value;
        if self.config.sort_keys {
            result = NormalizedOutput::sort_keys(result);
        }
        if self.config.remove_nulls {
            result = NormalizedOutput::remove_nulls(result);
        }
        result
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_output_from_bytes() {
        let input = r#"{"b": 2, "a": 1}"#;
        let output = NormalizedOutput::from_bytes(input.as_bytes()).unwrap();
        assert_eq!(output.data, serde_json::json!({"a": 1, "b": 2}));
        assert!(output.transformations.contains(&"sort_keys".to_string()));
    }

    #[test]
    fn test_normalized_output_from_bytes_invalid_json() {
        let input = b"{invalid json}";
        let result = NormalizedOutput::from_bytes(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalized_output_to_bytes() {
        let input = r#"{"a": 1}"#;
        let output = NormalizedOutput::from_bytes(input.as_bytes()).unwrap();
        let bytes = output.to_bytes().unwrap();
        assert_eq!(bytes, b"{\"a\":1}");
    }

    #[test]
    fn test_normalizer_new() {
        let normalizer = Normalizer::new();
        assert!(normalizer.config.sort_keys);
    }

    #[test]
    fn test_normalizer_normalize() {
        let normalizer = Normalizer::new();
        let input = r#"{"z": 1, "a": 2, "null": null}"#;
        let result = normalizer.normalize(input.as_bytes()).unwrap();
        // Keys should be sorted
        assert_eq!(result.data, serde_json::json!({"a": 2, "null": null, "z": 1}));
    }

    #[test]
    fn test_sort_keys_nested() {
        let input = serde_json::json!({"c": {"z": 1, "a": 2}, "b": 3});
        let sorted = NormalizedOutput::sort_keys(input);
        assert_eq!(sorted, serde_json::json!({"b": 3, "c": {"a": 2, "z": 1}}));
    }

    #[test]
    fn test_remove_nulls() {
        let input = serde_json::json!({"a": 1, "b": null, "c": {"d": null, "e": 2}});
        let cleaned = NormalizedOutput::remove_nulls(input);
        assert_eq!(cleaned, serde_json::json!({"a": 1, "c": {"e": 2}}));
    }

    #[test]
    fn test_normalization_error_display() {
        let err = NormalizationError::InvalidJson {
            reason: "unexpected token".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid JSON: unexpected token");
    }
}
