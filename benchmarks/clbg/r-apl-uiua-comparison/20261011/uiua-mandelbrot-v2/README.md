# Uiua Mandelbrot native packing candidate

This additive 11-LOC candidate preserves the original Uiua Mandelbrot application,
all its recurrence and escape tests, and the original frozen sources and timings.
Only `Pack` changes. The original body performs multiplication and reduction in a
user function applied once per byte. The new body first multiplies the complete
row's bit matrix by a singleton row of eight bit weights, then applies plain
`Rows Reduce Add`.

The pinned Uiua 0.19.1 compiler recognizes plain `Rows Reduce` and replaces it
with its existing depth-aware reduction. The runtime's ordinary pervasive
multiplication broadcasts the weights; its ordinary depth-aware addition reduces
every eight-bit group. This introduces no host helper or workload-specific
executor. `Pack` retains zero padding of partial final bytes.

The complete algorithm remains visible in Uiua: runtime `N` determines both
dimensions, every row covers all `N` pixels, and each row performs its ordinary
complex recurrence until all pixels have escaped or 50 iterations have occurred.
There is no analytic interior test or input-specific shortcut.

`validate.py` acquires `/tmp/mech-clbg-timing.lock` before any reference/runtime
probe. It checks 15 complete small PBM outputs, five recurrence traces, and 14
generic packing cases. Widths include one, odd widths, byte boundaries, and a
large packing-only tail. Instrumentation reports each unchanged recurrence
input (including every pixel's real/imaginary state and live mask) and the exact
bytes returned by `Pack`; it does not change the arithmetic
or loop condition. Inputs to packing-only tests arrive through standard input.

Run `prepare.py`, `validate.py`, and then `prepare_manifest.py` to reproduce the
prepared evidence. Source, runtime, compiler primitive proofs, validation tools,
fixtures, and raw outputs are SHA-256 pinned. Validation performs no full
`N=16000` timing. This folder's manifest is a candidate for separate audited
freezing and three fresh full trials; it does not modify the original cohort or
admit a performance result to the chart.
