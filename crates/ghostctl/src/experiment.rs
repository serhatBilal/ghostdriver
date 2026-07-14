use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use ghost_core::{
    ArtifactHashes, CommandOutput, CommandRequest, CommandRunner, ExperimentConfig,
    ExperimentManifest, ExperimentStatus, ExperimentSummary, GroupManifest, RunArtifacts,
    RunManifest, RunStatus, SchemaVersion, SystemCommandRunner, VariableDefinition,
    VerificationStatus, sha256_file,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{doctor, probe};

pub(crate) fn capture(config_path: &Path) -> Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("experiment capture requires native Linux; Windows and WSL are unsupported");
    }
    let root = probe::repository_root()?;
    let config_path = root.join(config_path);
    let config_text = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let config: ExperimentConfig = toml::from_str(&config_text)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    config.validate().map_err(anyhow::Error::msg)?;

    let environment = doctor::collect_system();
    if !environment.supported_host {
        bail!("doctor did not confirm this host as supported; run `ghostctl doctor`");
    }
    if config.validation.require_clean_git && environment.git.working_tree_clean != Some(true) {
        bail!("configuration requires a clean Git working tree");
    }

    let executable = root.join(&config.probe.path);
    let fatbin = executable
        .parent()
        .context("probe path has no parent directory")?
        .join("ghost_cuda_probe.fatbin");
    let hashes = ArtifactHashes {
        schema_version: SchemaVersion::current(),
        probe_sha256: sha256_file(&executable)?,
        fatbin_sha256: sha256_file(&fatbin)?,
    };

    let id = format!("{}_{}", Utc::now().format("%Y-%m-%dT%H%M%SZ"), config.name);
    let experiment_root = root.join("experiments").join(&id);
    fs::create_dir(&experiment_root).with_context(|| {
        format!(
            "failed to create unique experiment directory {}",
            experiment_root.display()
        )
    })?;
    fs::create_dir(experiment_root.join("groups"))?;
    fs::write(experiment_root.join("config.toml"), config_text)?;
    write_json(&experiment_root.join("environment.json"), &environment)?;
    write_json(&experiment_root.join("hashes.json"), &hashes)?;

    let runner = SystemCommandRunner;
    let mut group_paths = Vec::new();
    let mut successful_runs = 0_u32;
    let mut failed_runs = 0_u32;
    for value in &config.variable.values {
        for _ in 0..config.warmup_runs {
            let output = run_probe(&runner, &executable, *value, config.timeout_seconds, None);
            if !run_passed(&output) {
                bail!("warmup failed for threads={value}; partial experiment preserved");
            }
        }

        let group_id = format!("{}-{value}", config.variable.name);
        let group_root = experiment_root.join("groups").join(&group_id);
        fs::create_dir(&group_root)?;
        let mut run_paths = Vec::new();
        let mut group_failed = false;
        for sequence in 1..=config.repetitions {
            let run_id = format!("run-{sequence:03}");
            let run_root = group_root.join(&run_id);
            fs::create_dir(&run_root)?;
            let before = capture_nvidia_smi(
                &runner,
                config.capture.nvidia_smi_before,
                &run_root.join("nvidia-smi-before.txt"),
            )?;
            let started_at = Utc::now();
            let started = std::time::Instant::now();
            let trace_prefix = config.capture.strace.then(|| run_root.join("strace"));
            let output = run_probe(
                &runner,
                &executable,
                *value,
                config.timeout_seconds,
                trace_prefix.as_deref(),
            );
            let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            fs::write(run_root.join("stdout.txt"), &output.stdout)?;
            fs::write(run_root.join("stderr.txt"), &output.stderr)?;
            let after = capture_nvidia_smi(
                &runner,
                config.capture.nvidia_smi_after,
                &run_root.join("nvidia-smi-after.txt"),
            )?;
            let passed = run_passed(&output);
            if passed {
                successful_runs += 1;
            } else {
                failed_runs += 1;
                group_failed = true;
            }
            let manifest = RunManifest {
                schema_version: SchemaVersion::current(),
                id: run_id.clone(),
                group_id: group_id.clone(),
                sequence,
                started_at,
                completed_at: Some(Utc::now()),
                status: if output.timed_out {
                    RunStatus::TimedOut
                } else if passed {
                    RunStatus::Completed
                } else {
                    RunStatus::Failed
                },
                exit_code: output.status,
                duration_ms: Some(duration_ms),
                verification: Some(
                    if output
                        .stdout
                        .lines()
                        .any(|line| line == "verification=passed")
                    {
                        VerificationStatus::Passed
                    } else if output
                        .stdout
                        .lines()
                        .any(|line| line == "verification=failed")
                    {
                        VerificationStatus::Failed
                    } else {
                        VerificationStatus::Unavailable
                    },
                ),
                artifacts: RunArtifacts {
                    stdout: "stdout.txt".into(),
                    stderr: "stderr.txt".into(),
                    strace_prefix: trace_prefix.map(|_| "strace".into()),
                    nvidia_smi_before: before.then(|| "nvidia-smi-before.txt".into()),
                    nvidia_smi_after: after.then(|| "nvidia-smi-after.txt".into()),
                },
            };
            write_json(&run_root.join("run.json"), &manifest)?;
            run_paths.push(format!("{run_id}/run.json"));
        }
        let group = GroupManifest {
            schema_version: SchemaVersion::current(),
            id: group_id.clone(),
            experiment_id: id.clone(),
            variable_name: config.variable.name.clone(),
            variable_value: *value,
            repetitions: config.repetitions,
            status: if group_failed {
                ExperimentStatus::Failed
            } else {
                ExperimentStatus::Completed
            },
            runs: run_paths,
        };
        write_json(&group_root.join("group.json"), &group)?;
        group_paths.push(format!("groups/{group_id}/group.json"));
    }

    let summary = ExperimentSummary {
        schema_version: SchemaVersion::current(),
        total_runs: successful_runs + failed_runs,
        successful_runs,
        failed_runs,
    };
    write_json(&experiment_root.join("summary.json"), &summary)?;
    let experiment = ExperimentManifest {
        schema_version: SchemaVersion::current(),
        id: id.clone(),
        name: config.name,
        description: config.description,
        created_at: environment.captured_at,
        status: if failed_runs == 0 {
            ExperimentStatus::Completed
        } else {
            ExperimentStatus::Failed
        },
        variable: VariableDefinition {
            name: config.variable.name,
            values: config.variable.values,
        },
        repetitions: config.repetitions,
        warmup_runs: config.warmup_runs,
        groups: group_paths,
    };
    write_json(&experiment_root.join("experiment.json"), &experiment)?;
    validate(&experiment_root)?;
    println!("experiment={}", experiment_root.display());
    if failed_runs > 0 && config.validation.require_verification_passed {
        bail!("{failed_runs} recorded runs failed; experiment was preserved");
    }
    Ok(())
}

