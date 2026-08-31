# Parallel EKF source-edit cost

This report measures source edits behind the benchmark variants. `Changed lines` is the number of line positions touched by a baseline-to-advanced diff; `changed chars` counts character slots in those changed line blocks. File size is included only for context. The base reference is `hosts/gpu/fixtures/ekf-kernel.mec`.

## Variant matrix

| Language | Baseline source | Baseline lines | Baseline chars | Advanced source | Advanced lines | Advanced chars | Changed lines | Changed chars | Baseline vs Mech lines/chars | Advanced vs Mech lines/chars | Checked M/s | Unchecked M/s |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech | `ekf-kernel.mec` | 116 | 4,974 | `ekf-kernel.mec` | 116 | 4,974 | 0 | 0 | 0 / 0 | 0 / 0 | 246.151 | 241.028 |
| Rust | `parallel_ekf_rust_scalar.rs` | 220 | 6,464 | `parallel_ekf_rust_simd.rs` | 411 | 12,058 | 533 | 15,990 | 256 / 8,883 | 431 / 15,278 | 25.642 | 22.237 |
| Python | `numpy_scalar.py` | 93 | 4,150 | `numpy_scalar.py` | 93 | 4,150 | 0 | 0 | 168 / 8,484 | 168 / 8,484 | -- | 0.055 |
| NumPy | `numpy_scalar.py` | 93 | 4,150 | `numpy_vectorized.py` | 173 | 6,731 | 145 | 6,312 | 168 / 8,484 | 205 / 9,492 | 10.854 | 12.255 |
| Julia | `julia_scalar.jl` | 103 | 4,765 | `julia_simd_intrinsics.jl` | 217 | 8,549 | 245 | 10,636 | 155 / 7,258 | 215 / 9,487 | 31.311 | 32.842 |
| LuaJIT | `luajit_scalar.lua` | 43 | 3,369 | `luajit_fast.lua` | 169 | 7,609 | 174 | 9,238 | 116 / 4,974 | 182 / 9,573 | 1.277 | 15.977 |
| Lua | `luajit_fast.lua` | 169 | 7,609 | `luajit_fast.lua` | 169 | 7,609 | 0 | 0 | 182 / 9,573 | 182 / 9,573 | 0.565 | 0.835 |
| Taichi | `taichi_comparable.py` | 307 | 11,996 | `taichi_optimized.py` | 318 | 14,346 | 255 | 13,096 | 333 / 14,642 | 351 / 17,149 | 168.798 | 217.297 |

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
