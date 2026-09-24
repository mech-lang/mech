# One EKF, many machines: why Mech for robotics?

Rust can match Mech's performance. That is not the surprising result here.
The useful result is that the Mech program does not have to turn into a
hand-written SIMD and worker-pool implementation to get there.

This repository snapshot puts the IROS workshop evidence on one branch based
on `origin/integration/v0.4`. It preserves the broad cross-language benchmark,
the matched Rust–Mech comparison, the exact source-size audit, the benchmark
programs, raw result records, and the scripts used to inspect them.

## The broad result: one Mech program spans many backends

![Representative checked EKF throughput with raw samples and observed ranges](charts/representative-checked-variability.svg)

The publication figure keeps representative scalar, multicore SIMD, and native
Metal lanes for which every retained process sample is available. Diamonds are
medians; whiskers are observed min-max ranges, not confidence intervals. Some
rows use 10,000 filters × 20 turns and the native/runtime rows use 500,000
filters × 40 turns, so the figure is a map of implementation strategies—not a
league table of languages. The horizontal scale is logarithmic.

That heterogeneity is the point. The Mech rows come from one high-level EKF
program lowered through different execution backends. In the retained
same-machine Apple M1 measurement campaigns, Mech ranges from a checked scalar
evaluator at 0.919 million EKF turns/s to direct Metal at 422.702 million
turns/s. Changing the backend changes the physical execution strategy without
requiring a new user-level EKF implementation.

The chart also keeps optimized controls for Rust, Mojo, Julia, Taichi, Halide,
Futhark, NumPy/Numba, Lua/LuaJIT, CPython, and PyPy. Those programs are valuable
controls, but they are not all source-identical or boundary-identical. Use them
to understand the available performance spectrum, not to claim a universal
ranking among languages. Bar endpoints are medians when repeated samples are
available. Because the combined campaigns do not share one randomized run
order or machine-state protocol, small gaps between rows are not meaningful.
See the [statistics and uncertainty policy](STATISTICS.md).

[Open the full checked mega chart](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-checked.svg) ·
[open the unchecked SVG](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-unchecked.svg) ·
[open both charts](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-full.html)

## The matched result: Rust and Mech are effectively tied

The clearest one-to-one comparison is the checked, fused SIMD block from one
measurement campaign on the same Apple M1. Both paths run 500,000 filters for
40 turns with four-wide SIMD and eight workers. Both retain a block-start
checkpoint, reject an invalid block, restore the complete prior state, and
return fault metadata.

![Matched Mech and Rust throughput with all samples and observed ranges](charts/matched-mech-rust-variability.svg)

| Implementation | n | Median checked throughput | Observed min-max | Audited application source |
| --- | ---: | ---: | ---: | ---: |
| Rust packed SIMD, eight workers | 3 | 146.509 M turns/s | 141.568-146.999 | 5,243 normalized characters |
| Mech SIMD/JIT, eight workers | 3 | 145.573 M turns/s | 139.668-147.381 | 1,079 normalized characters |

The Rust median is 0.64% higher, but the observed ranges overlap and the median
gap is smaller than either implementation's run-to-run spread. These three-run
samples support comparable throughput, not a claim that either implementation
is faster. The whiskers are observed ranges, not confidence intervals; no
significance test is claimed. The audited Rust application is 4.86× the size
of the unchanged Mech application. Put another way, Mech expresses this
application boundary with 79.4% fewer normalized source characters.

![Audited Mech and Rust source size](../archive/compute/parallel-ekf/source-size-audit/ekf-source-size-reconciled.svg)

The source metric removes comments and nonliteral whitespace and counts every
programmer-chosen identifier occurrence as one character, so long descriptive
Mech names do not inflate the comparison. It retains constants, initialization,
the EKF equations, validation, SIMD adapters, packing, worker dispatch,
checkpointing, and rollback when those are application-authored. It excludes
timers, warmup, reporting, tests, synthetic input generation, and general
compiler/runtime/library internals.

This is a result about these audited implementations, not a proof that every
Rust EKF must be five times larger. A different Rust matrix or parallelism
library would move code across the application/library boundary. The reason to
use Mech is that the language makes that boundary stable: the same 1,079-character
program supplies both the textbook and SIMD/eight-worker Mech columns, while
backend selection and execution plumbing remain reusable runtime concerns.

## Where the extra Rust source goes

The audit partitions every normalized character:

| Category | Mech | Rust SIMD / 8 workers |
| --- | ---: | ---: |
| Setup, constants, declarations | 340 | 561 |
| EKF update and candidate handling | 356 | 1,119 |
| Validation and component extraction | 383 | 396 |
| Application-written matrix helpers | 0 | 310 |
| SIMD wrapper and adapters | 0 | 1,243 |
| Batch dispatch, workers, rollback | 0 | 1,614 |
| **Total** | **1,079** | **5,243** |

