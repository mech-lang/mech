# One EKF, many machines: why Mech for robotics?

Rust can match Mech's performance. That is not the surprising result here.
The useful result is that the Mech program does not have to turn into a
hand-written SIMD and worker-pool implementation to get there.

This repository snapshot puts the IROS workshop evidence on one branch based
on `origin/integration/v0.4`. It preserves the broad cross-language benchmark,
the matched Rust–Mech comparison, the exact source-size audit, the benchmark
programs, raw result records, and the scripts used to inspect them.

## The same-source result: CPU and Metal from one application

![One EKF source per system across CPU and Metal](charts/post-portable-combo.svg)

Mech, Taichi, and Halide are the three measured systems in this package that
select both CPU and Apple Metal execution for this EKF from the same
application source. The equations and publication contract stay in one `.mec`,
`.py`, or `.cpp` file; a backend choice and schedule select the device. The
standalone [CPU chart](charts/post-portable-cpu-comparison.svg) and [Metal
chart](charts/post-portable-metal-comparison.svg) are poster-ready versions of
the two panels above.

For this workload on this Apple M1, Mech has the highest retained median in
both modes on both devices: CPU 104.783 M turns/s checked and 110.469 M/s
unchecked; Metal 420.404 and 419.523 M/s. Taichi measures 87.723/95.759 on CPU
and 332.584/409.530 on Metal; Halide measures 23.426/23.940 and
292.500/394.263. Those are descriptive results for these implementations and
campaigns—not evidence that Mech is intrinsically faster than either language.
Compiler lowering, schedules, fault-status observation, campaign dates, and
system state all differ and are called out directly in the figure.

All rows use 500,000 filters × 40 turns, f32 state, resident ping-pong
publication, and a synchronization boundary after every turn; CPU rows use
eight workers. Every comparison panel uses the first three retained processes
per row. Bars report medians and whiskers report
median absolute deviation (MAD), so isolated interference samples cannot
dominate the visual range. These are not confidence intervals. Sample counts
and `C`/`U` prefixes are intentionally omitted from the chart; the legend and
bar order identify checked and unchecked. Every raw sample remains in JSON.

## The cross-language CPU result: checked and unchecked

![Full checked and unchecked CPU EKF comparison across eight implementations](charts/post-cross-language-comparison.svg)

This full CPU figure includes Mech, Rust, Mojo, Julia, Futhark, Taichi,
NumPy/Numba, and Halide. Every implementation runs 500,000 filters for 40 turns
in f32 with eight workers on the same Apple M1. The chart uses the first three
retained processes from every row, so its comparison window is equal.

| Implementation | CPU strategy | Checked median ± MAD | Unchecked median ± MAD |
| --- | --- | ---: | ---: |
| Rust | packed f32x4 | 146.509 ± 0.490 M/s | 163.866 ± 1.446 M/s |
| Mech | SIMD/JIT, f32x4 | 145.573 ± 1.808 M/s | 165.830 ± 2.304 M/s |
| Mojo | explicit SIMD-4 | 143.986 ± 0.144 M/s | 145.219 ± 13.294 M/s |
| Julia | SIMD.jl | 128.544 ± 0.106 M/s | 133.605 ± 3.149 M/s |
| Futhark | ISPC AOT | 108.903 ± 0.372 M/s | 152.479 ± 1.386 M/s |
| Taichi | LLVM CPU, per-turn publication | 87.723 ± 0.441 M/s | 95.759 ± 0.023 M/s |
| NumPy/Numba | compiled parallel kernel | 80.323 ± 0.036 M/s | 81.972 ± 0.270 M/s |
| Halide | native CPU, per-turn publication | 23.426 ± 0.026 M/s | 23.940 ± 0.044 M/s |

The strict one-to-one anchor inside the wider figure is Mech versus Rust: both
use four-wide packed SIMD, eight workers, fused execution, and block-atomic
rollback. Their observed ranges overlap in both modes. Checked Rust is 0.64%
ahead at the median; unchecked Mech is 1.20% ahead. Those small reversals are
why the supported conclusion is comparable throughput, not a language winner.

