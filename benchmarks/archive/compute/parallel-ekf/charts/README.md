# Parallel EKF Mega Chart

These are the checked and unchecked cross-language charts for the parallel EKF
comparison. GPU rows use the established diagonal hatch pattern; CPU rows are
solid. The PyPy rows are the scalar 10,000-filter x 20-turn rerun recorded in
`../results/apple-m1-pypy-2026-09-05.json`.

- `parallel-ekf-cross-language-checked.svg`
- `parallel-ekf-cross-language-unchecked.svg`
- `parallel-ekf-cross-language-full.html` (both SVGs stacked)

The historical rows retain their original workload labels. The current direct
Metal rows use the 500,000-filter x 40-turn Apple M1 audit.
