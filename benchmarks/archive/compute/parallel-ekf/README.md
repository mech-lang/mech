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
| Mech artifact benchmark harness | `hosts/gpu/examples/parallel_ekf_benchmark.rs` |
| Generic scalar, SIMD, and WGPU lowering/execution | `hosts/gpu/src/batched/mod.rs` |
| Cranelift lowering/execution | `hosts/gpu/src/batched/jit.rs` |
| Direct Metal resident execution (macOS, opt-in) | `hosts/gpu/src/batched/metal.rs` |
| Optimized Rust control | `hosts/gpu/examples/parallel_ekf_rust_scalar.rs` |
| NumPy control | `benchmarks/archive/compute/parallel-ekf/numpy_scalar.py` |
| Julia control | `benchmarks/archive/compute/parallel-ekf/julia_scalar.jl` |
| Revised Julia SoA control | `benchmarks/archive/compute/parallel-ekf/julia_mojo_style.jl` |
| Revised Taichi SoA control | `benchmarks/archive/compute/parallel-ekf/taichi_mojo_style.py` |
| LuaJIT control | `benchmarks/archive/compute/parallel-ekf/luajit_scalar.lua` |
| PyPy textbook-fidelity control | `benchmarks/archive/compute/parallel-ekf/pypy_textbook.py` |
| PyPy optimized control | `benchmarks/archive/compute/parallel-ekf/pypy_optimized.py` |
| Mojo textbook-fidelity control | `benchmarks/archive/compute/parallel-ekf/mojo_textbook.mojo` |
| Mojo textbook fixed-matrix control | `benchmarks/archive/compute/parallel-ekf/mojo_textbook_fixed.mojo` |
| Mojo expanded scalar control | `benchmarks/archive/compute/parallel-ekf/mojo_scalar.mojo` |
| Controlled runner | `benchmarks/archive/compute/parallel-ekf/run.py` |
| Correctness tests | `hosts/gpu/tests/parallel_ekf.rs` |

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

### PyPy controls

`pypy_textbook.py` is the baseline: ordinary Python lists, generic matrix
products, and direct prediction/update phases. `pypy_optimized.py` is the
optimized control: structure-of-arrays float32 storage, expanded fixed-shape
products, and allocation-free checked publication. Both are pure Python; they
do not import NumPy or call a native matrix library. The optional `checked`
mode performs the same finite-state, positive-diagonal, and symmetry checks as
the Mech control and retains the previous candidate on failure.

Run it with a PyPy 3 interpreter:

```text
pypy3 pypy_textbook.py 10000 20 checked
pypy3 pypy_optimized.py 10000 20 checked
```

To include both interpreters in the controlled language runner, pass
`--pypy /path/to/pypy3`. The runner executes each identical source once under
CPython and once under PyPy, records both interpreter versions, and preserves
all raw outputs in its evidence JSON.

The controlled scalar rerun used the archive's 10,000-filter x 20-turn
workload, five measured processes after one discarded warmup, and the timing
boundary above. On PyPy 7.3.23 and the matching CPython interpreter, the
scalar medians were:

| Identical source lane | Checked M/s | Unchecked M/s |
| --- | ---: | ---: |
| CPython textbook-fidelity | 0.027 | 0.028 |
| PyPy textbook-fidelity | 0.097 | 0.102 |
| CPython optimized SoA/fixed-shape | 0.272 | 0.307 |
| PyPy optimized SoA/fixed-shape | 1.572 | 1.569 |

All samples reported zero faults. The textbook lane intentionally retains
generic nested-list matrix operations; the optimized lane uses the same
fixed-shape SoA source under both interpreters. Raw stdout, interpreter
versions, machine metadata, and source hashes are preserved in
[`results/apple-m1-pypy-2026-09-05.json`](results/apple-m1-pypy-2026-09-05.json).

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

GPU one-turn samples ranged from `47.859` to `62.495 M/s`; per-turn host
synchronization still dominates and makes this too noisy to attribute the
median change to device predicate lowering. Raw samples, command parameters,
implementation commit, and validation commands are preserved in
[`results/apple-m1-checked-integrity-optimized-2026-08-14.json`](results/apple-m1-checked-integrity-optimized-2026-08-14.json).

