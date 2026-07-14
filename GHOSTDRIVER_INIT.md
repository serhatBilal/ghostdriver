# GhostDriver — Project Initialization

## 1. Project Identity

**Project name:** GhostDriver  
**Initial module name:** GhostTrace  
**Repository type:** Open-source systems research project  
**Primary language:** Rust  
**Secondary languages:** C, C++, CUDA C++, Bash  
**Initial platform:** Native Linux  
**Initial GPU target:** NVIDIA GTX 1650 / Turing / SM 7.5  
**License:** MIT unless changed later

GhostDriver is a long-term research project for observing, analysing, and eventually understanding GPU command submission behaviour through controlled differential experiments.

The first development phase is called **GhostTrace**.

GhostTrace must not attempt to replace CUDA, generate a production-ready GPU driver, replay arbitrary command buffers, or reverse-engineer the entire NVIDIA stack.

The first objective is much narrower:

> Build a deterministic GPU experiment laboratory that can run minimal CUDA Driver API workloads, capture the surrounding userspace and kernel-visible behaviour, record immutable experiment metadata, and compare repeated runs statistically.

The first milestone must be safe, reproducible, and entirely passive.

---

## 2. Core Research Question

The long-term research question is:

> Can an undocumented GPU submission protocol be partially inferred through repeated controlled experiments and differential command-stream analysis?

The initial engineering question is:

> Can we build a reproducible observation pipeline where only one CUDA launch parameter changes between experiment groups and every other relevant variable is recorded or controlled?

---

## 3. Non-Goals for the Initial Phase

Do not implement any of the following in Milestone 0:

- Raw GPU command-buffer replay
- Doorbell modification
- GPU MMIO writes
- NVIDIA kernel module patching
- Custom NVIDIA driver
- CUDA replacement runtime
- Arbitrary kernel interception
- Kernel-space memory mutation
- GSP firmware analysis
- Production-ready GPU scheduling
- Support for AMD or Intel GPUs
- Support for Windows or WSL
- PyTorch, TensorFlow, llama.cpp, or other large frameworks
- A graphical user interface
- Network services
- Cloud deployment

Milestone 0 is a passive observation and experiment-control system only.

---

## 4. Safety Rules

GhostDriver is a low-level systems project and later phases may cause GPU hangs or full system freezes.

For Milestone 0:

1. Never write to GPU command buffers.
2. Never replay captured submissions.
3. Never patch a running NVIDIA kernel module.
4. Never access GPU MMIO directly.
5. Never assume `strace` exposes the raw GPU command stream.
6. Never claim that a captured ioctl sequence is the hardware protocol.
7. Every experiment must record driver, kernel, CUDA, executable, and fatbin versions.
8. Only one independent variable may change between experiment groups.
9. Every experiment must verify numerical correctness.
10. Any unsafe feature must be hidden behind an explicit compile-time feature flag in future milestones.

The project must prefer correctness and reproducibility over speed.

---

## 5. Initial Repository Structure

Create the repository with the following structure:

```text
ghostdriver/
├── Cargo.toml
├── README.md
├── LICENSE
├── .gitignore
├── rust-toolchain.toml
├── crates/
│   ├── ghostctl/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── doctor.rs
│   │       ├── experiment.rs
│   │       ├── manifest.rs
│   │       └── util.rs
│   ├── ghost-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── environment.rs
│   │       ├── hashing.rs
│   │       ├── process.rs
│   │       └── schema.rs
│   └── ghost-analyze/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── normalize.rs
│           ├── compare.rs
│           ├── statistics.rs
│           └── report.rs
├── probes/
│   └── cuda-vector-add/
│       ├── CMakeLists.txt
│       ├── main.cpp
│       ├── vector_add.cu
│       └── README.md
├── scripts/
│   ├── bootstrap_ubuntu.sh
│   ├── build_probe.sh
│   ├── capture_group.sh
│   ├── capture_matrix.sh
│   └── verify_environment.sh
├── schemas/
│   ├── environment.schema.json
│   ├── experiment.schema.json
│   └── run.schema.json
├── configs/
│   └── milestone-0.toml
├── docs/
│   ├── ARCHITECTURE.md
│   ├── MILESTONE-0.md
│   ├── SAFETY.md
│   ├── EXPERIMENT-DESIGN.md
│   └── ROADMAP.md
├── experiments/
│   └── .gitkeep
└── tests/
    └── fixtures/
```

Keep generated experiment output out of Git, except small deterministic fixtures.

---

## 6. Rust Workspace Responsibilities

### `ghostctl`

Command-line interface for the project.

Initial commands:

