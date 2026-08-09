# N-body cross-language benchmark

This suite uses the Computer Language Benchmarks Game five-body Jovian-system
task as a submission baseline and keeps whole-program and retained-runtime
measurements separate.

## Programs

The files under `benchmarksgame/` are unmodified copies from the official
Benchmarks Game source archive at commit
`40296663ed350d5fe4a6ab5e367bab61cb77c219`:

- Rust #2: portable scalar Rust;
- Python 3 #1: the standard pure-Python implementation;
- Lua #2: the optimized scalar Lua implementation;
- Julia #5: the optimized Julia implementation.

They retain their original contributor notices and are covered by the copied
BSD 3-Clause license. LuaJIT runs the exact Lua #2 source. The fastest current
Rust #9 program is not used because it requires x86 SSE/AVX intrinsics and
cannot compile on the Apple Silicon benchmark host.

`numpy_benchmark.py` is a custom NumPy implementation of the same integrator.
It preallocates its work arrays and uses incidence/weight matrices to evaluate
the ten body pairs. It is not an official Benchmarks Game program.

`nbody.mec` is a scalar, unrolled Mech implementation of those same ten pairs.
The Rust host in `examples/nbody_runtime_benchmark.rs` installs it into a
persistent runtime and drives one simulation step per live host-input turn.
The timed Mech region includes input admission, dirty dependency scheduling,
all scalar nodes, state staging/commit, and rollback or fail-stop transaction
semantics. Source parsing, specialization, bytecode compilation, installation,
and correctness validation are untimed.

Source-installed Mech is not reparsed or interpreted every turn. It and a
bytecode-installed program are expected to reconstruct specialized reactive
plans. The current bytecode v1 installer does not preserve this fixture's
activation sampling/register topology: the harness detects a large state error
after 1,000 turns and rejects the bytecode lane instead of timing incorrect
work. This is the same limitation detected by the EKF fixture.

## Correctness

Every whole-program runner must print the same initial and final energy as the
portable Rust program to an absolute tolerance of `1e-8`. At 1,000 steps the
required output is:

```text
-0.169075164
-0.169087605
```

The Mech harness compares all 30 evolving position/velocity scalars to a raw
Rust reference. Both atomic and fail-stop source lanes pass after warmup and
all timed turns.

## Protocol

The canonical lane times a fresh process, matching the Benchmarks Game. It
therefore includes Python/NumPy imports and Julia startup/JIT compilation.
Rust compilation is outside timing. All programs run sequentially with one
requested BLAS/Julia thread, retain normal garbage-collector behavior, and are
restarted for every repetition. The exact scalar programs allocate no growing
per-step log; the NumPy implementation reuses all large work buffers.

Run the portable validation/workload size used by the game:

```sh
python3 machines/matrix/benchmarks/nbody/run.py \
  --steps 500000 --repetitions 5
```

Run the official performance input before submitting:

```sh
python3 machines/matrix/benchmarks/nbody/run.py \
  --steps 50000000 --repetitions 3
```

Run the retained Mech application loop:

```sh
cargo run --release --manifest-path machines/matrix/Cargo.toml \
  --example nbody_runtime_benchmark
```

The command runner accepts `--python`, `--lua`, `--luajit`, `--julia`, and
`--rustc` paths so all version choices are explicit.

## Apple M1 results

The 5,000,000-step run on 2026-08-08 used Rust nightly 1.96.0, Python 3.14.6,
NumPy 2.5.1, Lua 5.5.1, LuaJIT 2.1.1785763465, and Julia 1.12.6 on macOS
15.6.1. Values are whole-process seconds; lower is better.

| Runtime | Median | Min | Max |
| --- | ---: | ---: | ---: |
| Rust #2 | 0.134606 | 0.134047 | 0.427307 |
| LuaJIT running Lua #2 | 0.454088 | 0.453932 | 0.454130 |
| Julia #5 | 0.464840 | 0.464588 | 0.466165 |
| Lua #2 | 9.641558 | 9.413402 | 10.068439 |
| custom NumPy | 25.378926 | 25.206613 | 25.578364 |
| Python #1 | 26.623993 | 26.604422 | 26.632531 |

The NumPy result is the useful weakness here: vectorization cannot amortize
dispatch over only five bodies and ten pairs, so its sequence of tiny ufunc and
matrix operations is only 4.7% faster than the scalar Python program. This is
not evidence that NumPy is generally slow; it is a small-fixed-system boundary.

The retained Mech loop measured `0.579316 ms/step` atomic and `0.601144
ms/step` fail-stop in one run. Their wide, overlapping ranges make that
inversion noise, not a rollback conclusion. These per-turn results should not
be placed in the whole-process table: Mech deliberately crosses the complete
host/runtime transaction boundary for every simulation step, while the game
programs keep their inner loops inside one language function.

Raw CSV is recorded in `RESULTS_APPLE_M1_2026-08-08.csv` (500,000 steps) and
`RESULTS_APPLE_M1_2026-08-08_5M.csv` (5,000,000 steps).
