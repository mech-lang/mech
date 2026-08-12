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

`KernelIr` contains explicit f64 value types, fixed matrix shapes, constants, input ordinals, state bindings, and semantic operations. It contains no Rust expressions. The Rust backend consumes only this IR; it does not inspect the `ProgramArtifact`, resident plan, or Mech operation-name strings.

The lowering pass does not recognize the EKF or call a handwritten EKF kernel. It translates supported operations node by node: fixed-shape matrix multiplication, transpose, concatenation, dot product, elementwise arithmetic, trigonometry, and assignment. Unsupported nodes fail with a diagnostic that names both the node and operation, for example:

```text
node 17 (example/Unsupported) is not eligible for a native numeric kernel:
operation has no native numeric lowering
```

The generated ABI reads a published state buffer and writes a separate candidate state buffer. This preserves the memory layout needed for rollback, but the prototype does not yet emit integrity checks, dirty tracking, receipts, or publication records.

## Generality proof

The executable compiles and validates two unrelated Mech programs through the same lowering and backend code:

- [`../ekf-high-level.mec`](../ekf-high-level.mec): a 101-node nonlinear EKF with two persistent matrices;
- [`../numeric-kernel-proof.mec`](../numeric-kernel-proof.mec): a 17-node linear state-space recurrence with independent inputs and state.

The linear proof is not embedded in the emitter. Its 17 normal Mech artifact nodes become 17 kernel instructions and a separate native library. Over 4,096 turns, that library matches an independent recurrence with zero absolute error. Inspecting the generated source finds no EKF, nalgebra, or reference implementation names.

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
| Handwritten fixed nalgebra | 101.9 ns/turn | 9.809 MHz |
| Mech AOT generated Rust | 107.8 ns/turn | 9.277 MHz |
| Mech resident, full summary | 1,200.3 ns/turn | 0.833 MHz |
| Mech resident, no summary | 1,166.4 ns/turn | 0.857 MHz |

After introducing the typed kernel IR, the generated EKF lane measured 107.8 ns/turn and again matched the fixed-size nalgebra reference with zero absolute error over the deterministic 4,096-turn stream. The IR therefore added no measurable steady-state overhead. Native compilation took 166.1 ms for the EKF and 144.3 ms for the independent linear proof on this run. The resident lanes use the current `feat/general-pure-resident-execution` executor at commit `c539987c3`.

These are prototype results, not a stable cross-machine score. The useful conclusion is architectural: build-time scalarization of the ordinary typed artifact removes almost all of the gap without adding an EKF-specific engine operation.

## Current boundary

This proves that multiple dense, pure, fixed-shape programs can share one typed lowering and native backend. It still compiles the complete eligible program rather than extracting supported regions from a mixed program. A production `mech build` backend still needs:

- a stable native artifact and host ABI;
- phase-aware pure-region extraction with resident fallback at unsupported boundaries;
- guarantee profiles for checked, transactional, and unchecked execution;
- cache keys and invalidation based on program revision, shapes, catalog versions, and target features;
- source spans and precise diagnostics for unsupported operations;
- tests for dynamic shapes, fallible operations, constraints, and mixed native/resident execution.

The next implementation step is to partition the artifact into maximal eligible numeric regions and describe their live-in/live-out values. The current `KernelIr` can then represent each region, allowing the resident executor to call compiled kernels without requiring the entire turn to be native-compatible.
