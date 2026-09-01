# Parallel EKF source-edit cost

This report measures source edits and runtime factors behind the parallel EKF variants. Source sizes count non-empty, non-comment code only, so comments and formatting do not make a control look larger. `Edit L/C` is the line/character span changed from baseline to advanced; the two `vs Mech` columns use the same metric against the compact checked-in Mech EKF source. The full teaching listing and each row's exact baseline-to-advanced workload are retained in the JSON. Throughput is reported for both baseline and advanced controls, with checked and unchecked kept separate; their headers identify the row workload. The compile/JIT column reports baseline / advanced medians from separately measured compiler or first-run phases; `--` means that timing was unavailable, not that compilation was free. The three max columns are the best retained result in that execution class for each family, shown as checked / unchecked M/s; GPU maxima use synchronized per-turn rows. Throughput provenance, including strict Mech and Halide evidence when present, is recorded in the JSON `benchmark_evidence` field.

## Variant matrix

| Language | Baseline model | Advanced model | Baseline L/C | Advanced L/C | Edit L/C | Baseline vs Mech L/C | Advanced vs Mech L/C | Compile/JIT ms (baseline / advanced; phase in JSON) | Baseline checked M/s (baseline row workload) | Baseline unchecked M/s (baseline row workload) | Advanced checked M/s (advanced row workload) | Advanced unchecked M/s (advanced row workload) | Max single-core M/s (10,000 x 20; checked / unchecked) | Max SIMD/multicore M/s (500,000 x 40 where available; checked / unchecked) | Max GPU M/s (500,000 x 40 synchronized per-turn; checked / unchecked) |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech | compact high-level `.mec` program | same compact `.mec`; native backend selected at build | 42 / 1,513 | 42 / 1,513 | 0 / 0 | 0 / 0 | 0 / 0 | 197.377 / 4.303 | 0.919 | 1.029 | 187.999 | 275.534 | 41.496 / 49.787 | 145.573 / 165.830 | 187.999 / 275.534 |
| Rust | compact fixed-shape scalar control | compact packed four-lane SIMD control | 243 / 6,059 | 566 / 13,848 | 708 / 24,453 | 248 / 7,865 | 596 / 20,653 | 405.700 / 708.794 | 19.799 | 20.544 | 25.650 | 18.627 | 25.650 / 18.627 | 146.509 / 163.866 | -- / -- |
| NumPy | compact per-filter scalar loop | compact batched fixed-shape vectorized operations | 66 / 1,819 | 108 / 2,974 | 94 / 3,123 | 72 / 2,365 | 116 / 3,772 | 86.338 / 85.805 | 0.040 | 0.053 | 11.129 | 12.558 | 10.673 / 12.032 | 80.323 / 81.972 | -- / -- |
| Python | standard-library scalar control | same scalar control; no optimized Python variant | 158 / 5,118 | 158 / 5,118 | 0 / 0 | 183 / 6,356 | 183 / 6,356 | 86.207 / 85.986 | 0.246 | 0.356 | 0.246 | 0.356 | 0.246 / 0.356 | -- / -- | -- / -- |
| Julia | compact generic scalar Julia | compact explicit four-lane SIMD.jl intrinsics | 126 / 4,322 | 196 / 6,935 | 204 / 8,960 | 126 / 4,828 | 196 / 8,156 | 1282.440 / 1379.128 | 3.073 | 3.123 | 30.835 | 32.260 | 30.835 / 35.100 | 128.544 / 133.605 | 178.135 / 199.454 |
| LuaJIT | compact generic matrix helper loop | compact flat fixed-shape scalarized state | 205 / 4,810 | 153 / 7,031 | 242 / 9,393 | 215 / 5,447 | 153 / 7,444 | 3.373 / 3.599 | 1.068 | 1.740 | 1.263 | 15.991 | 1.263 / 15.991 | -- / -- | -- / -- |
| Lua | same compact flat source under PUC Lua | same compact flat source under PUC Lua | 153 / 7,031 | 153 / 7,031 | 0 / 0 | 153 / 7,444 | 153 / 7,444 | 2.943 / 2.843 | 0.565 | 0.835 | 0.565 | 0.835 | 0.565 / 0.835 | -- / -- | -- / -- |
| Taichi | compact Vector/Matrix resident fields | compact scalar SoA fields and unrolled 3x3 arithmetic | 260 / 8,891 | 277 / 11,406 | 224 / 11,640 | 260 / 11,243 | 277 / 13,907 | 446.897 / 391.229 | 176.710 | 194.793 | 168.798 | 217.297 | -- / -- | 86.047 / 98.140 | 176.710 / 217.297 |
| Halide | same fixed-shape JIT pipeline | same pipeline; strict checked publication and fault output | 324 / 8,928 | 324 / 8,928 | 0 / 0 | 349 / 12,174 | 349 / 12,174 | 1440.497 / 1435.551 | 111.474 | 212.283 | 111.474 | 212.283 | 2.707 / 5.058 | 3.270 / 5.593 | 111.474 / 212.283 |
| Futhark | same data-parallel program | same program; multicore worker count | 56 / 3,098 | 56 / 3,098 | 0 / 0 | 87 / 4,332 | 87 / 4,332 | 802.385 / 1501.165 | 19.614 | 19.635 | 48.391 | 47.824 | -- / -- | 48.391 / 47.824 | -- / -- |

