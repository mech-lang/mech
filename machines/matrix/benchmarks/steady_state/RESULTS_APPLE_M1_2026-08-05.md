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
- Julia 1.12.6

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

## Rectangular follow-up: NumPy's rank-one boundary

Follow-up run date: 2026-08-06.

The rectangular sweep tested `M x K` by `K x N` products through raw nalgebra,
the persistent Mech function, direct reactive scheduling, complete retained
source and bytecode runtimes, and NumPy. All paths reuse their output buffers.
The same warmup, nine-sample median, single-thread environment, deterministic
inputs, and correctness checks apply.

`K = 1` exposes a sharp NumPy/Accelerate general-matmul boundary. To keep the
comparison honest, the table includes both `numpy.matmul(..., out=...)` and the
equivalent optimized `numpy.multiply(..., out=...)` broadcast formulation.
Times are milliseconds per operation.

| Shape | Raw nalgebra | Mech source runtime | Mech bytecode runtime | NumPy matmul | NumPy optimized outer |
| --- | ---: | ---: | ---: | ---: | ---: |
| 512 x 1 x 512 | 0.0272 | 0.5471 | 0.5172 | 1.2118 | 0.0969 |
| 1024 x 1 x 1024 | 0.2185 | 1.3009 | 1.3069 | 6.4649 | 0.4077 |
| 2048 x 1 x 2048 | 0.8819 | 4.3334 | 4.3637 | 34.7773 | 1.8089 |
| 3072 x 1 x 3072 | 2.0630 | 9.9527 | n/a | 47.2163 | 2.9732 |
| 4096 x 1 x 4096 | 3.6868 | 17.3846 | n/a | 139.2732 | 5.3459 |

Against NumPy's general matrix-multiplication API, the complete source runtime
is 2.2x faster at 512, 5.0x at 1024, 8.0x at 2048, 4.7x at 3072, and 8.0x at
4096. The 1024 result and representative rectangular one-million-element
outputs were reproduced in independent runs. This is a specific `K = 1`
dispatch weakness: at `1024 x 2 x 1024`, NumPy takes 0.249 ms while the Mech
source runtime takes 1.452 ms.

The specialized NumPy outer-product formulation reverses the runtime result.
It is 2.4x to 3.4x faster than the complete Mech turn from 1024 through 4096.
Raw nalgebra is still 1.4x to 2.1x faster than optimized NumPy at those sizes,
which locates the remaining gap above the multiplication kernel. The Mech turn
cost grows with the retained output state: at 4096, the kernel takes 3.69 ms
but the full source turn takes 17.38 ms.

Bytecode entries are unavailable at 3072 and 4096 because the dense `f64`
result alone is about 72 MiB and 128 MiB, respectively. Bytecode v1's default
read limit is 64 MiB (`67,108,864` bytes), and the retained result is serialized
into the program image. The source-only benchmark mode records these sizes
without weakening that safety limit.

## Interpretation

The plan-level Mech execution machinery is not the matrix bottleneck. On the
shared nalgebra kernel, `solve_result`, virtual dispatch, borrowing, and direct
reactive scheduling add roughly one tenth of one percent at 256. Complete
runtime transaction cost is visible for shorter kernels but is amortized as
square matrix work grows. Source and bytecode installation converge on the same
steady-state executor. NumPy's optimized BLAS/LAPACK kernels dominate ordinary
dense square work, while nalgebra is stronger on rank-one output generation.
For large retained outputs, transaction-level state handling can outweigh that
kernel advantage even though source and bytecode execution remain equivalent.

## Julia follow-up

Julia 1.12.6 was added on 2026-08-08 with `JULIA_NUM_THREADS=1` and the same
single-thread environment, deterministic inputs, warmup, nine samples, and
75 ms target batches. Julia's garbage collector remains enabled and any
allocation or collection caused by `\` is measured.

| Size | Multiply | Scale + transpose | Solve |
| ---: | ---: | ---: | ---: |
| 64 | 0.012364 ms | 0.002240 ms | 0.021687 ms |
| 128 | 0.091621 ms | 0.010094 ms | 0.094692 ms |
| 256 | 0.700237 ms | 0.040333 ms | 0.482867 ms |
| 512 | 5.525095 ms | 0.163608 ms | 2.818546 ms |

At 256, Julia multiplication matches portable nalgebra/Mech within 2.5%, its
materialized scale-plus-transpose is faster than the prior NumPy result, and
its solve is 2.7x faster than portable nalgebra but 2.7x slower than
NumPy/Accelerate. This official Julia binary uses its own BLAS/LAPACK routing;
the table is a language-runtime baseline, not a claim that all rows share the
same backend library.