The other implementations match workload, CPU, worker count, and precision,
but their fault interfaces are not identical. Mech, Rust, Mojo, Julia,
Futhark, and Numba fuse the 40-turn worker-local block; Taichi and Halide
synchronize publication after each turn. NumPy/Numba means an LLVM-compiled
Numba kernel launched from Python—not interpreted Python or eager NumPy. Read
this full chart as an implementation landscape, not a strict language ranking.
MAD whiskers are robust descriptive spread, not confidence intervals.

The larger archive also contains CPython, PyPy, and other controls. They remain
in the mega charts when their retained evidence
changes the workload size, execution shape, device, or arithmetic contract.

[Open the full checked mega chart](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-checked.svg) ·
[open the unchecked SVG](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-unchecked.svg) ·
[open both charts](../archive/compute/parallel-ekf/charts/parallel-ekf-cross-language-full.html)

## The Metal result: six implementation ecosystems

![Checked and unchecked Apple M1 Metal EKF throughput across six implementation ecosystems](charts/post-metal-comparison.svg)

This Metal-only figure keeps the workload and synchronization boundary fixed:
500,000 filters × 40 turns, resident f32 state, and one completed Metal
publication per turn. It compares Mech-generated MSL, a Rust host dispatching
hand-written MSL, Mojo, Julia/Metal.jl, Taichi, and Halide.

| Implementation | Metal path | Checked median ± MAD | Unchecked median ± MAD |
| --- | --- | ---: | ---: |
| Mech | generated MSL, direct Metal | 420.404 ± 3.491 M/s | 419.523 ± 2.940 M/s |
| Rust + MSL | hand-written MSL, `metal-rs` host | 418.602 ± 0.067 M/s | 418.519 ± 1.816 M/s |
| Mojo | native Metal, matched packed SoA | 404.932 ± 2.388 M/s | 401.865 ± 0.444 M/s |
| Julia | Metal.jl, matched packed SoA | 406.432 ± 0.626 M/s | 409.896 ± 4.804 M/s |
| Taichi | native Metal, matched packed SoA | 332.584 ± 0.581 M/s | 409.530 ± 2.704 M/s |
| Halide | packed SoA, native Metal | 292.500 ± 0.583 M/s | 394.263 ± 0.053 M/s |

The Rust control was added specifically to test whether Mech's 422.702 M/s
result depended on an unavailable trick. It uses resident SoA buffers, one GPU
thread per filter, paired trigonometry, explicit fused arithmetic, 64-thread
threadgroups, ping-pong publication, a two-word shared fault status, and one
command buffer plus wait per turn. Its range overlaps Mech in both modes. This
supports the useful claim: Mech generates a Metal execution strategy comparable
to a hand-written Metal kernel; it does not show that Metal itself favors Mech.

Stable Rust does not directly compile Rust kernels to Apple Metal, so that row
is accurately labeled “Rust + MSL”: Rust owns the host and a hand-written MSL
kernel owns GPU execution.

The matched Mojo control now uses the same component-major packed SoA,
resident ping-pong publication, fast device transcendentals, 64-thread launch
geometry, and per-turn synchronization as the Mech and Rust paths. Checked
mode adds candidate predicates and a two-word device fault status; after the
required synchronization, Mojo reads those eight status bytes directly from
Apple unified memory. That removes the old checked path's full host mapping
and raises the equal-window chart median from the archived 244.493 to 404.932
M turns/s. Checked and unchecked differ by only 0.76%, so this session does
not resolve any checking cost. All raw samples remain in JSON, including the
215.945 M/s interference event; MAD prevents that event from controlling the
chart whisker.

The matched Mojo median remains 3.68% below Mech in checked mode and 4.21%
below it unchecked. A checked Mojo sample reached 420.309 M/s, and the observed
ranges overlap, so the remaining 402/401-to-423/422 separation is too small
and session-sensitive to support a ranking. Matching state layout, publication,
math lowering, launch geometry, and fault transport closed the large checked
gap but did not erase this final few percent; it can still come from compiler
code generation and host dispatch overhead.

Julia compiles its Julia kernel through Metal.jl. The matched Julia control
gives both modes the same component-major packed SoA, resident ping-pong
buffers, 64-thread launch geometry, and per-turn synchronization. Checked mode
adds only the candidate predicates and a shared two-word fault status. Its
checked median is 0.09% below unchecked and the observed ranges overlap,
replacing the earlier host-transport-heavy path with a like-for-like
measurement of checking.

