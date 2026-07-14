//! Versioned experiment, group, and run manifests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SchemaVersion;

/// Lifecycle state of an experiment or group.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    /// The record has been created but execution has not started.
    Planned,
    /// One or more runs are in progress.
    Running,
    /// All required runs completed successfully.
    Completed,
    /// At least one required operation failed.
    Failed,
}

/// The single independent variable used by an experiment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableDefinition {
    /// Command-line and manifest name of the variable.
    pub name: String,
    /// Ordered values assigned to experiment groups.
    pub values: Vec<u32>,
}

/// Top-level immutable description of an experiment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentManifest {
    /// Format version for this document.
    pub schema_version: SchemaVersion,
    /// Unique directory-safe experiment identifier.
    pub id: String,
    /// Human-readable experiment name.
    pub name: String,
    /// Purpose and controlled comparison performed by the experiment.
    pub description: String,
    /// UTC creation time.
    pub created_at: DateTime<Utc>,
    /// Current lifecycle state.
    pub status: ExperimentStatus,
    /// The experiment's sole independent variable.
    pub variable: VariableDefinition,
    /// Number of recorded runs required for each group.
    pub repetitions: u32,
    /// Number of unrecorded warmup runs for each group.
    pub warmup_runs: u32,
    /// Relative paths to each standalone group manifest.
    pub groups: Vec<String>,
}

/// Manifest for one value of an experiment's independent variable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupManifest {
    /// Format version for this document.
    pub schema_version: SchemaVersion,
    /// Unique group identifier.
    pub id: String,
    /// Parent experiment identifier.
    pub experiment_id: String,
    /// Independent-variable name.
    pub variable_name: String,
    /// Independent-variable value assigned to this group.
    pub variable_value: u32,
    /// Number of recorded runs required for this group.
    pub repetitions: u32,
    /// Current lifecycle state.
    pub status: ExperimentStatus,
    /// Relative paths to preserved run manifests.
    pub runs: Vec<String>,
}

/// Lifecycle state of a single recorded run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run record exists but execution has not started.
    Pending,
    /// The child process is running.
    Running,
    /// Execution and required verification succeeded.
    Completed,
    /// Execution or required verification failed.
    Failed,
    /// The configured execution deadline elapsed.
    TimedOut,
}

/// Numerical verification result reported by a probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Probe output passed numerical verification.
    Passed,
    /// Probe output failed numerical verification.
    Failed,
    /// No verification result could be obtained.
    Unavailable,
}

/// Relative paths to immutable artifacts captured for one run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunArtifacts {
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// strace output prefix when tracing was enabled.
    pub strace_prefix: Option<String>,
    /// Pre-run `nvidia-smi` snapshot when enabled.
    pub nvidia_smi_before: Option<String>,
    /// Post-run `nvidia-smi` snapshot when enabled.
    pub nvidia_smi_after: Option<String>,
}

/// Immutable metadata for one attempted probe execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    /// Format version for this document.
    pub schema_version: SchemaVersion,
    /// Unique run identifier.
    pub id: String,
    /// Parent group identifier.
    pub group_id: String,
    /// One-based run number within the group.
    pub sequence: u32,
    /// UTC process start time.
    pub started_at: DateTime<Utc>,
    /// UTC completion time, absent while a run is active.
    pub completed_at: Option<DateTime<Utc>>,
    /// Current lifecycle state.
    pub status: RunStatus,
    /// Child exit code when the process exited normally.
    pub exit_code: Option<i32>,
    /// Wall-clock process duration in milliseconds when known.
    pub duration_ms: Option<u64>,
    /// Probe numerical-verification result when known.
    pub verification: Option<VerificationStatus>,
    /// Relative paths to artifacts preserved for this run.
    pub artifacts: RunArtifacts,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp() -> DateTime<Utc> {
        "2026-07-14T18:00:00Z".parse().unwrap()
    }

    fn round_trip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).unwrap();
        assert_eq!(&serde_json::from_str::<T>(&json).unwrap(), value);
    }

    #[test]
    fn experiment_manifest_round_trips() {
        round_trip(&ExperimentManifest {
            schema_version: SchemaVersion::current(),
            id: "2026-07-14T180000Z_cuda-block-size-baseline".into(),
            name: "cuda-block-size-baseline".into(),
            description: "Block size is the only independent variable.".into(),
            created_at: timestamp(),
            status: ExperimentStatus::Planned,
            variable: VariableDefinition {
                name: "threads".into(),
                values: vec![32, 64, 128, 256],
            },
            repetitions: 10,
            warmup_runs: 2,
            groups: vec!["groups/threads-32/group.json".into()],
        });
    }

    #[test]
    fn group_manifest_round_trips() {
        round_trip(&GroupManifest {
            schema_version: SchemaVersion::current(),
            id: "threads-32".into(),
            experiment_id: "fixture-experiment".into(),
            variable_name: "threads".into(),
            variable_value: 32,
            repetitions: 10,
            status: ExperimentStatus::Running,
            runs: vec!["run-001/run.json".into()],
        });
    }

    #[test]
    fn run_manifest_round_trips() {
        round_trip(&RunManifest {
            schema_version: SchemaVersion::current(),
            id: "run-001".into(),
            group_id: "threads-32".into(),
            sequence: 1,
            started_at: timestamp(),
            completed_at: Some(timestamp()),
            status: RunStatus::Completed,
            exit_code: Some(0),
            duration_ms: Some(42),
            verification: Some(VerificationStatus::Passed),
            artifacts: RunArtifacts {
                stdout: "stdout.txt".into(),
                stderr: "stderr.txt".into(),
                strace_prefix: Some("strace".into()),
                nvidia_smi_before: Some("nvidia-smi-before.txt".into()),
                nvidia_smi_after: Some("nvidia-smi-after.txt".into()),
            },
        });
    }
}
