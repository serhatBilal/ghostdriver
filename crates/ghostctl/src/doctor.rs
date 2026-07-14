use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use ghost_core::{
    CommandOutput, CommandRequest, CommandRunner, EnvironmentReport, GitState, GpuInfo,
    NvidiaModuleInfo, PrivilegeInfo, SchemaVersion, SystemCommandRunner, ToolVersions,
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn run(json: bool) -> Result<()> {
    let report = collect(&SystemCommandRunner, HostSnapshot::read());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("failed to encode doctor report")?
        );
    } else {
        print_human(&report);
    }
    Ok(())
}

fn collect(runner: &dyn CommandRunner, host: HostSnapshot) -> EnvironmentReport {
    let mut warnings = Vec::new();
    let is_linux = host.operating_system == "linux";
    if !is_linux {
        warnings.push("GhostDriver Milestone 0 requires native Linux.".into());
    }

    let kernel_version = if is_linux {
        command_text(runner, "Linux kernel", "uname", &["-r"], &mut warnings)
    } else {
        None
    };
    let distribution_release = parse_os_release(host.os_release.as_deref());
    let cpu_model = parse_cpu_model(host.cpu_info.as_deref());
    let total_ram_bytes = parse_total_ram(host.mem_info.as_deref());

    if is_linux && distribution_release.is_none() {
        warnings.push("Distribution release could not be read from /etc/os-release.".into());
    }
    if is_linux && cpu_model.is_none() {
        warnings.push("CPU model could not be read from /proc/cpuinfo.".into());
    }
    if is_linux && total_ram_bytes.is_none() {
        warnings.push("Total RAM could not be read from /proc/meminfo.".into());
    }

    let gpu_output = command_text(
        runner,
        "NVIDIA GPU",
        "nvidia-smi",
        &[
            "--query-gpu=name,pci.bus_id,memory.total,driver_version,compute_cap",
            "--format=csv,noheader,nounits",
        ],
        &mut warnings,
    );
    let (gpu, nvidia_driver_version) = match gpu_output.as_deref().and_then(parse_gpu) {
        Some((gpu, driver)) => (Some(gpu), Some(driver)),
        None => {
            if gpu_output.is_some() {
                warnings.push("NVIDIA GPU metadata had an unexpected format.".into());
            }
            (None, None)
        }
    };

    let nvcc = command_text(
        runner,
        "CUDA compiler",
        "nvcc",
        &["--version"],
        &mut warnings,
    );
    let rustc = command_text(
        runner,
        "Rust compiler",
        "rustc",
        &["--version"],
        &mut warnings,
    );
    let cargo = command_text(runner, "Cargo", "cargo", &["--version"], &mut warnings);
    let cmake = command_text(runner, "CMake", "cmake", &["--version"], &mut warnings);
    let cxx = command_text(runner, "C++ compiler", "c++", &["--version"], &mut warnings);
    let strace = command_text(runner, "strace", "strace", &["--version"], &mut warnings);
    let lspci_available =
        command_text(runner, "lspci", "lspci", &["--version"], &mut warnings).is_some();
    let modinfo_available =
        command_text(runner, "modinfo", "modinfo", &["--version"], &mut warnings).is_some();

    let nvidia_module = collect_module(runner, modinfo_available, &host, &mut warnings);
    let secure_boot_state = command_text(
        runner,
        "Secure Boot state",
        "mokutil",
        &["--sb-state"],
        &mut warnings,
    );
    let iommu_state = host.iommu_groups_present.map(|present| {
        if present {
            "enabled (IOMMU groups detected)".into()
        } else {
            "no IOMMU groups detected".into()
        }
    });

    let effective_user_id = if is_linux {
        command_text(runner, "effective user ID", "id", &["-u"], &mut warnings)
            .and_then(|value| value.lines().next()?.trim().parse().ok())
    } else {
        None
    };
    let privileges = PrivilegeInfo {
        user: host.user,
        effective_user_id,
        is_root: effective_user_id == Some(0),
    };

    let commit = command_text(
        runner,
        "Git commit",
        "git",
        &["rev-parse", "HEAD"],
        &mut warnings,
    );
    let working_tree_clean = command_output(runner, "git", &["status", "--porcelain"])
        .filter(CommandOutput::success)
        .map(|output| output.stdout.is_empty());
    if working_tree_clean.is_none() {
        warnings.push("Git working-tree state could not be determined.".into());
    }

    let tools = ToolVersions {
        nvcc,
        rustc,
        cargo,
        cmake,
        cxx,
        strace,
        lspci_available,
        modinfo_available,
    };
    let gpu_supported = gpu
        .as_ref()
        .and_then(|value| value.compute_capability.as_deref())
        .and_then(parse_compute_capability)
        .is_some_and(|capability| capability >= (7, 5));
    if gpu.is_some() && !gpu_supported {
        warnings
            .push("GPU compute capability could not be confirmed as Turing (7.5) or newer.".into());
    }

    let required_tools_available = tools.nvcc.is_some()
        && tools.rustc.is_some()
        && tools.cargo.is_some()
        && tools.cmake.is_some()
        && tools.cxx.is_some()
        && tools.strace.is_some()
        && tools.lspci_available
        && tools.modinfo_available;
    let supported_host = is_linux
        && gpu.is_some()
        && gpu_supported
        && nvidia_driver_version.is_some()
        && nvidia_module.is_some()
        && required_tools_available;

    EnvironmentReport {
        schema_version: SchemaVersion::current(),
        captured_at: Utc::now(),
        operating_system: host.operating_system,
        kernel_version,
        distribution_release,
        cpu_model,
        total_ram_bytes,
        gpu,
        nvidia_driver_version,
        nvidia_module,
        tools,
        secure_boot_state,
        iommu_state,
        privileges,
        git: GitState {
            commit,
            working_tree_clean,
        },
        supported_host,
        warnings,
    }
}

