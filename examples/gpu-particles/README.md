# GPU particle kernel

[`particles.mec`](particles.mec) defines the particle integration equations in
Mech. It does not call a named, precompiled particle kernel.

The benchmark harness supplies initial `f32` matrices and scalar controls so
their types and shapes flow through normal Mech inference. The recurrence is
not host-managed: `positions` and `velocities` are ordinary mutable Mech state,
and their state slots drive CPU/GPU residency automatically. The compiler
produces a typed `ProgramArtifact`; `mech-gpu::GpuHost` inspects that artifact,
reports placement, rejects unsupported semantics with structured diagnostics,
and fuses the admitted operations into WGSL.

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

On the measured Apple M1, the artifact-derived state path completed 120
resident turns over two million particles at 1.738 ms per turn, or 1.150
billion particle-turns per second. The final full readback took 15.332 ms and
sampled recurrence error was 5.96e-8. The previous hand-wired feedback path was
1.703 ms per turn, a difference of about 2% in this run.

The benchmark reports the older one-shot path separately. Resident CPU and GPU
turns execute the same lowered state graph. GPU turns keep the pipeline and
particle state on the device, alternate state-buffer sets inferred from the
artifact, and perform one readback after the measured loop.

The harness still injects the initial benchmark dataset before compilation.
That is not the intended application interface. A general resource-produced
initializer requires an activation-time initializer representation in
`ProgramArtifact`; the current model only permits constant initializers. The
GPU host README records this runtime integration boundary explicitly. At two
million particles, snapshotting those initializers made artifact and WGSL
construction take 22.8 seconds even though resident preparation took 5.6 ms.
