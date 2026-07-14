# Milestone 0 - Deterministic Observation Laboratory

## Objective

Build a reproducible experiment harness before touching the NVIDIA kernel
module or attempting GPU command replay.

The sole independent variable in the first experiment is CUDA block size:

```text
Group A: 32 threads
Group B: 64 threads
Group C: 128 threads
Group D: 256 threads
```

All groups use:

- the same executable hash,
- the same fatbin hash,
- the same vector length,
- the same allocation order,
- the same driver,
- the same CUDA toolkit,
- the same Linux kernel,
- one CUDA context,
- one kernel launch,
- one synchronization point.

## Deliverables

1. `ghostctl doctor` host report.
2. Correct CUDA Driver API vector-add probe.
3. Ten traces per block-size group.
4. Immutable experiment manifests.
5. Initial noise inventory.

## Noise inventory

Before interpreting any byte or ioctl difference, classify observed fields as:

- stable within and across groups,
- stable within a group but different between groups,
- process-specific,
- address-like,
- sequence-like,
- timestamp-like,
- high-variance/unknown.

Milestone 0 does not claim to capture the raw GPU pushbuffer. `strace` is only
the first outer layer. Its purpose is to establish experimental discipline,
driver call timing and repeatability.

## Go / no-go gate

Proceed to passive kernel-path discovery only after a written Milestone 1
design review and when:

- all probe runs are correct,
- no uncontrolled application-level variable changes,
- traces are automatically grouped,
- environment versions are locked,
- failures are reproducible and diagnosable.