fn command_output(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
) -> Option<CommandOutput> {
    runner
        .run(&CommandRequest::new(program, COMMAND_TIMEOUT).with_args(args.iter().copied()))
        .ok()
}

fn command_text(
    runner: &dyn CommandRunner,
    label: &str,
    program: &str,
    args: &[&str],
    warnings: &mut Vec<String>,
) -> Option<String> {
    let request = CommandRequest::new(program, COMMAND_TIMEOUT).with_args(args.iter().copied());
    match runner.run(&request) {
        Ok(output) if output.success() => match output_text(&output) {
            Some(text) => Some(text),
            None => {
                warnings.push(format!("{label} check succeeded but produced no output."));
                None
            }
        },
        Ok(output) if output.timed_out => {
            warnings.push(format!("{label} check timed out after 5 seconds."));
            None
        }
        Ok(output) => {
            let detail = output_text(&output)
                .and_then(|text| text.lines().next().map(str::to_owned))
                .unwrap_or_else(|| "no diagnostic output".into());
            warnings.push(format!(
                "{label} check failed with status {:?}: {detail}",
                output.status
            ));
            None
        }
        Err(error) => {
            warnings.push(format!("{label} check could not start: {error}"));
            None
        }
    }
}

fn output_text(output: &CommandOutput) -> Option<String> {
    if !output.stdout.is_empty() {
        Some(output.stdout.clone())
    } else if !output.stderr.is_empty() {
        Some(output.stderr.clone())
    } else {
        None
    }
}

fn collect_module(
    runner: &dyn CommandRunner,
    modinfo_available: bool,
    host: &HostSnapshot,
    warnings: &mut Vec<String>,
) -> Option<NvidiaModuleInfo> {
    if !modinfo_available {
        warnings.push("NVIDIA kernel module metadata could not be recorded.".into());
        return None;
    }
    let filename = command_text(
        runner,
        "NVIDIA module filename",
        "modinfo",
        &["-F", "filename", "nvidia"],
        warnings,
    )?;
    let version = command_text(
        runner,
        "NVIDIA module version",
        "modinfo",
        &["-F", "version", "nvidia"],
        warnings,
    )?;
    let license = command_text(
        runner,
        "NVIDIA module license",
        "modinfo",
        &["-F", "license", "nvidia"],
        warnings,
    )?;
    let open_module_confirmed = host
        .nvidia_proc_version
        .as_deref()
        .is_some_and(|text| text.to_ascii_lowercase().contains("open kernel module"));
    if !open_module_confirmed {
        warnings
            .push("Could not positively identify the NVIDIA open kernel module flavour.".into());
    }
    Some(NvidiaModuleInfo {
        filename,
        version,
        license,
        open_module_confirmed,
    })
}

fn parse_os_release(contents: Option<&str>) -> Option<String> {
    contents?
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|value| value.trim_matches('"').to_owned())
}

fn parse_cpu_model(contents: Option<&str>) -> Option<String> {
    contents?.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().to_owned())
    })
}

fn parse_total_ram(contents: Option<&str>) -> Option<u64> {
    let line = contents?
        .lines()
        .find(|line| line.starts_with("MemTotal:"))?;
    let kibibytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    kibibytes.checked_mul(1024)
}

fn parse_gpu(line: &str) -> Option<(GpuInfo, String)> {
    let fields: Vec<_> = line.lines().next()?.split(',').map(str::trim).collect();
    let [
        name,
        pci_identifier,
        memory_mib,
        driver_version,
        compute_capability,
    ] = fields.as_slice()
    else {
        return None;
    };
    let total_vram_bytes = memory_mib.parse::<u64>().ok()?.checked_mul(1024 * 1024)?;
    Some((
        GpuInfo {
            name: (*name).to_owned(),
            pci_identifier: (*pci_identifier).to_owned(),
            total_vram_bytes,
            compute_capability: Some((*compute_capability).to_owned()),
        },
        (*driver_version).to_owned(),
    ))
}

