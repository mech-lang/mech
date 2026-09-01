# Parallel EKF: GPU batch ceiling

A device-resident multi-turn submission. This is a throughput ceiling, not a replacement for per-turn observation. Workload: **500,000 filters x 40 turns, one submission where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; device-resident one-submit control | 42 / 1,513 | 0 / 0 | N/A | 3729.673 | partial: missing checked |
| Rust | not applicable: no GPU row | N/A | N/A | N/A | N/A | N/A: no implementation |
| NumPy | not applicable on Apple M1 | N/A | N/A | N/A | N/A | N/A: no implementation |
| Python | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Julia | not applicable: no batch row | N/A | N/A | N/A | N/A | N/A: no implementation |
| LuaJIT | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Lua | not applicable: no GPU backend | N/A | N/A | N/A | N/A | N/A: no implementation |
| Taichi | not applicable: no batch row | N/A | N/A | N/A | N/A | N/A: no implementation |
| Halide | not applicable: no batch row | N/A | N/A | N/A | N/A | N/A: no implementation |
| Futhark | not applicable: no Metal backend | N/A | N/A | N/A | N/A | N/A: no implementation |

`N/A` means the language/backend does not provide this strategy in the retained comparison. `partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