## Revised Mojo-style execution controls

The current implementation applies the reusable parts of the fast Mojo
representation to the Mech artifact rather than adding an EKF-specific
operation. Fixed-shape lowering now folds constant aliases and preserves
IEEE-sensitive products before backend generation, then prunes the resulting dead scalar
instructions. The Cranelift session stores every input/state component in a
contiguous component-major (SoA) buffer and materializes the existing
instance-major state view only at the publication/read boundary. Its native
turn ABI accepts an instance range, so the same compiled function can run over
disjoint ranges on multiple workers while preserving one checked publication
decision for the whole turn. `without_integrity_constraints()` is an explicit
control-lane opt-out; normal Mech sessions retain all constraints and fault
reporting.

The optimization is generic: it applies to fixed-shape programs containing
matrix products, broadcasts, state assignments, and predicates. It does not
recognize the EKF or replace `atan2`, and it leaves the source artifact and
bytecode representation unchanged. The public state snapshot remains in the
canonical Mech layout, so CPU, JIT, and GPU outputs continue to compare
against one another.

The revised Julia and Taichi controls use the same lowered representation:
explicit scalar SoA fields, a fused fixed-shape recurrence, exact `atan2`,
per-turn synchronization, and checked reject-before-store validation. They are
benchmark controls, not special Mech runtime primitives. Run them with:

```text
JULIA_NUM_THREADS=8 julia --startup-file=no julia_mojo_style.jl 500000 40 checked
JULIA_NUM_THREADS=8 julia --startup-file=no julia_mojo_style.jl 500000 40 unchecked
/tmp/mech-taichi-venv/bin/python taichi_mojo_style.py --backend cpu --instances 500000 --turns 40 --threads 8 --sync-each-turn
/tmp/mech-taichi-venv/bin/python taichi_mojo_style.py --backend gpu --instances 500000 --turns 40 --sync-each-turn
```

Taichi reports its selected backend and synchronization mode in the output;
the GPU command is valid only on a machine with a Taichi-supported GPU.

### Mech-first rerun

The generic Mech artifact was rerun after scalar IR identity/alias reduction and
component-major native GPU storage. This is the same 500,000-filter, 40-turn
workload with a per-turn publication boundary. The checked lane retains all
three integrity predicates; the unchecked lane is created only through the
explicit control API. Values are one Apple M1 run and should be treated as
steady-state samples, not cross-machine guarantees.

| Mech lane | Checked M/s | Unchecked M/s |
| --- | ---: | ---: |
| Cranelift JIT, 8 workers | 78.614 | 85.449 |
| WGPU resident, one submission/turn | 322.139 | 327.961 |

The WGPU result is above the Mojo native-Metal checked reference (244.493 M/s)
but below its unchecked native-Metal reference (405.047 M/s). WGPU is the
portable Windows/macOS lane; it is not the direct native-Metal comparison.

### Direct Metal lane (no WGPU)

On macOS, build with the opt-in `metal-native` feature and set
`MECH_METAL_ONLY=1`. This mode does not create a WGPU device or session. It
feeds the same generic Mech scalar IR through WGSL-to-MSL translation, binds
component-major state buffers, submits one Metal command buffer per turn, and
waits after every turn. Checked mode retains all three Mech integrity
predicates and reads only the compact fault status before publishing; the
unchecked mode is the explicit control lane.

```text
cargo build -p mech-gpu --release --features native,jit,metal-native \
  --example parallel_ekf_benchmark
MECH_METAL_ONLY=1 ./target/release/examples/parallel_ekf_benchmark \
  500000 40 1 40
```

Five post-fix Apple M1 runs at 500,000 filters x 40 turns measured **422.702
M/s checked** and **421.651 M/s unchecked** (medians). The timed session is
recreated after five discarded warmup turns, so it starts from the same initial
state as the Mojo control. The direct checked and unchecked states differed
from an independent scalar Mech reference by at most `1.221e-4`; setup,
compilation, warmup, and readback were outside the throughput timer. These are
steady-state samples, not a hardware ceiling. This is the native-Metal lane to
compare with Mojo's 244.493/405.047 M/s results only after matching the
arithmetic and fault-status paths described below. Raw samples and commands are in
[`results/apple-m1-mech-metal-2026-09-04.json`](results/apple-m1-mech-metal-2026-09-04.json).