The matched Halide control likewise replaces its older fixed-shape tuple path
with packed resident state, ping-pong publication, and a 256-thread Metal
schedule. The equal-window unchecked median rises from 212.283 to 394.263 M turns/s. Halide
21's generated Metal path does not expose the compact device-wide atomic status
used by the other matched controls, so checked mode uses a resident per-lane
fault plane that must be observed after each turn; that remaining interface
cost is reported rather than hidden.

That exact Halide source also selects an eight-worker CPU schedule. The
equal-window chart medians are 23.426 M turns/s checked and 23.940 M/s
unchecked. Together with the same-source Taichi result and
Mech's unchanged high-level EKF, this supplies the focused portability story:
all three target CPU and Metal from one application source; Mech has the
highest retained median on both devices in these measurements. Toolchain,
schedule, and status-observation differences make that a descriptive result,
not a general language-speed claim.

The matched Taichi control replaces the archived comparison in this figure.
Both modes now use one packed component-major field, identical resident double
buffers, a 64-thread launch, and the same synchronized ping-pong publication.
Checked mode adds the integrity predicates, atomics only on fault, and one
compact cumulative-status read after synchronization; the cumulative counter
avoids a per-turn reset transfer while preserving whole-turn rollback. Its
332.584 M turns/s checked median versus 409.530 M/s unchecked is therefore an
18.79% measured cost of Taichi's checked protocol, not a comparison between
different state layouts or in-place versus double-buffered execution.

The same Taichi source selects its LLVM CPU backend with one option and uses a
backend-specialized packed axis order without changing the EKF equations or
publication contract. The equal-window CPU chart medians are 87.723 M turns/s
checked and 95.759 M/s unchecked. This per-turn synchronized result documents
Taichi's same-source backend portability. It now appears in the full CPU panel
with its different publication boundary labeled explicitly; NumPy/Numba
remains the separate compiled Python-ecosystem row.

All rows share the successful-path workload and resident packed SoA,
ping-pong publication, and per-turn synchronization; all but Halide use a
compact shared fault status. Compiler contraction, transcendental lowering,
launch geometry, host API overhead, and campaign dates can still move results.
Use medians and MAD as a backend landscape; do not turn small gaps into a
ranking.

## The Mech result: one EKF across execution backends

![One Mech EKF across eight execution backends](charts/post-mech-backend-stack.svg)

These eight rows use the same high-level Mech EKF and change the execution
backend: the scalar artifact evaluator, Cranelift JIT, scalar and four-lane
Cranelift AOT, one- and eight-worker SIMD/JIT, WGPU on Metal, and direct Metal.
All rows are checked and publish after every turn. The 10,000-filter CPU rows
and 500,000-filter parallel/GPU rows come from retained same-machine campaigns,
so normalized throughput makes the backend span visible, but small cross-row
gaps are not ranking claims.

The scalar AOT path shares the JIT's Cranelift lowering, emits a host object,
links a reusable native library, and reloads its exported turn function. Five
release processes produced a 14.593 M turns/s AOT median (14.089-14.644
observed range) and a 14.618 M turns/s JIT median (14.068-14.641) in the same
processes. Both matched scalar state bit-for-bit. A cold emit/link/load took
202.029 ms; cached loads had a 3.283 ms median. These timings support equivalent
steady-state execution, not a claim that either compiler mode is faster.

The new `cpu-aot-simd` option saves a different AOT library with four-filter
`f32x4` arithmetic, packed resident state, paired sine/cosine evaluation, and
the same checked rollback contract. It is selected by the backend registry or
directly through `compile_aot_simd_cpu`; the scalar `cpu-aot` option remains as
the exact-ABI baseline.

AOT and SIMD are orthogonal here. AOT describes when the native code is
compiled and that it is saved as a reusable library; SIMD describes how the
kernel executes independent filters. Mech supports scalar AOT and SIMD AOT,
and the performance difference comes from the lowering strategy rather than
from whether the library was loaded from disk.

### AOT code-generation diagnostic

