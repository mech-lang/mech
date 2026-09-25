# Benchmark statistics and uncertainty

The publication figures use medians, never raw arithmetic averages. Comparison
panels use equal, deterministic process windows: the first three retained runs
per row. Their whiskers and `±` labels are
median absolute deviation (MAD), which prevents an isolated interference run
from dominating the plotted spread. The raw JSON retains every sample.

MAD is a robust descriptive error bar, not a confidence interval. Three
charted runs are too few to justify a precise distributional claim. No
p-value or claim of statistical significance is made. Evidence tables outside
the poster charts may still report all runs and observed min-max ranges.

## Headline matched comparison

These retained campaign records share one workload, physical execution shape,
machine, and campaign date: 500,000 filters x 40 turns, four-wide SIMD, eight
workers, and fused execution. Within each mode Mech and Rust use the same
integrity contract. Checked rows
validate every candidate and provide block-start rollback plus fault metadata;
unchecked rows omit those checks.

| Mode | Implementation | n | Median | Observed min-max |
| --- | --- | ---: | ---: | ---: |
| Checked | Mech SIMD/JIT | 3 | 145.573 M turns/s | 139.668-147.381 |
| Checked | Rust packed SIMD | 3 | 146.509 M turns/s | 141.568-146.999 |
| Unchecked | Mech SIMD/JIT | 3 | 165.830 M turns/s | 163.526-171.894 |
| Unchecked | Rust packed SIMD | 3 | 163.866 M turns/s | 162.420-169.581 |

Rust's checked median is 0.64% higher; Mech's unchecked median is 1.20% higher.
The observed ranges overlap in both modes. The supported conclusion is that
these retained runs show comparable throughput; they do not establish that
either implementation is faster. Relative to unchecked execution, validation
reduces median throughput by 12.2% for Mech and 10.6% for Rust in these
campaigns.

## Same-source CPU and Metal figure

The focused poster figure includes only the systems measured here that select
both CPU and Apple Metal from one application file: Mech, Taichi, and Halide.
The workload is 500,000 filters × 40 turns with f32 resident ping-pong state
and a synchronization boundary after every turn; CPU uses eight workers.

| Device | Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | --- | ---: | ---: |
| CPU | Mech Cranelift SIMD/JIT | 104.783 ± 6.092 | 110.469 ± 10.279 |
| CPU | Taichi LLVM | 87.723 ± 0.441 | 95.759 ± 0.023 |
| CPU | Halide native | 23.426 ± 0.026 | 23.940 ± 0.044 |
| Metal | Mech generated MSL | 420.404 ± 3.491 | 419.523 ± 2.940 |
| Metal | Taichi native Metal | 332.584 ± 0.581 | 409.530 ± 2.704 |
| Metal | Halide Metal schedule | 292.500 ± 0.583 | 394.263 ± 0.053 |

Mech has the highest retained median in both modes on both devices. This is a
descriptive result for the measured programs and sessions, not a claim that
Mech is intrinsically faster. The compilers, CPU/GPU schedules, fault-status
observation mechanisms, campaign dates, and system state differ. The figure
puts that claim limit in-frame rather than relying on surrounding prose.

## Cross-language CPU publication figure

The full CPU figure extends the matched Mech–Rust anchor with Mojo, Julia,
Futhark, Taichi, NumPy/Numba, and Halide. All eight run on the Apple M1 CPU
with 500,000 filters × 40 turns, f32 state, and eight workers. Checked and
unchecked modes remain separate, and every row uses the first three processes.

| Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | ---: | ---: |
| Rust | 146.509 ± 0.490 | 163.866 ± 1.446 |
| Mech | 145.573 ± 1.808 | 165.830 ± 2.304 |
| Mojo | 143.986 ± 0.144 | 145.219 ± 13.294 |
| Julia | 128.544 ± 0.106 | 133.605 ± 3.149 |
| Futhark | 108.903 ± 0.372 | 152.479 ± 1.386 |
| Taichi | 87.723 ± 0.441 | 95.759 ± 0.023 |
| NumPy/Numba | 80.323 ± 0.036 | 81.972 ± 0.270 |
| Halide | 23.426 ± 0.026 | 23.940 ± 0.044 |

Mech and Rust additionally match block-atomic rollback and fault metadata.
Fault interfaces differ elsewhere. Taichi and Halide publish every turn while
the other six fuse the 40-turn worker-local block. NumPy/Numba is a
Numba-compiled parallel kernel, not interpreted Python. All samples reported
zero faults.

## Metal publication figure

The Metal figure fixes the Apple M1 GPU workload at 500,000 filters × 40 turns,
resident f32 state, and synchronization after every turn. It compares complete
compiler/runtime paths rather than isolated kernel languages.

Every Metal row uses the first three retained processes.

