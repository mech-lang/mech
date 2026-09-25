# Benchmark statistics and uncertainty

The publication figures use medians, never raw arithmetic averages. Every
headline comparison reports the number of retained process runs and the
observed minimum-to-maximum range. When a figure has room, it also shows every
retained sample.

The range whiskers are descriptive error bars. They are not confidence
intervals. Three to seven runs are too few to justify a precise distributional
claim, and a bootstrap interval for a three-sample median would mostly restate
the sample range. No p-value or claim of statistical significance is made.

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

## Cross-language CPU publication figure

The publication-facing CPU figure extends the matched Mech–Rust anchor with
the closest retained Mojo, Julia, Futhark, and NumPy/Numba rows. All six run on
the Apple M1 CPU with 500,000 filters × 40 turns, f32 state, eight workers, and
fused execution. Checked and unchecked modes remain separate.

| Implementation | Checked median (observed min-max) | Unchecked median (observed min-max) |
| --- | ---: | ---: |
| Mech | 145.573 (139.668-147.381), n=3 | 165.830 (163.526-171.894), n=3 |
| Rust | 146.509 (141.568-146.999), n=3 | 163.866 (162.420-169.581), n=3 |
| Mojo | 143.519 (138.764-144.131), n=5 | 158.512 (121.695-159.013), n=5 |
| Julia | 128.544 (126.952-128.650), n=3 | 133.605 (108.763-136.755), n=3 |
| Futhark | 108.718 (107.497-109.275), n=5 | 152.330 (151.093-154.089), n=5 |
| NumPy/Numba | 80.323 (80.111-80.358), n=3 | 81.972 (79.427-82.242), n=3 |

Mech and Rust additionally match block-atomic rollback and fault metadata.
The other implementations match the successful-path physical shape but expose
different fault interfaces. NumPy/Numba is a Numba-compiled parallel kernel,
not interpreted Python. All samples reported zero faults.

## Metal publication figure

The Metal figure fixes the Apple M1 GPU workload at 500,000 filters × 40 turns,
resident f32 state, and synchronization after every turn. It compares complete
compiler/runtime paths rather than isolated kernel languages.

| Implementation | Checked median (observed min-max) | Unchecked median (observed min-max) |
| --- | ---: | ---: |
| Mech generated MSL | 422.702 (401.943-428.966), n=5 | 421.651 (365.737-422.463), n=5 |
| Rust host + hand-written MSL | 416.215 (401.396-424.149), n=7 | 418.697 (410.126-420.335), n=7 |
| Mojo native Metal | 244.493 (242.386-247.933), n=5 | 405.047 (393.600-407.075), n=5 |
| Julia Metal.jl, matched packed SoA | 406.432 (402.129-407.495), n=7 | 406.803 (405.092-414.707), n=7 |
| Taichi native Metal | 168.798 (136.096-171.660), n=5 | 217.297 (202.628-285.400), n=5 |
| Halide Metal | 111.474 (97.001-120.741), n=5 | 212.283 (175.664-309.115), n=5 |

The Rust control adopts the direct Mech backend's resident SoA, 64-thread
threadgroups, ping-pong publication, compact shared fault status, and per-turn
command/wait boundary. Its ranges overlap Mech's in both modes. This supports
comparable generated-versus-hand-written Metal execution, not a Rust-to-Metal
compiler claim: stable Rust hosts a hand-written MSL kernel here.

The new Julia control uses a component-major packed SoA and gives checked and
unchecked modes identical resident buffers, bindings, 64-thread launch
geometry, publication, and synchronization. Checked adds only the candidate
predicates and shared two-word fault status. Its median checking cost is 0.09%
and the observed ranges overlap. This replaces the older Julia path whose
checked mode paid for extra bindings and host fault-array transport.

## Mech backend publication figure

The backend figure stacks eight backends for the same high-level Mech EKF. All
rows use checked publication after every turn and show every retained process
sample, but they combine 10,000-filter × 20-turn and 500,000-filter × 40-turn
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
