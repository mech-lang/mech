# Mech GPU host

`mech-gpu` admits a deliberately small, portable subset of a typed
`ProgramArtifact` and lowers the admitted graph to WGSL. The host, rather than
the source program, owns the capability policy.

The first slice supports fused, element-wise `f32` and relaxed `f64` addition,
subtraction, multiplication, and division over fixed-size matrices with scalar
broadcasting. Relaxed `f64` source is explicitly lowered to `f32` GPU storage. Operations
must be pure signal operations with full-write, no-alias outputs. Dynamic
shapes, effects, integrity constraints, opaque operation contracts, and unknown
kernels are rejected with structured diagnostics. Whole-value Mech registers
are admitted as state: their initializers are uploaded once and their buffers
remain resident between turns.

The returned `GpuProgram` includes generated WGSL, its binding layout, and CPU
and GPU resident executors. Both are lowered from the same `ProgramArtifact`.
`GpuHost::plan` reports every node target, GPU region, slot residence, and
upload/readback boundary. An unsupported node is assigned to CPU with a reason;
mixed-region execution still fails closed until the runtime scheduler can
coordinate those boundaries.

With the `native` feature, `GpuProgram::run_gpu` dispatches that WGSL through
`wgpu`. It uses Metal on macOS and an available Vulkan or Direct3D 12 backend
on Linux and Windows. `run_cpu` and `run_gpu` therefore select execution hosts
without changing the Mech source.

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

1. Mech source compiles to a typed artifact with explicit state slots.
2. Capability admission and placement use operation contracts and schemas, not
   variable names or a precompiled particle kernel.
3. CPU and GPU sessions execute the same stateful graph.
4. State residency and transfer boundaries are derived and inspectable.

The `runtime-host` feature also exposes a configured `gpu://` resource host.
That host lets a regular transactional CPU graph dispatch a GPU-resident Mech
kernel after commit and receive adapter/timing telemetry through normal live
resource reads. See `examples/mixed-cpu-gpu-particles`.

The host can prepare the admitted kernel through either its `wgpu` or fused CPU
executor. Projects declare the permitted executors with `backends` and select a
default with `backend`; `mech run --backend cpu|wgpu` overrides that selection
for a run. The surrounding reactive graph remains on the CPU in both cases.
There is no implicit fallback from a failed GPU preparation to CPU execution.

Two contracts are still needed before `mech run` can automatically partition
an arbitrary graph:

- The runtime needs a generic activated-plan/execution-provider interface that
  can schedule CPU and GPU regions on the same reactive turn.
- `InitializerReference` needs an activation-time input or initialization-graph
  form. Its current constant-only form cannot represent a large state matrix
  produced by a resource host without snapshotting that matrix into the
  artifact.

Those are artifact/executor design requirements, not particle-demo concerns.
The GPU provider should be selected in `.mcfg` once that interface exists; Mech
source files should not contain device directives or Rust callbacks.
