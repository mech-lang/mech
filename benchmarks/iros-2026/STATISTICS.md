# Benchmark statistics and uncertainty

The publication figures use medians, never raw arithmetic averages. Every CPU
and Metal comparison row contains ten fresh processes. Each round schedules
every implementation/mode once in a deterministic shuffled order. Whiskers and
`±` labels report median absolute deviation (MAD), which prevents an isolated
interference run from dominating the plotted spread. No samples were removed
or replaced; the raw JSON retains the process order, command, stdout, checksum,
fault count, and throughput for every run.

MAD is a robust descriptive error bar, not a confidence interval. Ten processes
are enough to make these poster summaries substantially less fragile than the
old n=3 windows, but still do not justify p-values or claims of statistical
significance. Tables below also report observed min-max where useful.

## Headline matched comparison

These retained campaign records share one workload, physical execution shape,
machine, and campaign date: 500,000 filters x 40 turns, four-wide SIMD, eight
workers, and fused execution. Within each mode Mech and Rust use the same
integrity contract. Checked rows
validate every candidate and provide block-start rollback plus fault metadata;
unchecked rows omit those checks.

| Mode | Implementation | n | Median | Observed min-max |
| --- | --- | ---: | ---: | ---: |
| Checked | Mech SIMD/JIT | 10 | 151.323 M turns/s | 141.186-154.585 |
| Checked | Rust packed SIMD | 10 | 149.941 M turns/s | 138.559-155.125 |
| Unchecked | Mech SIMD/JIT | 10 | 184.137 M turns/s | 125.852-199.403 |
| Unchecked | Rust packed SIMD | 10 | 170.490 M turns/s | 142.441-172.268 |

Mech's median is 0.92% higher checked and 8.00% higher unchecked. The observed
ranges overlap in both modes. These are implementation/compiler results, not a
claim that either language is intrinsically faster. Relative to unchecked
execution, validation reduces median throughput by 17.8% for Mech and 12.1%
for Rust in this campaign.

## Same-source CPU and Metal figure

The focused poster figure includes only the systems measured here that select
both CPU and Apple Metal from one application file: Mech, Taichi, and Halide.
The workload is 500,000 filters × 40 turns with f32 resident ping-pong state
and a synchronization boundary after every turn; CPU uses eight workers.

| Device | Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | --- | ---: | ---: |
| CPU | Mech Cranelift SIMD/JIT | 141.362 ± 3.379 | 166.407 ± 4.134 |
| CPU | Taichi LLVM | 88.344 ± 0.421 | 95.363 ± 0.316 |
| CPU | Halide native | 22.526 ± 0.649 | 23.490 ± 0.092 |
| Metal | Mech generated MSL | 409.765 ± 2.954 | 410.439 ± 4.921 |
| Metal | Taichi native Metal | 342.872 ± 2.030 | 410.771 ± 3.794 |
| Metal | Halide Metal schedule | 293.291 ± 5.583 | 398.765 ± 10.403 |

Mech has the highest CPU and checked-Metal median. Taichi's unchecked-Metal
median is 0.08% above Mech's, far below either MAD. This is descriptive, not a
claim that any language is intrinsically faster. Compilers, schedules, and
fault-status observation mechanisms differ.

## Cross-language CPU publication figure

The full CPU figure extends the matched Mech–Rust anchor with Mojo, Julia,
Futhark, Taichi, NumPy/Numba, and Halide. All eight run on the Apple M1 CPU
with 500,000 filters × 40 turns, f32 state, and eight workers. Checked and
unchecked modes remain separate, and every row uses ten fresh processes.

| Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | ---: | ---: |
| Mech | 151.323 ± 2.564 | 184.137 ± 8.125 |
| Rust | 149.941 ± 2.494 | 170.490 ± 1.720 |
| Julia | 129.047 ± 2.844 | 136.402 ± 1.077 |
| Mojo | 119.291 ± 0.442 | 128.152 ± 0.229 |
| Futhark | 97.897 ± 1.162 | 149.931 ± 1.370 |
| Taichi | 88.344 ± 0.421 | 95.363 ± 0.316 |
| NumPy/Numba | 79.380 ± 0.550 | 81.557 ± 0.350 |
| Halide | 22.526 ± 0.649 | 23.490 ± 0.092 |

Mech and Rust additionally match block-atomic rollback and fault metadata.
Fault interfaces differ elsewhere. Taichi and Halide publish every turn while
the other six fuse the 40-turn worker-local block. NumPy/Numba is a
Numba-compiled parallel kernel, not interpreted Python. All samples reported
zero faults.

## Metal publication figure

The Metal figure fixes the Apple M1 GPU workload at 500,000 filters × 40 turns,
resident f32 state, and synchronization after every turn. It compares complete
compiler/runtime paths rather than isolated kernel languages.

