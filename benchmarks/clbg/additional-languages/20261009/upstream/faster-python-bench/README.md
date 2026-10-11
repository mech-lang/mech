# faster-python-bench

Companion repo for [The Optimization Ladder](https://cemrehancavdar.com/2026/03/06/optimization-ladder/). Every benchmark from the blog post — reproduced, verified, and runnable.

All benchmarks on Apple M4 Pro, CPython 3.14 baseline, median of 5 runs. Every implementation produces identical output — verified against official Benchmarks Game expected values (minor floating-point ordering differences in the last decimal place are expected across compilers).

## Results

### N-body (500K iterations, tight FP loops)

| Approach | Time | Speedup | What it costs you |
|---|---|---|---|
| CPython 3.10 | 1,663ms | 0.75x | Old version |
| CPython 3.14 | 1,242ms | 1.0x | Nothing |
| CPython 3.14t | 1,513ms | 0.82x | GIL-free but slower single-thread |
| Mypyc | 518ms | 2.4x | Type annotations |
| PyPy | 98ms | 13x | Ecosystem compatibility |
| Codon | 47ms | 26x | Separate runtime, limited stdlib |
| Numba | 22ms | 56x | `@njit` + NumPy arrays |
| Taichi | 16ms | 78x | Python 3.13 only (no 3.14 wheels) |
| Mojo | 16ms | 78x | New language + toolchain |
| Cython | 10ms | 124x | C knowledge + landmines |
| Rust (PyO3) | 11ms | 113x | Learning Rust |

### Spectral-norm (N=2000, matrix-vector multiply)

| Approach | Time | Speedup | What it costs you |
|---|---|---|---|
| CPython 3.10 | 16,826ms | 0.83x | Old version |
| CPython 3.14 | 14,046ms | 1.0x | Nothing |
| CPython 3.14t | 14,551ms | 0.97x | GIL-free but slower single-thread |
| Mypyc | 990ms | 14x | Type annotations |
| PyPy | 1,065ms | 13x | Ecosystem compatibility |
| Codon | 99ms | 142x | Separate runtime, limited stdlib |
| Numba | 104ms | 135x | `@njit` + NumPy arrays |
| Mojo | 118ms | 119x | New language + toolchain |
| Rust (PyO3) | 91ms | 154x | Learning Rust |
| Cython | 142ms | 99x | C knowledge + landmines |
| Taichi | 71ms | 198x | Python 3.13 only (no 3.14 wheels) |
| NumPy | 27ms | 520x | Knowing NumPy + O(N^2) memory |

### JSON Pipeline (100K events, end-to-end from raw bytes)

| Approach | Time | Speedup | What it costs you |
|---|---|---|---|
| CPython 3.14 (json.loads + pipeline) | 105ms | 1.0x | Nothing |
| Mypyc (json.loads + pipeline) | 77ms | 1.4x | Type annotations |
| Cython (json.loads + pipeline) | 67ms | 1.6x | C-API dict access |
| Rust (serde, from bytes) | 21ms | 5.0x | New language + bindings |
| Cython (yyjson, from bytes) | 17ms | 6.3x | C library + Cython declarations |

## Deep dives

- [CPython version benchmarks](docs/cpython-versions.md) — 3.10 through 3.14t, the 3.11 jump, free-threading overhead
- [The new wave: Codon, Mojo, Taichi](docs/new-wave-compilers.md) — code, DX verdicts, gotchas
- [The Cython minefield](docs/cython-minefield.md) — how I went from 10.5x to 124x, and the landmines along the way
- [Profiling guide](docs/profiling.md) — cProfile, py-spy, line_profiler workflow

## Running the benchmarks

```bash
# Install dependencies
uv sync

# Generate test data (100K events, 13.6MB)
uv run python data/generate.py

# Run JSON pipeline benchmarks
uv run bench.py

# Run all suites (n-body, spectral-norm, pipeline)
uv run bench_all.py

# Run specific suite
uv run bench_all.py --suite nbody

# Verify correctness
uv run verify_correctness.py

# CPython version comparison (standalone, no dependencies)
uv run bench_cpython_versions.py
```

### Building native extensions

```bash
# Cython (dict pipeline)
uv run python cython_benchmark/setup.py build_ext --inplace

# Cython (yyjson raw pipeline — requires yyjson installed via homebrew)
uv run python cython_benchmark/setup_raw.py build_ext --inplace

# Cython (n-body, spectral-norm)
uv run python cython_benchmark/setup_nbody.py build_ext --inplace
uv run python cython_benchmark/setup_spectral_norm.py build_ext --inplace

# Mypyc
uv run python -c "from mypyc.build import mypycify; from setuptools import setup; setup(packages=[], ext_modules=mypycify(['mypyc_benchmark/nbody.py']))" build_ext --inplace

# Rust (PyO3)
cd rust_benchmark && VIRTUAL_ENV=../.venv uv tool run maturin develop --release && cd ..
```

### Running standalone compilers

These tools can't be imported as Python modules — they compile and run as standalone binaries.

```bash
# Codon (build + run)
~/.codon/bin/codon build -release -o /tmp/bench_codon codon_benchmark/bench.py
DYLD_LIBRARY_PATH=~/.codon/lib/codon /tmp/bench_codon

# Taichi (requires Python 3.13 venv with taichi installed)
/tmp/taichi-venv/bin/python taichi_benchmark/bench.py

# Mojo (via uv, from mojo_benchmark/ directory)
cd mojo_benchmark && uv run --prerelease=allow --extra-index-url https://whl.modular.com/nightly/simple/ --with "mojo>=0.26.3.0.dev0" -- mojo build -O3 -o /tmp/bench_mojo bench.mojo && /tmp/bench_mojo && cd ..
```

## Project structure

Every benchmark directory follows a consistent layout: one file per benchmark problem,
plus a runner for standalone tools that can't be imported as Python modules.

```
xxx_benchmark/
  nbody.py              N-body algorithm   (exports run_benchmark / run_nbody)
  spectral_norm.py      Spectral-norm      (exports run_benchmark / run_spectral)
  pipeline.py           JSON pipeline      (exports run_pipeline, where applicable)
  bench.py              Runner for standalone tools (Codon, Taichi)
  # + build artifacts, setup scripts, toolchain files as needed
```

### Directory listing

```
baseline/               CPython reference implementations (all 3 benchmarks)
numba_benchmark/        Numba @njit (all 3 benchmarks)
cython_benchmark/       Cython pure Python mode (all 3 + yyjson variant)
numpy_benchmark/        NumPy vectorized (spectral-norm only)
mypyc_benchmark/        Mypyc-compiled (all 3 benchmarks)
pypy_benchmark/         PyPy (all 3, identical to baseline — JIT does the work)
rust_benchmark/         Rust/PyO3 (all 3, single lib.rs)
codon_benchmark/        Codon standalone (n-body + spectral-norm)
taichi_benchmark/       Taichi standalone (n-body + spectral-norm, Python 3.13)
mojo_benchmark/         Mojo standalone (n-body + spectral-norm)
data/                   Test data generator (100K events)
docs/                   Deep-dive writeups
bench.py                JSON pipeline benchmark runner
bench_all.py            Unified runner for all 3 suites
bench_cpython_versions.py  CPython version comparison (self-contained)
verify_correctness.py   Correctness verification against reference values
```

## Acknowledgment

Claude was used for fact-checking and for producing more accurate code in languages I'm less familiar with (Rust, Mojo).

## License

Code is MIT. Benchmark numbers are specific to Apple M4 Pro — your hardware will differ.