```text
ghostctl doctor
ghostctl doctor --json
ghostctl probe build
ghostctl probe run --threads 32
ghostctl experiment capture --config configs/milestone-0.toml
ghostctl experiment list
ghostctl experiment validate <path>
ghostctl analyze compare <group-a> <group-b>
ghostctl analyze summarize <experiment-root>
```

Responsibilities:

- Check host compatibility
- Collect environment information
- Build and execute the CUDA probe
- Create experiment directories
- Generate immutable manifests
- Validate experiment structure
- Invoke tracing tools
- Produce human-readable and JSON reports

### `ghost-core`

Shared types and low-level utilities.

Responsibilities:

- Environment metadata
- Hashing
- Process execution
- Path handling
- Manifest models
- JSON serialization
- Validation primitives
- Timestamp handling
- Stable schema versioning

### `ghost-analyze`

Trace normalization and statistical comparison.

Milestone 0 input is primarily:

- Program stdout/stderr
- `strace` output
- Environment metadata
- Run timing
- Executable and fatbin hashes

Responsibilities:

- Normalize process IDs
- Normalize timestamps
- Detect address-like values
- Detect sequence-like values
- Group repeated syscall patterns
- Compare within-group and between-group variance
- Produce candidate parameter-correlated fields
- Never claim hardware semantics without evidence

---

## 7. Coding Standards

### Rust

- Use Rust 2024 edition if toolchain compatibility allows; otherwise use 2021.
- Use `clap` for CLI parsing.
- Use `serde` and `serde_json` for schemas.
- Use `toml` for experiment configuration.
- Use `sha2` for SHA-256.
- Use `thiserror` for library errors.
- Use `anyhow` only in CLI boundaries.
- Use `tracing` and `tracing-subscriber` for logs.
- Use `chrono` or `time` consistently; do not mix both.
- Use `camino` for UTF-8 paths if useful.
- Avoid `unsafe` in Milestone 0.
- Deny warnings in CI.
- Add unit tests for parsers and normalization rules.
- Add integration tests for manifest creation and validation.
- Public types and functions must have documentation comments.

### C++ and CUDA

- Use CUDA Driver API rather than CUDA Runtime API for the probe.
- Keep the probe minimal and deterministic.
- Avoid dynamic framework dependencies.
- Use one CUDA context.
- Use one module.
- Use one kernel launch.
- Use one synchronization point.
- Verify every output element.
- Return non-zero exit code on any CUDA or verification failure.
- Print machine-readable `key=value` lines.
- Compile specifically for `sm_75` initially.
- Do not use managed memory.
- Do not use CUDA graphs.
- Do not use multiple streams in Milestone 0.
- Do not introduce random input values.

### Bash

- Always use:

```bash
set -euo pipefail
```

- Quote all variables.
- Resolve repository root robustly.
- Fail with clear messages.
- Do not silently install packages.
- Bootstrap scripts must print commands before running privileged operations.
- Never overwrite experiment directories.

---

## 8. Minimal CUDA Probe Specification

Create a CUDA Driver API vector-add probe.

### Fixed properties

```text
Element count: 4096
Input A[i]: i * 0.5
Input B[i]: i * 0.25
Output C[i]: A[i] + B[i]
CUDA contexts: 1
CUDA streams: default stream only
Kernel launches: 1
Synchronizations: 1
```

### Variable property

Only block size changes:

```text
32
64
128
256
```

Grid size is derived from:

```text
ceil(element_count / block_size)
```

### Required output

```text
device=<GPU name>
compute_capability=<major.minor>
elements=4096
threads=<value>
blocks=<value>
verification=passed
```

### Required CUDA Driver API calls

The probe should use explicit Driver API calls similar to:

```text
cuInit
cuDeviceGet
cuDeviceGetName
cuDeviceComputeCapability
cuCtxCreate
cuModuleLoad
cuModuleGetFunction
cuMemAlloc
cuMemcpyHtoD
cuLaunchKernel
cuCtxSynchronize
cuMemcpyDtoH
cuMemFree
cuModuleUnload
cuCtxDestroy
```

All CUDA calls must use a shared error-checking helper.

---

## 9. Environment Doctor Specification

`ghostctl doctor` must inspect and report:

- Operating system
- Linux kernel version
- Distribution release
- CPU model
- Total system RAM
- GPU name
- GPU PCI identifier
- GPU total VRAM
- NVIDIA driver version
- Loaded NVIDIA module filename
- NVIDIA module version
- NVIDIA module license
- Whether the open kernel module can be positively identified
- CUDA compiler version
- Rust compiler version
- Cargo version
- CMake version
- C++ compiler version
- `strace` version
- `lspci` availability
- `modinfo` availability
- Secure Boot state if detectable
- IOMMU state if detectable
- Current user privileges
- Repository Git commit
- Working tree cleanliness

