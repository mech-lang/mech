# AOT dynamic-library control

This directory contains a hand-written Rust implementation of the checked
scalar EKF turn ABI exported by the Mech Cranelift AOT backend, plus a minimal
common loader. The loader also understands the packed symbol exported by the
four-lane Mech AOT backend. It measures steady-state throughput and
whole-process peak resident memory without including either compiler in the
measured process.

The control is deliberately separate from the packed-SIMD/eight-worker Rust
program used by the source-size result. This library uses one thread and no
explicit cross-filter SIMD. LLVM nevertheless fuses paired sine/cosine calls
and SLP-vectorizes independent arithmetic within a filter. Comparing the AOT
library directly to the eight-worker Rust maximum would conflate compiler and
execution strategy.

On macOS, build both pieces with:

```sh
mkdir -p target/iros-rust-dylib
rustc --crate-type cdylib -C opt-level=3 -C target-cpu=native \
  -o target/iros-rust-dylib/librust_ekf.dylib \
  benchmarks/iros-2026/rust-dylib/rust-ekf-dylib.rs
rustc -C opt-level=3 -C target-cpu=native \
  -o target/iros-rust-dylib/dylib-runner \
  benchmarks/iros-2026/rust-dylib/dylib-runner.rs
```

Run either library at 10,000 filters for 200 checked turns. The common loader
first runs 100 untimed turns, then resets all state before starting the timer:

```sh
target/iros-rust-dylib/dylib-runner path/to/library.dylib 10000 200
```

For a packed SIMD AOT library, select its symbol and layout explicitly:

```sh
target/iros-rust-dylib/dylib-runner path/to/simd-library.dylib 10000 200 --simd
```

Pass a second scalar library path before `--simd` to validate the complete
logical final state against it.
Peak RSS is measured by running the same command under `/usr/bin/time -l` in a
fresh process. `../measure_dylib_comparison.py` automates the build, alternates
the run order, and emits JSON retaining every throughput and RSS sample rather
than only an average.
