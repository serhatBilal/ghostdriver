//! Versioned environment-report models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SchemaVersion;

/// Metadata collected about a GPU visible to the host.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuInfo {
    /// Driver-reported device name.
    pub name: String,
    /// PCI identifier, including domain and function when available.
    pub pci_identifier: String,
    /// Total device memory in bytes.
    pub total_vram_bytes: u64,
}

/// Metadata reported by the loaded NVIDIA kernel module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NvidiaModuleInfo {
    /// Filesystem path of the loaded module when detectable.
    pub filename: String,
    /// Module version string.
    pub version: String,
    /// Module license string.
    pub license: String,
    /// Whether the open module flavour was positively identified.
    pub open_module_confirmed: bool,
}

/// Versions of build and observation tools required by Milestone 0.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolVersions {
    /// CUDA compiler version, if available.
    pub nvcc: Option<String>,
    /// Rust compiler version, if available.
    pub rustc: Option<String>,
    /// Cargo version, if available.
    pub cargo: Option<String>,
    /// CMake version, if available.
    pub cmake: Option<String>,
    /// C++ compiler version, if available.
    pub cxx: Option<String>,
    /// strace version, if available.
    pub strace: Option<String>,
    /// Whether `lspci` is available.
    pub lspci_available: bool,
    /// Whether `modinfo` is available.
    pub modinfo_available: bool,
}

/// Privilege information for the process collecting the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeInfo {
    /// Effective user name when available.
    pub user: Option<String>,
    /// Effective numeric user ID on Unix hosts.
    pub effective_user_id: Option<u32>,
    /// Whether the collector has root privileges.
    pub is_root: bool,
}

/// Repository revision information captured with an environment report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitState {
    /// Current commit hash when the repository has a commit.
    pub commit: Option<String>,
    /// Whether tracked files were clean when inspected.
    pub working_tree_clean: Option<bool>,
}

/// Immutable host metadata associated with an experiment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentReport {
    /// Format version for this document.
    pub schema_version: SchemaVersion,
    /// UTC time at which collection began.
    pub captured_at: DateTime<Utc>,
    /// Operating-system name.
    pub operating_system: String,
    /// Linux kernel release, if available.
    pub kernel_version: Option<String>,
    /// Distribution release description, if available.
    pub distribution_release: Option<String>,
    /// CPU model, if available.
    pub cpu_model: Option<String>,
    /// Total system memory in bytes, if available.
    pub total_ram_bytes: Option<u64>,
    /// Visible NVIDIA GPU metadata, if available.
    pub gpu: Option<GpuInfo>,
    /// NVIDIA userspace driver version, if available.
    pub nvidia_driver_version: Option<String>,
    /// Loaded NVIDIA module metadata, if available.
    pub nvidia_module: Option<NvidiaModuleInfo>,
    /// Build and observation tool versions.
    pub tools: ToolVersions,
    /// Secure Boot state when detectable.
    pub secure_boot_state: Option<String>,
    /// IOMMU state when detectable.
    pub iommu_state: Option<String>,
    /// Privileges held by the collector.
    pub privileges: PrivilegeInfo,
    /// Repository revision information.
    pub git: GitState,
    /// Whether all Milestone 0 support requirements were positively met.
    pub supported_host: bool,
    /// Non-fatal missing data and support concerns.
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_report_round_trips() {
        let report = EnvironmentReport {
            schema_version: SchemaVersion::current(),
            captured_at: "2026-07-14T18:00:00Z".parse().unwrap(),
            operating_system: "Linux".into(),
            kernel_version: Some("6.8.0".into()),
            distribution_release: Some("Ubuntu 24.04".into()),
            cpu_model: Some("Fixture CPU".into()),
            total_ram_bytes: Some(16 * 1024 * 1024 * 1024),
            gpu: Some(GpuInfo {
                name: "NVIDIA GeForce GTX 1650".into(),
                pci_identifier: "0000:01:00.0".into(),
                total_vram_bytes: 4 * 1024 * 1024 * 1024,
            }),
            nvidia_driver_version: Some("fixture-driver".into()),
            nvidia_module: None,
            tools: ToolVersions::default(),
            secure_boot_state: None,
            iommu_state: None,
            privileges: PrivilegeInfo {
                user: Some("researcher".into()),
                effective_user_id: Some(1000),
                is_root: false,
            },
            git: GitState {
                commit: Some("0123456789abcdef".into()),
                working_tree_clean: Some(true),
            },
            supported_host: false,
            warnings: vec!["fixture warning".into()],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(
            serde_json::from_str::<EnvironmentReport>(&json).unwrap(),
            report
        );
    }
}
