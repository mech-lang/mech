# Compact source controls

These files are the source-minimized controls used by the source-size report.
They preserve the numerical bodies and command-line contract of the retained
benchmark controls while removing explanatory comments, docstrings, and blank
scaffolding. The Mech file is additionally shortened at the language level:
one-letter bindings, compact matrix literals, broadcast input arrays, and
direct state indexing replace the long teaching listing.

The full reference programs remain one directory above (and under
`hosts/gpu/`) so every compact control can be audited against its original.
The compact sources are not a license to change the workload: all checked
controls still validate finite state/covariance, positive covariance
diagonals, and covariance symmetry before publishing a candidate.

`measure.py` currently executes the compact NumPy, Halide, and Futhark
controls. `numpy_numba.py` is a separate eight-worker `@njit(parallel=True)`
control because Numba is an additional JIT runtime rather than plain NumPy.
`julia_metal_ekf.jl` is a direct Julia/Metal control: it stores the same
structure-of-arrays state on the device, launches one thread per filter, and
calls `Metal.synchronize()` after every turn. Its first kernel compilation and
all allocation/readback are outside the timed section. `numpy_gpu.py` is a
CuPy capability probe; it deliberately does not relabel MLX or another Metal
library as NumPy.
The `futhark-ispc-compat.sh` wrapper is used only when Futhark 0.27 is paired
with ISPC 1.31; it removes four unused declarations that otherwise conflict
with ISPC's standard library.
The Rust, Julia, Lua, and Taichi files are compact copies of their
measured controls; their existing throughput rows are retained in the report
until a compact-source rerun is recorded. Runtime availability remains
environment-dependent (for example, Rust SIMD needs its Cargo dependency and
Taichi needs its Python environment).

## GPU controls

The Julia Metal control was measured on the Apple M1 at 500,000 filters x 40
turns (five fresh processes). The medians are **197.078 M turns/s checked**
and **216.462 M turns/s unchecked**, with zero faults and matching checksums.
This is a synchronous per-turn GPU boundary, not a one-submit batch.

Plain NumPy has no GPU backend. CuPy is the NumPy-compatible CUDA option, but
it requires an NVIDIA CUDA device; it cannot target this machine's Apple Metal
GPU. The capability result is retained in
`results/apple-m1-numpy-gpu-2026-08-31.json`, and the probe exits cleanly with
an explicit unavailable result. Run the same source family on the RTX host
after installing CUDA and CuPy rather than inventing a Metal NumPy number.