pub(crate) fn list() -> Result<()> {
    let root = probe::repository_root()?.join("experiments");
    let mut entries: Vec<_> = fs::read_dir(&root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        println!("{}", entry.file_name().to_string_lossy());
    }
    Ok(())
}

pub(crate) fn validate(path: &Path) -> Result<()> {
    let experiment: ExperimentManifest = read_json(&path.join("experiment.json"))?;
    let _: ghost_core::EnvironmentReport = read_json(&path.join("environment.json"))?;
    let _: ArtifactHashes = read_json(&path.join("hashes.json"))?;
    let summary: ExperimentSummary = read_json(&path.join("summary.json"))?;
    let config: ExperimentConfig = toml::from_str(&fs::read_to_string(path.join("config.toml"))?)?;
    config.validate().map_err(anyhow::Error::msg)?;
    let mut run_count = 0_u32;
    for group_path in &experiment.groups {
        let group_file = path.join(group_path);
        let group: GroupManifest = read_json(&group_file)?;
        let group_root = group_file
            .parent()
            .context("group manifest has no parent")?;
        for run_path in &group.runs {
            let run_file = group_root.join(run_path);
            let run: RunManifest = read_json(&run_file)?;
            let run_root = run_file.parent().context("run manifest has no parent")?;
            for artifact in [&run.artifacts.stdout, &run.artifacts.stderr] {
                if !run_root.join(artifact).is_file() {
                    bail!("missing run artifact {}", run_root.join(artifact).display());
                }
            }
            for artifact in [
                run.artifacts.nvidia_smi_before.as_deref(),
                run.artifacts.nvidia_smi_after.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                if !run_root.join(artifact).is_file() {
                    bail!("missing run artifact {}", run_root.join(artifact).display());
                }
            }
            if let Some(prefix) = &run.artifacts.strace_prefix {
                let prefix = format!("{prefix}.");
                let found = fs::read_dir(run_root)?
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name().to_string_lossy().starts_with(&prefix));
                if !found {
                    bail!("missing strace artifacts in {}", run_root.display());
                }
            }
            run_count += 1;
        }
    }
    if run_count != summary.total_runs {
        bail!("summary run count does not match preserved run manifests");
    }
    println!("valid_experiment={}", path.display());
    Ok(())
}

fn run_probe(
    runner: &dyn CommandRunner,
    executable: &Path,
    threads: u32,
    timeout_seconds: u64,
    trace_prefix: Option<&Path>,
) -> CommandOutput {
    let mut request = if let Some(prefix) = trace_prefix {
        CommandRequest::new("strace", Duration::from_secs(timeout_seconds)).with_args([
            "-ff".into(),
            "-ttt".into(),
            "-yy".into(),
            "-s".into(),
            "256".into(),
            "-o".into(),
            prefix.as_os_str().to_owned(),
            executable.as_os_str().to_owned(),
            "--threads".into(),
            threads.to_string().into(),
        ])
    } else {
        CommandRequest::new(executable, Duration::from_secs(timeout_seconds)).with_args([
            OsString::from("--threads"),
            OsString::from(threads.to_string()),
        ])
    };
    request.timeout = Duration::from_secs(timeout_seconds);
    runner.run(&request).unwrap_or_else(|error| CommandOutput {
        status: None,
        stdout: String::new(),
        stderr: format!("failed to execute probe: {error}"),
        timed_out: false,
    })
}

