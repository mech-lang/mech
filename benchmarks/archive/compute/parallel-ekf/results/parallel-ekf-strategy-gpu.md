# Parallel EKF: Synchronized GPU

One GPU dispatch and completion wait per turn; checked rows retain the prior published state on a fault. Workload: **500,000 filters x 40 turns, synchronized per turn**. Rows are ordered by checked throughput, fastest to slowest; checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; native Metal dispatch | 42 / 1,513 | 0 / 0 | 187.999 | 275.534 | measured |
| Julia | direct Metal kernel with retained state | 320 / 9,298 | 361 / 13,273 | 178.135 | 199.454 | measured |
| Taichi | optimized native Metal kernel | 282 / 11,424 | 224 / 11,640 | 168.798 | 217.297 | measured |
| Halide | strict native Metal pipeline | 324 / 8,928 | 0 / 0 | 111.474 | 212.283 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.

## Missing backends and untested controls

- **Rust**: backend/strategy unavailable.
- **NumPy**: backend/strategy unavailable.
- **Python**: backend/strategy unavailable.
- **LuaJIT**: backend/strategy unavailable.
- **Lua**: backend/strategy unavailable.
- **Futhark**: backend/strategy unavailable.
