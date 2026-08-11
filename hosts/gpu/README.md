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

The one-shot native call is a correctness path that creates and reads back a
GPU dispatch for one turn. `GpuProgram::prepare_resident` creates a persistent
pipeline and ping-pong state buffers. Its feedback map connects physical result
buffers to the next turn's input bindings, and readback is deferred until the
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
