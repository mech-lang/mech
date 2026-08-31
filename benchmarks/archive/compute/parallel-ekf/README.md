# Parallel EKF backend and scalar-language comparison

This benchmark produces two distinct comparisons from the high-level EKF in
`../../fixtures/ekf-kernel.mec`.

## Exact checkout and source map

The complete spike is on branch `codex/mech-program-gpu` and is reviewed in
[PR #757](https://github.com/mech-lang/mech/pull/757). It is not present in a
plain `integration/value-executor-v0.4` checkout.

```text
git fetch origin codex/mech-program-gpu
git switch --track origin/codex/mech-program-gpu
```

| Role | Checked-in source |
| --- | --- |
| High-level Mech EKF | `hosts/gpu/fixtures/ekf-kernel.mec` |
| Taichi-comparable Mech EKF | `hosts/gpu/fixtures/ekf-kernel-taichi-comparable.mec` |
| Mech artifact benchmark harness | `hosts/gpu/examples/parallel_ekf_benchmark.rs` |
| Generic scalar, SIMD, and WGPU lowering/execution | `hosts/gpu/src/batched.rs` |
| Cranelift lowering/execution | `hosts/gpu/src/batched/jit.rs` |
| Optimized Rust control | `hosts/gpu/examples/parallel_ekf_rust_scalar.rs` |
| Rust packed-lane SIMD control | `hosts/gpu/examples/parallel_ekf_rust_simd.rs` |
| NumPy control | `benchmarks/archive/compute/parallel-ekf/numpy_scalar.py` |
| Julia generic control | `benchmarks/archive/compute/parallel-ekf/julia_scalar.jl` |
| Julia fixed-shape control | `benchmarks/archive/compute/parallel-ekf/julia_flat.jl` |
| Julia packed-lane control | `benchmarks/archive/compute/parallel-ekf/julia_simd.jl` |
| Julia SIMD intrinsics control | `benchmarks/archive/compute/parallel-ekf/julia_simd_intrinsics.jl` |
| LuaJIT control | `benchmarks/archive/compute/parallel-ekf/luajit_scalar.lua` |
| NumPy batched fixed-shape control | `benchmarks/archive/compute/parallel-ekf/numpy_vectorized.py` |
| LuaJIT flat fixed-shape control | `benchmarks/archive/compute/parallel-ekf/luajit_fast.lua` |
| Controlled runner | `benchmarks/archive/compute/parallel-ekf/run.py` |
| Dependency-free chart renderer | `benchmarks/archive/compute/parallel-ekf/plot.py` |
| Matched Mech/Taichi chart renderer | `benchmarks/archive/compute/parallel-ekf/plot_runtime_comparison.py` |
| Correctness tests | `hosts/gpu/tests/parallel_ekf.rs` |

## Taichi parity harness

`taichi_comparable.py` is a checked-in control, not a hand-written result
stub. It uses Taichi `Vector.field` and `Matrix.field` values for the same
3-state/3x3-covariance resident layout, the same f32 constants and three
resident lane inputs, and the same Joseph covariance update as
`ekf-kernel-taichi-comparable.mec`. The unchecked kernel advances the resident
fields directly. The checked kernel uses two resident state/covariance pairs,
validates the complete candidate, and publishes the alternate pair only when
the candidate is valid. A failed lane records a two-word atomic fault summary
and the prior published pair remains selected. That is the Mech checked
publication contract, rather than a post-hoc assertion after overwriting
state.

Both modes call `ti.sync()` once per measured turn. This is intentional: it
measures a steady-state host-driven loop and does not let asynchronous device
work accumulate. Mech's checked path likewise maps the compact two-word fault
status before publishing each turn. For a device-resident batch comparison,
use Mech's explicit fused unchecked mode; it is a different boundary and is
reported separately.

The harness requires a Python version supported by the installed Taichi
release (the Apple run used Python 3.12 and Taichi 1.7.4):

```text
python3 -m venv .venv312
.venv312/bin/python -m pip install "taichi==1.7.4" "numpy>=2"
TAICHI_ARCH=metal .venv312/bin/python taichi_comparable.py 100000 120 unchecked
TAICHI_ARCH=metal .venv312/bin/python taichi_comparable.py 100000 120 checked
TAICHI_ARCH=metal .venv312/bin/python taichi_comparable.py 100000 120 unchecked-batched
```

For a CPU comparison, select Taichi's LLVM backend explicitly. `--cpu-threads
1` is the closest available SIMD-only control: it removes thread-level
parallelism, but Taichi still does not promise that every operation is emitted
as a vector instruction. Omit the option to use Taichi's default CPU worker
pool, or pin it to the machine's worker count when reproducing a run:

```text
TAICHI_ARCH=cpu .venv312/bin/python taichi_comparable.py 100000 20 unchecked --cpu-threads 1
TAICHI_ARCH=cpu .venv312/bin/python taichi_comparable.py 100000 20 checked --cpu-threads 1
TAICHI_ARCH=cpu .venv312/bin/python taichi_comparable.py 100000 20 unchecked --cpu-threads 8
TAICHI_ARCH=cpu .venv312/bin/python taichi_comparable.py 100000 20 checked --cpu-threads 8
```

The three-process median controls below were measured on the Apple M1 (8 logical
CPUs), with 100,000 resident filters and `ti.sync()` after every turn. The
single-worker rows isolate Taichi's LLVM lowering and any compiler
auto-vectorization from worker-pool parallelism; they are not a guarantee of a
particular SIMD width. The eight-worker rows include Taichi's CPU scheduling
and parallel outer loop.

| Taichi CPU mode | Million EKF-turns/s |
| --- | ---: |
| Unchecked, one worker | 23.616 |
| Checked, one worker | 20.695 |
| Unchecked, eight workers | 94.381 |
| Checked, eight workers | 82.607 |

The same resident Mech artifact on this checkout measured approximately 41.2M
unchecked-fast and 36.1M checked-fast turns/s through the four-lane Cranelift
SIMD-JIT path in one benchmark process. These are directional comparisons,
not a claim that Taichi's one-worker mode is a hand-written SIMD kernel:
Taichi receives an explicit `for i in range(N)` and lets LLVM decide its CPU
parallel/vector lowering, while Mech's SIMD lane width is explicit. Use the
worker count and synchronization policy in the result table whenever comparing
the two.

## Matched eight-worker CPU/GPU comparison

The parallel SIMD-JIT entry point partitions complete four-lane groups across
eight scoped workers. Workers join before the next turn begins, so this remains
a synchronous resident loop with the same state publication and fault boundary
as the single-worker path. The checked path retains the previously published
state when any worker reports an invalid candidate.

The chart below uses the same 500,000 resident filters, 40 measured turns,
three isolated process samples, and synchronization after every turn for both
runtimes. Mech's GPU rows use one host dispatch per turn; Taichi's Metal rows
call `ti.sync()` after each kernel turn. CPU rows use eight workers. Setup,
compilation, allocation, warmup, and final readback are excluded.

![Matched Mech and Taichi EKF throughput](results/apple-m1-mech-taichi-runtime-2026-08-31.svg)

| Runtime/backend | Checked | Unchecked |
| --- | ---: | ---: |
| Mech SIMD/JIT CPU, 8 workers | 104.783M/s | 110.469M/s |
| Taichi LLVM CPU, 8 workers | 86.047M/s | 98.140M/s |
| Mech WGPU GPU, per-turn dispatch | 152.972M/s | 157.141M/s |
| Taichi Metal GPU, per-turn sync | 179.504M/s | 222.210M/s |

The checked Mech GPU path is within 15% of the Taichi Metal control, while the
parallel checked and unchecked CPU paths exceed Taichi's eight-worker CPU
throughput. The remaining unchecked GPU gap is a launch/device-code tuning
target; it is not hidden by batching, because every row above waits at the
turn boundary. Raw medians and all individual samples are recorded in
`results/apple-m1-mech-taichi-runtime-2026-08-31.json`.

Regenerate the SVG from the checked-in measurements with:

```text
python3 plot_runtime_comparison.py \
  results/apple-m1-mech-taichi-runtime-2026-08-31.json \
  results/apple-m1-mech-taichi-runtime-2026-08-31.svg
```

The complete runner can execute both controls and compare their fresh-session
checksums with the Mech GPU results. It pins the Taichi backend explicitly and
records the raw process output when evidence is requested:

```text
python3 run.py --taichi-python /path/to/.venv312/bin/python \
  --taichi-arch metal --backend-instances 100000 --backend-gpu-turns 120 \
  --evidence-output results/apple-m1-taichi-parity.json
```

On the Apple M1/Metal sanity runs used while adding this harness, five
per-turn synchronized turns measured approximately 264M unchecked and 103M
checked Taichi filter-turns/s. The corresponding Mech generic WGPU path was
approximately 65M unchecked and 54--64M checked one-turn filter-turns/s, with
matching f32 checksums. Mech's ordinary unchecked multi-dispatch path reached
approximately 325M turns/s when five turns were encoded into one submission;
that is the relevant apples-to-apples comparison against a device-resident
Taichi batch, not the per-turn host boundary. Metal scheduling is noisy, so
new evidence must use the runner's medians rather than a single process.

`unchecked-batched` is the device-resident control: it advances all requested
turns inside one Taichi kernel and synchronizes once. It is compared with
Mech's `prepare_resident_unchecked_fused` result. It must not be compared with
the checked per-turn rows, because neither control performs a publication or
fault readback at every intermediate turn in that mode.

The result is not that Taichi has a capability Mech fundamentally lacks. Its
compiler receives the outer `for i in range(N)`, fixed field shapes, and a
device-selected kernel as explicit program structure. Mech currently derives
the same outer broadcast from array extents, scalarizes the generic artifact,
and preserves host-visible transaction boundaries. That gives Taichi more
room for loop fusion, matrix scalar replacement, and backend-specific launch
tuning. Mech can recover those opportunities by specializing the lowered
region and by making device-resident batching an explicit execution policy;
the source-level recurrence and its checked semantics do not need to change.

## Mech physical backends

The scalar CPU, four-lane SIMD CPU, Cranelift JIT, and GPU lanes execute the
same compiler artifact and persistent per-filter state. The SIMD
implementation changes only the physical value type of the scalarized
instruction stream to `wide::f32x4`; it uses NEON on Apple Silicon and SSE
where available. The JIT converts that instruction stream into one native SSA
function containing the complete outer filter loop. The primary GPU lane
submits and synchronizes one Mech turn at a time.

The Mech source itself declares `finite-candidate!`,
`positive-covariance!`, and `symmetric-covariance!` using generic numeric,
comparison, Boolean, and matrix-index operations. There is no EKF validation
primitive or separate Rust publication policy. Constraint names survive
artifact and bytecode encoding, affect artifact identity, and are reported by
structured faults. A failed candidate is rejected before the published buffer
changes. The session records only a fault count and latest named fault, so
fault evidence cannot grow an unbounded log. GPU turns read a compact device
fault status before advancing the published ping-pong buffer; checked
multi-turn calls therefore execute as repeated checked turns.

The table below is the preserved **unchecked** Apple M1 baseline from commit
`6b27e4cdbcdd53ddb0c646169be0bb597bd2a39e`: five-process median after one
discarded warmup, 100,000 filters, 2026-08-14. It predates the integrity policy
and must not be presented as checked throughput.

| Mech backend | Million EKF-turns/s | Scalar speedup |
| --- | ---: | ---: |
| Mech scalar artifact evaluator | 1.216 | 1.00x |
| Mech SIMD (`4xf32`) | 4.414 | 3.63x |
| Mech Cranelift JIT | 17.306 | 14.23x |
| Mech GPU, one submission/turn | 53.557 | 44.04x |
| Mech GPU, 120 turns/submission | 343.969 | 282.87x |

Parsing, artifact compilation, scalarization, JIT compilation, input
construction, allocation, GPU setup, warmup, final readback, and correctness
checks are outside the timed regions. Cranelift `0.131.3` is pinned because it
supports the repository's Rust `1.92` minimum. JIT preparation took `3.340 ms`
in the first recorded hardware run. Its state matched the scalar evaluator
bit-for-bit after four validation turns.

## Initial checked evaluated artifact

Commit `7605c5c9081a22d7bcba0b0c288570a7c3a41f41` compiles the three
source-authored constraints into every backend. Five release-mode processes
on Apple M1 Metal, with 100,000 filters, three scalar reference turns, 20
single GPU samples, and 120 repeated checked GPU turns, produced these
medians:

| Checked Mech backend | Time/turn | Million EKF-turns/s | Unchecked reference | Relative change |
| --- | ---: | ---: | ---: | ---: |
| Scalar artifact evaluator | 122.212 ms | 0.818 | 1.216 | -32.7% |
| SIMD (`4xf32`) | 31.702 ms | 3.154 | 4.414 | -28.5% |
| Cranelift JIT | 8.105 ms | 12.339 | 17.306 | -28.7% |
| GPU, one checked submission/turn | 1.942 ms | 51.497 | 53.557 | -3.8% |
| GPU, repeated checked turns | 1.767 ms | 56.580 | not comparable | not comparable |

Source parsing, artifact construction, and scalarization took a median
`107.022 ms`; JIT preparation took `3.573 ms`. Maximum CPU/GPU absolute error
was `6.866e-5`, and JIT output matched scalar output bit-for-bit. The Apple
Metal correctness suite passed all nine tests, including injected finite,
positive-diagonal, and symmetry failures and proof that an invalid GPU
candidate leaves the previous state published.

The old `343.969 M/s` GPU number is deliberately excluded from the overhead
calculation: it publishes only after a 120-turn command batch, while the
checked repeated lane validates before every publication. Comparing those
numbers would attribute a guarantee-boundary change to constraint arithmetic.
All five checked process samples are preserved in
[`results/apple-m1-checked-integrity-2026-08-14.json`](results/apple-m1-checked-integrity-2026-08-14.json).

## Optimized checked artifact

Commit `efc85d48e562fe4ccc1af535e04f9bf4617e05a6` keeps the same source
constraints, named faults, candidate rejection, bounded fault state, and
previous-estimate retention. It changes their generic execution strategy:

- constraint-only Boolean graphs compile as predicates rather than `f32`
  result registers;
- dead numeric instructions are removed after tracing state outputs and
  predicate inputs;
- `abs(x) <= f32::MAX` lowers to exact `f32` finiteness testing and
  `abs(left - right) <= tolerance` lowers to one predicate;
- SIMD comparisons remain native masks until the final fault decision; and
- JIT code returns its first packed fault instead of writing and rescanning a
  result for every filter.

Five isolated release processes on the same Apple M1 Metal adapter, with
100,000 filters, five scalar reference turns, 40 single GPU samples, and 120
repeated checked GPU turns, produced:

| Checked Mech backend | Time/turn | Million EKF-turns/s | Initial checked | Change | Unchecked reference | Remaining checked cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar artifact evaluator | 106.286 ms | 0.941 | 0.818 | +15.0% | 1.216 | -22.6% |
| SIMD (`4xf32`) | 26.233 ms | 3.812 | 3.154 | +20.9% | 4.414 | -13.6% |
| Cranelift JIT | 6.849 ms | 14.600 | 12.339 | +18.3% | 17.306 | -15.6% |
| GPU, one checked submission/turn | 1.630 ms | 61.348 | 51.497 | +19.1% | 53.557 | noisy |
| GPU, repeated checked turns | 1.820 ms | 54.959 | 56.580 | -2.9% | not comparable | not comparable |

Source parsing, artifact construction, and scalarization took a median
`64.134 ms`; JIT preparation took `3.494 ms`. Maximum CPU/GPU absolute error
remained `6.866e-5`, and JIT output remained bit-for-bit equal to scalar.
All 27 package tests passed, including the three injected integrity failures
and GPU publication retention. The generated WGSL shrank from 13,908 to
11,733 bytes as dead constraint arithmetic disappeared.

## Explicit unchecked GPU path

The benchmark now has a separate opt-in unchecked artifact. Calling
`FixedShapeKernel::without_integrity_constraints` removes the three source
predicates from the generated WGSL and removes the atomic fault binding. The
native session exposes two distinct measurements:

- `dispatch_turns(1)` is the one-turn, host-driven loop. It still waits for
  completion after every submission, but it performs no predicate or fault
  work on the device.
- `dispatch_turns(120)` on the same unchecked artifact batches 120 ordinary
  dispatches into one command submission. This isolates command/state traffic
  from validation without changing the one-turn kernel.
- `prepare_resident_unchecked_fused(..., turns)` plus
  `dispatch_unchecked_fused()` loads each lane once, advances the configured
  number of turns in device-local state, and writes the final state once. It
  uses one command submission and has no rollback boundary.

Both paths are checked against the generic CPU lowering before timing. On the
Apple M1, a 100,000-filter sanity run measured approximately `62 M/s` for the
unchecked one-turn loop and `3,900 M/s` when 120 unchecked turns were fused
inside one device invocation. The latter is deliberately reported as a
batched/device-resident result; it is not a one-turn kernel comparison. The
checked path remains the reference for per-turn publication and retains the
previous estimate on an invalid candidate.

GPU one-turn samples ranged from `47.859` to `62.495 M/s`; per-turn host
synchronization still dominates and makes this too noisy to attribute the
median change to device predicate lowering. Raw samples, command parameters,
implementation commit, and validation commands are preserved in
[`results/apple-m1-checked-integrity-optimized-2026-08-14.json`](results/apple-m1-checked-integrity-optimized-2026-08-14.json).

## Scalar outer-loop languages

Every lane owns 10,000 persistent filters and executes one filter at a time
for five warmup turns, a state reset, and 20 measured turns. Inputs, equations,
`f32` state, Joseph covariance update, and checksums agree. "Scalar" here means
the outer filter loop is sequential. It does not claim that a language's
scalar math or small matrix library avoids every SIMD instruction internally.

Apple M1 median of five processes after one discarded process warmup, except
seven LuaJIT samples:

| Scalar outer-loop lane | Million EKF-turns/s | Relative to Mech scalar |
| --- | ---: | ---: |
| Mech scalar artifact evaluator | 1.213 | 1.00x |
| Mech Cranelift JIT | 17.390 | 14.34x |
| Rust optimized fixed-shape | 20.299 | 16.73x |
| NumPy sequential small matrices | 0.055 | 0.05x |
| Julia sequential small matrices | 2.786 | 2.30x |
| LuaJIT sequential FFI `f32` state | 1.089 | 0.90x |

The Rust control permits inlining of the EKF step and its fixed-shape matrix
helpers. The previous `#[inline(never)]` control measured `12.947 M/s`, but it
was not a fair native-code ceiling once the JIT owned and fused the outer
filter loop. Under identical 10,000-filter, 20-turn settings, the JIT reaches
`85.7%` of the optimized Rust throughput.

Versions were Rust `1.96.0-nightly`, Python `3.14.6`, NumPy `2.5.2`, Julia
`1.12.6`, and LuaJIT `2.1.1785763465`. NumPy, Julia BLAS, and related native
thread counts were pinned to one.

The publication evidence is checked in as
[`results/apple-m1-2026-08-14.json`](results/apple-m1-2026-08-14.json). It was
generated from commit `6b27e4cdbcdd53ddb0c646169be0bb597bd2a39e` and contains
all discarded warmups and measured stdout. This file is retained as pre-policy
evidence rather than silently relabeled. The raw samples also show why these
figures remain provisional: synchronized GPU samples ranged from `48.613` to
`65.510 M/s`, while the JIT backend samples stayed between `17.225` and
`17.343 M/s` at the 100,000-filter setting.

Build the native Mech benchmark, then run the complete comparison:

```text
cargo build -p mech-gpu --release --features native,jit --example parallel_ekf_benchmark
python3 benchmarks/archive/compute/parallel-ekf/run.py --python /path/to/python-with-numpy
```

Add `--evidence-output /path/to/results.json` to record the exact Git commit,
platform, tool versions, thread environment, commands, discarded warmups,
every measured process stdout, parsed checksums, and summary medians. Published
results should include this generated JSON rather than only the tables above.

The runner compiles the checked-in Rust control directly with
`rustc -C opt-level=3 -C target-cpu=native`, validates every scalar and JIT
checksum, and prints both Markdown tables.

## Julia inlining probe

The Julia control in this checkout uses `Base.@inline` on `step!`.  This is a
deliberate optimization hint for the scalar outer loop, not a change to the
EKF equations or storage.  On the Apple M1, nine isolated 20-turn processes
with 10,000 filters produced `3.091M` lane-turns/s median.  The same source
with the annotation removed produced `2.875M`; the original `Base.@noinline`
source produced `2.852M`.  A longer 100-turn corroboration produced `3.069M`,
`2.820M`, and `2.800M`, respectively.  Checksums were identical across all
variants and one-process startup wall time remained within `1.57--1.59s`.  The
detailed commands and raw medians are in
[`results/julia-inline-apple-m1-2026-08-30.md`](results/julia-inline-apple-m1-2026-08-30.md).

The Julia comparison has four sequential modes plus two packed-lane
implementations. The generic source uses
ordinary heap-backed `Matrix` values and `mul!`, which is the closest
translation of the high-level Mech matrix expressions. The fixed-shape source
uses flat `Float32` buffers and compile-time 3x3/3x2 products, matching the
storage and operation shape of the optimized Rust control. Both sources accept
`unchecked` or `checked` as a third argument. Checked mode evaluates the same
finite-state, finite-covariance, positive-diagonal, and covariance-symmetry
predicates as the Mech artifact before publishing a candidate; a failed
candidate leaves the prior state unchanged and increments the fault count.
The runner executes all eight Julia rows plus scalar and packed-SIMD Rust
controls. The scalar Rust control remains an unchecked reference; the packed
Rust control has both checked and unchecked modes.

The source-shaped NumPy and LuaJIT controls remain available as
`numpy_scalar.py` and `luajit_scalar.lua`. Their companion fast lanes,
`numpy_vectorized.py` and `luajit_fast.lua`, batch the outer population and
replace generic matrix loops with fixed 3x3 products. Both accept `checked` or
`unchecked`: checked mode validates every candidate and publishes it only when
the finite, positive-diagonal, and symmetry predicates pass. The LuaJIT fast
lane keeps scalar intermediate registers, so its aggregate checksum is allowed
the same scale-aware `f32` tolerance recorded by the runner; this does not
change its state-update or validation policy.

In a five-process Apple M1 probe with 10,000 filters and 20 measured turns,
the current medians were:

| Julia implementation | Validation | Million lane-turns/s |
| --- | --- | ---: |
| Generic Matrix/`mul!` | unchecked | 3.09 |
| Generic Matrix/`mul!` | checked | 3.08 |
| Fixed-shape flat tuples | unchecked | 21.9 |
| Fixed-shape flat tuples | checked | 19.0 |

All four modes produced the same checksum within the existing `f32`
tolerance. The fixed-shape checked result is the relevant comparison to a
checked Mech numeric backend; the unchecked result isolates arithmetic and
storage cost only.

## Julia packed-lane comparison

`julia_simd.jl` gives Julia the same four-filter physical execution shape as
Mech's SIMD-JIT lane. It stores each state and covariance component as a
`StaticArrays.SVector{4,Float32}`, advances four independent filters per outer
iteration, uses Julia's `sincos` pair, and keeps the same checked-mode
finite/positive-diagonal/symmetry predicates. A fully valid group takes a
branch-only publication path; per-lane `ifelse` rollback is materialized only
when a lane fails. This is a fair packed-lane Julia comparison, while the
generic and fixed-shape sequential rows remain available to show the cost of
the language/runtime shape itself.

On this Apple M1 checkout with 100,000 filters and 100 measured turns (after
the script's five-turn warmup), the direct runs were:

| Julia implementation | Validation | Million lane-turns/s |
| --- | --- | ---: |
| Fixed-shape flat tuples | unchecked | 22.69 |
| Fixed-shape flat tuples | checked | 19.57 |
| Fixed-shape packed `SVector{4}` | unchecked | 37.72 |
| Fixed-shape packed `SVector{4}` | checked | 29.83 |
| Packed `SIMD.jl Vec{4,Float32}` | unchecked | 34.55 |
| Packed `SIMD.jl Vec{4,Float32}` | checked | 32.88 |

The packed source produced the same `26,697,851.679688` checksum as the flat
source (within `f32` accumulation rounding). The checked packed result is
therefore close to Mech's checked SIMD-JIT result on this run, rather than an
unchecked Julia-only advantage. The `StaticArrays` source needs `StaticArrays`;
the intrinsic source needs `SIMD.jl` (tested with SIMD.jl 3.7.2). Both are
ordinary Julia packages and must be installed in the Julia environment used by
the runner.

At the runner's 10,000-filter/20-turn setting, five isolated Julia intrinsic
processes measured a median of `31.34M` checked and `32.54M` unchecked
lane-turns/s. Five corresponding Mech SIMD-JIT processes measured `31.16M`
checked-fast and `32.65M` unchecked-fast. The remaining difference is within
normal process noise; this is now the relevant performance target for the
SIMD-capable path, not the sequential `19M` result.

## Rust packed-lane control and current cross-language chart

`parallel_ekf_rust_simd.rs` is a separate Rust ceiling control. It stores each
state and covariance component in structure-of-arrays form, advances four
filters with `wide::f32x4`, uses the same scalar transcendental fallback as the
current Mech Cranelift SIMD-JIT, and implements the same finite, positive
diagonal, and covariance-symmetry publication checks. It is therefore a real
Rust SIMD comparison, not a scalar Rust result relabeled as SIMD. It is still
specialized source: it does not demonstrate that the Rust compiler can infer
this layout from the high-level EKF automatically.

The current three-process Apple M1 evidence is recorded in
[`results/apple-m1-simd-cross-language-2026-08-30.json`](results/apple-m1-simd-cross-language-2026-08-30.json).
The chart below is generated only from that evidence file by `plot.py` and uses
one shared 0--60 million-turns/s axis:

![Parallel EKF cross-language throughput](apple-m1-simd-cross-language-2026-08-30.svg)

The checked-only view is available separately for reviews that require every
row to retain the integrity policy. The latest checked rerun uses the packed
SIMD-JIT Mech executor from the current branch:

![Parallel EKF checked throughput](apple-m1-checked-cross-language-2026-08-31.svg)

| Control | Validation | Million EKF-turns/s |
| --- | --- | ---: |
| Rust fixed-shape scalar | unchecked | 16.69 |
| Rust packed `wide::f32x4` | checked | 25.68 |
| Rust packed `wide::f32x4` | unchecked | 20.87 |
| Mech Cranelift SIMD-JIT | checked-fast | 37.21 |
| Mech Cranelift SIMD-JIT | unchecked-fast | 41.34 |
| Julia `SIMD.jl Vec{4,Float32}` | checked | 31.18 |
| Julia `SIMD.jl Vec{4,Float32}` | unchecked | 32.87 |
| NumPy vectorized fixed-shape | checked | 10.69 |
| NumPy vectorized fixed-shape | unchecked | 12.31 |
| LuaJIT flat fixed-shape | checked | 1.27 |
| LuaJIT flat fixed-shape | unchecked | 15.98 |

On this run, the specialized Rust control does **not** beat Julia's packed
SIMD control, and the new Mech packed SIMD-JIT is faster than both while
preserving the source-authored publication policy. These are implementation
results, not language limits: Rust, Julia, NumPy, and LuaJIT can each move
closer with a generated fixed-shape kernel and a matching packed layout.

To regenerate the chart from a new run:

```text
python3 plot.py results/apple-m1-simd-cross-language-2026-08-30.json results/apple-m1-simd-cross-language-2026-08-30.svg
python3 plot.py --checked-only results/apple-m1-checked-cross-language-2026-08-31.json results/apple-m1-checked-cross-language-2026-08-31.svg
```

## What "checked-fast" means

The Rust control currently has ordinary `checked` and `unchecked` modes. It
does not have a Rust-specific `checked-fast` mode because that would be a new
floating-point policy, not a free compiler switch. The Mech checked-fast path
keeps candidate validation, rollback, and fault reporting, but permits a
small set of arithmetic simplifications that are only valid under finite
inputs. It is **not** equivalent to applying unrestricted `-ffast-math`.

Unrestricted fast math can reassociate operations, contract multiplies and
adds, treat NaNs or infinities as impossible, change signed-zero behavior, and
replace transcendental functions with lower-accuracy approximations. In this
EKF, those changes can alter a residual, make a covariance fail symmetry or
positivity, or worse, hide an exceptional operand (for example, replacing
`0 * NaN` with `0`) before the integrity check sees it. A checked wrapper does
not make those arithmetic transformations safe by itself.

A defensible checked-fast policy is possible: validate all external and state
inputs before entering the fast region, restrict transformations to proofs
that hold for finite operands, retain the candidate finite/diagonal/symmetry
checks, and fall back to strict recomputation when the fast candidate fails.
The fallback must be armed before publication, and the fast path must not
silently erase NaN/Inf evidence. That policy is the next step if we want a
Rust checked-fast row; the current chart deliberately reports the measured
Rust checked row instead of inventing one.
