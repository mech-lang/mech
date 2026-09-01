# Parallel EKF execution-strategy reports

The existing source-edit mega report remains the complete cross-variant view. These compact reports select one representative source and one checked/unchecked result per language for each execution strategy. The baseline is split into interpreted and compiled controls; the historical mixed baseline remains for compatibility.

| Strategy | Diff table | Graph |
| --- | --- | --- |
| Interpreted baseline | [`parallel-ekf-strategy-interpreted-baseline.md`](parallel-ekf-strategy-interpreted-baseline.md) | [`parallel-ekf-strategy-interpreted-baseline.svg`](parallel-ekf-strategy-interpreted-baseline.svg) |
| Compiled baseline | [`parallel-ekf-strategy-compiled-baseline.md`](parallel-ekf-strategy-compiled-baseline.md) | [`parallel-ekf-strategy-compiled-baseline.svg`](parallel-ekf-strategy-compiled-baseline.svg) |
| Baseline | [`parallel-ekf-strategy-baseline.md`](parallel-ekf-strategy-baseline.md) | [`parallel-ekf-strategy-baseline.svg`](parallel-ekf-strategy-baseline.svg) |
| Single-core | [`parallel-ekf-strategy-single-core.md`](parallel-ekf-strategy-single-core.md) | [`parallel-ekf-strategy-single-core.svg`](parallel-ekf-strategy-single-core.svg) |
| Eight-worker multicore | [`parallel-ekf-strategy-multicore.md`](parallel-ekf-strategy-multicore.md) | [`parallel-ekf-strategy-multicore.svg`](parallel-ekf-strategy-multicore.svg) |
| Synchronized GPU | [`parallel-ekf-strategy-gpu.md`](parallel-ekf-strategy-gpu.md) | [`parallel-ekf-strategy-gpu.svg`](parallel-ekf-strategy-gpu.svg) |
| GPU batch ceiling | [`parallel-ekf-strategy-gpu-batched.md`](parallel-ekf-strategy-gpu-batched.md) | [`parallel-ekf-strategy-gpu-batched.svg`](parallel-ekf-strategy-gpu-batched.svg) |

## Omitted controls

Languages with no measured result are omitted from the corresponding tables and graphs. The reason is retained here:

| Strategy | Language | Reason |
| --- | --- | --- |
| single-core | Python | backend/strategy unavailable |
| single-core | Taichi | backend/strategy unavailable |
| multicore | Python | backend/strategy unavailable |
| multicore | LuaJIT | backend/strategy unavailable |
| multicore | Lua | backend/strategy unavailable |
| gpu | Rust | backend/strategy unavailable |
| gpu | NumPy | backend/strategy unavailable |
| gpu | Python | backend/strategy unavailable |
| gpu | LuaJIT | backend/strategy unavailable |
| gpu | Lua | backend/strategy unavailable |
| gpu | Futhark | backend/strategy unavailable |
| gpu-batched | Rust | backend/strategy unavailable |
| gpu-batched | NumPy | backend/strategy unavailable |
| gpu-batched | Python | backend/strategy unavailable |
| gpu-batched | Julia | backend/strategy unavailable |
| gpu-batched | LuaJIT | backend/strategy unavailable |
| gpu-batched | Lua | backend/strategy unavailable |
| gpu-batched | Taichi | backend/strategy unavailable |
| gpu-batched | Halide | backend/strategy unavailable |
| gpu-batched | Futhark | backend/strategy unavailable |

## Evidence gaps

The generator does not turn an absent measurement into zero. Applicable cells still awaiting a run are listed here and in the JSON `missing_cells` array:

| Strategy | Language | Mode |
| --- | --- | --- |
| gpu-batched | Mech | checked |

`partial` means a source exists but a checked/unchecked measurement is not retained; those cells are listed above rather than being fabricated.
