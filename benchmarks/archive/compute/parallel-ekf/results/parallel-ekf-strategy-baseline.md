# Parallel EKF: Baseline

The most direct scalar or fixed-shape control retained for each language. Workload: **10,000 filters x 20 turns where available**. Rows are ordered by checked throughput, fastest to slowest; checked and unchecked are separate columns; source edits are measured against each language's baseline source.

**Scope note:** Historical mixed view retained for compatibility. Interpreted rows are hatched and compiled rows are solid; checked and unchecked remain separate by opacity. Use the interpreted and compiled baseline views for like-for-like execution-boundary comparisons.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Rust | fixed-shape scalar arrays | 243 / 6,059 | 0 / 0 | 19.799 | 20.544 | measured |
| Futhark | data-parallel array program | 56 / 3,098 | 0 / 0 | 19.614 | 19.635 | measured |
| Taichi | Vector/Matrix resident fields | 265 / 8,909 | 0 / 0 | 16.323 | 19.008 | measured |
| Julia | generic scalar arrays | 126 / 4,322 | 0 / 0 | 3.073 | 3.123 | measured |
| Halide | fixed-shape pipeline | 324 / 8,928 | 0 / 0 | 2.707 | 5.058 | measured |
| LuaJIT | generic FFI helper loop | 205 / 4,810 | 0 / 0 | 1.068 | 1.740 | measured |
| Mech | same high-level `.mec` recurrence | 42 / 1,513 | 0 / 0 | 0.919 | 1.029 | measured |
| Lua | flat fixed-shape Lua tables (PUC Lua baseline) | 153 / 7,031 | 0 / 0 | 0.564 | 0.836 | measured |
| Python | standard-library lists and math | 158 / 5,118 | 0 / 0 | 0.246 | 0.356 | measured |
| NumPy | per-filter NumPy loop | 66 / 1,819 | 0 / 0 | 0.040 | 0.053 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
