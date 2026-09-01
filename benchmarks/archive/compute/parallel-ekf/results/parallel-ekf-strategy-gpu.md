# Parallel EKF: Synchronized GPU

One GPU dispatch and completion wait per turn; checked rows retain the prior published state on a fault. Workload: **500,000 filters x 40 turns, synchronized per turn**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; native Metal dispatch | 42 / 1,513 | 0 / 0 | 187.999 | 275.534 | measured |
| Rust | not applicable: no GPU row | N/A | N/A | N/A | N/A | N/A: no implementation |
| NumPy | not applicable on Apple M1 | N/A | N/A | N/A | N/A | N/A: no implementation |
| Python | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Julia | direct Metal kernel with retained state | 320 / 9,298 | 361 / 13,273 | 178.135 | 199.454 | measured |
| LuaJIT | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Lua | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Taichi | optimized native Metal kernel | 277 / 11,406 | 224 / 11,640 | 168.798 | 217.297 | measured |
| Halide | strict native Metal pipeline | 324 / 8,928 | 0 / 0 | 111.474 | 212.283 | measured |
| Futhark | not applicable: no Metal backend | N/A | N/A | N/A | N/A | N/A: no implementation |

`N/A` means the language/backend does not provide this strategy in the retained comparison. `partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
