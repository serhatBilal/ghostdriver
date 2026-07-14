mod doctor;
mod probe;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        /// Emit a versioned machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// Build or run the deterministic CUDA Driver API probe.
    Probe {
        #[command(subcommand)]
        command: ProbeCommand,
    },
}

#[derive(Subcommand)]
enum ProbeCommand {
    /// Configure and build the probe and its SM 7.5 fatbin with CMake.
    Build,
    /// Run the probe once without trace capture.
    Run {
        /// CUDA block size; this is the probe's only variable parameter.
        #[arg(long, default_value_t = 32, value_parser = parse_thread_count)]
        threads: u32,
    },
}

fn parse_thread_count(value: &str) -> Result<u32, String> {
    let threads = value
        .parse::<u32>()
        .map_err(|_| "threads must be an integer".to_owned())?;
    if [32, 64, 128, 256].contains(&threads) {
        Ok(threads)
    } else {
        Err("threads must be one of 32, 64, 128, or 256".into())
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Doctor { json } => doctor::run(json),
        Commands::Probe { command } => match command {
            ProbeCommand::Build => probe::build(),
            ProbeCommand::Run { threads } => probe::run(threads),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse_thread_count;

    #[test]
    fn accepts_supported_thread_counts() {
        for threads in [32, 64, 128, 256] {
            assert_eq!(parse_thread_count(&threads.to_string()), Ok(threads));
        }
    }

    #[test]
    fn rejects_malformed_or_unsupported_thread_counts() {
        for value in ["not-a-number", "0", "33", "1024"] {
            assert!(parse_thread_count(value).is_err(), "accepted {value}");
        }
    }
}
