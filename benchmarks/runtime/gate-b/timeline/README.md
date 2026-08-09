# EKF latency over time

This benchmark preserves the order of repeated steady-state EKF episode timings so
runtime pauses remain visible instead of being collapsed into a median. Each sample
contains 4,096 turns of the frozen Gate B EKF v1 trace. Runtime activation, fixture
construction, and state reset occur outside the timer. Garbage collection remains
enabled.

Run all installed controls with:

```text
python3 scripts/run-gate-b-ekf-timeline.py \
  --samples 60 \
  --output benchmarks/runtime/gate-b/timeline/results.json
```

The lanes are raw Rust, retained Mech resident complete turns, current atomic Mech,
NumPy, scalar Python, Lua, LuaJIT, and Julia. Lua and LuaJIT use process CPU time
because this Lua installation has no monotonic wall-clock module; every other lane
uses a monotonic wall clock. MATLAB is absent when it is not installed. No bytecode
Mech lane is reported until an actual EKF bytecode execution path exists.

Python, NumPy, and Julia report collector time directly. Standard Lua does not
expose collector callbacks or pause duration here, so a lower heap size after an
episode is marked only as an inferred collection. It is evidence that a collector
cycle ran during the sample, not proof that GC caused the full latency deviation.

The horizontal axis is cumulative EKF turns rather than wall time. This gives every
lane the same amount of work at every x position. The graph first overlays every
runtime on one shared linear nanoseconds-per-turn scale for direct throughput
comparison. The stacked panels then use adaptive per-lane scales around the median
so pauses remain visible instead of being flattened by the roughly 350x range.

## Apple M1, 2026-08-09

| Lane | Median per turn | p99 per turn | Maximum / median |
| --- | ---: | ---: | ---: |
| Raw Rust | 112.0 ns | 112.3 ns | 1.003x |
| Mech retained complete | 192.9 ns | 193.5 ns | 1.005x |
| Julia persistent | 252.7 ns | 253.4 ns | 1.003x |
| LuaJIT scalar | 5.419 us | 5.464 us | 1.009x |
| Mech current atomic | 11.097 us | 11.197 us | 1.013x |
| NumPy persistent | 21.228 us | 21.274 us | 1.002x |
| Lua scalar | 25.139 us | 25.388 us | 1.010x |
| Python scalar | 39.315 us | 39.615 us | 1.029x |

No exact GC interval was reported by Python, NumPy, or Julia. Lua heap drops
indicate 22 inferred standard-Lua and 38 inferred LuaJIT collection cycles, but
they do not align with material pauses in this run. See
[`ekf-latency-timeline-apple-m1.svg`](ekf-latency-timeline-apple-m1.svg) and
the separate shared-scale-only throughput graph in Hz,
[`ekf-throughput-shared-scale-apple-m1.svg`](ekf-throughput-shared-scale-apple-m1.svg).
Raw samples are in
[`RESULTS_APPLE_M1_2026-08-09.json`](RESULTS_APPLE_M1_2026-08-09.json).
