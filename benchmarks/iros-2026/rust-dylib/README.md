# AOT dynamic-library control

This directory contains a hand-written Rust implementation of the same
checked scalar EKF turn ABI exported by the Mech Cranelift AOT backend, plus a
minimal common loader. The loader supplies identical input and ping-pong state
buffers to either library, so steady-state throughput and whole-process peak
resident memory can be compared without including the Mech compiler in the
measured process.

The control is deliberately separate from the packed-SIMD/eight-worker Rust
program used by the source-size result. This library is the one-thread scalar
control for the AOT artifact; comparing the AOT library directly to the
eight-worker Rust maximum would conflate compiler and execution strategy.

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

Pass a second library path to validate the complete final state against it.
Peak RSS is measured by running the same command under `/usr/bin/time -l` in a
fresh process. `../measure_dylib_comparison.py` automates the build, alternates
the run order, and emits JSON retaining every throughput and RSS sample rather
than only an average.