Every Metal row uses ten fresh processes.

| Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | ---: | ---: |
| Rust host + hand-written MSL | 425.180 ± 3.083 | 422.890 ± 4.917 |
| Julia Metal.jl, matched packed SoA | 410.420 ± 4.744 | 411.398 ± 13.386 |
| Mech generated MSL | 409.765 ± 2.954 | 410.439 ± 4.921 |
| Mojo native Metal, matched packed SoA | 406.038 ± 5.165 | 405.355 ± 6.941 |
| Taichi native Metal, matched packed SoA | 342.872 ± 2.030 | 410.771 ± 3.794 |
| Halide Metal, matched packed SoA | 293.291 ± 5.583 | 398.765 ± 10.403 |

The Rust control adopts the direct Mech backend's resident SoA, 64-thread
threadgroups, ping-pong publication, compact shared fault status, and per-turn
command/wait boundary. Rust's median is 3.76% above Mech checked and 3.03%
above it unchecked; raw observed ranges overlap. This supports comparable
generated-versus-hand-written Metal execution, not a Rust-to-Metal compiler
claim: stable Rust hosts a hand-written MSL kernel here.

The new Julia control uses a component-major packed SoA and gives checked and
unchecked modes identical resident buffers, bindings, 64-thread launch
geometry, publication, and synchronization. Checked adds only the candidate
predicates and shared two-word fault status. Its median checking cost is 0.24%
and the observed ranges overlap. This replaces the older Julia path whose
checked mode paid for extra bindings and host fault-array transport.

The matched Mojo control uses the same physical strategy and reads its
two-word status directly from Apple unified memory. Its checked and unchecked
medians differ by 0.17%, so no checking penalty is resolved. Matching the path
raises checked performance from the archived 244.493 result to 406.038 M/s.
Mojo is 0.91% below Mech checked and 1.24% below unchecked, less than the
reported MADs.

The new Taichi control gives both modes the same packed component-major state,
resident double buffers, launch geometry, and publication boundary. Its
cumulative two-word fault status avoids a reset transfer but still requires a
compact host-visible read after every synchronized checked turn. Checked is
16.53% below unchecked at the median; this is a measured protocol/API cost,
not an in-place-versus-ping-pong comparison.

That exact source also targets the eight-worker LLVM CPU backend by changing
one runtime option and selecting a CPU-contiguous packed axis order. The equal
window measures 88.344 ± 0.421 M turns/s checked and 95.363 ± 0.316 unchecked.
The result corroborates the archived 86.047 M/s Taichi CPU row. Its per-turn
synchronization differs from the fused rows and is labeled accordingly in the
full CPU panel.

The matched Halide control raises the unchecked median from the archived
212.283 to 398.765 M turns/s through packed resident state and ping-pong
publication. Halide 21's generated Metal interface uses a per-lane fault plane
rather than the compact device-wide atomic status available to the other
matched controls, leaving a 25.81% checked penalty that is visible in the
reported result.

The same Halide source selects an eight-worker host schedule. The equal window
measures 22.526 ± 0.649 M turns/s checked and 23.490 ± 0.092 unchecked.
Alongside the same-source Taichi
CPU/Metal pair and Mech's unchanged high-level EKF, this supports a focused
backend-portability comparison. Mech leads both CPU modes and checked Metal;
Taichi is 0.08% higher on unchecked Metal, far below either MAD. The result
remains descriptive because schedules, compilers, and status APIs differ.

## Mech backend publication figure

The backend figure stacks eight backends for the same high-level Mech EKF. All
rows use checked publication after every turn and retain every process sample
in the evidence, but they combine 10,000-filter × 20-turn and 500,000-filter × 40-turn
campaigns. The logarithmic scale shows backend reach; it is not evidence for
fine rankings between rows.

| Backend | Workload | n | Median | Observed min-max |
| --- | --- | ---: | ---: | ---: |
| Direct Metal GPU | 500k × 40 | 10 | 409.765 M turns/s | 365.479-423.398 |
| WGPU on Metal | 500k × 40 | 3 | 152.972 M turns/s | 152.314-160.313 |
| SIMD/JIT CPU, 8 workers | 500k × 40 | 10 | 141.362 M turns/s | 122.781-144.882 |
| SIMD/JIT CPU, 1 worker | 10k × 20 | 3 | 41.496 M turns/s | 41.202-41.508 |
| Cranelift SIMD AOT CPU | 10k × 200 | 7 | 34.863 M turns/s | 34.847-34.927 |
| Cranelift JIT CPU | 10k × 20 | 5 | 14.618 M turns/s | 14.068-14.641 |
| Cranelift AOT CPU | 10k × 20 | 5 | 14.593 M turns/s | 14.089-14.644 |
| Scalar artifact evaluator | 10k × 20 | 5 | 1.032 M turns/s | 1.031-1.035 |