| Implementation | Checked median ± MAD | Unchecked median ± MAD |
| --- | ---: | ---: |
| Mech generated MSL | 420.404 ± 3.491 | 419.523 ± 2.940 |
| Rust host + hand-written MSL | 418.602 ± 0.067 | 418.519 ± 1.816 |
| Mojo native Metal, matched packed SoA | 404.932 ± 2.388 | 401.865 ± 0.444 |
| Julia Metal.jl, matched packed SoA | 406.432 ± 0.626 | 409.896 ± 4.804 |
| Taichi native Metal, matched packed SoA | 332.584 ± 0.581 | 409.530 ± 2.704 |
| Halide Metal, matched packed SoA | 292.500 ± 0.583 | 394.263 ± 0.053 |

The Rust control adopts the direct Mech backend's resident SoA, 64-thread
threadgroups, ping-pong publication, compact shared fault status, and per-turn
command/wait boundary. The raw ranges overlap Mech's in both modes. This supports
comparable generated-versus-hand-written Metal execution, not a Rust-to-Metal
compiler claim: stable Rust hosts a hand-written MSL kernel here.

The new Julia control uses a component-major packed SoA and gives checked and
unchecked modes identical resident buffers, bindings, 64-thread launch
geometry, publication, and synchronization. Checked adds only the candidate
predicates and shared two-word fault status. Its median checking cost is 0.09%
and the observed ranges overlap. This replaces the older Julia path whose
checked mode paid for extra bindings and host fault-array transport.

The matched Mojo control uses the same physical strategy and reads its
two-word status directly from Apple unified memory. Its checked and unchecked
medians differ by 0.76%, with checked nominally higher, so no checking penalty
is resolved. Matching the path raises checked performance from the archived
244.493 result to 404.932 M turns/s in the equal window. The retained 215.945
M/s unchecked interference sample remains in JSON but does not control the MAD
whisker. Mojo remains 3.68% below Mech checked and 4.21% below unchecked.

The new Taichi control gives both modes the same packed component-major state,
resident double buffers, launch geometry, and publication boundary. Its
cumulative two-word fault status avoids a reset transfer but still requires a
compact host-visible read after every synchronized checked turn. Checked is
18.79% below unchecked at the median; this is a measured protocol/API cost,
not an in-place-versus-ping-pong comparison.

That exact source also targets the eight-worker LLVM CPU backend by changing
one runtime option and selecting a CPU-contiguous packed axis order. The equal
window measures 87.723 ± 0.441 M turns/s checked and 95.759 ± 0.023 unchecked.
The result corroborates the archived 86.047 M/s Taichi CPU row. Its per-turn
synchronization differs from the fused rows and is labeled accordingly in the
full CPU panel.

The matched Halide control raises the unchecked median from the archived
212.283 to 394.263 M turns/s through packed resident state and ping-pong
publication. Halide 21's generated Metal interface uses a per-lane fault plane
rather than the compact device-wide atomic status available to the other
matched controls, leaving a 25.81% checked penalty that is visible in the
reported result.

The same Halide source selects an eight-worker host schedule. The equal window
measures 23.426 ± 0.026 M turns/s checked and 23.940 ± 0.044 unchecked.
Alongside the same-source Taichi
CPU/Metal pair and Mech's unchanged high-level EKF, this supports a focused
backend-portability comparison. Mech has the highest retained median on both
devices in these runs; the result remains descriptive because schedules,
compiler versions, and status-observation APIs differ.

## Mech backend publication figure

The backend figure stacks eight backends for the same high-level Mech EKF. All
rows use checked publication after every turn and retain every process sample
in the evidence, but they combine 10,000-filter × 20-turn and 500,000-filter × 40-turn
campaigns. The logarithmic scale shows backend reach; it is not evidence for
fine rankings between rows.

| Backend | Workload | n | Median | Observed min-max |
| --- | --- | ---: | ---: | ---: |
| Direct Metal GPU | 500k × 40 | 5 | 422.702 M turns/s | 401.943-428.966 |
| WGPU on Metal | 500k × 40 | 3 | 152.972 M turns/s | 152.314-160.313 |
| SIMD/JIT CPU, 8 workers | 500k × 40 | 3 | 104.783 M turns/s | 98.691-128.144 |
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

| Implementation | n | Median throughput | Observed min-max | Library bytes | Median peak RSS | RSS min-max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech scalar Cranelift AOT | 7 | 14.671 M turns/s | 14.458-14.679 | 33,544 | 2,818,048 B | 2,818,048-3,014,656 B |
| Optimized Rust `cdylib` | 7 | 21.121 M turns/s | 21.097-21.129 | 50,016 | 2,818,048 B | 2,818,048-2,916,352 B |
| Mech four-lane Cranelift AOT | 7 | 34.863 M turns/s | 34.847-34.927 | 33,864 | 2,818,048 B | 2,818,048-2,818,048 B |

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
