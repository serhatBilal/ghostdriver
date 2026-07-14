use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use ghost_core::{CommandOutput, CommandRequest, CommandRunner, SystemCommandRunner};

const CONFIGURE_TIMEOUT: Duration = Duration::from_secs(60);
const BUILD_TIMEOUT: Duration = Duration::from_secs(600);
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn build() -> Result<()> {
    let root = repository_root()?;
    let source = root.join("probes/cuda-vector-add");
    let build = root.join("build/cuda-vector-add");

    execute(
        CommandRequest::new("cmake", CONFIGURE_TIMEOUT).with_args([
            OsString::from("-S"),
            source.into_os_string(),
            OsString::from("-B"),
            build.clone().into_os_string(),
        ]),
        "CMake configuration",
    )?;
    execute(
        CommandRequest::new("cmake", BUILD_TIMEOUT).with_args([
            OsString::from("--build"),
            build.clone().into_os_string(),
            OsString::from("--parallel"),
        ]),
        "probe build",
    )?;

    println!("probe={}", probe_path(&build).display());
    println!("fatbin={}", build.join("ghost_cuda_probe.fatbin").display());
    Ok(())
}

pub(crate) fn run(threads: u32) -> Result<()> {
    let root = repository_root()?;
    let build = root.join("build/cuda-vector-add");
    let probe = probe_path(&build);
    let fatbin = build.join("ghost_cuda_probe.fatbin");
    if !probe.is_file() {
        bail!(
            "probe executable not found at {}; run `ghostctl probe build` first",
            probe.display()
        );
    }
    if !fatbin.is_file() {
        bail!(
            "probe fatbin not found at {}; run `ghostctl probe build` first",
            fatbin.display()
        );
    }

    execute(
        CommandRequest::new(probe, RUN_TIMEOUT)
            .with_args([OsString::from("--threads"), threads.to_string().into()]),
        "CUDA probe",
    )?;
    Ok(())
}

fn execute(request: CommandRequest, operation: &str) -> Result<CommandOutput> {
    let output = SystemCommandRunner
        .run(&request)
        .with_context(|| format!("{operation} could not start"))?;
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
    if output.timed_out {
        bail!(
            "{operation} timed out after {} seconds",
            request.timeout.as_secs()
        );
    }
    if !output.success() {
        bail!("{operation} failed with exit status {:?}", output.status);
    }
    Ok(output)
}

pub(crate) fn repository_root() -> Result<PathBuf> {
    let current = env::current_dir().context("failed to read current directory")?;
    find_repository_root(&current).with_context(|| {
        format!(
            "could not find GhostDriver repository root from {}; run this command inside the repository",
            current.display()
        )
    })
}

fn find_repository_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|path| path.join("probes/cuda-vector-add/CMakeLists.txt").is_file())
        .map(Path::to_path_buf)
}

fn probe_path(build: &Path) -> PathBuf {
    build.join(format!("ghost-cuda-probe{}", env::consts::EXE_SUFFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_repository_from_nested_directory() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let nested = root.join("crates/ghostctl/src");
        assert_eq!(find_repository_root(&nested), Some(root));
    }

    #[test]
    fn rejects_directory_outside_repository() {
        let temporary = env::temp_dir();
        assert_eq!(find_repository_root(&temporary), None);
    }
}