The direct dynamic-library control uses a longer, steadier campaign than the
backend overview: 10,000 filters × 200 checked turns, preceded by 100 untimed
turns and a full state reset. The same minimal loader measures scalar Mech AOT,
four-lane Mech AOT, and a hand-specialized Rust `cdylib`; compiler/build time,
allocation, packing, warmup, and reset are outside the timed region. All rows
use one host thread and checked publication after every turn.

This table is a backend diagnostic, not the Rust–Mech language comparison.
Scalar Mech and Rust share an ABI and workload, but not the same generated
optimization: LLVM applies local SLP vectorization and combines sine/cosine
calls in the Rust kernel, while scalar Cranelift AOT does not. Conversely,
four-lane Mech AOT uses packed cross-filter SIMD that the Rust dylib does not.

| Implementation | Steady-state throughput, median (observed min-max), n=7 | Library size | Peak process RSS, median (observed min-max), n=7 |
| --- | ---: | ---: | ---: |
| Mech scalar Cranelift AOT | 14.671 M/s (14.458-14.679) | 33,544 B | 2,818,048 B (2,818,048-3,014,656) |
| Optimized Rust `cdylib` | 21.121 M/s (21.097-21.129) | 50,016 B | 2,818,048 B (2,818,048-2,916,352) |
| Mech four-lane Cranelift AOT | 34.863 M/s (34.847-34.927) | 33,864 B | 2,818,048 B (2,818,048-2,818,048) |

Rust is 43.96% ahead of scalar AOT because of that code-generation and
hand-specialization gap, not because dynamic libraries favor Rust or because
Mech is intrinsically slower. Scalar AOT and scalar JIT are effectively tied
in the matched Mech-only measurement above, demonstrating that AOT packaging
does not itself impose the gap.

Changing only the Mech backend reverses that result. Four-lane AOT is 65.07%
faster than the scalar Rust control and 137.63% faster than scalar AOT in this
campaign. This is likewise not a language ranking: its packed state and
four-filter vector body are a different physical strategy. Rust can and does
use the same strategy in the matched checked/unchecked comparison above. The
result demonstrated here is that Mech can select AOT packaging and SIMD
lowering together without changing the user-level EKF source.

Both generated Mech libraries are about one-third smaller than the Rust
library. All three median peak-RSS values are identical and their observed
ranges overlap, so these seven processes do not establish a memory-use
difference. Peak RSS includes the common loader and live buffers; it is not
private library memory. All runs reported zero faults, and
complete final states agreed within 4.09e-4 after 200 turns. See the [raw
seven-process record](results/apple-m1-aot-vs-rust-dylib-2026-09-24.json) and
the [common-loader control](rust-dylib/README.md).

That span is the point. In these campaigns the medians range from 1.032 million
EKF turns/s in the scalar evaluator to 422.702 million in direct Metal. Mech
changes the physical execution strategy without requiring the user-level EKF
to be rewritten around SIMD adapters, worker dispatch, or GPU kernels.

## Why Mech if Rust can match it?

The matched comparison answers the performance question in both modes: Rust
and Mech are comparable when workload, SIMD width, worker count, fusion, and
integrity contract are aligned. Checked Rust is 0.64% ahead at the median;
unchecked Mech is 1.20% ahead. Both observed ranges overlap, so neither small
gap supports a winner.

The engineering difference is how much application code is required to reach
that execution shape. The audited Rust SIMD/eight-worker application contains
5,243 normalized characters; the unchanged Mech application contains 1,079.
The Rust implementation is therefore 4.86× the size at this boundary, while
Mech expresses it with 79.4% fewer normalized source characters. The same Mech
source also supplies the scalar, JIT, AOT, SIMD AOT, WGPU, and Metal rows.

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
  and the matched [Rust SIMD control](../archive/compute/parallel-ekf/source-size-audit/selected/rust_simd.rs),
  plus the [Rust-hosted MSL control](rust-metal/README.md).
- [NumPy/Numba](../archive/compute/parallel-ekf/minimal/numpy_numba.py),
  [Julia](../archive/compute/parallel-ekf/minimal/julia_simd_threads.jl),
  [matched Taichi Metal](taichi-metal-matched.py),
  [matched Halide Metal](halide-metal-matched.cpp), and
  [Futhark](../archive/compute/parallel-ekf/minimal/futhark_ekf.fut) optimized controls.