## AOT code-generation diagnostic

This comparison uses the same minimal loader, four-argument ABI shape,
one-thread checked publication boundary, and run order alternated by sample.
Each fresh process runs 100 untimed warmup turns, resets state, then measures
10,000 filters × 200 turns. The scalar rows share the exact symbol and AoS
layout. SIMD AOT uses a separate packed four-lane symbol; packing occurs before
the timed region. “One thread” does not mean identical machine instructions:
LLVM SLP-vectorizes local Rust arithmetic, scalar AOT does not, and SIMD AOT
evaluates four filters per generated vector body.

Consequently, neither scalar Mech versus Rust nor SIMD Mech versus scalar Rust
is a matched language comparison. These rows diagnose generated code, AOT
artifact size, and whole-process RSS. The headline language comparison is the
matched four-wide/eight-worker checked and unchecked result above.

| Implementation | n | Median throughput ± MAD | Observed min-max | Library bytes | Median peak RSS ± MAD | RSS min-max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech scalar Cranelift AOT | 7 | 14.671 ± 0.008 M turns/s | 14.458-14.679 | 33,544 | 2,818,048 ± 0 B | 2,818,048-3,014,656 B |
| Optimized Rust `cdylib` | 7 | 21.121 ± 0.005 M turns/s | 21.097-21.129 | 50,016 | 2,818,048 ± 0 B | 2,818,048-2,916,352 B |
| Mech four-lane Cranelift AOT | 7 | 34.863 ± 0.010 M turns/s | 34.847-34.927 | 33,864 | 2,818,048 ± 0 B | 2,818,048-2,818,048 B |

Rust is 43.96% faster than scalar AOT, while four-lane AOT is 65.07% faster than
the scalar Rust control and 137.63% faster than scalar AOT. Those deltas reflect
different code-generation strategies and must not be presented as language
speedups. The generated SIMD AOT library is 32.29% smaller than Rust. Median
whole-process peak RSS is identical and the observed ranges overlap, so the
sample does not establish a memory difference. These are descriptive min-max
ranges over seven independent processes, not confidence intervals. Final
states agree within 4.09e-4 after 200 turns; all samples report zero faults.

Peak RSS includes the common loader and live workload buffers, so it must not
be relabeled as the private memory cost of either library. Likewise, binary
size must not be conflated with the normalized application-source metric in
the separate SIMD/eight-worker audit.

## Full mega chart

The larger archival mega chart keeps additional lanes. Its bar endpoints are
medians when the raw record contains repeated samples. Some legacy rows retain
only a point result; those rows cannot support an uncertainty estimate and
must not be read as precise ranks.

Use the mega chart to show the execution spectrum available to Mech and the
range of comparison implementations. Do not use small gaps between its rows as
evidence that one language or backend is faster.

## Same-machine rerun sensitivity

A September 24 audit found the original Halide installation, Taichi and Mojo
compiler caches, the Mojo build products, and the raw records on the same
Macmini9,1 used for the original campaigns. Exact Halide and Taichi toolchain
versions were rerun. Mojo 1.1.0 release was rerun only as a diagnostic because
the exact September 3 nightly executable had lived in a temporary environment;
its cache and compiled artifacts survive, but the executable does not.

| Lane | Campaign | n | Median | Observed min-max | Exact archived compiler? |
| --- | --- | ---: | ---: | ---: | :---: |
| Halide native Metal, checked | 2026-08-31 retained | 5 | 111.474 | 97.001-120.741 | yes |
| Halide native Metal, checked | 2026-09-24 rerun | 7 | 145.950 | 144.227-149.995 | yes |
| Taichi optimized Metal, checked | 2026-08-31 retained | 5 | 168.798 | 136.096-171.660 | yes |
| Taichi optimized Metal, checked | 2026-09-24 rerun | 7 | 261.847 | 259.784-263.267 | yes |
| Mojo scalar, checked | 2026-09-04 retained | 5 | 22.568 | 22.548-22.573 | nightly |
| Mojo scalar, checked | 2026-09-24 diagnostic | 7 | 19.305 | 18.667-19.320 | no; 1.1.0 release |

The exact-version GPU reruns differ materially from the earlier campaign even
though their within-session ranges are narrow. That can reflect system load,
GPU clock and thermal state, campaign ordering, or other machine state not
captured by a short benchmark. This is why the mega chart is presented as a
performance landscape rather than a precise ranking.

For a paper-quality rerun, use randomized or interleaved implementation order,
multiple sessions separated in time, an explicit thermal/power policy, and at
least 10 retained independent process runs per session. Continue to publish all
raw samples and summarize each session with its median and observed range.

The raw September 24 samples and toolchain audit are in
[`results/same-machine-reruns-2026-09-24.json`](results/same-machine-reruns-2026-09-24.json).