fn capture_nvidia_smi(runner: &dyn CommandRunner, enabled: bool, path: &Path) -> Result<bool> {
    if !enabled {
        return Ok(false);
    }
    let text = match runner
        .run(&CommandRequest::new("nvidia-smi", Duration::from_secs(10)).with_args(["-q"]))
    {
        Ok(output) => format!("{}{}", output.stdout, output.stderr),
        Err(error) => format!("error=failed to execute nvidia-smi: {error}\n"),
    };
    fs::write(path, text)?;
    Ok(true)
}

fn run_passed(output: &CommandOutput) -> bool {
    output.success()
        && output
            .stdout
            .lines()
            .any(|line| line == "verification=passed")
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("invalid {}", path.display()))
}

#[cfg(test)]
mod tests {
    use ghost_core::{EnvironmentReport, GitState, PrivilegeInfo, ToolVersions};

    use super::*;

    #[test]
    fn validates_complete_fixture_directory() {
        let root = std::env::temp_dir().join(format!(
            "ghostdriver-experiment-fixture-{}",
            std::process::id()
        ));
        let run_root = root.join("groups/threads-32/run-001");
        fs::create_dir_all(&run_root).unwrap();
        fs::write(
            root.join("config.toml"),
            include_str!("../../../configs/milestone-0.toml"),
        )
        .unwrap();
        fs::write(run_root.join("stdout.txt"), "verification=passed\n").unwrap();
        fs::write(run_root.join("stderr.txt"), "").unwrap();

        let run = RunManifest {
            schema_version: SchemaVersion::current(),
            id: "run-001".into(),
            group_id: "threads-32".into(),
            sequence: 1,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            status: RunStatus::Completed,
            exit_code: Some(0),
            duration_ms: Some(1),
            verification: Some(VerificationStatus::Passed),
            artifacts: RunArtifacts {
                stdout: "stdout.txt".into(),
                stderr: "stderr.txt".into(),
                strace_prefix: None,
                nvidia_smi_before: None,
                nvidia_smi_after: None,
            },
        };
        write_json(&run_root.join("run.json"), &run).unwrap();
        write_json(
            &root.join("groups/threads-32/group.json"),
            &GroupManifest {
                schema_version: SchemaVersion::current(),
                id: "threads-32".into(),
                experiment_id: "fixture".into(),
                variable_name: "threads".into(),
                variable_value: 32,
                repetitions: 1,
                status: ExperimentStatus::Completed,
                runs: vec!["run-001/run.json".into()],
            },
        )
        .unwrap();
        write_json(
            &root.join("experiment.json"),
            &ExperimentManifest {
                schema_version: SchemaVersion::current(),
                id: "fixture".into(),
                name: "fixture".into(),
                description: "fixture experiment".into(),
                created_at: Utc::now(),
                status: ExperimentStatus::Completed,
                variable: VariableDefinition {
                    name: "threads".into(),
                    values: vec![32],
                },
                repetitions: 1,
                warmup_runs: 0,
                groups: vec!["groups/threads-32/group.json".into()],
            },
        )
        .unwrap();
        write_json(
            &root.join("environment.json"),
            &EnvironmentReport {
                schema_version: SchemaVersion::current(),
                captured_at: Utc::now(),
                operating_system: "linux".into(),
                kernel_version: None,
                distribution_release: None,
                cpu_model: None,
                total_ram_bytes: None,
                gpu: None,
                nvidia_driver_version: None,
                nvidia_module: None,
                tools: ToolVersions::default(),
                secure_boot_state: None,
                iommu_state: None,
                privileges: PrivilegeInfo {
                    user: None,
                    effective_user_id: None,
                    is_root: false,
                },
                git: GitState {
                    commit: None,
                    working_tree_clean: None,
                },
                supported_host: false,
                warnings: vec![],
            },
        )
        .unwrap();
        write_json(
            &root.join("hashes.json"),
            &ArtifactHashes {
                schema_version: SchemaVersion::current(),
                probe_sha256: "a".repeat(64),
                fatbin_sha256: "b".repeat(64),
            },
        )
        .unwrap();
        write_json(
            &root.join("summary.json"),
            &ExperimentSummary {
                schema_version: SchemaVersion::current(),
                total_runs: 1,
                successful_runs: 1,
                failed_runs: 0,
            },
        )
        .unwrap();

        validate(&root).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