This lane is not evidence that Mech's checked math is inherently 1.7x faster
than Mojo. The Mech shader emits explicit FMA and uses a compile-time checked
or unchecked artifact. Mojo was built with `--fp-mode contract=off`, passes a
runtime `checked` flag, and its checked loop performs a device fault-buffer
memset plus fault-buffer map/unmap after every turn. Mech's direct Metal path
clears and reads a shared two-word status buffer directly. The opt-in
`MECH_AUDIT_DISABLE_FMA=1` control measured 389.535 checked and 390.424
unchecked M/s in one run, isolating the arithmetic-contraction effect. The
remaining checked gap is therefore primarily the Mojo validation/status path,
not a missing Mech integrity check.

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

### Mojo scalar control

The Mojo control is the same explicit fixed-shape scalar recurrence, with flat
`f32` state and covariance storage. It is compiled to a native executable
before timing; the timed region contains only the 10,000-filter outer loop and
20 turns. Checked mode performs the same finite, positive-diagonal, and
symmetry checks and retains the prior value when a candidate fails. No Mojo
GPU kernel or worker pool is included in this scalar row.

On the Apple M1 Mac mini (8 GB, 4 performance + 4 efficiency cores, macOS
15.6.1, Mojo `1.1.0.dev2026090305`), five processes after one discarded
warmup produced:

| Mojo mode | Million EKF-turns/s | Checksum |
| --- | ---: | ---: |
| Mojo fixed-shape scalar, unchecked | 25.887 | 2682056.074092 |
| Mojo fixed-shape scalar, checked | 22.568 | 2682056.074092 |

The runner invokes `mojo build -O3 --fp-mode contract=off` so this comparison
does not use Mojo's default floating-point contraction setting. The strict
checksum matches the Rust fixed-shape control to the reported precision. The
source is [`mojo_scalar.mojo`](mojo_scalar.mojo); include it in the complete
runner with (the same invocation also builds and runs the textbook fixed-matrix
lane below):

```text
python3 benchmarks/archive/compute/parallel-ekf/run.py \
  --mojo /path/to/mojo --python /path/to/python-with-numpy
```

The existing Rust scalar row in the runner is an unchecked control, so the
Mojo checked number is not a checked-vs-checked Rust comparison.

### Mojo textbook-fidelity control

`mojo_textbook.mojo` is the separate faithful baseline. It keeps the EKF
written as matrix and vector operations: it constructs `F`, `G`, `Q`, `H`,
`K`, and `A`, then evaluates the textbook expressions
`F * P * F' + G * Q * G'` and `A * P * A' + R * K * K'`. The helper functions
use generic loops over the fixed 3x3, 3x2, 2x2, and 3-vector shapes; they are
not the manually expanded scalar formulas in `mojo_scalar.mojo`.

Build and run it with the same resident timing boundary:

```text
mojo build -O3 --fp-mode contract=off mojo_textbook.mojo -o mojo-textbook
./mojo-textbook 10000 20 unchecked
./mojo-textbook 10000 20 checked
```

On the Apple M1 Mac mini, five process runs after the built-in warmup gave a
median of **0.587 M EKF-turns/s unchecked** and **0.586 M EKF-turns/s checked**.
The checksum was `2682056.075673` in both modes. This is intentionally a
textbook-fidelity reference, not the optimized scalar control; its temporary
matrix/list construction is part of the measured implementation.

### Mojo textbook fixed-matrix control

`mojo_textbook_fixed.mojo` keeps the matrix-shaped equations but stores
`Vec3`, `Mat2`, `Mat32`, and `Mat3` as fixed-size value types. Their operations
are `@always_inline`, allowing register allocation without changing the
source-level EKF structure or validation policy.

On the same 10,000-filter, 20-turn workload, five process runs gave medians of
**20.81 M EKF-turns/s unchecked** and **21.21 M EKF-turns/s checked**. The
checksum was `2682056.075673` in both modes.

### Mojo fused workers and native Metal

