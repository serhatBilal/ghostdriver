use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ghostctl")]
#[command(about = "GhostDriver environment and experiment utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect whether the host is suitable for GhostDriver Milestone 0.
    Doctor {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct CommandResult {
    available: bool,
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    supported_host: bool,
    warnings: Vec<String>,
    checks: BTreeMap<String, CommandResult>,
}

fn run(program: &str, args: &[&str]) -> CommandResult {
    match Command::new(program).args(args).output() {
        Ok(output) => CommandResult {
            available: true,
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(error) => CommandResult {
            available: false,
            status: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor { json } => doctor(json),
    }
}

fn doctor(json: bool) -> Result<()> {
    let mut checks = BTreeMap::new();

    checks.insert("uname".into(), run("uname", &["-a"]));
    checks.insert(
        "linux_release".into(),
        run("sh", &["-c", "cat /etc/os-release"]),
    );
    checks.insert(
        "gpu".into(),
        run(
            "nvidia-smi",
            &[
                "--query-gpu=name,driver_version,pci.device_id,memory.total",
                "--format=csv,noheader",
            ],
        ),
    );
    checks.insert(
        "nvidia_module_version".into(),
        run("sh", &["-c", "cat /proc/driver/nvidia/version"]),
    );
    checks.insert(
        "nvidia_module_license".into(),
        run("modinfo", &["-F", "license", "nvidia"]),
    );
    checks.insert(
        "nvidia_module_filename".into(),
        run("modinfo", &["-F", "filename", "nvidia"]),
    );
    checks.insert(
        "pci_driver".into(),
        run("sh", &["-c", "lspci -nnk | grep -A3 -i 'VGA\\|3D'"]),
    );
    checks.insert("nvcc".into(), run("nvcc", &["--version"]));
    checks.insert("rustc".into(), run("rustc", &["--version"]));
    checks.insert("cargo".into(), run("cargo", &["--version"]));
    checks.insert("cmake".into(), run("cmake", &["--version"]));
    checks.insert("strace".into(), run("strace", &["--version"]));

    let mut warnings = Vec::new();

    if !cfg!(target_os = "linux") {
        warnings.push("GhostDriver Milestone 0 requires native Linux.".into());
    }

    for required in ["gpu", "nvcc", "rustc", "cargo", "cmake", "strace"] {
        if !checks.get(required).map(|c| c.available).unwrap_or(false) {
            warnings.push(format!("Required command/check is unavailable: {required}"));
        }
    }

    let module_text = checks
        .get("nvidia_module_version")
        .map(|c| c.stdout.to_lowercase())
        .unwrap_or_default();
    let license_text = checks
        .get("nvidia_module_license")
        .map(|c| c.stdout.to_lowercase())
        .unwrap_or_default();

    if !module_text.contains("open kernel module")
        && !license_text.contains("mit")
        && !license_text.contains("gpl")
    {
        warnings.push(
            "Could not positively identify the NVIDIA open kernel module flavour. \
             Verify the installed module manually before instrumentation."
                .into(),
        );
    }

    let supported_host = cfg!(target_os = "linux") && warnings.is_empty();
    let report = DoctorReport {
        supported_host,
        warnings,
        checks,
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("failed to encode report")?
        );
    } else {
        println!("GhostDriver doctor");
        println!("supported_host={}", report.supported_host);
        for (name, result) in &report.checks {
            let summary = if result.available {
                result.stdout.lines().next().unwrap_or("<no output>")
            } else {
                "<missing>"
            };
            println!("{name}: {summary}");
        }
        for warning in &report.warnings {
            eprintln!("warning: {warning}");
        }
    }

    Ok(())
}
