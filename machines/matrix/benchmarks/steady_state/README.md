# Matrix steady-state benchmark

This benchmark compares persistent, warmed matrix multiplication and linear
solve execution across Mech, raw Rust/nalgebra, Python, NumPy, Lua, and LuaJIT.
Parsing, specialization, process startup, input generation, and correctness
validation are outside the timed regions.

Mech is reported in two modes:

- `mech-kernel`: repeated virtual calls to the specialized function's
  `solve_result` method.
- `mech-reactive`: repeated dirty-cell scheduling directly on the same
  persistent `ReactivePlan`. This is a plan-level control, not a complete
  `MechRuntime` turn.

`rectangular_numpy_benchmark.py` reports general `numpy.matmul` for every shape.
For `K = 1`, it additionally reports the equivalent optimized broadcast
multiplication as `numpy-outer`; this avoids claiming a general NumPy loss when
a knowledgeable caller can select the rank-one operation explicitly.

Matrix multiplication reuses the output allocation in every runtime. Linear
solve includes factorization on every iteration, matching the current Mech
implementation. Python's cyclic GC and Lua's collector are disabled only
during timed samples; the runners reuse their large scratch buffers.

Each result is the median of nine samples after at least 250 ms of warmup.
Sample batches target 75 ms and all runners emit the same CSV columns.
Example commands:

```sh
cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example steady_state_benchmark --no-default-features \
  --features compiler,source,f64,matrixd,vectord,matmul,solve -- 64 128 256 512

python3 machines/matrix/benchmarks/steady_state/python_benchmark.py \
  numpy 64 128 256 512

python3 machines/matrix/benchmarks/steady_state/rectangular_numpy_benchmark.py \
  1024x1x1024 2048x1x2048

lua machines/matrix/benchmarks/steady_state/lua_benchmark.lua \
  lua 64 128 256
```

Set `VECLIB_MAXIMUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, and
`OMP_NUM_THREADS=1` for the primary single-threaded NumPy comparison.

Recorded hardware runs:

- [Apple M1, 2026-08-05](RESULTS_APPLE_M1_2026-08-05.md)
