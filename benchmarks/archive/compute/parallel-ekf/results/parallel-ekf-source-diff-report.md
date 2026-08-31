# Parallel EKF source-edit cost

This report measures source edits and runtime factors behind the parallel EKF variants. Source sizes count non-empty, non-comment code only, so comments and formatting do not make a control look larger. `Edit L/C` is the line/character span changed from baseline to advanced; the two `vs Mech` columns use the same metric against the checked-in Mech EKF. The workload column shows lanes x turns for each side; throughput is reported for both baseline and advanced controls, with checked and unchecked kept separate.

## Variant matrix

| Language | Baseline model | Advanced model | Workload (baseline -> advanced) | Baseline L/C | Advanced L/C | Edit L/C | Baseline vs Mech L/C | Advanced vs Mech L/C | Baseline checked M/s | Baseline unchecked M/s | Advanced checked M/s | Advanced unchecked M/s |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech | same high-level `.mec` program | same `.mec`; native backend selected at build | 10,000 x 20 -> 500,000 x 40 | 85 / 3,908 | 85 / 3,908 | 0 / 0 | 0 / 0 | 0 / 0 | 0.869 | -- | 246.151 | 241.028 |
| Rust | fixed-shape scalar control | packed four-lane SIMD control | 10,000 x 20 -> 10,000 x 20 | 206 / 5,003 | 359 / 8,900 | 533 / 15,990 | 256 / 8,883 | 431 / 15,278 | -- | 16.728 | 25.642 | 22.237 |
| NumPy | per-filter scalar loop | batched fixed-shape vectorized operations | 10,000 x 20 -> 10,000 x 20 | 80 / 3,651 | 149 / 5,700 | 145 / 6,312 | 168 / 8,484 | 205 / 9,492 | 0.040 | 0.053 | 11.129 | 12.558 |
| Julia | generic scalar Julia | explicit four-lane SIMD.jl intrinsics | 10,000 x 20 -> 10,000 x 20 | 96 / 4,382 | 194 / 6,939 | 245 / 10,636 | 155 / 7,258 | 215 / 9,487 | 3.119 | 3.137 | 31.311 | 32.842 |
| LuaJIT | generic matrix helper loop | flat fixed-shape scalarized state | 10,000 x 20 -> 10,000 x 20 | 43 / 3,288 | 153 / 7,031 | 174 / 9,238 | 116 / 4,974 | 182 / 9,573 | -- | 1.094 | 1.277 | 15.977 |
| Lua | same flat source under PUC Lua | same flat source under PUC Lua | 10,000 x 20 -> 10,000 x 20 | 153 / 7,031 | 153 / 7,031 | 0 / 0 | 182 / 9,573 | 182 / 9,573 | 0.565 | 0.835 | 0.565 | 0.835 |
| Taichi | Vector/Matrix resident fields | scalar SoA fields and unrolled 3x3 arithmetic | 500,000 x 40 -> 500,000 x 40 | 260 / 8,891 | 277 / 11,406 | 255 / 13,096 | 333 / 14,642 | 351 / 17,149 | 176.710 | 194.793 | 168.798 | 217.297 |
| Halide | same fixed-shape JIT pipeline | same pipeline; checked publication select | 10,000 x 20 -> 10,000 x 20 | 18 / 3,875 | 18 / 3,875 | 0 / 0 | 124 / 5,152 | 124 / 5,152 | 2.707 | 5.058 | 2.707 | 5.058 |
| Futhark | same data-parallel program | same program; multicore worker count | 10,000 x 20 -> 10,000 x 20 | 45 / 2,307 | 45 / 2,307 | 0 / 0 | 116 / 4,974 | 116 / 4,974 | 19.614 | 19.635 | 48.391 | 47.824 |

## Runtime factors

| Language | Data layout | Turn/dispatch boundary | Validation and publication |
| --- | --- | --- | --- |
| Mech | column-major resident graph values | resident host turn; backend selected at build | checked rejects candidate and keeps prior; unchecked omits checks |
| Rust | fixed scalar arrays -> four-lane packed values | synchronous host loop, one update per turn | checked and unchecked controls; no rollback in unchecked |
| NumPy | per-lane arrays -> batched SoA arrays | Python host loop (scalar) or one vectorized call per turn | checked masked copyback keeps prior lane; unchecked overwrites |
| Julia | generic arrays -> explicit four-lane SIMD values | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| LuaJIT | matrix helpers -> flat fixed-shape scalar state | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| Lua | flat fixed-shape Lua tables/FFI-compatible arrays | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| Taichi | Vector/Matrix fields -> scalar SoA fields | resident kernel with per-turn device synchronization | checked alternate fields keep prior; unchecked writes in place |
| Halide | fixed-shape lane buffers, vectorized by eight | one JIT pipeline call per host turn | checked select keeps prior lane; unchecked selects candidate |
| Futhark | fixed-size array of 12-value lane states | turn loop inside one compiled invocation; multicore map | checked select keeps prior lane; unchecked selects candidate |

## Interpretation

`--` means that exact checked/unchecked baseline was not part of the retained evidence; it is not a zero-throughput result. Futhark baseline/advanced values differ only by worker count, while Halide and Mech keep the same source across both sides. The source pair and execution-boundary columns make those cases explicit.

- **Mech**: The source recurrence does not change. Native Metal specialization is backend support, not a second Mech program. Baseline -> advanced touches **0 lines / 0 characters**.
- **Rust**: The advanced control changes the value representation and execution loop. Baseline -> advanced touches **533 lines / 15990 characters**.
- **NumPy**: The baseline is a per-filter NumPy call from a Python loop; the advanced control uses fixed-shape batched arrays. The row is labeled NumPy because both variants use NumPy for the numeric work. Baseline -> advanced touches **145 lines / 6312 characters**.
- **Julia**: The advanced source introduces an explicit packed value type and lane loop. Baseline -> advanced touches **245 lines / 10636 characters**.
- **LuaJIT**: The advanced source removes helper-level matrix temporaries and writes each component directly. Baseline -> advanced touches **174 lines / 9238 characters**.
- **Lua**: The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control. Baseline -> advanced touches **0 lines / 0 characters**.
- **Taichi**: This is the source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync. Baseline -> advanced touches **255 lines / 13096 characters**.
- **Halide**: Halide is a fixed-shape C++ pipeline JIT. Checked mode selects the previous lane state when the candidate fails the finite/diagonal/symmetry checks. Baseline -> advanced touches **0 lines / 0 characters**.
- **Futhark**: Futhark expresses the lane map in the source. The reported advanced control uses the same source with eight multicore workers and keeps the turns loop inside one compiled invocation; OpenCL is recorded separately when the local driver can execute it. Baseline -> advanced touches **0 lines / 0 characters**.

## Mech backend support footprint

The high-level Mech source delta is zero, but the native-Metal backend support changed **433 line slots** (348 added / 85 deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.

The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.
