//! Data redaction for sensitive information.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Redaction rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionRule {
    /// Rule name
    pub name: String,
    /// Pattern to match for redaction
    pub pattern: String,
    /// Replacement string
    pub replacement: String,
    /// Whether to use regex matching
    pub is_regex: bool,
}

impl RedactionRule {
    /// Create a new redaction rule
    #[must_use]
    pub fn new(name: String, pattern: String, replacement: String) -> Self {
        Self {
            name,
            pattern,
            replacement,
            is_regex: false,
        }
    }

    /// Create a regex rule
    #[must_use]
    pub fn regex(name: String, pattern: String, replacement: String) -> Self {
        Self {
            name,
            pattern,
            replacement,
            is_regex: true,
        }
    }

    /// Apply rule to text
    #[must_use]
    pub fn apply(&self, text: &str) -> String {
        if self.is_regex {
            // For simplicity, just do string replacement
            // A real implementation would use regex
            text.replace(&self.pattern, &self.replacement)
        } else {
            text.replace(&self.pattern, &self.replacement)
        }
    }
}

/// Redacted view of data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedView {
    /// Original data (may be empty for security)
    pub original: String,
    /// Redacted data
    pub redacted: String,
    /// Number of redactions applied
    pub redaction_count: usize,
    /// Rules that were applied
    pub applied_rules: Vec<String>,
}

impl RedactedView {
    /// Create a new redacted view
    #[must_use]
    pub fn new(original: String, redacted: String) -> Self {
        Self {
            original,
            redacted,
            redaction_count: 0,
            applied_rules: Vec::new(),
        }
    }

    /// Get the redacted content
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.redacted
    }

    /// Get the redacted content as bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.redacted.as_bytes()
    }

    /// Check if any redactions were applied
    #[must_use]
    pub fn is_redacted(&self) -> bool {
        self.redaction_count > 0
    }
}

/// Redactor for applying redaction rules
pub struct Redactor {
    /// Redaction rules
    rules: Vec<RedactionRule>,
    /// Fields to always redact
    sensitive_fields: HashSet<String>,
}

impl Redactor {
    /// Create a new redactor
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            sensitive_fields: HashSet::new(),
        }
    }

    /// Add a redaction rule
    #[must_use]
    pub fn with_rule(mut self, rule: RedactionRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add a sensitive field name
    #[must_use]
    pub fn with_sensitive_field(mut self, field: String) -> Self {
        self.sensitive_fields.insert(field);
        self
    }

    /// Redact a string value
    #[must_use]
    pub fn redact(&self, value: &str) -> RedactedView {
        let mut redacted = value.to_string();
        let mut redaction_count = 0;
        let mut applied_rules = Vec::new();

        for rule in &self.rules {
            let before = redacted.len();
            redacted = rule.apply(&redacted);
            let after = redacted.len();

            if before != after || redacted.contains(&rule.replacement) {
                redaction_count += 1;
                applied_rules.push(rule.name.clone());
            }
        }

        RedactedView {
            original: value.to_string(),
            redacted,
            redaction_count,
            applied_rules,
        }
    }

    /// Redact a specific field
    #[must_use]
    pub fn redact_field(&self, field_name: &str, value: &str) -> RedactedView {
        // Always redact sensitive fields
        if self.sensitive_fields.contains(field_name) {
            return RedactedView::new(value.to_string(), "***REDACTED***".to_string());
        }

        self.redact(value)
    }

    /// Check if a field is sensitive
    #[must_use]
    pub fn is_sensitive(&self, field_name: &str) -> bool {
        self.sensitive_fields.contains(field_name)
            || field_name.contains("password")
            || field_name.contains("secret")
            || field_name.contains("token")
            || field_name.contains("key")
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction_rule_new() {
        let rule = RedactionRule::new("test".to_string(), "secret".to_string(), "***".to_string());
        assert_eq!(rule.name, "test");
        assert!(!rule.is_regex);
    }

    #[test]
    fn test_redaction_rule_apply() {
        let rule = RedactionRule::new("test".to_string(), "secret".to_string(), "***".to_string());
        let result = rule.apply("this is a secret message");
        assert_eq!(result, "this is a *** message");
    }

    #[test]
    fn test_redacted_view_new() {
        let view = RedactedView::new("original".to_string(), "redacted".to_string());
        assert_eq!(view.redacted, "redacted");
        assert!(!view.is_redacted());
    }

    #[test]
    fn test_redacted_view_is_redacted() {
        let mut view = RedactedView::new("original".to_string(), "redacted".to_string());
        assert!(!view.is_redacted());

        view.redaction_count = 1;
        assert!(view.is_redacted());
    }

    #[test]
    fn test_redacted_view_as_str() {
        let view = RedactedView::new("original".to_string(), "redacted".to_string());
        assert_eq!(view.as_str(), "redacted");
    }

    #[test]
    fn test_redactor_new() {
        let redactor = Redactor::new();
        let view = redactor.redact("test");
        assert_eq!(view.redacted, "test");
    }

    #[test]
    fn test_redactor_with_rule() {
        let rule = RedactionRule::new("test".to_string(), "secret".to_string(), "***".to_string());
        let redactor = Redactor::new().with_rule(rule);
        let view = redactor.redact("this is a secret");
        assert_eq!(view.redacted, "this is a ***");
    }

    #[test]
    fn test_redactor_redact_field() {
        let redactor = Redactor::new()
            .with_sensitive_field("password".to_string());

        let view = redactor.redact_field("username", "value");
        assert_eq!(view.redacted, "value");

        let view = redactor.redact_field("password", "secret123");
        assert_eq!(view.redacted, "***REDACTED***");
    }

    #[test]
    fn test_redactor_is_sensitive() {
        let redactor = Redactor::new();
        assert!(redactor.is_sensitive("password"));
        assert!(redactor.is_sensitive("api_secret"));
        assert!(redactor.is_sensitive("auth_token"));
        assert!(redactor.is_sensitive("encryption_key"));
    }

    #[test]
    fn test_redactor_with_sensitive_field() {
        let redactor = Redactor::new().with_sensitive_field("custom".to_string());
        assert!(redactor.is_sensitive("custom"));
    }
}
