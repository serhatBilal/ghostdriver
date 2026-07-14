# Safety

## Milestone 0 boundary

Milestone 0 is passive. It must not write or replay GPU command buffers, access
GPU MMIO directly, patch NVIDIA kernel modules, mutate kernel-space memory, or
intercept arbitrary kernels. `strace` output is a syscall observation layer,
not a raw GPU command stream.

## Future risk

Later command replay or mutation research can hang the GPU, freeze the host,
corrupt in-flight work, or cause data loss. A container does not isolate these
failures because it shares the host kernel, GPU, driver, and physical device.

Raw replay therefore requires a dedicated test machine with recoverable
storage, no valuable workloads, remote recovery where practical, and an
explicit design and safety review. It is not permitted by the current
milestone.

All experiment output must record failures honestly. Missing driver or module
information produces a warning rather than a guessed support claim.
