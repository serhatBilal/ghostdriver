#!/usr/bin/env bash
set -euo pipefail

THREADS="${1:-}"
REPETITIONS="${2:-10}"

if [[ -z "${THREADS}" ]]; then
  echo "usage: $0 <threads> [repetitions]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROBE="${ROOT}/build/cuda-vector-add/ghost-cuda-probe"
FATBIN="${ROOT}/build/cuda-vector-add/ghost_cuda_probe.fatbin"

if [[ ! -x "${PROBE}" ]]; then
  echo "probe not found: ${PROBE}" >&2
  echo "build it first with CMake; see README.md" >&2
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
GROUP="${ROOT}/experiments/threads-${THREADS}-${STAMP}"
mkdir -p "${GROUP}"

{
  echo "schema_version=1"
  echo "variable=threads"
  echo "value=${THREADS}"
  echo "repetitions=${REPETITIONS}"
  echo "created_utc=${STAMP}"
  echo "kernel=$(uname -r)"
  echo "probe_sha256=$(sha256sum "${PROBE}" | awk '{print $1}')"
  echo "fatbin_sha256=$(sha256sum "${FATBIN}" | awk '{print $1}')"
  echo "driver=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -n1)"
  echo "gpu=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -n1)"
  echo "cuda=$(nvcc --version | tail -n1)"
} > "${GROUP}/manifest.env"

for run in $(seq 1 "${REPETITIONS}"); do
  RUN_DIR="${GROUP}/run-$(printf '%03d' "${run}")"
  mkdir -p "${RUN_DIR}"

  nvidia-smi -q > "${RUN_DIR}/nvidia-smi-before.txt"

  strace \
    -ff \
    -ttt \
    -yy \
    -s 256 \
    -o "${RUN_DIR}/strace" \
    "${PROBE}" --threads "${THREADS}" \
    > "${RUN_DIR}/stdout.txt" \
    2> "${RUN_DIR}/stderr.txt"

  nvidia-smi -q > "${RUN_DIR}/nvidia-smi-after.txt"

  if ! grep -q '^verification=passed$' "${RUN_DIR}/stdout.txt"; then
    echo "run ${run} failed verification" >&2
    exit 1
  fi
done

echo "capture_group=${GROUP}"
