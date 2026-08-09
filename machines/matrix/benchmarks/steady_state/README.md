# Matrix steady-state benchmark

This benchmark compares persistent, warmed matrix multiplication and linear
solve execution across Mech, raw Rust/nalgebra, Python, NumPy, Lua, LuaJIT, and
Julia. The Julia runner additionally reports scale plus a materialized
transpose. Parsing, specialization, process startup, input generation, and
correctness validation are outside the timed regions.

Mech is reported in two modes:

- `mech-kernel`: repeated virtual calls to the specialized function's
  `solve_result` method.
- `mech-reactive`: repeated dirty-cell scheduling directly on the same
  persistent `ReactivePlan`. This is a plan-level control, not a complete
  `MechRuntime` turn.

The separate `runtime_loop_benchmark` reports two complete retained-runtime
modes:

- `mech-runtime-source`: a persistent program installed from Mech source.
- `mech-runtime-bytecode`: the same source compiled once, installed into a
  fresh runtime from bytecode, and retained.

Both runtime modes time `apply_host_input_with_context`, including live input
admission, reactive transaction/savepoint work, dirty scheduling, execution,
turn validation, and commit. Source parsing, bytecode compilation, bytecode
decoding, installation, and initial execution are untimed. A toggled scalar
input drives a matrix-scaling node so the matrix kernel is dirty every turn.
The runtime uses trusted local limits so large setup and turns are not rejected
by the default one-second safety limit.

`rectangular_runtime_benchmark` applies the same retained-runtime protocol to
`M x K` by `K x N` products and also reports raw nalgebra, the persistent Mech
function, and direct reactive scheduling. Its default mode requires source and
bytecode fixtures. `--source-only` supports result matrices whose serialized
bytecode exceeds bytecode v1's 64 MiB read limit, while `--direct-only` measures
only the raw, kernel, and plan-level controls.

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
The runtime-loop runner measures source and bytecode as paired samples and
alternates their order to limit thermal and performance/efficiency-core bias.

Example commands:

```sh
cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example steady_state_benchmark --features runtime_default -- 64 128 256 512

cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example runtime_loop_benchmark --no-default-features \
  --features compiler,source,f64,matrixd,vectord,matmul,solve -- 64 128 256 512

cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example rectangular_runtime_benchmark --no-default-features \
  --features compiler,source,f64,matrixd,matmul -- \
  1024x1x1024 2048x1x2048

cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example rectangular_runtime_benchmark --no-default-features \
  --features compiler,source,f64,matrixd,matmul -- \
  --source-only 3072x1x3072 4096x1x4096

python3 machines/matrix/benchmarks/steady_state/python_benchmark.py \
  numpy 64 128 256 512

python3 machines/matrix/benchmarks/steady_state/rectangular_numpy_benchmark.py \
  1024x1x1024 2048x1x2048

lua machines/matrix/benchmarks/steady_state/lua_benchmark.lua \
  lua 64 128 256

JULIA_NUM_THREADS=1 julia --startup-file=no \
  machines/matrix/benchmarks/steady_state/julia_benchmark.jl \
  64 128 256 512
```

Set `VECLIB_MAXIMUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, and
`OMP_NUM_THREADS=1` for the primary single-threaded NumPy comparison.

Recorded hardware runs:

- [Apple M1, 2026-08-05](RESULTS_APPLE_M1_2026-08-05.md)
