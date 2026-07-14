#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD="${ROOT}/build/cuda-vector-add"

cmake -S "${ROOT}/probes/cuda-vector-add" -B "${BUILD}"
cmake --build "${BUILD}" -j

echo "probe=${BUILD}/ghost-cuda-probe"
echo "fatbin=${BUILD}/ghost_cuda_probe.fatbin"
