# Parallel EKF source-edit cost

This report measures source edits behind the benchmark variants. `Changed lines` is the number of line positions touched by a baseline-to-advanced diff; `changed chars` counts character slots in those changed line blocks. File size is included only for context. The base reference is `hosts/gpu/fixtures/ekf-kernel.mec`.

## Variant matrix

| Language | Baseline source | Advanced source | Base -> advanced lines | Base -> advanced chars | Baseline vs Mech lines | Baseline vs Mech chars | Advanced vs Mech lines | Advanced vs Mech chars | Checked M/s | Unchecked M/s |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech | `ekf-kernel.mec` | `ekf-kernel.mec` | 0 | 0 | 0 | 0 | 0 | 0 | 246.151 | 241.028 |
| Rust | `parallel_ekf_rust_scalar.rs` | `parallel_ekf_rust_simd.rs` | 533 | 15990 | 256 | 8883 | 431 | 15278 | 25.642 | 22.237 |
| Python | `numpy_scalar.py` | `numpy_scalar.py` | 0 | 0 | 168 | 8484 | 168 | 8484 | -- | 0.055 |
| NumPy | `numpy_scalar.py` | `numpy_vectorized.py` | 145 | 6312 | 168 | 8484 | 205 | 9492 | 10.854 | 12.255 |
| Julia | `julia_scalar.jl` | `julia_simd_intrinsics.jl` | 245 | 10636 | 155 | 7258 | 215 | 9487 | 31.311 | 32.842 |
| LuaJIT | `luajit_scalar.lua` | `luajit_fast.lua` | 174 | 9238 | 116 | 4974 | 182 | 9573 | 1.277 | 15.977 |
| Lua | `luajit_fast.lua` | `luajit_fast.lua` | 0 | 0 | 182 | 9573 | 182 | 9573 | 0.565 | 0.835 |
| Taichi | `taichi_comparable.py` | `taichi_optimized.py` | 255 | 13096 | 333 | 14642 | 351 | 17149 | 168.798 | 217.297 |

## Interpretation

- **Mech**: The source recurrence does not change. Native Metal specialization is backend support, not a second Mech program. Baseline -> advanced touches **0 lines / 0 characters**.
- **Rust**: The advanced control changes the value representation and execution loop. Baseline -> advanced touches **533 lines / 15990 characters**.
- **Python**: There is no separate optimized Python source in the evidence set; NumPy vectorization is reported as its own row. Baseline -> advanced touches **0 lines / 0 characters**.
- **NumPy**: This is a whole-program rewrite around NumPy array operations. Baseline -> advanced touches **145 lines / 6312 characters**.
- **Julia**: The advanced source introduces an explicit packed value type and lane loop. Baseline -> advanced touches **245 lines / 10636 characters**.
- **LuaJIT**: The advanced source removes helper-level matrix temporaries and writes each component directly. Baseline -> advanced touches **174 lines / 9238 characters**.
- **Lua**: The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control. Baseline -> advanced touches **0 lines / 0 characters**.
- **Taichi**: This is the source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync. Baseline -> advanced touches **255 lines / 13096 characters**.

## Mech backend support footprint

The high-level Mech source delta is zero, but the native-Metal backend support changed **433 line slots** (348 added / 85 deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.

The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.
