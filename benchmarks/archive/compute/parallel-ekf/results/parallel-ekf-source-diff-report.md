# Parallel EKF source-edit cost

This report measures source edits behind the benchmark variants. Source sizes count non-empty, non-comment code only, so comments and formatting do not make a control look larger. `Changed lines` is the number of line positions touched by a baseline-to-advanced diff; `changed chars` counts character slots in those changed line blocks. The base reference is the checked-in Mech EKF.

## Variant matrix

| Language | Baseline L/C | Advanced L/C | Changed L/C | Baseline vs Mech L/C | Advanced vs Mech L/C | Checked M/s | Unchecked M/s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech | 85 / 3,908 | 85 / 3,908 | 0 / 0 | 0 / 0 | 0 / 0 | 246.151 | 241.028 |
| Rust | 206 / 5,003 | 359 / 8,900 | 533 / 15,990 | 256 / 8,883 | 431 / 15,278 | 25.642 | 22.237 |
| NumPy | 80 / 3,651 | 149 / 5,700 | 145 / 6,312 | 168 / 8,484 | 205 / 9,492 | 10.854 | 12.255 |
| Julia | 96 / 4,382 | 194 / 6,939 | 245 / 10,636 | 155 / 7,258 | 215 / 9,487 | 31.311 | 32.842 |
| LuaJIT | 43 / 3,288 | 153 / 7,031 | 174 / 9,238 | 116 / 4,974 | 182 / 9,573 | 1.277 | 15.977 |
| Lua | 153 / 7,031 | 153 / 7,031 | 0 / 0 | 182 / 9,573 | 182 / 9,573 | 0.565 | 0.835 |
| Taichi | 260 / 8,891 | 277 / 11,406 | 255 / 13,096 | 333 / 14,642 | 351 / 17,149 | 168.798 | 217.297 |
| Halide | 18 / 3,875 | 18 / 3,875 | 0 / 0 | 124 / 5,152 | 124 / 5,152 | 2.707 | 5.058 |
| Futhark | 45 / 2,307 | 45 / 2,307 | 0 / 0 | 116 / 4,974 | 116 / 4,974 | 48.391 | 47.824 |

## Interpretation

- **Mech**: The source recurrence does not change. Native Metal specialization is backend support, not a second Mech program. Baseline -> advanced touches **0 lines / 0 characters**.
- **Rust**: The advanced control changes the value representation and execution loop. Baseline -> advanced touches **533 lines / 15990 characters**.
- **NumPy**: The baseline is a per-filter NumPy call from a Python loop; the advanced control uses fixed-shape batched arrays. The row is labeled NumPy because both variants use NumPy for the numeric work. Baseline -> advanced touches **145 lines / 6312 characters**.
- **Julia**: The advanced source introduces an explicit packed value type and lane loop. Baseline -> advanced touches **245 lines / 10636 characters**.
- **LuaJIT**: The advanced source removes helper-level matrix temporaries and writes each component directly. Baseline -> advanced touches **174 lines / 9238 characters**.
- **Lua**: The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control. Baseline -> advanced touches **0 lines / 0 characters**.
- **Taichi**: This is the source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync. Baseline -> advanced touches **255 lines / 13096 characters**.
- **Halide**: Halide is a fixed-shape C++ pipeline JIT. Checked mode selects the previous lane state when the candidate fails the finite/diagonal/symmetry checks. Baseline -> advanced touches **0 lines / 0 characters**.
- **Futhark**: Futhark expresses the lane map in the source. The reported advanced control uses the same source with eight multicore workers; OpenCL is recorded separately when the local driver can execute it. Baseline -> advanced touches **0 lines / 0 characters**.

## Mech backend support footprint

The high-level Mech source delta is zero, but the native-Metal backend support changed **433 line slots** (348 added / 85 deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.

The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.