fn parse_compute_capability(value: &str) -> Option<(u32, u32)> {
    let (major, minor) = value.trim().split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn print_human(report: &EnvironmentReport) {
    println!("GhostDriver doctor");
    println!("schema_version={}", report.schema_version.get());
    println!("supported_host={}", report.supported_host);
    println!("operating_system={}", report.operating_system);
    println!(
        "kernel_version={}",
        display(report.kernel_version.as_deref())
    );
    println!(
        "distribution_release={}",
        display(report.distribution_release.as_deref())
    );
    println!("cpu_model={}", display(report.cpu_model.as_deref()));
    println!(
        "total_ram_bytes={}",
        report
            .total_ram_bytes
            .map_or_else(|| "unknown".into(), |v| v.to_string())
    );
    println!(
        "gpu_name={}",
        display(report.gpu.as_ref().map(|gpu| gpu.name.as_str()))
    );
    println!(
        "gpu_pci_identifier={}",
        display(report.gpu.as_ref().map(|gpu| gpu.pci_identifier.as_str()))
    );
    println!(
        "gpu_total_vram_bytes={}",
        report
            .gpu
            .as_ref()
            .map_or_else(|| "unknown".into(), |gpu| gpu.total_vram_bytes.to_string())
    );
    println!(
        "gpu_compute_capability={}",
        display(
            report
                .gpu
                .as_ref()
                .and_then(|gpu| gpu.compute_capability.as_deref())
        )
    );
    println!(
        "nvidia_driver_version={}",
        display(report.nvidia_driver_version.as_deref())
    );
    println!(
        "nvidia_module_filename={}",
        display(
            report
                .nvidia_module
                .as_ref()
                .map(|module| module.filename.as_str())
        )
    );
    println!(
        "nvidia_module_version={}",
        display(
            report
                .nvidia_module
                .as_ref()
                .map(|module| module.version.as_str())
        )
    );
    println!(
        "nvidia_module_license={}",
        display(
            report
                .nvidia_module
                .as_ref()
                .map(|module| module.license.as_str())
        )
    );
    println!(
        "open_kernel_module_confirmed={}",
        report
            .nvidia_module
            .as_ref()
            .is_some_and(|module| module.open_module_confirmed)
    );
    println!("nvcc={}", display(report.tools.nvcc.as_deref()));
    println!("rustc={}", display(report.tools.rustc.as_deref()));
    println!("cargo={}", display(report.tools.cargo.as_deref()));
    println!("cmake={}", display(report.tools.cmake.as_deref()));
    println!("cxx={}", display(report.tools.cxx.as_deref()));
    println!("strace={}", display(report.tools.strace.as_deref()));
    println!("lspci_available={}", report.tools.lspci_available);
    println!("modinfo_available={}", report.tools.modinfo_available);
    println!(
        "secure_boot_state={}",
        display(report.secure_boot_state.as_deref())
    );
    println!("iommu_state={}", display(report.iommu_state.as_deref()));
    println!("user={}", display(report.privileges.user.as_deref()));
    println!("is_root={}", report.privileges.is_root);
    println!("git_commit={}", display(report.git.commit.as_deref()));
    println!(
        "working_tree_clean={}",
        report
            .git
            .working_tree_clean
            .map_or_else(|| "unknown".into(), |value| value.to_string())
    );
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
}

fn display(value: Option<&str>) -> &str {
    value.unwrap_or("unknown")
}

#[derive(Clone, Debug)]
struct HostSnapshot {
    operating_system: String,
    os_release: Option<String>,
    cpu_info: Option<String>,
    mem_info: Option<String>,
    nvidia_proc_version: Option<String>,
    iommu_groups_present: Option<bool>,
    user: Option<String>,
}

impl HostSnapshot {
    fn read() -> Self {
        let iommu_path = Path::new("/sys/kernel/iommu_groups");
        let iommu_groups_present = fs::read_dir(iommu_path)
            .ok()
            .map(|mut entries| entries.next().is_some());
        Self {
            operating_system: env::consts::OS.into(),
            os_release: fs::read_to_string("/etc/os-release").ok(),
            cpu_info: fs::read_to_string("/proc/cpuinfo").ok(),
            mem_info: fs::read_to_string("/proc/meminfo").ok(),
            nvidia_proc_version: fs::read_to_string("/proc/driver/nvidia/version").ok(),
            iommu_groups_present,
            user: env::var("USER").or_else(|_| env::var("USERNAME")).ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io;

    use ghost_core::ProcessError;

    use super::*;

    struct FixtureRunner {
        failures: BTreeMap<String, CommandOutput>,
    }

    impl FixtureRunner {
        fn supported() -> Self {
            Self {
                failures: BTreeMap::new(),
            }
        }

        fn with_failure(mut self, program: &str, output: CommandOutput) -> Self {
            self.failures.insert(program.into(), output);
            self
        }
    }

    impl CommandRunner for FixtureRunner {
        fn run(&self, request: &CommandRequest) -> Result<CommandOutput, ProcessError> {
            let program = request.program.to_string_lossy();
            if let Some(output) = self.failures.get(program.as_ref()) {
                return Ok(output.clone());
            }
            let args: Vec<_> = request
                .args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect();
            let stdout = match program.as_ref() {
                "uname" => "6.8.0-fixture".into(),
                "nvidia-smi" => {
                    "NVIDIA GeForce GTX 1650, 00000000:01:00.0, 4096, 555.42, 7.5".into()
                }
                "modinfo" if args.first().is_some_and(|arg| arg == "--version") => {
                    "kmod version 31".into()
                }
                "modinfo" if args.get(1).is_some_and(|arg| arg == "filename") => {
                    "/lib/modules/nvidia.ko".into()
                }
                "modinfo" if args.get(1).is_some_and(|arg| arg == "version") => "555.42".into(),
                "modinfo" if args.get(1).is_some_and(|arg| arg == "license") => {
                    "Dual MIT/GPL".into()
                }
                "id" => "1000".into(),
                "git" if args.first().is_some_and(|arg| arg == "rev-parse") => {
                    "0123456789abcdef".into()
                }
                "git" => String::new(),
                "mokutil" => "SecureBoot disabled".into(),
                _ => format!("{program} fixture version"),
            };
            Ok(CommandOutput {
                status: Some(0),
                stdout,
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn linux_host() -> HostSnapshot {
        HostSnapshot {
            operating_system: "linux".into(),
            os_release: Some("PRETTY_NAME=\"Fixture Linux\"\n".into()),
            cpu_info: Some("model name : Fixture CPU\n".into()),
            mem_info: Some("MemTotal:       16384 kB\n".into()),
            nvidia_proc_version: Some("NVIDIA UNIX Open Kernel Module 555.42".into()),
            iommu_groups_present: Some(true),
            user: Some("researcher".into()),
        }
    }

    #[test]
    fn supported_fixture_produces_complete_report() {
        let report = collect(&FixtureRunner::supported(), linux_host());

        assert!(report.supported_host);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(report.kernel_version.as_deref(), Some("6.8.0-fixture"));
        assert_eq!(report.cpu_model.as_deref(), Some("Fixture CPU"));
        assert_eq!(report.total_ram_bytes, Some(16_777_216));
        assert_eq!(
            report
                .gpu
                .as_ref()
                .and_then(|gpu| gpu.compute_capability.as_deref()),
            Some("7.5")
        );
        assert_eq!(report.git.working_tree_clean, Some(true));
    }

    #[test]
    fn failed_required_tool_is_visible_and_unsupported() {
        let runner = FixtureRunner::supported().with_failure(
            "strace",
            CommandOutput {
                status: Some(1),
                stdout: String::new(),
                stderr: "fixture failure".into(),
                timed_out: false,
            },
        );
        let report = collect(&runner, linux_host());

        assert!(!report.supported_host);
        assert!(report.tools.strace.is_none());
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("strace check failed"))
        );
    }

    #[test]
    fn timeout_is_reported_without_fabricating_data() {
        let runner = FixtureRunner::supported().with_failure(
            "nvidia-smi",
            CommandOutput {
                status: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            },
        );
        let report = collect(&runner, linux_host());

        assert!(!report.supported_host);
        assert!(report.gpu.is_none());
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("NVIDIA GPU check timed out"))
        );
    }

    #[test]
    fn command_start_error_is_actionable() {
        struct MissingRunner;
        impl CommandRunner for MissingRunner {
            fn run(&self, request: &CommandRequest) -> Result<CommandOutput, ProcessError> {
                Err(ProcessError::Spawn {
                    program: request.program.clone(),
                    source: io::Error::new(io::ErrorKind::NotFound, "fixture missing"),
                })
            }
        }

        let mut warnings = Vec::new();
        let value = command_text(
            &MissingRunner,
            "fixture tool",
            "missing",
            &[],
            &mut warnings,
        );
        assert!(value.is_none());
        assert!(warnings[0].contains("fixture tool check could not start"));
        assert!(warnings[0].contains("fixture missing"));
    }

    #[test]
    fn parsers_reject_malformed_input() {
        assert_eq!(parse_total_ram(Some("MemTotal: nope kB")), None);
        assert_eq!(parse_gpu("missing,fields"), None);
        assert_eq!(parse_compute_capability("unknown"), None);
    }
}