- [Mojo fixed-matrix](../archive/compute/parallel-ekf/mojo_textbook_fixed.mojo),
  [matched Mojo Metal](mojo-metal-matched.mojo), and the
  identical-source [CPython/PyPy control](../archive/compute/parallel-ekf/pypy_optimized.py).
- [Matched Julia Metal.jl](julia-metal-matched.jl)
  and the retained direct-Metal record for Mech.
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
during the August/September campaigns, and the exact Mojo nightly compiler
cache. The temporary Mojo environment itself no longer existed. Taichi 1.7.4
was reconstructed exactly. The publication Metal figure now uses a fresh,
matched Mojo control built with Mojo 1.1.0 and MAX 26.6.0; this is a new
measurement with its toolchain pinned in the raw record, not a relabeling of
the archived nightly result.

Fresh seven-process reruns of the historical Halide and Taichi programs
confirmed that the sources and toolchains still execute correctly, but their
medians moved materially from the original campaigns. That is useful evidence
about benchmark sensitivity, not a reason to overwrite the retained results.
The publication chart instead uses separately recorded matched Mojo, Taichi,
and Halide implementations. Historical rerun samples and the audit remain in
[the same-machine rerun record](results/same-machine-reruns-2026-09-24.json).

## Reproduce and verify

Verify the consolidated package from the repository root:

```sh
python3 benchmarks/iros-2026/verify.py
```

Regenerate all three publication figures from the retained raw samples:

```sh
python3 benchmarks/iros-2026/plot_post_charts.py
```

Rebuild and rerun the Rust-hosted hand-written Metal control:

```sh
python3 benchmarks/iros-2026/measure_rust_metal.py \
  --samples 7 --instances 500000 --turns 40
```

Rerun the matched Julia/Metal.jl control:

```sh
python3 benchmarks/iros-2026/measure_julia_metal.py \
  --samples 7 --instances 500000 --turns 40
```

Rebuild and rerun the matched Mojo, Taichi, and Halide Metal controls:

```sh
python3 benchmarks/iros-2026/measure_mojo_metal.py \
  --mojo /path/to/mojo --samples 7 --instances 500000 --turns 40
python3 benchmarks/iros-2026/measure_taichi_metal.py \
  --python /path/to/taichi-python --arch metal \
  --samples 7 --instances 500000 --turns 40
python3 benchmarks/iros-2026/measure_halide_metal.py \
  --samples 7 --instances 500000 --turns 40
```

Rebuild and remeasure the Rust/Mech dynamic-library control after first
producing the Mech AOT library:

```sh
python3 benchmarks/iros-2026/measure_dylib_comparison.py \
  target/mech-aot-final-v2/mech-0a9d1856e310433b75da7d519ccc856597d751f7af345a38c70d43d289ec24fd.dylib \
  --mech-simd-dylib target/mech-aot-final-v2/mech-simd-633e21301b05367c3cadd6c59b1fdbdcf102cd909a23acd1655f897aeb3045db.dylib \
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

> On an Apple M1, matched four-wide SIMD/eight-worker runs produced checked
> medians of 145.573 million EKF turns/s for Mech and 146.509 million for Rust,
> and unchecked medians of 165.830 million for Mech and 163.866 million for
> Rust. The observed ranges overlap in both modes. The audited Mech application
> used 1,079 normalized source characters versus 5,243 for the Rust SIMD
> implementation—comparable measured throughput with about one-fifth of the
> application source for these implementations.

The Mech backend chart supports a different claim:

> A single high-level Mech EKF source can target scalar, SIMD, JIT, scalar AOT,
> SIMD AOT, WGPU, and native Metal execution. The retained rows span more than
> two orders of magnitude in normalized throughput; because the workloads and
> campaigns differ, that span demonstrates backend reach rather than a fine
> ranking.

The 14.671 M/s scalar Mech AOT and 21.121 M/s scalar-ABI Rust dylib values must
not be quoted as the language comparison: the generated optimization
strategies differ. They are retained only to diagnose scalar Cranelift versus
LLVM code generation. Likewise, 34.863 M/s SIMD AOT versus the scalar Rust
dylib demonstrates backend selection, not a matched Rust–Mech speedup.
