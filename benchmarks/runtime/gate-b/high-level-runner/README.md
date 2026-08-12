# Native numeric-kernel prototype

This benchmark compiles ordinary high-level Mech programs through the normal source compiler. It then exercises two backends from the same `ProgramArtifact`:

- the schema-driven resident executor;
- an experimental native numeric-kernel pipeline that invokes `rustc -O` and loads the resulting native library.

The pipeline now has a backend-neutral typed kernel IR between the Mech artifact and Rust generation:

```text
Mech source
  -> ProgramArtifact + resolved resident shapes
  -> typed numeric KernelIr
  -> Rust backend
  -> rustc -O native library
```

`KernelIr` contains explicit f64 value types, fixed matrix shapes, constants, flattened input bindings, state bindings, optional batch layout, and semantic operations. It contains no Rust expressions. The Rust backend consumes only this IR; it does not inspect the `ProgramArtifact`, resident plan, or Mech operation-name strings.

The lowering pass does not recognize the EKF or call a handwritten EKF kernel. It translates supported operations node by node: fixed-shape matrix multiplication, transpose, concatenation, dot product, elementwise arithmetic, trigonometry, and assignment. Unsupported nodes fail with a diagnostic that names both the node and operation, for example:

```text
node 17 (example/Unsupported) is not eligible for a native numeric kernel:
operation has no native numeric lowering
```

The generated ABI reads a published state buffer and writes a separate candidate state buffer. This preserves the memory layout needed for rollback, but the prototype does not yet emit integrity checks, dirty tracking, receipts, or publication records.

## Generality proof

The executable compiles and validates four Mech programs through the same lowering and backend code:

- [`../ekf-high-level.mec`](../ekf-high-level.mec): a 101-node nonlinear EKF with two persistent matrices;
- [`../numeric-kernel-proof.mec`](../numeric-kernel-proof.mec): a 17-node linear state-space recurrence with independent inputs and state;
- [`../numeric-batch-proof.mec`](../numeric-batch-proof.mec): a 64-lane typed row-vector recurrence that proves the batch plumbing independently of the EKF;
- [`../ekf-batch-high-level.mec`](../ekf-batch-high-level.mec): 1,024 independent EKFs expressed entirely as ordinary elementwise Mech equations.

The linear proof is not embedded in the emitter. Its 17 normal Mech artifact nodes become 17 kernel instructions and a separate native library. Over 4,096 turns, that library matches an independent recurrence with zero absolute error. Inspecting the generated source finds no EKF, nalgebra, or reference implementation names.

The batch EKF stores each state and covariance component in a typed `1 x 1024` row vector. This structure-of-arrays representation is visible in the Mech source. The lowering pass detects one shared lane shape across state and elementwise operations, then emits one native loop over lanes. It does not make 1,024 host calls and does not unroll 1,024 copies of the graph. The generated ABI receives one scalar pulse plus three `1 x 1024` input vectors and owns two 12,288-element state buffers for candidate publication.

## Run

```console
CARGO_TARGET_DIR=/tmp/mech-target \
MECH_AOT_DIR=/tmp/mech-aot \
cargo run --release --offline \
  --manifest-path benchmarks/runtime/gate-b/high-level-runner/Cargo.toml
```

`MECH_AOT_DIR` retains the generated `.rs` file and native library for inspection. The command requires `rustc` on `PATH`. The loader selects `.dll`, `.dylib`, or `.so` for Windows, macOS, and Linux respectively.

## Current result

Apple M1 Mac mini, 8 GB, arm64, Rust 1.96.0-nightly:

| Lane | Median | Throughput |
| --- | ---: | ---: |
| Handwritten fixed nalgebra | 102.6 ns/turn | 9.750 MHz |
| Mech AOT generated Rust | 107.8 ns/turn | 9.277 MHz |
| Handwritten fixed nalgebra, 1,024 filters | 74.891 ns/filter | 13.353 M filter-turns/s |
| Mech AOT generated Rust, 1,024 filters | 81.888 ns/filter | 12.212 M filter-turns/s |
| Mech resident, full summary | 1,189.3 ns/turn | 0.841 MHz |
| Mech resident, no summary | 1,133.2 ns/turn | 0.882 MHz |

The scalar generated EKF again matched the fixed-size nalgebra reference with zero absolute error over the deterministic 4,096-turn stream. The outer-lifted program compiles the same natural 101-node matrix graph once rather than compiling a manually expanded component graph. All 1,024 lanes matched independent nalgebra filters for 256 turns with zero absolute error in the recorded release run. Scalar frontend and artifact construction took 59.6 ms, while scalar native compilation took 175.7 ms. The outer-lift frontend took 36.0 ms and its native compilation took 160.2 ms.

The batch result is 1.32x the scalar Mech throughput per filter and 9.3% slower than looping over 1,024 fixed-nalgebra Rust filters in the same harness. This is not a SIMD result: arm64 assembly still uses scalar floating-point operations around the transcendental library calls, although LLVM combines adjacent sine and cosine operations into `sincos`. The per-filter gain over scalar Mech comes from a single native batch boundary, structure-of-arrays state, and direct scalarized equations.

The branch is rebased on the current `feat/general-pure-resident-execution`
executor at commit `0f6c89e9a`.

These are prototype results, not a stable cross-machine score. The useful conclusion is architectural: build-time scalarization of the ordinary typed artifact removes almost all of the gap without adding an EKF-specific engine operation.

## Current boundary

This proves that scalar and lane-wise dense, pure, fixed-shape programs can share one typed lowering and native backend. `mech-build` now also activates the semantic artifact and partitions the turn graph into maximal connected numeric regions. Each region contains ordered typed instructions, embedded constants, resolved slot shapes and arena offsets, and explicit runtime live-ins/live-outs. An integration test proves that an unsupported `pow` node becomes a diagnosed fallback boundary while its downstream assignment remains compilable.

The batch detector currently admits only uniform `1 x N` state plus scalar or `1 x N` temporaries and lane-wise operations. A production `mech build` backend still needs:

- a compiled-region executor hook that can stage multiple slot writes before publication;
- a runtime installation adapter that connects host observations to a resident instance;
- a stable numeric-op identity in the catalog or artifact (the prototype currently maps concrete operation names once during build analysis);
- guarantee profiles for checked, transactional, and unchecked execution;
- cache keys and invalidation based on program revision, shapes, catalog versions, and target features;
- source spans and precise diagnostics for unsupported operations;
- tests for dynamic shapes, fallible operations, constraints, and mixed native/resident execution.

PR-D does not need to wait for this work. Its artifact, activation, typed arenas, and published/candidate state model are the correct substrate and the region analyzer passes unchanged on its current head. The compiled-region and runtime-install hooks can remain additive follow-up APIs. The important constraint is not to treat the current one-output `BoundResidentKernel` as the only permanent extension point: the natural EKF region has two live state outputs, and committing both must remain atomic.
