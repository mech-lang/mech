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
cargo run -p mech-gpu --release --features native,jit \
  --example particle_benchmark -- 2000000 2 2 120
```

The arguments are particle count, CPU turns, one-shot GPU samples, and resident
GPU turns. The benchmark checks one-shot GPU output against the CPU executor,
checks sampled resident output against the repeated recurrence, and reports
compilation, one-shot execution, resident dispatch, and final readback
separately.

To compare the selectable backends using the exact `particle-field @ compute`
region from the served example, run:

```text
cargo run -p mech-gpu --release --features native,jit \
  --example compute_backend_benchmark -- 1000000 5 60
```

This reports resident CPU compute, CPU compute plus a full position snapshot,
synchronized per-turn GPU latency, and batched resident GPU throughput. It
does not include the transactional pointer graph or WebGPU rendering, which
are common to both served modes. The native CPU snapshot is only a lower bound
for browser CPU presentation: browser mode also copies the snapshot across the
WASM/JavaScript boundary and uploads it to the WebGPU render buffer.

To record only steady-state compute throughput over time, with no readback,
rendering, transactional graph, or batched dispatch, add a duration and CSV
path:

```text
cargo run -p mech-gpu --release --features native,jit \
  --example compute_backend_benchmark -- \
  1000000 1 1 20 /tmp/mech-compute-timeline.csv
```

Each CPU observation is one turn. Each GPU observation aggregates 100
individually submitted and synchronized turns to make the plot readable; it
does not submit a 100-turn batch.

## Generic parallel EKF proof

[`fixtures/ekf-kernel.mec`](fixtures/ekf-kernel.mec) is one Mech document. Its
ordinary section constructs arrays of controls and observations, while its
named `EKF step @ compute` region contains one extended Kalman filter update
written with ordinary matrix operations. The filter contains matrix
construction, `**`, transpose, dot products, scalar broadcasting, `sin`,
`cos`, and `atan2`, including the Joseph-form covariance update. It contains
no EKF-specific operation or precompiled EKF call.

`GpuHost::compile_broadcast_with_regions` consumes the compiler's typed
`ProgramArtifact`, named-region metadata, and the actual Mech array values. It
derives their common outer extent, rejects inconsistent extents, and never
receives a filter count. For fixed `f32` inner shapes it scalarizes generic
matrix multiply, transpose, dot, concatenation, arithmetic, and trigonometry
into one register program. That program can run in the portable scalar
evaluator, the four-lane SIMD evaluator, a Cranelift native JIT function, or
WGSL. Both native code and WGSL own the outer filter loop, so there is no
per-filter host call. Intermediate matrices remain invocation-local values;
only filter inputs and persistent state/covariance buffers cross the kernel
boundary.

The Mech source declares three named integrity constraints:
`finite-candidate!`, `positive-covariance!`, and `symmetric-covariance!`.
They use ordinary absolute value, comparison, Boolean, and fixed matrix-index
operations; there is no EKF validation primitive or separate Rust policy.
Their names survive artifact and bytecode encoding, participate in artifact
identity, and appear in structured faults. Scalar, SIMD, and JIT executors
evaluate the constraints against a candidate buffer before swapping it live.
WGSL writes a compact device fault record, and the GPU host advances its
published ping-pong buffer only after that record passes. On failure the whole
candidate turn is rejected, the previous estimate remains published, and the
session retains a bounded fault count plus its latest named fault rather than
an append-only transaction log.

Run the proof with the number of filters followed by CPU reference turns,
single-turn GPU samples, and checked repeated GPU turns:

```text
cargo run -p mech-gpu --release --features native,jit \
  --example parallel_ekf_benchmark -- 100000 3 20 120
```

The following Apple M1 data is the preserved unchecked baseline from commit
`6b27e4cdbcdd53ddb0c646169be0bb597bd2a39e`. It predates the publication
checks and must not be labeled as checked execution:

| Filters | Scalar evaluator | SIMD evaluator | Cranelift JIT | GPU, one submission/turn | GPU, 120 turns/submission |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 100,000 | 1.216 M EKF-turns/s | 4.414 M EKF-turns/s | 17.306 M EKF-turns/s | 53.557 M EKF-turns/s | 343.969 M EKF-turns/s |

The maximum CPU/GPU absolute error after four validation turns was
`6.866e-5`; JIT output matched the scalar evaluator bit-for-bit. The test suite
separately compares one scalarized CPU turn with the ordinary Mech execution
result, so accelerator validation does not rely only on executors sharing the
same lowered implementation.

These labels are narrow on purpose. "Generic scalar CPU" is the portable
scalar IR evaluator in this crate, not the retained runtime or raw Rust. The
four-lane SIMD executor runs the same scalarized artifact using `wide::f32x4`.
The JIT translates the same generic instructions to Cranelift IR and keeps the
finalized function resident in executable memory; it contains no EKF-specific
operation. The old 120-turn GPU number records multiple dependent dispatches
in one command submission and does not include final readback. Checked GPU
execution now validates before every publication, so it intentionally submits
and reads compact fault status once per turn; safe multi-turn batching needs a
future device-side transaction protocol.

The checked evaluated artifact at commit
`7605c5c9081a22d7bcba0b0c288570a7c3a41f41` produced these five-process
medians on Apple M1 Metal with 100,000 filters:

| Checked backend | Time/turn | Million EKF-turns/s |
| --- | ---: | ---: |
| Scalar artifact evaluator | 122.212 ms | 0.818 |
| SIMD (`4xf32`) | 31.702 ms | 3.154 |
| Cranelift JIT | 8.105 ms | 12.339 |
| GPU, one checked submission/turn | 1.942 ms | 51.497 |
| GPU, repeated checked turns | 1.767 ms | 56.580 |

Source parsing, artifact construction, and scalarization took a median
`107.022 ms`; JIT preparation took `3.573 ms`. The maximum CPU/GPU absolute
error was `6.866e-5`, and JIT output matched the scalar evaluator bit-for-bit.
The checked GPU figures include validation before every publication. The old
unchecked 120-turn batched figure is not comparable because it crosses the
host publication boundary only after all 120 turns. Raw checked samples are
recorded in
[`apple-m1-checked-integrity-2026-08-14.json`](benchmarks/parallel-ekf/results/apple-m1-checked-integrity-2026-08-14.json).

[`benchmarks/parallel-ekf`](benchmarks/parallel-ekf) contains the reproducible
two-panel comparison: Mech scalar versus SIMD versus JIT versus GPU, and scalar
outer-loop Mech/JIT versus optimized Rust, NumPy, Julia, and LuaJIT. The latter
comparison uses matching inputs and checksums and keeps setup outside timing.

The benchmark's optional count argument changes the `filter-count` value in
the Mech source before parsing so the same workload can be scaled. The backend
still receives only the resulting arrays and infers their extent. State and
covariance are genuine per-lane resident arrays, initialized by broadcasting
the one-filter source initializer.

This is a region-level broadcast spike, not yet a general compiled user-call.
The compiler still needs a compact representation for a literal
`ekf(states, observations)` user-function broadcast, and the generic batched
kernel is not yet connected to the mixed-region runtime host used by the
particle example.

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
