//! Version types for CATHEDRAL.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Semantic version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Create a new version
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse from string
    ///
    /// # Errors
    ///
    /// Returns error if format is invalid
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat(s.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| VersionError::InvalidComponent(parts[0].to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| VersionError::InvalidComponent(parts[1].to_string()))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| VersionError::InvalidComponent(parts[2].to_string()))?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// Get as array
    #[must_use]
    pub const fn as_array(&self) -> [u64; 3] {
        [self.major, self.minor, self.patch]
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Version-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionError {
    /// Invalid format
    InvalidFormat(String),
    /// Invalid component
    InvalidComponent(String),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(s) => write!(f, "Invalid version format: {}", s),
            Self::InvalidComponent(s) => write!(f, "Invalid version component: {}", s),
        }
    }
}

impl std::error::Error for VersionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_new() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(format!("{}", v), "1.2.3");
    }

    #[test]
    fn test_version_ord() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert_eq!(v1, Version::new(1, 2, 3));
    }

    #[test]
    fn test_version_parse_error() {
        let result = Version::parse("1.2");
        assert!(matches!(result, Err(VersionError::InvalidFormat(_))));

        let result = Version::parse("a.b.c");
        assert!(matches!(result, Err(VersionError::InvalidComponent(_))));
    }

    #[test]
    fn test_version_default() {
        let v = Version::default();
        assert_eq!(v, Version::new(0, 1, 0));
    }
}
