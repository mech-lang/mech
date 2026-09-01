# Parallel EKF execution-strategy reports

The existing source-edit mega report remains the complete cross-variant view. These compact reports select one representative source and one checked/unchecked result per language for each execution strategy.

| Strategy | Diff table | Graph |
| --- | --- | --- |
| Baseline | [`parallel-ekf-strategy-baseline.md`](parallel-ekf-strategy-baseline.md) | [`parallel-ekf-strategy-baseline.svg`](parallel-ekf-strategy-baseline.svg) |
| Single-core | [`parallel-ekf-strategy-single-core.md`](parallel-ekf-strategy-single-core.md) | [`parallel-ekf-strategy-single-core.svg`](parallel-ekf-strategy-single-core.svg) |
| Eight-worker multicore | [`parallel-ekf-strategy-multicore.md`](parallel-ekf-strategy-multicore.md) | [`parallel-ekf-strategy-multicore.svg`](parallel-ekf-strategy-multicore.svg) |
| Synchronized GPU | [`parallel-ekf-strategy-gpu.md`](parallel-ekf-strategy-gpu.md) | [`parallel-ekf-strategy-gpu.svg`](parallel-ekf-strategy-gpu.svg) |
| GPU batch ceiling | [`parallel-ekf-strategy-gpu-batched.md`](parallel-ekf-strategy-gpu-batched.md) | [`parallel-ekf-strategy-gpu-batched.svg`](parallel-ekf-strategy-gpu-batched.svg) |

## Evidence gaps

The generator does not turn an absent measurement into zero. Applicable cells still awaiting a run are listed here and in the JSON `missing_cells` array:

| Strategy | Language | Mode |
| --- | --- | --- |
| baseline | Rust | checked |
| baseline | LuaJIT | checked |
| gpu-batched | Mech | checked |

`N/A` means the backend or strategy is not available in this comparison. `partial` means a source exists but a checked/unchecked measurement is not retained; those cells are listed above rather than being fabricated.
