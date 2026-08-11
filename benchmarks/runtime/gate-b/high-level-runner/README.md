# High-level EKF resident and AOT prototype

This benchmark compiles [`../ekf-high-level.mec`](../ekf-high-level.mec) through the normal Mech source compiler. It then exercises two backends from the same `ProgramArtifact`:

- the schema-driven resident executor;
- an experimental ahead-of-time Rust emitter that invokes `rustc -O` and loads the resulting native library.

The AOT emitter does not recognize the EKF or call a handwritten EKF kernel. It walks artifact nodes and emits scalarized code for the f64 operations used by the program: fixed-shape matrix multiplication, transpose, concatenation, dot product, elementwise arithmetic, trigonometry, and assignment. Unsupported operations fail the build with their operation name.

The generated ABI reads a published state buffer and writes a separate candidate state buffer. This preserves the memory layout needed for rollback, but the prototype does not yet emit integrity checks, dirty tracking, receipts, or publication records.

## Run

```console
CARGO_TARGET_DIR=/tmp/mech-target \
MECH_AOT_DIR=/tmp/mech-aot \
cargo run --release --offline \
  --manifest-path benchmarks/runtime/gate-b/high-level-runner/Cargo.toml
```

`MECH_AOT_DIR` retains the generated `.rs` file and native library for inspection. The command requires `rustc` on `PATH`. The loader selects `.dll`, `.dylib`, or `.so` for Windows, macOS, and Linux respectively.

## Initial result

Apple M1 Mac mini, 8 GB, arm64, Rust 1.96.0-nightly:

| Lane | Median | Throughput |
| --- | ---: | ---: |
| Handwritten fixed nalgebra | 102.0 ns/turn | 9.800 MHz |
| Mech AOT generated Rust | 107.8 ns/turn | 9.279 MHz |
| Mech resident, full summary | 1,188.1 ns/turn | 0.842 MHz |
| Mech resident, no summary | 1,157.7 ns/turn | 0.864 MHz |

The generated lane matched the fixed-size nalgebra reference with zero absolute error over the deterministic 4,096-turn stream. On this run frontend plus artifact construction took 64.0 ms, and native `rustc -O` compilation took 163.1 ms. The generated file was 243 lines of straight-line scalar Rust with no nalgebra dependency or handwritten EKF helper. The resident lanes use the current `feat/general-pure-resident-execution` executor at commit `c539987c3`.

These are prototype results, not a stable cross-machine score. The useful conclusion is architectural: build-time scalarization of the ordinary typed artifact removes almost all of the gap without adding an EKF-specific engine operation.

## Current boundary

This proves build-time native compilation for one dense, pure, fixed-shape region. A production `mech build` backend still needs:

- a stable native artifact and host ABI;
- phase and pure-region extraction instead of compiling the whole supported turn;
- guarantee profiles for checked, transactional, and unchecked execution;
- cache keys and invalidation based on program revision, shapes, catalog versions, and target features;
- source spans and precise diagnostics for unsupported operations;
- tests for dynamic shapes, fallible operations, constraints, and mixed native/resident execution.
