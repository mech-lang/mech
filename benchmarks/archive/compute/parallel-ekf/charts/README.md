# Parallel EKF Mega Chart

These are the checked and unchecked cross-language charts for the parallel EKF
comparison. GPU rows use the established diagonal hatch pattern; CPU rows are
solid. The PyPy rows are the scalar 10,000-filter x 20-turn rerun recorded in
`../results/apple-m1-pypy-2026-09-05.json`.

The scalar rows compare identical textbook-fidelity and optimized sources under
CPython and PyPy. The earlier historical Python row is intentionally omitted;
it used a different source and workload and was not a valid interpreter
comparison.

- `parallel-ekf-cross-language-checked.svg`
- `parallel-ekf-cross-language-unchecked.svg`
- `parallel-ekf-cross-language-full.html` (both SVGs stacked)

The current direct Metal rows use the 500,000-filter x 40-turn Apple M1 audit.

The chart is a performance-spectrum overview, not a fine-grained ranking.
Endpoints are medians where repeated raw samples are retained, but the combined
rows come from multiple same-machine campaigns and some legacy rows have only
a point result. The publication-facing matched Mech/Rust figure shows every
sample and observed min-max whiskers; see
[`../../../../iros-2026/STATISTICS.md`](../../../../iros-2026/STATISTICS.md).
