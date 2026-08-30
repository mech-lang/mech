# Julia EKF inlining probe: Apple M1, 2026-08-30

The benchmark is the checked-in scalar outer-loop Julia EKF with 10,000
persistent filters.  Julia `1.12.6` was run with `--startup-file=no`;
`BLAS.set_num_threads(1)` remains in the source.  Each process includes the
existing five-turn warmup before timing.  Nine isolated processes were measured
for each variant.  No equations, layouts, inputs, or checksum code changed.

The first table matches the archive's original 20 measured-turn protocol:

| Variant | Julia source change | Median million lane-turns/s | Checksum |
| --- | --- | ---: | ---: |
| Original | `Base.@noinline step!` | 2.852 | `2,682,056.074361449` |
| Heuristic | no inline annotation | 2.875 | `2,682,056.074361449` |
| Hint | `Base.@inline step!` | 3.091 | `2,682,056.074361449` |

The forced-inline result is approximately 8.4% above the no-inline result and
7.5% above the unannotated version.  The nine 20-turn throughputs were:

- forced inline: `3.197, 2.985, 3.089, 3.101, 3.095, 3.091, 3.038,
  3.128, 3.040` million lane-turns/s;
- no inline: `2.899, 2.844, 2.875, 2.834, 2.858, 2.921, 2.826, 2.848,
  2.852`;
- unannotated: `2.798, 2.911, 2.821, 2.875, 2.828, 2.887, 2.880, 2.796,
  2.892`.

As a duration-sensitivity check, nine 100-turn processes gave medians of
`3.069M` (forced inline), `2.820M` (unannotated), and `2.800M` (no inline).

One cold-process wall-time check (including Julia startup and compilation) was
`1.59s` for no-inline, `1.57s` for the unannotated version, and `1.58s` for
the inline hint.  This probe therefore keeps `Base.@inline` in the checked-in
control; it does not claim that all Julia functions should be forced inline.