## Runtime factors

| Language | Data layout | Turn/dispatch boundary | Validation and publication |
| --- | --- | --- | --- |
| Mech | column-major resident graph values | resident host turn; backend selected at build | checked rejects candidate and keeps prior; unchecked omits checks |
| Rust | fixed scalar arrays -> four-lane packed values | synchronous host loop, one update per turn | checked and unchecked controls; no rollback in unchecked |
| NumPy | per-lane arrays -> batched SoA arrays | Python host loop (scalar) or one vectorized call per turn | checked masked copyback keeps prior lane; unchecked overwrites |
| Python | ordinary Python lists of scalar state/covariance values | synchronous Python loop, one update per turn | checked candidate publication; unchecked overwrites |
| Julia | generic arrays -> explicit four-lane SIMD values | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| LuaJIT | matrix helpers -> flat fixed-shape scalar state | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| Lua | flat fixed-shape Lua tables/FFI-compatible arrays | synchronous host loop, one update per turn | checked candidate publication; unchecked omits checks |
| Taichi | Vector/Matrix fields -> scalar SoA fields | resident kernel with per-turn device synchronization | checked alternate fields keep prior; unchecked writes in place |
| Halide | fixed-shape lane buffers, vectorized by eight | one JIT pipeline call per host turn | checked validates finite/positive/symmetric candidates, reports per-lane faults, and keeps prior; unchecked omits checks |
| Futhark | fixed-size array of 12-value lane states | turn loop inside one compiled invocation; multicore map | checked select keeps prior lane; unchecked selects candidate |

## Interpretation

`--` means that exact checked/unchecked baseline was not part of the retained evidence; it is not a zero-throughput result. Futhark baseline/advanced values differ only by worker count, while Halide, Mech, and the pure-Python control keep the same source across both sides. The source pair and execution-boundary columns make those cases explicit.
Max columns are checked / unchecked M/s. The GPU column uses synchronized/per-turn GPU rows only. Single-thread SIMD/JIT rows remain in the single-core column; the SIMD/multicore column requires an explicit worker, thread, pool, or parallel marker. Multi-turn/fused GPU maxima are retained under gpu_batched in the JSON and in the ranked throughput table; Mech's 3,729.673 M/s one-submit control is a device-resident ceiling, not an equivalent synchronized GPU lane.
Compile/JIT values are baseline / advanced medians. AOT and bytecode phases are source-to-artifact commands; Julia and Taichi are cold first-process/first-kernel measurements; Mech values are emitted source/artifact or backend preparation intervals. See the JSON `compile_times` object for exact phase labels and commands. A missing value means unavailable evidence, not zero compilation cost.

- **Mech**: The compact source recurrence does not change. Native Metal specialization is backend support, not a second Mech program. Baseline -> advanced touches **0 lines / 0 characters**.
- **Rust**: The compact controls preserve the checked-in Rust algorithms while removing narrative scaffolding; the advanced control still changes the value representation and execution loop. Baseline -> advanced touches **708 lines / 24453 characters**.
- **NumPy**: The baseline is a per-filter NumPy call from a Python loop; the advanced control uses fixed-shape batched arrays. The row is labeled NumPy because both variants use NumPy for the numeric work. Baseline -> advanced touches **94 lines / 3123 characters**.
- **Python**: This is a pure-Python lower-bound control using only math and ordinary lists. Baseline and advanced intentionally reference the same source because no optimized Python source variant is retained. Baseline -> advanced touches **0 lines / 0 characters**.
- **Julia**: The compact controls preserve the Julia algorithms; the advanced source introduces an explicit packed value type and lane loop. Baseline -> advanced touches **204 lines / 8960 characters**.
- **LuaJIT**: The compact controls preserve the Lua algorithms; the advanced source removes helper-level matrix temporaries and writes each component directly. Baseline -> advanced touches **242 lines / 9393 characters**.
- **Lua**: The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control. Baseline -> advanced touches **0 lines / 0 characters**.
- **Taichi**: This is the compact source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync. Baseline -> advanced touches **224 lines / 11640 characters**.
- **Halide**: Halide is a fixed-shape C++ pipeline JIT. Checked mode selects the previous lane state when the candidate fails the finite/diagonal/symmetry checks and emits a per-lane fault code for host observation. Baseline -> advanced touches **0 lines / 0 characters**.
- **Futhark**: Futhark expresses the lane map in the source. The reported advanced control uses the same source with eight multicore workers and keeps the turns loop inside one compiled invocation; OpenCL is recorded separately when the local driver can execute it. Baseline -> advanced touches **0 lines / 0 characters**.

## Mech backend support footprint

The high-level Mech source delta is zero, but the native-Metal backend support changed **1082 line slots** (555 added / 527 deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.

The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.
