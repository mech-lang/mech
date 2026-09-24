# One EKF, many machines: why Mech for robotics?

Rust can match Mech's performance. That is not the surprising result here.
The useful result is that the Mech program does not have to turn into a
hand-written SIMD and worker-pool implementation to get there.

This repository snapshot puts the IROS workshop evidence on one branch based
on `origin/integration/v0.4`. It preserves the broad cross-language benchmark,
the matched Rust–Mech comparison, the exact source-size audit, the benchmark
programs, raw result records, and the scripts used to inspect them.

## The cross-language result: matched checked CPU implementations

![Checked CPU EKF throughput under a matched execution shape](charts/post-cross-language-comparison.svg)

This is the most defensible cross-language slice in the archive: every row is
an f32 CPU implementation running 500,000 filters for 40 turns with eight
workers, a fused worker-local block, and checked candidate publication on the
same Apple M1. Every retained process sample is visible. Diamonds are medians;
whiskers are observed minimum-to-maximum ranges, not confidence intervals.

The publication boundary is still not identical in every respect. Mech and
Rust additionally implement block-atomic rollback to the block-start
checkpoint and return fault metadata. Julia and Numba reject invalid
candidates per lane. All measured samples reported zero faults, so this
difference did not change the successful execution path, but it should remain
in the caption instead of being hidden.

The larger archive also contains Mojo, Futhark, Halide, Taichi, CPython, PyPy, and
other controls. They are excluded from this figure when their retained evidence
changes the worker count, workload size, publication boundary, device, or
strict arithmetic contract. They remain useful context, not evidence for a
fine language ranking.

[Open the full checked mega chart](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-checked.svg) ·
[open the unchecked SVG](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-unchecked.svg) ·
[open both charts](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-full.html)

## The Mech result: one EKF across execution backends

![One Mech EKF across seven execution backends](charts/post-mech-backend-stack.svg)

These seven rows use the same high-level Mech EKF and change the execution
backend: the scalar artifact evaluator, Cranelift JIT, Cranelift AOT, one- and
eight-worker SIMD/JIT, WGPU on Metal, and direct Metal. All rows are checked and
publish after every turn. The 10,000-filter CPU rows and 500,000-filter
parallel/GPU rows come from retained same-machine campaigns, so normalized
throughput makes the backend span visible, but small cross-row gaps are not
ranking claims.

The AOT path shares the JIT's Cranelift lowering, emits a host object, links a
reusable native library, and reloads its exported turn function. Five release
processes produced a 14.593 M turns/s AOT median (14.089-14.644 observed range)
and a 14.618 M turns/s JIT median (14.068-14.641) in the same processes. Both
matched scalar state bit-for-bit. A cold emit/link/load took 202.029 ms; cached
loads had a 3.283 ms median. These timings support equivalent steady-state
execution, not a claim that either compiler mode is faster.

### Mech AOT versus a Rust dynamic library

The direct dynamic-library control uses a longer, steadier campaign than the
backend overview: 10,000 filters × 200 checked turns, preceded by 100 untimed
turns and a full state reset. A hand-specialized Rust `cdylib` exports the same
`mech_fixed_numeric_turn` symbol as the generated Mech library. The same
minimal loader supplies identical input and ping-pong state buffers to both,
so the measurement excludes the Mech compiler and Rust build process.

| Metric | Mech Cranelift AOT | Rust `cdylib` | Measured differential |
| --- | ---: | ---: | ---: |
| Steady-state throughput, median (observed min-max), n=7 | 14.672 M/s (14.381-14.682) | 20.789 M/s (20.324-20.805) | Rust +41.69% |
| Dynamic-library file size | 33,544 bytes | 50,016 bytes | Mech -32.93% |
| Peak process RSS, median (observed min-max), n=7 | 2,834,432 B (2,834,432-2,949,120) | 2,834,432 B (2,834,432-2,965,504) | no measured median difference |

The complete final states agree within 4.05e-4 after 200 turns and every run
reported zero faults. Rust is clearly faster in this hand-expanded scalar
control; Mech's generated library is smaller. The peak-RSS medians are
identical at `/usr/bin/time -l` precision and the ranges overlap, so this
campaign does not support a memory-use difference. File size and peak process
RSS are intentionally separate: one is the on-disk library, while the other
includes the common loader and live EKF buffers.

This is not the eight-worker SIMD source-size comparison below. It isolates
the AOT artifact boundary on one scalar thread. The Rust control specializes
the fixed 3×3 algebra by hand, while Mech reaches the same ABI from the generic
matrix source. See the [raw seven-process record](results/apple-m1-aot-vs-rust-dylib-2026-09-24.json)
and the [common-loader control](rust-dylib/README.md).

That span is the point. In these campaigns the medians range from 1.032 million
EKF turns/s in the scalar evaluator to 422.702 million in direct Metal. Mech
changes the physical execution strategy without requiring the user-level EKF
to be rewritten around SIMD adapters, worker dispatch, or GPU kernels.

## Why Mech if Rust can match it?

The clearest one-to-one comparison is the checked, fused SIMD block from one
measurement campaign on the same Apple M1. Both paths run 500,000 filters for
40 turns with four-wide SIMD and eight workers. Both retain a block-start
checkpoint, reject an invalid block, restore the complete prior state, and
return fault metadata.

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
- [NumPy/Numba](../archive/compute/parallel-ekf/minimal/numpy_numba.py),
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

Regenerate both publication figures from the retained raw samples:

```sh
python3 benchmarks/iros-2026/plot_post_charts.py
```

Rebuild and remeasure the Rust/Mech dynamic-library control after first
producing the Mech AOT library:

```sh
python3 benchmarks/iros-2026/measure_dylib_comparison.py \
  target/mech-aot/mech-42c18f83c35ea5415d4a333b3e9e41602adfc62ed0032d105f3ec9812953b00e.dylib \
  --samples 7 --instances 10000 --turns 200
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
cargo run -p mech-gpu --release --features native,aot \
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

The Mech backend chart supports a different claim:

> A single high-level Mech EKF source can target scalar, SIMD, JIT, AOT, WGPU,
> and native Metal execution. The retained rows span more than two orders of
> magnitude in normalized throughput; because the workloads and campaigns
> differ, that span demonstrates backend reach rather than a fine ranking.
