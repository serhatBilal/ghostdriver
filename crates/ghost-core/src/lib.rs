//! Shared, side-effect-free models for GhostDriver manifests.
//!
//! Hardware inspection and experiment execution belong to later phases. This
//! crate currently defines only the versioned records those phases will emit.

pub mod environment;
pub mod manifest;
pub mod schema;

pub use environment::{
    EnvironmentReport, GitState, GpuInfo, NvidiaModuleInfo, PrivilegeInfo, ToolVersions,
};
pub use manifest::{
    ExperimentManifest, ExperimentStatus, GroupManifest, RunArtifacts, RunManifest, RunStatus,
    VariableDefinition, VerificationStatus,
};
pub use schema::{CURRENT_SCHEMA_VERSION, SchemaVersion, SchemaVersionError};
