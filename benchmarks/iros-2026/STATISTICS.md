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

These samples share one workload, contract, machine, and measurement campaign:
500,000 filters x 40 turns, four-wide SIMD, eight workers, fused execution, and
checked publication with block-start rollback.

| Implementation | n | Median | Observed min-max |
| --- | ---: | ---: | ---: |
| Mech SIMD/JIT | 3 | 145.573 M turns/s | 139.668-147.381 |
| Rust packed SIMD | 3 | 146.509 M turns/s | 141.568-146.999 |

The median gap is 0.936 M turns/s, or 0.64%. The observed ranges overlap. The
supported conclusion is that these retained runs show comparable throughput;
they do not establish that either implementation is faster.

## Broad comparison and full mega chart

The publication-facing representative chart shows every retained sample,
median diamonds, and observed-range whiskers. It combines retained campaigns
with different implementation strategies and workload boundaries, so it is
still a performance landscape rather than a rank.

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
