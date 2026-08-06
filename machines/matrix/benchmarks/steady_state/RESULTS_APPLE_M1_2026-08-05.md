# Apple M1 steady-state matrix results

Date: 2026-08-05

Code base: `feat/native-application-builds` at `2ca07947b`, plus the benchmark
harness in this directory.

## Environment

- Mac mini (2020), Apple M1, 4 performance plus 4 efficiency cores, 8 GB RAM
- macOS 15.6, arm64
- Rust nightly 1.96.0 (2026-03-02)
- Python 3.14.6
- NumPy 2.5.1 using Apple's Accelerate BLAS/LAPACK backend
- Lua 5.5.1
- LuaJIT 2.1.1785763465

NumPy ran with `VECLIB_MAXIMUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, and
`OMP_NUM_THREADS=1`. The native and NumPy values below are the means of two
independent benchmark runs. Python, Lua, and LuaJIT are single runs; each
reported value is still the median of nine timed samples.

## Protocol

- Dense `f64` square matrices and deterministic identical input formulas
- At least 250 ms of untimed warmup before measurement
- Nine samples with approximately 75 ms timed batches
- Persistent inputs, outputs, specialized Mech functions, and reactive plans
- Matrix multiplication reuses its output allocation
- Linear solve performs a fresh LU factorization each iteration in all runners
- Parsing, startup, construction, specialization, and validation are untimed
- Mech reactive runs dirty-cell scheduling on every iteration
- Python cyclic GC and Lua GC are paused only during timed samples
- Every runner validates multiplication output and solve correctness

`mech-kernel` is a virtual `solve_result()` call on the persistent specialized
function. `mech-reactive` additionally includes dirty-cell tracking and
reactive scheduling directly on a `ReactivePlan`; it is not a complete
`MechRuntime` transaction.

## Common size: 256 x 256

Lower is better. Times are milliseconds per operation.

| Runtime | Multiply | Solve |
| --- | ---: | ---: |
| raw Rust / nalgebra | 0.7180 | 1.2211 |
| Mech kernel | 0.7185 | 1.2200 |
| Mech reactive | 0.7189 | 1.2221 |
| NumPy / Accelerate | 0.09964 | 0.18066 |
| LuaJIT | 22.4518 | 8.2028 |
| Lua 5.5 | 650.231 | 211.513 |
| Python 3.14 | 1212.408 | 409.701 |

Relative to raw Rust at this size:

| Mech path | Multiply | Solve |
| --- | ---: | ---: |
| kernel | +0.069% | -0.093% |
| reactive | +0.119% | +0.079% |

The negative solve delta is measurement noise, not a faster implementation:
Mech and raw Rust execute the same nalgebra factorization kernel. At this size,
NumPy is 7.21x faster than Mech reactive for multiplication and 6.76x faster
for solve. LuaJIT is 31.2x and 6.71x slower than Mech reactive. The non-JIT
interpreters are hundreds to thousands of times slower.

## Large size: 1024 x 1024

| Runtime | Multiply (ms) | Solve (ms) |
| --- | ---: | ---: |
| raw Rust / nalgebra | 46.2419 | 77.7955 |
| Mech kernel | 46.2361 | 78.3007 |
| Mech reactive | 46.3464 | 78.1918 |
| NumPy / Accelerate | 8.4503 | 5.6542 |

The reactive multiplication path averages 0.226% over raw Rust. The direct
kernel path is indistinguishable from raw Rust. NumPy is 5.47x faster than raw
Rust for multiplication and 13.76x faster for solve.

Large solve timings have about one percent run-to-run allocator and system
noise because every iteration clones and factors an 8 MiB matrix. Mech kernel
measured +1.14% and +0.16% in the two runs; Mech reactive measured -0.11% and
+1.13%. These results do not support a measurable `Result`-handling cost in
the solve kernel.

## Retained runtime loop: source versus bytecode

This is the end-to-end steady-state Mech measurement. A retained runtime takes
a live scalar host input, opens a reactive transaction, updates the cell,
tracks dirty dependencies, runs a scaling node plus the requested matrix
kernel, validates the turn, and commits. The source and bytecode fixtures have
equal plan lengths and live-input binding counts.

The bytecode fixture is compiled and installed before warmup. Bytecode in this
branch is a serialized plan format: installation reconstructs the same
specialized runtime function nodes used by source execution. It is not a
separate bytecode VM that interprets matrix instructions on every turn.

The table contains the mean of three independent paired runs. Each run still
uses the median of nine samples and alternates source/bytecode sample order.

| Size | Source multiply | Bytecode multiply | Delta | Source solve | Bytecode solve | Delta |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 256 | 0.9850 ms | 0.9762 ms | -0.893% | 1.3200 ms | 1.3202 ms | +0.009% |
| 512 | 6.4800 ms | 6.4730 ms | -0.108% | 10.1330 ms | 10.1455 ms | +0.123% |
| 1024 | 49.3361 ms | 49.3547 ms | +0.038% | 77.2172 ms | 77.1804 ms | -0.048% |

The 64 and 128 results are intentionally omitted from the summary because
sub-millisecond turns are bimodal on this 4-performance/4-efficiency-core M1.
At compute-heavy sizes, source-installed and bytecode-installed steady-state
performance is equivalent.

For context, the source runtime loop versus a current raw-nalgebra kernel
control measured +37.0%, +13.7%, and +6.65% for multiplication at 256, 512,
and 1024. Solve measured +8.0%, +0.29%, and +0.01%. Multiplication includes
the extra O(n^2) scaling node needed to make the matrix input reactive, so
these are full-graph costs rather than isolated runtime overhead percentages.

## Interpretation

The plan-level Mech execution machinery is not the matrix bottleneck. On the
shared nalgebra kernel, `solve_result`, virtual dispatch, borrowing, and direct
reactive scheduling add roughly one tenth of one percent at 256. Complete
runtime transaction cost is visible for shorter kernels but is amortized as
matrix work grows. Source and bytecode installation converge on the same
steady-state executor. The largest performance gap remains between nalgebra's
generic native kernels and NumPy's Accelerate-backed kernels, not between Mech
and raw Rust or between source and bytecode Mech.
