# CUDA Vector-Add Probe

This probe is the deterministic workload for GhostDriver Milestone 0. It uses
the CUDA Driver API directly and performs exactly one kernel launch and one
explicit context synchronization.

## Fixed workload

- 4096 `float` elements
- `A[i] = i * 0.5`
- `B[i] = i * 0.25`
- `C[i] = A[i] + B[i]`
- One device, context, module, default stream, launch, and synchronization
- Fatbin compiled specifically for `sm_75`

Only `--threads` may vary, and its accepted values are 32, 64, 128, and 256.

## Build

On native Linux with the CUDA Toolkit installed:

```bash
./scripts/build_probe.sh
```

Equivalent direct CMake commands are:

```bash
cmake -S probes/cuda-vector-add -B build/cuda-vector-add
cmake --build build/cuda-vector-add -j
```

The executable and `ghost_cuda_probe.fatbin` are emitted together in
`build/cuda-vector-add/`.

## Run

```bash
./build/cuda-vector-add/ghost-cuda-probe --threads 32
```

A successful run exits with status zero and prints:

```text
device=<GPU name>
compute_capability=<major.minor>
elements=4096
threads=<value>
blocks=<value>
verification=passed
```

Every output element is checked. CUDA, argument, cleanup, or numerical
verification failures produce a non-zero exit status.
