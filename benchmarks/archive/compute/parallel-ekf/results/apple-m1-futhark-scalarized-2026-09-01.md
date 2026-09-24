# Futhark Mech-level control

Apple M1 (`Macmini9,1`, arm64), Futhark 0.27.1, ISPC 1.31.0. The measured
boundary is one ISPC worker, 10,000 independent filters, 20 resident turns,
and five samples after one warm-up invocation. Checked mode fixes the checked
entry point at compile time; each invalid candidate retains its prior state.

| Source | Checked (M turns/s) | Unchecked (M turns/s) |
| --- | ---: | ---: |
| `futhark_ekf.fut` (array-valued covariance) | 28.620 | 51.693 |
| `futhark_scalar_ekf.fut` (scalar-expanded covariance) | **37.608** | **52.673** |

Both versions return the same valid-workload result within f32 reassociation
(checksums `2682056.0609764401` and `2682056.0609516725`; absolute difference
`2.48e-5`). The scalar-expanded source is intentionally not a Futhark maximum:
it uses one worker to match Mech's four-lane single-core SIMD/JIT boundary. The
old source's one-worker multicore result (`19.614` checked / `19.635` unchecked)
is a scalar C-backend comparison and is not the same execution strategy.

Commands:

```text
futhark ispc --entry-point=main_unchecked --entry-point=main_checked \
  minimal/futhark_scalar_ekf.fut -o futhark-ekf-scalarized-ispc
futhark-ekf-scalarized-ispc --num-threads 1 --entry-point main_checked -r 1 -t time
futhark-ekf-scalarized-ispc --num-threads 1 --entry-point main_unchecked -r 1 -t time
```
