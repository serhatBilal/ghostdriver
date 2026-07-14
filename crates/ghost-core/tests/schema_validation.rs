use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use ghost_core::{
    ArtifactHashes, EnvironmentReport, ExperimentManifest, ExperimentStatus, ExperimentSummary,
    GitState, GpuInfo, GroupManifest, PrivilegeInfo, RunArtifacts, RunManifest, RunStatus,
    SchemaVersion, ToolVersions, VariableDefinition, VerificationStatus,
};
use serde::Serialize;
use serde_json::Value;

fn timestamp() -> DateTime<Utc> {
    "2026-07-14T18:00:00Z".parse().unwrap()
}

fn schema(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("schemas")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn assert_valid<T: Serialize>(schema_name: &str, value: &T) {
    let schema = schema(schema_name);
    let validator = jsonschema::validator_for(&schema).unwrap();
    let instance = serde_json::to_value(value).unwrap();
    let errors: Vec<_> = validator
        .iter_errors(&instance)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "schema validation failed: {errors:#?}");
}

#[test]
fn serialized_environment_matches_schema() {
    assert_valid(
        "environment.schema.json",
        &EnvironmentReport {
            schema_version: SchemaVersion::current(),
            captured_at: timestamp(),
            operating_system: "Linux".into(),
            kernel_version: Some("6.8.0".into()),
            distribution_release: Some("Ubuntu 24.04".into()),
            cpu_model: Some("Fixture CPU".into()),
            total_ram_bytes: Some(16_000_000_000),
            gpu: Some(GpuInfo {
                name: "NVIDIA GeForce GTX 1650".into(),
                pci_identifier: "00000000:01:00.0".into(),
                total_vram_bytes: 4_294_967_296,
                compute_capability: Some("7.5".into()),
            }),
            nvidia_driver_version: None,
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
            warnings: vec![],
        },
    );
}

#[test]
fn serialized_experiment_matches_schema() {
    assert_valid(
        "experiment.schema.json",
        &ExperimentManifest {
            schema_version: SchemaVersion::current(),
            id: "fixture-experiment".into(),
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
        },
    );
}

#[test]
fn serialized_group_matches_schema() {
    assert_valid(
        "group.schema.json",
        &GroupManifest {
            schema_version: SchemaVersion::current(),
            id: "threads-32".into(),
            experiment_id: "fixture-experiment".into(),
            variable_name: "threads".into(),
            variable_value: 32,
            repetitions: 10,
            status: ExperimentStatus::Running,
            runs: vec!["run-001/run.json".into()],
        },
    );
}

#[test]
fn serialized_run_matches_schema() {
    assert_valid(
        "run.schema.json",
        &RunManifest {
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
                nvidia_smi_before: None,
                nvidia_smi_after: None,
            },
        },
    );
}

#[test]
fn serialized_hashes_and_summary_match_schemas() {
    assert_valid(
        "hashes.schema.json",
        &ArtifactHashes {
            schema_version: SchemaVersion::current(),
            probe_sha256: "a".repeat(64),
            fatbin_sha256: "b".repeat(64),
        },
    );
    assert_valid(
        "summary.schema.json",
        &ExperimentSummary {
            schema_version: SchemaVersion::current(),
            total_runs: 40,
            successful_runs: 39,
            failed_runs: 1,
        },
    );
}

#[test]
fn schemas_reject_unknown_versions() {
    let documents = [
        (
            "environment.schema.json",
            serde_json::json!({
                "schema_version": 1,
                "captured_at": "2026-07-14T18:00:00Z",
                "operating_system": "Linux",
                "kernel_version": null,
                "distribution_release": null,
                "cpu_model": null,
                "total_ram_bytes": null,
            "gpu": null,
                "nvidia_driver_version": null,
                "nvidia_module": null,
                "tools": {
                    "nvcc": null,
                    "rustc": null,
                    "cargo": null,
                    "cmake": null,
                    "cxx": null,
                    "strace": null,
                    "lspci_available": false,
                    "modinfo_available": false
                },
                "secure_boot_state": null,
                "iommu_state": null,
                "privileges": {
                    "user": null,
                    "effective_user_id": null,
                    "is_root": false
                },
                "git": { "commit": null, "working_tree_clean": null },
                "supported_host": false,
                "warnings": []
            }),
        ),
        (
            "experiment.schema.json",
            serde_json::json!({
                "schema_version": 1,
                "id": "fixture",
                "name": "fixture",
                "description": "fixture experiment",
                "created_at": "2026-07-14T18:00:00Z",
                "status": "planned",
                "variable": { "name": "threads", "values": [32] },
                "repetitions": 10,
                "warmup_runs": 2,
                "groups": []
            }),
        ),
        (
            "group.schema.json",
            serde_json::json!({
                "schema_version": 1,
                "id": "threads-32",
                "experiment_id": "fixture",
                "variable_name": "threads",
                "variable_value": 32,
                "repetitions": 10,
                "status": "planned",
                "runs": []
            }),
        ),
        (
            "run.schema.json",
            serde_json::json!({
                "schema_version": 1,
                "id": "run-001",
                "group_id": "threads-32",
                "sequence": 1,
                "started_at": "2026-07-14T18:00:00Z",
                "completed_at": null,
                "status": "pending",
                "exit_code": null,
                "duration_ms": null,
                "verification": null,
                "artifacts": {
                    "stdout": "stdout.txt",
                    "stderr": "stderr.txt",
                    "strace_prefix": null,
                    "nvidia_smi_before": null,
                    "nvidia_smi_after": null
                }
            }),
        ),
    ];

    for (name, mut document) in documents {
        let schema = schema(name);
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(
            validator.is_valid(&document),
            "fixture for {name} must be valid"
        );
        document["schema_version"] = serde_json::json!(2);
        assert!(!validator.is_valid(&document), "{name} accepted version 2");
    }
}
