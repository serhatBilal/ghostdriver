//! Stable schema-version primitives shared by all manifest types.

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// The only schema version understood by this release.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// A validated GhostDriver schema version.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// Returns the schema version emitted by this release.
    #[must_use]
    pub const fn current() -> Self {
        Self(CURRENT_SCHEMA_VERSION)
    }

    /// Returns the numeric representation of this version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl TryFrom<u32> for SchemaVersion {
    type Error = SchemaVersionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == CURRENT_SCHEMA_VERSION {
            Ok(Self(value))
        } else {
            Err(SchemaVersionError::Unsupported(value))
        }
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

/// Failure returned when a document uses an unknown schema version.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum SchemaVersionError {
    /// The supplied numeric version is not supported by this release.
    #[error("unsupported schema version {0}; expected {CURRENT_SCHEMA_VERSION}")]
    Unsupported(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct VersionedDocument {
        schema_version: SchemaVersion,
    }

    #[test]
    fn accepts_current_version() {
        assert_eq!(SchemaVersion::try_from(1), Ok(SchemaVersion::current()));
    }

    #[test]
    fn rejects_unsupported_version() {
        assert_eq!(
            SchemaVersion::try_from(2),
            Err(SchemaVersionError::Unsupported(2))
        );
    }

    #[test]
    fn rejects_unsupported_version_during_deserialization() {
        let error = serde_json::from_str::<SchemaVersion>("2").unwrap_err();
        assert!(error.to_string().contains("unsupported schema version 2"));
    }

    #[test]
    fn rejects_missing_version_field() {
        let error = serde_json::from_str::<VersionedDocument>("{}").unwrap_err();
        assert!(error.to_string().contains("missing field `schema_version`"));
    }

    #[test]
    fn deserializes_version_field() {
        let document: VersionedDocument = serde_json::from_str(r#"{"schema_version":1}"#).unwrap();
        assert_eq!(document.schema_version, SchemaVersion::current());
    }
}
