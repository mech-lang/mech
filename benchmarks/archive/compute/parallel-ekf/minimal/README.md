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
controls. The Rust, Julia, Lua, and Taichi files are compact copies of their
measured controls; their existing throughput rows are retained in the report
until a compact-source rerun is recorded. Runtime availability remains
environment-dependent (for example, Rust SIMD needs its Cargo dependency and
Taichi needs its Python environment).
