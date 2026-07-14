//! Versioned Milestone 0 experiment configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SchemaVersion;

/// Complete configuration for one controlled experiment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentConfig {
    /// Configuration schema version.
    pub schema_version: SchemaVersion,
    /// Directory-safe experiment name.
    pub name: String,
    /// Human-readable experiment purpose.
    pub description: String,
    /// Number of recorded runs per variable value.
    pub repetitions: u32,
    /// Number of unrecorded warmups per variable value.
    pub warmup_runs: u32,
    /// Per-process deadline in seconds.
    pub timeout_seconds: u64,
    /// Probe executable configuration.
    pub probe: ProbeConfig,
    /// Sole independent variable configuration.
    pub variable: VariableConfig,
    /// Passive capture controls.
    pub capture: CaptureConfig,
    /// Required postconditions.
    pub validation: ValidationConfig,
}

/// Probe executable and fixed workload properties.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeConfig {
    /// Repository-relative executable path.
    pub path: PathBuf,
    /// Fixed vector element count.
    pub element_count: u32,
}

/// The experiment's only independent variable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableConfig {
    /// Probe argument name without leading dashes.
    pub name: String,
    /// Ordered group values.
    pub values: Vec<u32>,
}

/// Passive artifacts captured around each probe run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureConfig {
    /// Capture the process with strace.
    pub strace: bool,
    /// Record `nvidia-smi -q` before each run.
    pub nvidia_smi_before: bool,
    /// Record `nvidia-smi -q` after each run.
    pub nvidia_smi_after: bool,
    /// Reserved passive kernel-log excerpt control.
    pub kernel_log_excerpt: bool,
}

/// Conditions required for an experiment to be considered valid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationConfig {
    /// Require a clean repository before capture.
    pub require_clean_git: bool,
    /// Require an unchanged probe hash across groups.
    pub require_same_binary_hash: bool,
    /// Require an unchanged fatbin hash across groups.
    pub require_same_fatbin_hash: bool,
    /// Require every recorded probe run to verify numerically.
    pub require_verification_passed: bool,
}

impl ExperimentConfig {
    /// Checks invariants that cannot be represented by TOML types alone.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || !self
                .name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
        {
            return Err("name must contain only ASCII letters, digits, '-' or '_'".into());
        }
        if self.description.trim().is_empty() {
            return Err("description must not be empty".into());
        }
        if self.repetitions == 0 {
            return Err("repetitions must be at least 1".into());
        }
        if self.timeout_seconds == 0 {
            return Err("timeout_seconds must be at least 1".into());
        }
        if self.probe.element_count != 4096 {
            return Err("Milestone 0 requires probe.element_count = 4096".into());
        }
        if self.variable.name != "threads" {
            return Err("Milestone 0 requires variable.name = 'threads'".into());
        }
        if self.variable.values.is_empty()
            || self
                .variable
                .values
                .iter()
                .any(|value| ![32, 64, 128, 256].contains(value))
        {
            return Err("variable.values may only contain 32, 64, 128, and 256".into());
        }
        let mut unique = self.variable.values.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != self.variable.values.len() {
            return Err("variable.values must be unique".into());
        }
        if self.capture.kernel_log_excerpt {
            return Err("kernel_log_excerpt is not implemented in Milestone 0".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> ExperimentConfig {
        toml::from_str(include_str!("../../../configs/milestone-0.toml")).unwrap()
    }

    #[test]
    fn milestone_config_is_valid() {
        assert_eq!(valid_config().validate(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_or_unknown_values() {
        let mut config = valid_config();
        config.variable.values = vec![32, 32];
        assert!(config.validate().unwrap_err().contains("unique"));
        config.variable.values = vec![33];
        assert!(config.validate().unwrap_err().contains("may only contain"));
    }
}
