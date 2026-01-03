//! Policy matcher for pattern matching.

use cathedral_core::{CoreResult, Capability};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Match context for policy evaluation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchContext {
    /// Variables available for matching
    pub vars: HashMap<String, String>,
    /// Requested capability
    pub capability: Option<Capability>,
}

impl MatchContext {
    /// Create a new match context
    #[must_use]
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            capability: None,
        }
    }

    /// Set a variable
    #[must_use]
    pub fn with_var(mut self, key: String, value: String) -> Self {
        self.vars.insert(key, value);
        self
    }

    /// Set the capability
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capability = Some(capability);
        self
    }
}

impl Default for MatchContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Match result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchResult {
    /// Whether the pattern matched
    pub matched: bool,
    /// Captured variables
    pub captures: HashMap<String, String>,
}

impl MatchResult {
    /// Create a new match result
    #[must_use]
    pub fn new(matched: bool) -> Self {
        Self {
            matched,
            captures: HashMap::new(),
        }
    }

    /// Add a capture
    #[must_use]
    pub fn with_capture(mut self, key: String, value: String) -> Self {
        self.captures.insert(key, value);
        self
    }

    /// Check if matched
    #[must_use]
    pub fn is_matched(&self) -> bool {
        self.matched
    }
}

/// Pattern matcher
pub struct Matcher;

impl Matcher {
    /// Create a new matcher
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Match a pattern against context
    ///
    /// # Errors
    ///
    /// Returns error if matching fails
    pub fn match_pattern(&self, pattern: &str, ctx: &MatchContext) -> CoreResult<MatchResult> {
        // Simple pattern matching
        // In a real implementation, this would support regex or glob patterns

        if pattern == "*" {
            return Ok(MatchResult::new(true));
        }

        // Check if pattern matches a variable
        if pattern.starts_with('$') {
            let var_name = &pattern[1..];
            if let Some(value) = ctx.vars.get(var_name) {
                return Ok(MatchResult::new(true).with_capture(var_name.to_string(), value.clone()));
            }
            return Ok(MatchResult::new(false));
        }

        // Check if pattern matches capability
        if let Some(cap) = &ctx.capability {
            let cap_str = cap.to_string();
            if cap_str.contains(pattern) {
                return Ok(MatchResult::new(true));
            }
        }

        // Exact string match
        for value in ctx.vars.values() {
            if value.contains(pattern) {
                return Ok(MatchResult::new(true));
            }
        }

        Ok(MatchResult::new(false))
    }

    /// Match multiple patterns (all must match)
    ///
    /// # Errors
    ///
    /// Returns error if matching fails
    pub fn match_all(&self, patterns: &[&str], ctx: &MatchContext) -> CoreResult<MatchResult> {
        let mut all_captures = HashMap::new();

        for pattern in patterns {
            let result = self.match_pattern(pattern, ctx)?;
            if !result.matched {
                return Ok(MatchResult::new(false));
            }
            all_captures.extend(result.captures);
        }

        Ok(MatchResult::new(true).with_captures(all_captures))
    }

    /// Match multiple patterns (any can match)
    ///
    /// # Errors
    ///
    /// Returns error if matching fails
    pub fn match_any(&self, patterns: &[&str], ctx: &MatchContext) -> CoreResult<MatchResult> {
        for pattern in patterns {
            let result = self.match_pattern(pattern, ctx)?;
            if result.matched {
                return Ok(result);
            }
        }

        Ok(MatchResult::new(false))
    }
}

impl MatchResult {
    fn with_captures(mut self, captures: HashMap<String, String>) -> Self {
        self.captures.extend(captures);
        self
    }
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matcher_new() {
        let matcher = Matcher::new();
        let ctx = MatchContext::new();
        let result = matcher.match_pattern("*", &ctx).unwrap();
        assert!(result.matched);
    }

    #[test]
    fn test_match_context_new() {
        let ctx = MatchContext::new();
        assert!(ctx.vars.is_empty());
    }

    #[test]
    fn test_match_context_with_var() {
        let ctx = MatchContext::new().with_var("key".to_string(), "value".to_string());
        assert_eq!(ctx.vars.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_match_result_new() {
        let result = MatchResult::new(true);
        assert!(result.matched);
    }

    #[test]
    fn test_match_result_with_capture() {
        let result = MatchResult::new(true).with_capture("key".to_string(), "value".to_string());
        assert_eq!(result.captures.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_match_pattern_wildcard() {
        let matcher = Matcher::new();
        let ctx = MatchContext::new();
        let result = matcher.match_pattern("*", &ctx).unwrap();
        assert!(result.matched);
    }

    #[test]
    fn test_match_pattern_variable() {
        let matcher = Matcher::new();
        let ctx = MatchContext::new().with_var("test".to_string(), "value".to_string());
        let result = matcher.match_pattern("$test", &ctx).unwrap();
        assert!(result.matched);
    }

    #[test]
    fn test_match_all() {
        let matcher = Matcher::new();
        let ctx = MatchContext::new();
        let result = matcher.match_all(&["*", "*"], &ctx).unwrap();
        assert!(result.matched);
    }

    #[test]
    fn test_match_any() {
        let matcher = Matcher::new();
        let ctx = MatchContext::new();
        let result = matcher.match_any(&["not-match", "*"], &ctx).unwrap();
        assert!(result.matched);
    }
}
