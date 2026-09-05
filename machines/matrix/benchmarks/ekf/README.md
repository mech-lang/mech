# EKF steady-state runtime benchmark

This benchmark runs a persistent three-state, bearing-only extended Kalman
filter (EKF) in a warmed application loop. Setup, parsing, specialization,
bytecode compilation, input generation, and correctness validation are outside
the timed region.

The filter estimates robot pose `[x, y, theta]` and retains a `3 x 3`
covariance matrix. Each turn performs:

1. nonlinear unicycle motion prediction;
2. `3 x 3` motion and `3 x 2` control Jacobian covariance propagation;
3. bearing prediction to a fixed landmark and wrapped measurement innovation;
4. observation Jacobian, innovation variance, and Kalman gain calculation;
5. state correction and Joseph-form covariance update.

The dedicated [ekf.mec](ekf.mec) fixture is used instead of the educational
`examples/working/ekf.mec` document. The educational example is a one-shot
derivation and does not model a persistent host-driven loop. Its correction
also uses the predicted bearing directly rather than a wrapped measurement
innovation, so it is not a suitable cross-language benchmark fixture.

## Compared loops

- `raw-rust-loop`: fixed-size nalgebra matrices in a retained Rust `Ekf`.
- `mech-runtime-source`: source parsed and specialized once, followed by
  retained `apply_host_input_with_context` turns with atomic rollback.
- `mech-runtime-source-fail-stop`: the same retained runtime with successful
  turn rollback snapshots disabled.
- `python-loop`: persistent pure-Python lists and matrix operations.
- `numpy-loop`: persistent NumPy state with idiomatic small-array operations.
- `lua-loop`: persistent flat Lua matrices with generic matrix operations.
- `luajit-loop`: the identical Lua source under LuaJIT.
- `taichi-array-kernel`: the same recurrence with one persistent `f32` state
  and covariance per lane in Taichi fields.  Its `@ti.kernel` owns the outer
  lane loop and can target Taichi's CPU backend or Metal on Apple Silicon.

Mech receives four scalar host updates per turn: a pulse, linear velocity,
angular velocity, and measured bearing. Its pose and covariance remain inside
the runtime. The timed Mech region includes host packet construction,
admission, dependency bookkeeping, dirty scheduling, matrix/scalar execution,
register commit, and (except in fail-stop mode) rollback snapshot work. The
other runners time sample selection, the EKF calculation, and persistent state
replacement in their ordinary application loop.

Source installation does not mean that the source is reparsed or interpreted
on every turn. Both installation paths are intended to retain the same
specialized reactive plan; only installation should differ.

## Bytecode status

The harness compiles and installs the fixture as bytecode, but rejects that
lane before timing. Bytecode v1 currently does not reconstruct the activation
scope's sampled dependencies and register initialization semantics. For this
stateful fixture, installation advances the covariance before the first host
turn. Timing that plan would compare a different, numerically incorrect
program.

`--check` validates 256 source-installed turns element by element against raw
Rust and reports the bytecode rejection. `--bytecode` fails intentionally with
the semantic mismatch instead of emitting a benchmark row. A bytecode result
should be added only after the format and installer preserve activation plan
topology.

## Deterministic input

All inputs are built before timing. A 4,096-sample circular trajectory starts
at `[45, 15, 0]`. Linear and angular velocities have low-amplitude periodic
modulation, and bearing measurements include deterministic sinusoidal noise.
Every runner cycles this same formula-derived stream. The estimate starts at
`[55, 25, 0.4]`, with covariance `diag(100, 100, 0.15)`.

The Python script validates pure Python and NumPy against each other for 256
turns before measuring. The Lua script validates against the same 256-turn
reference. The Rust harness validates all state and covariance elements, not
only the emitted checksum.

## Measurement protocol

Each result is the median of nine samples after at least 250 ms of warmup.
Sample batches target 75 ms. Python's cyclic garbage collector is disabled only
during timed samples; normal reference-count destruction and NumPy temporary
allocation remain measured. The Lua collector stays enabled because the EKF
creates temporary tables and disabling its only reclamation mechanism would not
represent a steady-state loop. Rust and Mech use their normal allocation
behavior.

Run the benchmarks sequentially:

```sh
cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example ekf_runtime_benchmark

python3 machines/matrix/benchmarks/ekf/python_benchmark.py python

VECLIB_MAXIMUM_THREADS=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 \
  python3 machines/matrix/benchmarks/ekf/python_benchmark.py numpy

lua machines/matrix/benchmarks/ekf/lua_benchmark.lua lua

luajit machines/matrix/benchmarks/ekf/lua_benchmark.lua luajit
```

Run the semantic checks with:

```sh
cargo run --manifest-path machines/matrix/Cargo.toml \
  --example ekf_runtime_benchmark -- --check
```

Run the Taichi array-kernel lane (Python 3.12 is required by the current
Taichi wheel; install it in a virtual environment rather than replacing the
system Python):

```sh
/opt/homebrew/bin/python3.12 -m venv /tmp/taichi-ekf-venv
/tmp/taichi-ekf-venv/bin/python -m pip install taichi

/tmp/taichi-ekf-venv/bin/python machines/matrix/benchmarks/ekf/taichi_benchmark.py \
  --backend cpu --instances 100000 --turns 2000 --samples 5

/tmp/taichi-ekf-venv/bin/python machines/matrix/benchmarks/ekf/taichi_benchmark.py \
  --backend gpu --instances 100000 --turns 2000 --samples 5

# UI/host-loop latency variant: a device barrier after every turn
/tmp/taichi-ekf-venv/bin/python machines/matrix/benchmarks/ekf/taichi_benchmark.py \
  --backend gpu --instances 100000 --turns 500 --samples 5 --sync-each-turn
```

The Taichi lane is a compute-throughput comparison, not a claim of equivalent
runtime guarantees: inputs are preloaded into fields, there is no host packet
admission or rollback, and the supplied Taichi program does not implement the
Mech finite/positive/symmetric covariance constraints.  First-call kernel
compilation, reset, and final readback are outside the timed region.  No
Python-level allocation or host readback occurs inside a measured turn.  The
default GPU lane synchronizes once per measured batch; `--sync-each-turn`
reports the host/device barrier cost of a one-turn loop.  See
[RESULTS_APPLE_M1_2026-08-30.md](RESULTS_APPLE_M1_2026-08-30.md) for the
recorded run and the scalar-vs-parallel interpretation.

Recorded hardware runs:

- [Apple M1, 2026-08-06](RESULTS_APPLE_M1_2026-08-06.md)
- [Apple M1 Taichi array kernel, 2026-08-30](RESULTS_APPLE_M1_2026-08-30.md)