The advanced Mojo CPU lanes use a 500,000-filter, 40-turn resident workload.
`mojo_parallel.mojo` fuses the fixed-shape recurrence inside an eight-worker
CPU pool. `mojo_simd.mojo` packs four filters into each `SIMD[.float32, 4]`
value and keeps the same eight-worker outer partition. Both checked lanes
validate finite state and covariance, positive diagonal entries, and
covariance symmetry; an invalid lane keeps its previous state. The native
Metal lane uses the same instance and turn counts and a 64-thread group,
matching Mech; its target-native `atan2` intrinsic is called out below rather
than replaced with an approximation.

The measured Apple M1 medians were:

| Mojo lane | Checked M/s | Unchecked M/s |
| --- | ---: | ---: |
| Fused fixed-shape, 8 workers | 79.626 | 94.609 |
| Fused SIMD-4, 8 workers | 143.519 | 158.513 |
| Native Metal resident kernel | 244.493 | 405.047 |

The SIMD result is the comparable CPU result for the 145 M/s class. The
native Metal lane keeps all state and covariance in device buffers, enqueues
one kernel per turn, and synchronizes after every timed turn. Checked Metal
also reads one compact device fault counter at each publication boundary;
it does not map the per-filter state or covariance during timing. Its device
math uses Metal's target-native `llvm.air.atan2.f32` because this Mojo/MAX
toolchain exposes the host libm `atan2` entry point only for CPU targets; the
reported checksum is therefore evidence of the run, not a claim of
bit-for-bit equivalence with the host scalar lane because device and CPU
operation order can differ. Full samples, commands,
and synchronization details are preserved in
[`results/apple-m1-mojo-advanced-2026-09-04.json`](results/apple-m1-mojo-advanced-2026-09-04.json).
The optional `deferred` argument is a diagnostic transport experiment only;
it is excluded from these numbers because it moves publication and fault
observation outside the per-turn timing boundary.

The CPU and Metal binaries can be rebuilt with the matching MAX source and
runtime library:

```text
MOJO=/path/to/mojo
MAX_SRC=/tmp/modular-src/max/mojo
MAX_LIB=/tmp/mech-mojo-venv.AJOcql/lib/python3.14/site-packages/modular/lib
$MOJO build -I $MAX_SRC -Xlinker -L$MAX_LIB -Xlinker -lAsyncRTMojoBindings -O3 --fp-mode contract=off mojo_parallel.mojo -o mojo_parallel
$MOJO build -I $MAX_SRC -Xlinker -L$MAX_LIB -Xlinker -lAsyncRTMojoBindings -O3 --fp-mode contract=off mojo_simd.mojo -o mojo_simd
$MOJO build -I $MAX_SRC -Xlinker -L$MAX_LIB -Xlinker -lAsyncRTMojoBindings -O3 --fp-mode contract=off mojo_metal.mojo -o mojo_metal
./mojo_parallel 500000 40 checked
./mojo_simd 500000 40 checked
DYLD_LIBRARY_PATH=$MAX_LIB ./mojo_metal 500000 40 checked
```

The sparse `/tmp/modular-src` checkout is only a build-time source dependency
for the MAX APIs; it is not cloned into or required by the Mech runtime.

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

For a backend comparison that does not create a WGPU session, build with
`metal-native` and pass `--mech-metal-only` to the runner. This changes only
the Mech backend sample; the portable Mech scalar/JIT rows are omitted in this
mode, while language controls and their timing boundaries remain unchanged:

```text
cargo build -p mech-gpu --release --features native,jit,metal-native \
  --example parallel_ekf_benchmark
python3 benchmarks/archive/compute/parallel-ekf/run.py \
  --mech-metal-only --backend-instances 500000 --backend-cpu-turns 40 \
  --samples 2 --python /path/to/python-with-numpy
```

Add `--evidence-output /path/to/results.json` to record the exact Git commit,
platform, tool versions, thread environment, commands, discarded warmups,
every measured process stdout, parsed checksums, and summary medians. Published
results should include this generated JSON rather than only the tables above.

The runner compiles the checked-in Rust control directly with
`rustc -C opt-level=3 -C target-cpu=native`, validates every scalar and JIT
checksum, and prints both Markdown tables.
