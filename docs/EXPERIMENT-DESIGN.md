# Experiment Design

## Independent variable

Only one independent variable may change between experiment groups. The first
experiment varies CUDA block size across 32, 64, 128, and 256 threads. Element
count, input values, allocation order, executable, fatbin, driver, toolkit,
kernel, context count, launch count, and synchronization count remain fixed.

## Repetition strategy

Each group uses two unrecorded warmups followed by at least 10 recorded runs.
Failed attempts are preserved and reported rather than replaced invisibly. Run
order and timestamps are recorded so drift can be investigated.

Two traces are not enough: they cannot distinguish a parameter effect from
address randomization, process identifiers, timing noise, lazy initialization,
or driver-managed counters. Repetition estimates within-group variability
before differences between groups are considered.

## Noise sources

Expected noise includes PIDs and TIDs, absolute timestamps, file descriptors,
memory addresses, temporary paths, durations, sequence counters, and opaque
driver handles. Normalization must preserve original artifacts and operate on
derived data only.

## Version locking

Every experiment records the kernel, distribution, CPU, RAM, GPU, PCI ID,
VRAM, NVIDIA driver and module, CUDA compiler, Rust toolchain, build tools,
executable hash, fatbin hash, Git commit, and working-tree state. Required
binary and fatbin hashes must match across all groups.

## Statistical comparison

For candidate numeric fields, Phase F will report sample count, within-group
variance, between-group variance, correlation with thread count, and a
confidence score. Differences receive observational categories such as
`parameter_correlated` or `address_like`; no hardware semantic claim is made in
Milestone 0.
