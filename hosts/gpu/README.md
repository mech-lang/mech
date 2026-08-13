# Mech GPU host

`mech-gpu` admits a deliberately small, portable subset of a typed
`ProgramArtifact` and lowers the admitted graph to WGSL. The host, rather than
the source program, owns the capability policy.

The first slice supports fused, element-wise `f32` addition, subtraction, and
multiplication over fixed-size matrices with scalar broadcasting. Operations
must be pure signal operations with full-write, no-alias outputs. Dynamic
shapes, effects, integrity constraints, opaque operation contracts, and unknown
kernels are rejected with structured diagnostics. Whole-value Mech registers
are admitted as state: their initializers are uploaded once and their buffers
remain resident between turns.

The returned `GpuProgram` includes generated WGSL, its binding layout, and CPU
and GPU resident executors. Both are lowered from the same `ProgramArtifact`.
`GpuHost::plan` reports every node target, GPU region, slot residence, and
upload/readback boundary. `GpuHost::plan_with_regions` also honors named
Mechdown compute sections and preserves their boundaries. An unsupported node
is assigned to CPU with a reason; mixed-region execution still fails closed
until the runtime scheduler can coordinate those boundaries.

With the `native` feature, `GpuProgram::run_gpu` dispatches that WGSL through
`wgpu`. It uses Metal on macOS and an available Vulkan or Direct3D 12 backend
on Linux and Windows. `run_cpu` and `run_gpu` therefore select execution hosts
without changing the Mech source.

The native runtime host accepts a region name from `.mcfg`, compiles that named
region from the same `.mec` document as the ordinary application graph, and
prepares one resident session. A send to `gpu://<instance>/kernel/turn` is an
at-most-once after-commit effect; successful dispatch telemetry returns through
normal runtime ingress. The checked-in `gpu-particles` application runs the
unannotated projection through D4's resident external route and uses this GPU
path with real timer and console hosts.

The one-shot native call is a correctness path that creates and reads back a
GPU dispatch for one turn. `GpuProgram::prepare_cpu` and
`GpuProgram::prepare_resident` create persistent CPU and GPU sessions. The GPU
session derives ping-pong state buffers from artifact state slots; callers do
not provide an output-to-input feedback map. Readback is deferred until the
host asks for outputs.

Run the release benchmark with a particle count, CPU turn count, and GPU sample
count:

```text
cargo run -p mech-gpu --release --features native \
  --example particle_benchmark -- 2000000 2 2 120
```

The arguments are particle count, CPU turns, one-shot GPU samples, and resident
GPU turns. The benchmark checks one-shot GPU output against the CPU executor,
checks sampled resident output against the repeated recurrence, and reports
compilation, one-shot execution, resident dispatch, and final readback
separately.

## Runtime integration boundary

This crate now proves the lower half of automatic acceleration:

1. One Mech document compiles to a typed artifact with explicit state slots and
   named compute-region metadata.
2. Capability admission and placement use operation contracts and schemas, not
   variable names or a precompiled particle kernel.
3. CPU and GPU sessions execute the same stateful graph.
4. State residency and transfer boundaries are derived and inspectable.

Two contracts are still needed before arbitrary mixed applications can run:

- Section projection must become a compiler-produced dependency partition, and
  the runtime needs a multi-region scheduler for cross-region values and more
  than one GPU region.
- `InitializerReference` needs an activation-time input or initialization-graph
  form. Its current constant-only form cannot represent a large state matrix
  produced by a resource host without snapshotting that matrix into the
  artifact.

Those are artifact/executor design requirements, not particle-demo concerns.
The executor is selected in `.mcfg`. Source may use backend-neutral
`section @ compute` boundaries or hard `@ cpu`/`@ gpu` requirements, but never
contains device buffers, pointers, upload operations, or Rust callbacks.
