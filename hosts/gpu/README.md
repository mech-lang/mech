# Mech GPU host

`mech-gpu` admits a deliberately small, portable subset of a typed
`ProgramArtifact` and lowers the admitted graph to WGSL. The host, rather than
the source program, owns the capability policy.

The first slice supports fused, element-wise `f32` addition, subtraction, and
multiplication over fixed-size matrices with scalar broadcasting. Operations
must be pure signal operations with full-write, no-alias outputs. Dynamic
shapes, state, effects, integrity constraints, opaque operation contracts, and
unknown kernels are rejected with structured diagnostics.

The returned `GpuProgram` includes generated WGSL, its binding layout, and a
CPU reference executor. Both are lowered from the same `ProgramArtifact`; the
CPU executor is used for parity tests and hosts that select the CPU backend.

With the `native` feature, `GpuProgram::run_gpu` dispatches that WGSL through
`wgpu`. It uses Metal on macOS and an available Vulkan or Direct3D 12 backend
on Linux and Windows. `run_cpu` and `run_gpu` therefore select execution hosts
without changing the Mech source.

The current native call is a correctness path: it creates and reads back a GPU
dispatch for one turn. A resident session that retains the pipeline and device
buffers across turns is the next executor step; see
[`gpu-artifact-lowering-findings.md`](../../docs/design/gpu-artifact-lowering-findings.md).

Run the release benchmark with a particle count, CPU turn count, and GPU sample
count:

```text
cargo run -p mech-gpu --release --features native \
  --example particle_benchmark -- 50000 20 7
```

The benchmark checks every GPU output against the CPU executor and reports
artifact/WGSL compilation, CPU execution, cold GPU execution, and the median
warm one-shot GPU phase breakdown separately.
