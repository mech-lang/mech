# GPU particle kernel

[`particles.mec`](particles.mec) defines the particle integration equations in
Mech. It does not call a named, precompiled particle kernel.

The host supplies the initial `f32` matrices and scalar controls, so their
types and shapes flow through normal Mech inference. The compiler produces a
typed `ProgramArtifact`. `mech-gpu::GpuHost` inspects
that artifact, rejects unsupported semantics with structured diagnostics, and
fuses the admitted operations into WGSL. The same admitted plan has a CPU
backend for host selection and GPU parity checks.

This first capability slice accepts fixed-size `f32` matrices and scalar
broadcasting through pure `math/add`, `math/sub`, and `math/mul` nodes. The
matrix size in this small source fixture is only for readable tests; a host can
compile the same graph with a larger inferred input shape.

Run the source, artifact, and CPU checks with:

```text
cargo test -p mech-gpu --test particle_source
```

Run the same checks plus a real GPU dispatch with:

```text
cargo test -p mech-gpu --features native --test particle_source native_gpu
```

The native path is portable through `wgpu`, including Windows Direct3D 12 and
Vulkan adapters.

Benchmark the generated CPU and GPU executors in release mode with:

```text
cargo run -p mech-gpu --release --features native \
  --example particle_benchmark -- 2000000 2 2 120
```

On the measured Apple M1, two million particles completed 120 resident turns at
1.703 ms per turn, or 1.174 billion particle-turns per second. The final full
readback took 15.852 ms and sampled recurrence error was 5.96e-8.

The benchmark reports the older one-shot path separately. Resident turns keep
the pipeline and particle state on the GPU, alternate the two state-buffer
sets, and perform one readback after the measured loop.
