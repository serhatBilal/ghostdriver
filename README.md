# GhostDriver

GhostDriver is an open-source systems research project for controlled,
differential observation of GPU command-submission behavior. Its first phase,
GhostTrace, is building a deterministic experiment laboratory around minimal
CUDA Driver API workloads.

## Project status

Repository foundation, versioned data models, and the environment doctor are
currently accepted: Phases A-C of Milestone 0. The Phase D CUDA probe is
implemented and awaiting build and hardware verification on the native Linux
target. The existing capture code is preliminary and is not an accepted
implementation of Phase E.

Milestone 0 is passive. It does not capture or replay raw GPU command buffers,
modify MMIO, patch kernel modules, or claim that syscall traces are hardware
packets.

## Supported platform

The initial target is native Linux with an NVIDIA Turing-or-newer GPU. CUDA and
GPU access are not required to build or test the Phase A/B Rust workspace.
Windows, WSL, AMD GPUs, and Intel GPUs are not supported targets for Milestone
0.

## Workspace

```text
crates/ghostctl/        Command-line boundary
crates/ghost-core/      Versioned manifests and shared validation types
crates/ghost-analyze/   Analysis crate boundary; logic deferred to Phase F
schemas/                Draft 2020-12 JSON Schemas
docs/                   Architecture, safety, design, and roadmap
experiments/            Ignored generated output
```

## Build and test

Install the Rust toolchain declared in `rust-toolchain.toml`, then run:

```bash
cargo build --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The test suite includes serialization round trips, schema-version rejection,
and validation of serialized model fixtures against the checked-in schemas.

Inspect the current host in a human-readable or versioned JSON format:

```bash
cargo run -p ghostctl -- doctor
cargo run -p ghostctl -- doctor --json
```

The doctor never guesses that a host is supported. Missing tools and failed or
timed-out checks remain visible as warnings.

On a supported native Linux host with CUDA installed, build and run the Phase D
probe with:

```bash
cargo run -p ghostctl -- probe build
cargo run -p ghostctl -- probe run --threads 32
```

## First experiment

The first accepted experiment will compare 10 recorded runs at each of 32, 64,
128, and 256 CUDA threads, with two warmups and block size as the sole
independent variable. It must preserve environment metadata, failed runs,
binary hashes, and numerical verification results.

That workflow is intentionally unavailable until the CUDA probe and experiment
runner pass Phases D-E. See `docs/MILESTONE-0.md` and
`docs/EXPERIMENT-DESIGN.md` for the planned procedure.

## Safety

Milestone 0 is observation-only. `strace` can expose syscall behavior around a
CUDA process, but it does not expose the raw GPU command stream after userspace
mappings are established. Future replay or mutation work requires a dedicated
test machine and separate design review. See `docs/SAFETY.md`.

## Non-goals

- Replacing CUDA or producing a general GPU driver
- Raw command-buffer replay or mutation in Milestone 0
- Kernel-module patching, MMIO access, or firmware analysis
- Framework integration, graphical interfaces, or network services
- Claims about undocumented field semantics without repeated evidence

## License

GhostDriver is licensed under the MIT License. See `LICENSE`.
