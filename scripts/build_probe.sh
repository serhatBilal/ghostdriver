#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD="${ROOT}/build/cuda-vector-add"

cmake -S "${ROOT}/probes/cuda-vector-add" -B "${BUILD}"
cmake --build "${BUILD}" -j

nvcc \
  --fatbin \
  "${ROOT}/probes/cuda-vector-add/vector_add.cu" \
  -o "${BUILD}/ghost_cuda_probe.fatbin"

echo "probe=${BUILD}/ghost-cuda-probe"