Output formats:

```text
human-readable
JSON
```

Do not mark the system fully supported unless:

- Host is native Linux
- NVIDIA GPU is visible
- CUDA toolkit is available
- Required build and trace tools are available
- GPU is Turing or newer
- Driver information can be recorded

If the open kernel module cannot be confirmed, report a warning rather than guessing.

---

## 10. Experiment Configuration

Create `configs/milestone-0.toml`:

```toml
schema_version = 1
name = "cuda-block-size-baseline"
description = "Repeated CUDA Driver API vector-add runs with block size as the only independent variable."

repetitions = 10
warmup_runs = 2
timeout_seconds = 30

[probe]
path = "build/cuda-vector-add/ghost-cuda-probe"
element_count = 4096

[variable]
name = "threads"
values = [32, 64, 128, 256]

[capture]
strace = true
nvidia_smi_before = true
nvidia_smi_after = true
kernel_log_excerpt = false

[validation]
require_clean_git = false
require_same_binary_hash = true
require_same_fatbin_hash = true
require_verification_passed = true
```

The implementation must support schema versioning.

---

## 11. Experiment Directory Format

Each experiment must use a unique immutable directory:

```text
experiments/
└── 2026-07-14T180000Z_cuda-block-size-baseline/
    ├── experiment.json
    ├── environment.json
    ├── config.toml
    ├── hashes.json
    ├── summary.json
    └── groups/
        ├── threads-32/
        │   ├── group.json
        │   ├── run-001/
        │   │   ├── run.json
        │   │   ├── stdout.txt
        │   │   ├── stderr.txt
        │   │   ├── strace.*
        │   │   ├── nvidia-smi-before.txt
        │   │   └── nvidia-smi-after.txt
        │   └── ...
        ├── threads-64/
        ├── threads-128/
        └── threads-256/
```

Never mutate a completed run.

If a run fails, preserve the failed run and mark its status.

---

## 12. Trace Capture Rules

Milestone 0 uses `strace` only as an outer observation layer.

Recommended flags:

```bash
strace \
  -ff \
  -ttt \
  -yy \
  -s 256 \
  -o <output-prefix> \
  <probe> --threads <value>
```

The project must explicitly document:

> `strace` does not capture the raw GPU command stream after userspace mappings are established.

The purpose of `strace` in Milestone 0 is:

- Identify syscall patterns
- Record ioctl timing and frequency
- Record mmap activity
- Correlate userspace execution with later kernel instrumentation
- Establish deterministic experimental discipline

Do not label syscall traces as GPU packets.

---

## 13. Initial Analysis Requirements

Implement a first-pass analyzer that compares repeated runs.

### Normalization candidates

Normalize or classify:

- PID and TID values
- Absolute timestamps
- File descriptor numbers
- Memory addresses
- Temporary paths
- Sequence-like counters
- Duration values
- Driver-generated object handles

### Field categories

Every observed difference should be classified as one of:

```text
stable_global
stable_within_group
parameter_correlated
address_like
timestamp_like
sequence_like
process_specific
high_variance
unknown
```

### Statistical behaviour

For each candidate numeric field:

- Compute within-group variance
- Compute between-group variance
- Compute correlation with thread count
- Record sample count
- Record confidence score
- Avoid strong semantic labels in Milestone 0

Example output:

```json
{
  "candidate": "ioctl_argument_field_3",
  "classification": "parameter_correlated",
  "correlation_with_threads": 0.98,
  "within_group_variance": 0.01,
  "between_group_variance": 12.4,
  "confidence": 0.93,
  "semantic_claim": null
}
```

---

## 14. Milestone 0 Acceptance Criteria

Milestone 0 is complete only when all of the following are true:

1. `ghostctl doctor` creates valid human-readable and JSON reports.
2. CUDA probe builds successfully on the target machine.
3. Probe works for 32, 64, 128, and 256 threads.
4. Every probe run verifies all output values.
5. Each experiment group completes at least 10 repetitions.
6. Executable and fatbin hashes are identical across groups.
7. Environment metadata is saved.
8. Failed runs are preserved and reported.
9. Experiment structure passes schema validation.
10. Trace normalization removes obvious PID and timestamp noise.
11. Analyzer produces within-group and between-group summaries.
12. Documentation clearly states the limitations of `strace`.
13. No GPU replay or kernel modification exists in the repository.
14. Unit and integration tests pass.
15. `cargo fmt`, `cargo clippy`, and `cargo test` pass.

---

## 15. Milestone 1 Preview — Do Not Implement Yet

