# EKF benchmark results: Apple M1, 2026-08-06

## System

- Mac mini (`Macmini9,1`), Apple M1
- 8 cores: 4 performance and 4 efficiency
- 8 GB memory
- macOS 15.6.1 (24G90)
- Rust `1.96.0-nightly (ec818fda3 2026-03-02)`
- Python 3.14.6
- NumPy 2.5.1
- Lua 5.5.1
- LuaJIT 2.1.1785763465
- Mech base commit `a193ff740`
- Julia 1.12.6 (follow-up run 2026-08-08)

The Rust/Mech target was compiled in release mode. NumPy ran with
`VECLIB_MAXIMUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, and `OMP_NUM_THREADS=1`.
The runners were executed sequentially.

## Results

| Runtime | Median ms/turn | Min | Max | Relative to raw Rust | Relative to NumPy |
| --- | ---: | ---: | ---: | ---: | ---: |
| raw Rust/nalgebra | 0.000102119 | 0.000101845 | 0.000102279 | 1.0x | 0.007x |
| Julia | 0.001137467 | 0.001126371 | 0.001145758 | 11.1x | 0.083x |
| LuaJIT | 0.004209227 | 0.004201707 | 0.004236213 | 41.2x | 0.307x |
| NumPy | 0.013706892 | 0.013686449 | 0.013791977 | 134.2x | 1.0x |
| Lua | 0.018687785 | 0.018129830 | 0.019040797 | 183.0x | 1.36x |
| pure Python | 0.045235267 | 0.045221721 | 0.045362830 | 443.0x | 3.30x |
| Mech source, fail-stop | 0.588722915 | 0.329760896 | 1.157065865 | 5,765.1x | 42.95x |
| Mech source, atomic | 0.629605557 | 0.323549184 | 0.806790847 | 6,165.4x | 45.93x |

All values are medians of nine samples after at least 250 ms of warmup, with
sample batches targeting 75 ms. The two Mech modes were rerun separately from
the already-built release executable because the initial combined run had a
wider timing range. The full raw CSV rows were:

```csv
runtime,operation,median_ms,min_ms,max_ms,batch_iterations
raw-rust-loop,ekf,0.000102119,0.000101845,0.000102279,100000
julia-loop,ekf,0.001137467,0.001126371,0.001145758,30513
luajit-loop,ekf,0.004209227,0.004201707,0.004236213,18750
numpy-loop,ekf,0.013706892,0.013686449,0.013791977,5234
lua-loop,ekf,0.018687785,0.018129830,0.019040797,4167
python-loop,ekf,0.045235267,0.045221721,0.045362830,1640
mech-runtime-source-fail-stop,ekf,0.588722915,0.329760896,1.157065865,260
mech-runtime-source,ekf,0.629605557,0.323549184,0.806790847,255
```

The emitted checksums are intentionally omitted because each independently
calibrated runner completes a different number of persistent turns. Correctness
is established separately by the fixed 256-turn validations.

## Interpretation

This EKF is made of many very small scalar and fixed-size matrix operations.
Raw Rust keeps those operations monomorphized and optimizable as one function.
Julia compiles its small-array loop to about 1.14 microseconds, 11.1x slower
than fixed-size raw Rust but 3.7x faster than LuaJIT and 12.0x faster than
NumPy. LuaJIT compiles the generic flat-matrix loop to about 4.2 microseconds, 3.26x
faster than NumPy. NumPy pays Python-to-C dispatch and temporary-array costs,
but still completes a turn in about 13.7 microseconds. Lua takes about 18.7
microseconds with its garbage collector enabled, and pure Python's generic
matrix loops take about 45.2 microseconds.

The validated Mech fixture contains 145 reactive plan nodes and four live input
bindings. Its complete host turn takes 0.59-0.63 ms. Fail-stop mode is 6.5% faster
than the atomic mode in the isolated rerun, so rollback snapshots matter but do
not explain most of the difference. The dominant remaining work is the runtime
path around dozens of small nodes: four-value host input construction and
admission, dependency/dirty-cell traversal, virtual function dispatch, cell
borrowing, register staging/commit, and per-node matrix/scalar execution.

The high Mech min/max spread also indicates runtime/allocator or OS scheduling
sensitivity that is absent from the tight Rust, NumPy, and Python loops. The
next useful optimization target is fused native-plan execution for a validated
activation body, followed by reusable host input packets and reduced per-turn
dirty-set/allocation work. Optimizing the 3x3 nalgebra kernels will not close a
40x NumPy gap because the raw Rust result shows those kernels are already
negligible.

## Bytecode

No bytecode latency is reported. Bytecode installation changes the EKF
covariance before the first host turn because bytecode v1 does not preserve the
source activation's sampled-dependency and register-initialization topology.
The harness detects this and rejects the lane. A timing from that installed
plan would describe a different, numerically incorrect computation.
