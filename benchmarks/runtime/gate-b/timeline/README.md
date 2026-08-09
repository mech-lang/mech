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

The suite retains the original portable or generic programs and adds an optimized
control where the runtime has a materially different fixed-shape implementation:

- Rust has a portable generic matrix control and an exact fixed fused control.
  The fused control calls the same arithmetic kernel as fused Mech and keeps the
  same two state buffers plus per-turn integrity validation. It omits Mech epoch
  bookkeeping, reactive node/output marks, receipts, and atomic publication.
- Mech has the current atomic runtime, the complete resident transactional
  prototype, and the fused resident prototype that an eventual compiler could
  emit for a dense fixed-shape region.
- Julia has the preallocated dynamic-array program and a zero-allocation
  `StaticArrays` program with fixed dimensions.
- Python, Lua, and LuaJIT retain their allocating generic programs and add
  preallocated fixed-shape programs. Normal garbage collection remains enabled.
- NumPy already uses persistent Fortran-contiguous arrays and `out=` buffers; it
  remains a separate library-dispatch control rather than pretending its many
  tiny calls are a fused kernel.

Lua and LuaJIT use process CPU time because this Lua installation has no monotonic
wall-clock module; every other lane uses a monotonic wall clock. MATLAB is absent
when it is not installed. No bytecode Mech lane is reported until an actual EKF
bytecode execution path exists.

Python, NumPy, and Julia report collector time directly. Standard Lua does not
expose collector callbacks or pause duration here, so a lower heap size after an
episode is marked only as an inferred collection. It is evidence that a collector
cycle ran during the sample, not proof that GC caused the full latency deviation.

The horizontal axis is cumulative EKF turns rather than wall time. This gives every
lane the same amount of work at every x position. The graph first overlays every
runtime on one shared linear nanoseconds-per-turn scale for direct throughput
comparison. The stacked panels then use adaptive per-lane scales around the median
so pauses remain visible instead of being flattened by the roughly 350x range.

The Rust timeline binary warms each lane independently for at least 250 ms before
collecting ordered samples. This avoids attributing macOS core selection and CPU
frequency ramp-up to the first native lane.

Install the fixed-shape Julia dependency once with:

```text
JULIA_DEPOT_PATH=/path/to/julia-depot julia --startup-file=no \
  --project=benchmarks/runtime/gate-b/julia \
  -e 'using Pkg; Pkg.instantiate()'
```

Then use the same `JULIA_DEPOT_PATH` when running the collector.

## Apple M1 fair controls, 2026-08-09

| Lane | Median per turn | Rate | p99 per turn | Maximum / median |
| --- | ---: | ---: | ---: | ---: |
| Rust fixed fused | 69.5 ns | 14.395 MHz | 69.7 ns | 1.014x |
| Mech resident fused | 100.6 ns | 9.944 MHz | 101.0 ns | 1.046x |
| Rust generic | 112.2 ns | 8.915 MHz | 112.5 ns | 1.005x |
| Julia StaticArrays | 130.5 ns | 7.661 MHz | 131.3 ns | 1.007x |
| Mech retained complete | 196.1 ns | 5.098 MHz | 196.5 ns | 1.002x |
| Julia dynamic preallocated | 252.2 ns | 3.965 MHz | 253.6 ns | 1.020x |
| LuaJIT fixed preallocated | 2.244 us | 0.446 MHz | 2.270 us | 1.012x |
| LuaJIT generic allocating | 5.312 us | 0.188 MHz | 5.360 us | 1.010x |
| Mech current atomic | 11.113 us | 0.090 MHz | 11.183 us | 1.019x |
| Lua fixed preallocated | 16.021 us | 0.062 MHz | 16.087 us | 1.014x |
| NumPy preallocated | 21.396 us | 0.047 MHz | 21.447 us | 1.003x |
| Lua generic allocating | 25.258 us | 0.040 MHz | 25.493 us | 1.017x |
| Python fixed preallocated | 30.885 us | 0.032 MHz | 30.945 us | 1.009x |
| Python generic allocating | 39.330 us | 0.025 MHz | 39.424 us | 1.006x |

The optimized pairs improve Julia by 48.2%, LuaJIT by 57.8%, Lua by 36.6%,
and pure Python by 21.5%. The fused Mech target is 44.8% slower than the matched
shared-kernel Rust control and 23.0% faster than fixed-shape Julia. The
complete retained Mech turn is 50.3% slower than fixed-shape Julia; that is the
cost of the remaining transaction, validation, fingerprint, and retained-receipt
machinery, not a kernel claim.

No exact GC interval was reported by Python or NumPy. Both Julia lanes report
zero allocated bytes in their timed regions. Lua heap drops are marked only as
inferred collections and do not prove that GC caused an entire latency sample.
See [`ekf-latency-timeline-fair-apple-m1.svg`](ekf-latency-timeline-fair-apple-m1.svg)
and the separate shared-scale throughput graph in Hz,
[`ekf-throughput-shared-scale-fair-apple-m1.svg`](ekf-throughput-shared-scale-fair-apple-m1.svg).
Raw samples, runtime versions, and per-lane optimization classes are in
[`RESULTS_FAIR_APPLE_M1_2026-08-09.json`](RESULTS_FAIR_APPLE_M1_2026-08-09.json).