The SIMD adapters plus dispatch/rollback account for 87.5% of Rust's growth
from the publication-normalized textbook control to the SIMD/eight-worker
control. The EKF update itself grows by only 95 normalized characters. This is
the engineering leverage Mech is intended to provide: changing where and how
the program executes without rewriting the numerical application around the
new execution model.

## What is included

The full archive contains more variants than the article needs, so the
publication story should lead with only these representative sources:

- [Mech EKF](../archive/compute/parallel-ekf/source-size-audit/selected/ekf.mec)
  and the matched [Rust SIMD control](../archive/compute/parallel-ekf/source-size-audit/selected/rust_simd.rs).
- [NumPy](../archive/compute/parallel-ekf/minimal/numpy_fast.py),
  [Julia](../archive/compute/parallel-ekf/minimal/julia_simd_threads.jl),
  [Taichi](../archive/compute/parallel-ekf/minimal/taichi_optimized.py),
  [Halide](../archive/compute/parallel-ekf/minimal/halide_ekf.cpp), and
  [Futhark](../archive/compute/parallel-ekf/minimal/futhark_ekf.fut) optimized controls.
- [Mojo fixed-matrix](../archive/compute/parallel-ekf/mojo_textbook_fixed.mojo),
  [Mojo Metal](../archive/compute/parallel-ekf/mojo_metal.mojo), and the
  identical-source [CPython/PyPy control](../archive/compute/parallel-ekf/pypy_optimized.py).
- [Raw benchmark evidence](../archive/compute/parallel-ekf/results/) and the
  complete [source-size audit](../archive/compute/parallel-ekf/source-size-audit/README.md).

`manifest.json` pins the v0.4 base, evidence revisions, machine, headline
values, representative source hashes, selected chart rows, and the descriptive
statistics policy.

## Same-machine provenance

The Mojo, Halide, and Taichi rows are not measurements imported from another
computer. Their raw records identify this Mac mini (Macmini9,1), Apple M1,
8 GB, macOS 15.6.1, arm64—the same identity reported by the machine on
2026-09-24.

The filesystem audit also found the Halide 21.0.0_1 installation with an
August 31 install timestamp, the Taichi 1.7.4 compiled-kernel cache populated
during the August/September campaigns, the exact Mojo nightly compiler cache,
and a surviving Mojo native-Metal executable. The Taichi and Mojo executables
had originally lived in named temporary environments, so those particular
environment directories no longer existed. Taichi 1.7.4 was reconstructed
exactly; Mojo 1.1.0 release was reconstructed as a compatibility diagnostic,
not substituted for the archived nightly result.

Fresh seven-process reruns of Halide and Taichi confirmed that the sources and
toolchains still execute correctly, but their medians moved materially from
the original campaigns. That is useful evidence about benchmark sensitivity,
not a reason to overwrite the retained results. Full raw samples and the audit
are in [the same-machine rerun record](results/same-machine-reruns-2026-09-24.json).

## Reproduce and verify

Verify the consolidated package from the repository root:

```sh
python3 benchmarks/iros-2026/verify.py
```

Regenerate the matched raw-sample and range figure:

```sh
python3 benchmarks/iros-2026/plot_variability.py
```

Recompute the source audit with only the Python standard library:

```sh
cd benchmarks/archive/compute/parallel-ekf/source-size-audit
python3 measure.py
python3 -m unittest -v test_measure
python3 breakdown.py
```

Build and run the stable v0.4 Mech backend benchmark on a machine with a
supported GPU:

```sh
cargo run -p mech-gpu --release --features native,jit \
  --example parallel_ekf_benchmark -- 10000 20 3 20
```

The current-head rerun is a diagnostic compatibility check, not a replacement
for the matched historical eight-worker result. Never combine checked and
unchecked rows, per-turn and fused-block boundaries, or measurements from
different campaigns into a speedup claim.

On 2026-09-24, the stable v0.4 Mech benchmark passed the compile check above.
The exact Rust SIMD control was also rebuilt and rerun in isolation. Its five
checked fused samples had a 148.056 M turns/s median and a wide
110.795-154.772 M turns/s observed range, with the identical checksum and zero
faults. The bimodal samples make this a compatibility diagnostic, not
performance corroboration. See the [current-head verification
record](results/current-v0.4-verification-2026-09-24.json).

## Publication language

A concise claim supported by this package is:

> On an Apple M1, three retained checked eight-worker SIMD runs produced a
> median of 145.573 million EKF turns/s for Mech (139.668-147.381 observed
> range) and 146.509 million for Rust (141.568-146.999). The audited Mech
> application used 1,079 normalized source characters versus 5,243 for the
> Rust SIMD implementation—comparable measured throughput with about one-fifth
> of the application source for these implementations.

The broad chart supports a different claim:

> A single high-level Mech EKF source can target scalar, SIMD, JIT, WGPU, and
> native Metal execution. The surrounding language controls show the range of
> strategies available on the same workload; they are not a universal language
> ranking.
