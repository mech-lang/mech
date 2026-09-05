# EKF Taichi array-kernel results: Apple M1, 2026-08-30

## System and protocol

- Mac mini (`Macmini9,1`), Apple M1, 8 cores, Metal 3
- Taichi `1.7.4`, Python `3.12.14`, NumPy `2.5.2`
- 100,000 independent filters, `f32` state and covariance
- 2,000 measured turns per sample, five samples
- 25 warmup turns; the first kernel invocation compiles before timing
- state reset and final host readback outside the timed interval
- `ti.sync()` after each measured batch; no Python-level allocation or host
  readback per turn
- one independent f32 reference turn checked before timing

The lane is the generic bearing-only EKF recurrence used by the parallel Mech
fixture: unicycle prediction, covariance propagation, wrapped bearing
innovation, Kalman gain, and Joseph-form covariance update.  The outer loop is
explicitly compiled as Taichi's parallel kernel.  Taichi's sample does not add
Mech's host ingress, dirty scheduling, transaction snapshots, or integrity
failure publication policy, so this is a compute-kernel comparison only.

## Taichi results

| Backend | Worker setting | Median ms/turn | Throughput (million lane-turns/s) | Validation max abs error |
| --- | --- | ---: | ---: | ---: |
| Taichi CPU | default | 1.052151 | 95.043 | `5.960e-08` |
| Taichi CPU | one worker (`--threads 1`) | 4.257589 | 23.487 | `5.960e-08` |
| Taichi Metal | Apple GPU, batch sync | 0.164893 | 606.453 | `7.629e-06` |
| Taichi Metal | Apple GPU, sync each turn | 0.826048 | 121.058 | `7.629e-06` |

The batch Metal result is from five samples of 2,000 turns.  The one-turn-loop
row uses five samples of 500 turns with a device barrier after every kernel
invocation.  A shorter 20-turn batch showed large synchronization noise, so it
is not used here.

For an exact 120-turn comparison with the Mech resident benchmark, 25 samples
were run with 100,000 lanes:

| Taichi Metal mode | Median ms/turn | Throughput (million lane-turns/s) |
| --- | ---: | ---: |
| batch synchronization | 0.222375 | 449.691 |
| synchronization every turn | 0.810595 | 123.366 |

The 120-turn batch is lower than the 2,000-turn asymptote because the fixed
submission/synchronization cost is amortized over fewer turns.  This is the
proper comparison for Mech's `dispatch_turns(120)` result.

## Native control

The fixed-shape Rust control from the parallel EKF archive was compiled with
`rustc -C opt-level=3 -C target-cpu=native` and run with the same 100,000 lanes
and 2,000 turns:

| Runtime | Throughput (million lane-turns/s) |
| --- | ---: |
| Rust fixed-shape sequential outer loop | 20.451 |
| Taichi CPU, one worker | 23.487 |
| Taichi CPU, default workers | 95.043 |
| Taichi Metal, batch sync | 606.453 |
| Taichi Metal, sync each turn | 121.058 |

The one-worker comparison is the closest scalar comparison.  Taichi's default
CPU and Metal numbers include parallel execution of the independent outer
lanes; they should be compared with Mech SIMD/JIT/GPU lanes, not with a
sequential scalar loop.  Existing checked Mech figures are preserved in the
parallel-EKF archive and were not silently relabeled as this run.

Checksums after 2,000 turns were `31,840,691.151` for Rust,
`31,840,690.420` for Taichi CPU, and `31,840,683.442` for Taichi Metal.  The
small accumulated difference is expected from backend-specific `f32`
transcendental implementations; the pre-timing reference check is the stated
correctness gate for this benchmark.
