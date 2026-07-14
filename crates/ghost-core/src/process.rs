//! Safe external-command execution with bounded run time and captured output.

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use thiserror::Error;
use wait_timeout::ChildExt;

/// A side-effect-free description of an external command invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRequest {
    /// Executable name or path.
    pub program: PathBuf,
    /// Arguments passed directly to the executable without a shell.
    pub args: Vec<OsString>,
    /// Maximum time to wait before terminating the child process.
    pub timeout: Duration,
}

impl CommandRequest {
    /// Creates a request with no arguments and a caller-selected timeout.
    #[must_use]
    pub fn new(program: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            timeout,
        }
    }

    /// Replaces this request's argument list.
    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }
}

/// Captured result of an external command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    /// Numeric exit status, absent when the platform does not provide one.
    pub status: Option<i32>,
    /// Standard output decoded lossily as UTF-8.
    pub stdout: String,
    /// Standard error decoded lossily as UTF-8.
    pub stderr: String,
    /// Whether the child exceeded its deadline and was terminated.
    pub timed_out: bool,
}

impl CommandOutput {
    /// Returns whether the child exited successfully before its deadline.
    #[must_use]
    pub fn success(&self) -> bool {
        !self.timed_out && self.status == Some(0)
    }
}

/// Abstraction used to substitute deterministic command fixtures in tests.
pub trait CommandRunner {
    /// Executes one command request and captures its result.
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, ProcessError>;
}

/// Command runner backed by the host operating system.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, ProcessError> {
        let mut child = Command::new(&request.program)
            .args(&request.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| ProcessError::Spawn {
                program: request.program.clone(),
                source,
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or(ProcessError::MissingPipe("stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(ProcessError::MissingPipe("stderr"))?;
        let stdout_reader = thread::spawn(move || read_stream(stdout));
        let stderr_reader = thread::spawn(move || read_stream(stderr));

        let (status, timed_out) =
            match child
                .wait_timeout(request.timeout)
                .map_err(|source| ProcessError::Wait {
                    program: request.program.clone(),
                    source,
                })? {
                Some(status) => (status, false),
                None => {
                    child.kill().map_err(|source| ProcessError::Terminate {
                        program: request.program.clone(),
                        source,
                    })?;
                    let status = child.wait().map_err(|source| ProcessError::Wait {
                        program: request.program.clone(),
                        source,
                    })?;
                    (status, true)
                }
            };

        let stdout = join_reader(stdout_reader, "stdout")?;
        let stderr = join_reader(stderr_reader, "stderr")?;

        Ok(CommandOutput {
            status: status.code(),
            stdout: String::from_utf8_lossy(&stdout).trim().to_owned(),
            stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
            timed_out,
        })
    }
}

fn read_stream(mut stream: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &'static str,
) -> Result<Vec<u8>, ProcessError> {
    reader
        .join()
        .map_err(|_| ProcessError::ReaderPanicked(stream))?
        .map_err(|source| ProcessError::Read { stream, source })
}

/// Failure to start, supervise, or capture an external command.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// The executable could not be started.
    #[error("failed to start command {program:?}: {source}")]
    Spawn {
        /// Executable that failed to start.
        program: PathBuf,
        /// Operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// The process output pipe was unexpectedly unavailable.
    #[error("child process did not expose its {0} pipe")]
    MissingPipe(&'static str),
    /// Waiting for the child process failed.
    #[error("failed while waiting for command {program:?}: {source}")]
    Wait {
        /// Executable being supervised.
        program: PathBuf,
        /// Operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// A timed-out child could not be terminated.
    #[error("failed to terminate timed-out command {program:?}: {source}")]
    Terminate {
        /// Executable being terminated.
        program: PathBuf,
        /// Operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// Reading one of the output streams failed.
    #[error("failed to read child {stream}: {source}")]
    Read {
        /// Name of the failed stream.
        stream: &'static str,
        /// I/O error.
        #[source]
        source: std::io::Error,
    },
    /// An output-reader thread terminated unexpectedly.
    #[error("child {0} reader thread panicked")]
    ReaderPanicked(&'static str),
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::Instant;

    use super::*;

    fn fixture_request(test_name: &str, timeout: Duration) -> CommandRequest {
        CommandRequest::new(env::current_exe().unwrap(), timeout).with_args([
            "--exact",
            test_name,
            "--ignored",
            "--nocapture",
        ])
    }

    #[test]
    fn captures_successful_output() {
        let output = SystemCommandRunner
            .run(&fixture_request(
                "process::tests::fixture_success",
                Duration::from_secs(5),
            ))
            .unwrap();

        assert!(output.success());
        assert!(output.stdout.contains("fixture-stdout"));
        assert!(output.stderr.contains("fixture-stderr"));
    }

    #[test]
    fn reports_nonzero_exit_status() {
        let output = SystemCommandRunner
            .run(&fixture_request(
                "process::tests::fixture_failure",
                Duration::from_secs(5),
            ))
            .unwrap();

        assert!(!output.success());
        assert_eq!(output.status, Some(7));
        assert!(!output.timed_out);
    }

    #[test]
    fn terminates_timed_out_process() {
        let started = Instant::now();
        let output = SystemCommandRunner
            .run(&fixture_request(
                "process::tests::fixture_timeout",
                Duration::from_millis(100),
            ))
            .unwrap();

        assert!(output.timed_out);
        assert!(!output.success());
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn reports_missing_executable() {
        let error = SystemCommandRunner
            .run(&CommandRequest::new(
                "ghostdriver-command-that-does-not-exist",
                Duration::from_secs(1),
            ))
            .unwrap_err();

        assert!(matches!(error, ProcessError::Spawn { .. }));
    }

    #[test]
    #[ignore]
    fn fixture_success() {
        println!("fixture-stdout");
        eprintln!("fixture-stderr");
    }

    #[test]
    #[ignore]
    fn fixture_failure() {
        std::process::exit(7);
    }

    #[test]
    #[ignore]
    fn fixture_timeout() {
        thread::sleep(Duration::from_secs(5));
    }
}
