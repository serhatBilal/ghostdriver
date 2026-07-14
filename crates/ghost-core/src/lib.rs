//! Shared, side-effect-free models for GhostDriver manifests.
//!
//! Hardware inspection and experiment execution belong to later phases. This
//! crate currently defines only the versioned records those phases will emit.

pub mod config;
pub mod environment;
pub mod hashing;
pub mod manifest;
pub mod process;
pub mod schema;

pub use config::{CaptureConfig, ExperimentConfig, ProbeConfig, ValidationConfig, VariableConfig};
pub use environment::{
    EnvironmentReport, GitState, GpuInfo, NvidiaModuleInfo, PrivilegeInfo, ToolVersions,
};
pub use hashing::{HashingError, sha256_file};
pub use manifest::{
    ArtifactHashes, ExperimentManifest, ExperimentStatus, ExperimentSummary, GroupManifest,
    RunArtifacts, RunManifest, RunStatus, VariableDefinition, VerificationStatus,
};
pub use process::{
    CommandOutput, CommandRequest, CommandRunner, ProcessError, SystemCommandRunner,
};
pub use schema::{CURRENT_SCHEMA_VERSION, SchemaVersion, SchemaVersionError};
