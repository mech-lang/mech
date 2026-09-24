# Parallel EKF: GPU batch ceiling

Device-resident multi-turn submission. Workload: **500,000 filters x 40 turns, one submission where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; device-resident one-submit control | 42 / 1,513 | 0 / 0 | N/A | 3729.673 | partial: missing checked |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.

## Missing backends and untested controls

- **Rust**: backend/strategy unavailable.
- **NumPy**: backend/strategy unavailable.
- **Python**: backend/strategy unavailable.
- **Julia**: backend/strategy unavailable.
- **LuaJIT**: backend/strategy unavailable.
- **Lua**: backend/strategy unavailable.
- **Taichi**: backend/strategy unavailable.
- **Halide**: backend/strategy unavailable.
- **Futhark**: backend/strategy unavailable.
