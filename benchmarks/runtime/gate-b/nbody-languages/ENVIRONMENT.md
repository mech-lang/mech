# Benchmark environment

- Date: 2026-08-12
- Hardware: Apple M1 Mac mini, 8 cores (4 performance, 4 efficiency), 8 GB RAM
- Operating system: macOS 15.6.1 (24G90)
- Rust: rustc 1.96.0-nightly (ec818fda3 2026-03-02), LLVM 22.1.0
- Julia: 1.12.6
- Lua: 5.5.1
- LuaJIT: 2.1.1785763465
- Python: 3.14.6
- NumPy: 2.5.2

NumPy used its isolated environment at `target/benchmarks/nbody/venv` and ran
with `OPENBLAS_NUM_THREADS=1`. System load was not artificially isolated, so the
five-process median is the reported statistic rather than the minimum sample.