Milestone 1 will investigate Linux kernel-visible NVIDIA driver paths using passive methods.

Potential tools:

- ftrace
- tracepoints
- kprobes
- eBPF where appropriate
- NVIDIA open kernel module source inspection

Possible observation targets:

- ioctl dispatch paths
- mmap paths
- channel creation
- queue-related mappings
- doorbell-related mappings
- process/context identifiers

Milestone 1 must begin with a written design review.

Do not implement Milestone 1 until Milestone 0 is accepted.

---

## 16. Documentation Requirements

Create the following documents.

### `README.md`

Include:

- Project summary
- Current milestone
- Safety warning
- Supported platform
- Build instructions
- First experiment instructions
- Project status
- Non-goals
- License

### `docs/ARCHITECTURE.md`

Include:

- Workspace components
- Data flow
- Experiment lifecycle
- Analysis pipeline
- Schema versioning

### `docs/EXPERIMENT-DESIGN.md`

Include:

- Independent variable rules
- Repetition strategy
- Noise sources
- Version locking
- Statistical comparison strategy
- Why two traces are not enough

### `docs/SAFETY.md`

Include:

- Passive-only Milestone 0
- Future GPU hang risks
- Data-loss warning
- Why containers do not isolate GPU hangs
- Why raw replay requires a dedicated test machine

### `docs/ROADMAP.md`

Use these phases:

```text
Milestone 0: Deterministic observation laboratory
Milestone 1: Passive kernel-path discovery
Milestone 2: Command submission boundary identification
Milestone 3: Structured command-stream capture
Milestone 4: Differential field inference
Milestone 5: Unmodified replay in an isolated environment
Milestone 6: Controlled single-field mutation
Milestone 7: Minimal generated execution profile
```

Do not promise that all milestones are technically achievable.

---

## 17. CI Requirements

Create GitHub Actions for Rust-only checks.

The CI environment is not expected to have an NVIDIA GPU.

Run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
JSON schema validation tests
Shell script syntax checks
```

CUDA probe compilation should be optional and skipped when CUDA is unavailable.

Use mock fixtures for analysis tests.

---

## 18. Implementation Order

must implement the project in this order:

### Phase A — Repository foundation

1. Create workspace and crate structure.
2. Add license and Git configuration.
3. Add Rust toolchain file.
4. Add CI.
5. Add base documentation.

### Phase B — Core models

1. Define environment report types.
2. Define experiment manifest types.
3. Define run and group types.
4. Add schema versioning.
5. Add JSON schema files.
6. Add serialization tests.

### Phase C — Doctor command

1. Implement safe command execution helper.
2. Collect host metadata.
3. Generate human-readable output.
4. Generate JSON output.
5. Add warnings and support checks.
6. Add fixture-based tests.

### Phase D — CUDA probe

1. Implement CUDA Driver API vector-add.
2. Add CMake build.
3. Add explicit SM 7.5 fatbin generation.
4. Add numerical verification.
5. Add machine-readable output.

### Phase E — Experiment runner

1. Parse TOML configuration.
2. Create immutable directory layout.
3. Record environment metadata.
4. Record hashes.
5. Run warmups.
6. Run repeated experiment groups.
7. Execute `strace`.
8. Preserve failures.
9. Validate completed experiment.

### Phase F — Initial analyzer

1. Parse captured text traces.
2. Normalize timestamps and IDs.
3. Group repeated patterns.
4. Compute variance summaries.
5. Produce JSON and Markdown reports.

Do not skip ahead.

---

## 19. Quality Expectations

This repository should look like a serious systems research project from the first commit.

Requirements:

- No placeholder functions left silently incomplete.
- No fake hardware results.
- No hard-coded claims that the GPU is supported.
- No fabricated benchmark values.
- No claim of capturing raw GPU packets in Milestone 0.
- Every external command failure must be visible.
- Every file format must include a schema version.
- Errors must include actionable context.
- Logs must avoid leaking unnecessary private paths.
- Commands should support `--help`.
- All documentation should be written in clear technical English.
- Code comments should explain reasoning, not restate syntax.

---

## 20. First Task

Start by implementing **Phase A and Phase B only**.

Deliver:

1. Complete Rust workspace structure
2. `ghostctl`, `ghost-core`, and `ghost-analyze` crates
3. Shared manifest and schema types
4. JSON schema files
5. Base project documentation
6. GitHub Actions CI
7. Unit tests for serialization and schema version validation

Do not implement CUDA code, experiment execution, `strace`, kernel instrumentation, or analysis logic yet.

At the end:

- Run formatting
- Run Clippy
- Run tests
- Summarize created files
- List any assumptions
- List the exact next task for Phase C
