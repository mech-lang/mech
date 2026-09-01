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
controls. Pass `--halide-metal` to schedule the same Halide pipeline with
`gpu_tile` for Apple's native Metal backend; the state buffers remain resident
on the device across turns. The ordinary final state readback is outside the
timed region; strict checked mode additionally reads its fault buffer after each
turn because that observation is part of the checked contract.
The earlier fused-only row (275.831 M turns/s unchecked and 274.112 M turns/s
checked) did not expose per-lane fault observations and is retained only as a
kernel-throughput control. The strict checked control is in
`results/apple-m1-halide-metal-strict-2026-08-31.json`; it emits a fault code
for every lane, synchronizes, scans those codes, and retains the previous state
for invalid candidates. On the matched Apple M1 500,000-filter x 40-turn run,
the five-sample medians were **111.474 M turns/s strict checked** and
**212.283 M turns/s unchecked**. The Halide GPU pipeline emits one fused tuple
kernel, materializes shared per-lane intermediates once, and waits for
completion after every turn. `numpy_numba.py` is a separate eight-worker `@njit(parallel=True)`
control because Numba is an additional JIT runtime rather than plain NumPy.
For a fault-path smoke test, pass `fault` as the sixth Halide argument; lane 0
receives an infinite velocity, the candidate is rejected, and the checked
runner reports the fault while retaining that lane's prior publication.
`julia_metal_ekf.jl` is a direct Julia/Metal control: it stores the same
structure-of-arrays state on the device, launches one thread per filter, and
calls `Metal.synchronize()` plus a compact fault readback after every checked
turn. Checked mode uses two resident state groups and swaps the published
group only after the fault summary is clear; this matches Mech's whole-turn
retained-state contract. Its first kernel compilation and all allocation/final
readback are outside the timed section. `numpy_gpu.py` is a
CuPy capability probe; it deliberately does not relabel MLX or another Metal
library as NumPy.
`pure_python.py` is the corresponding standard-library-only scalar control; it
uses Python lists and `math` calls and deliberately does not import NumPy.
The `futhark-ispc-compat.sh` wrapper is used only when Futhark 0.27 is paired
with ISPC 1.31; it removes four unused declarations that otherwise conflict
with ISPC's standard library.
The Rust, Julia, Lua, and Taichi files are compact copies of their
measured controls; their existing throughput rows are retained in the report
until a compact-source rerun is recorded. Runtime availability remains
environment-dependent (for example, Rust SIMD needs its Cargo dependency and
Taichi needs its Python environment).

## GPU controls

The Julia Metal control was remeasured on the Apple M1 at 500,000 filters x 40
turns (five fresh processes). With the strict two-group publication boundary,
the medians are **178.135 M turns/s checked** and **199.454 M turns/s
unchecked**, with zero faults and matching checksums. This is a synchronous
per-turn GPU boundary, not a one-submit batch. The earlier 197.078 M/s checked
number used in-place valid-lane publication and post-loop fault observation;
it is retained only as historical evidence and must not be compared to Mech's
strict checked result.

Plain NumPy has no GPU backend. CuPy is the NumPy-compatible CUDA option, but
it requires an NVIDIA CUDA device; it cannot target this machine's Apple Metal
GPU. The capability result is retained in
`results/apple-m1-numpy-gpu-2026-08-31.json`, and the probe exits cleanly with
an explicit unavailable result. Run the same source family on the RTX host
after installing CUDA and CuPy rather than inventing a Metal NumPy number.

## Fixed-mode Futhark

The Futhark source exposes `main_checked` and `main_unchecked` entry points.
These fix the guarantee mode at compile time, allowing the ISPC backend to
remove the validation predicate from the unchecked kernel. At the matched
500,000-filter/40-turn workload, five samples measured **108.718 M/s checked**
and **152.330 M/s unchecked**. The dynamic-boolean entry points measured
**108.283 M/s checked** and **106.863 M/s unchecked**, which explains why the
earlier unchecked row looked artificially slow: it still paid for validation
arithmetic. Raw evidence is in
`results/apple-m1-futhark-ispc-fixed-2026-08-31.json`.

## Mech-level Futhark control

`futhark_scalar_ekf.fut` is the deliberately bounded Futhark comparison for
Mech's single-core SIMD/JIT strategy. It keeps the same EKF recurrence, f32
state, finite/positive-diagonal/symmetry checks, and per-candidate rollback,
but expands the 3x3 covariance products into scalar bindings. It is compiled
with Futhark's ISPC backend and run with one worker, giving the backend the
same SIMD-shaped boundary as Mech without using Futhark's eight-worker or GPU
maximum. On the Apple M1, 10,000 filters x 20 resident turns, five samples
after warm-up measured **37.569 M turns/s checked** and **52.597 M turns/s
unchecked**. The old array-valued source under the same one-worker ISPC
boundary measured **28.533 M/s checked** and **51.593 M/s unchecked**. The
checksums differ only by f32 reassociation (about 2.5e-5 over 2.68e6); no
faults occur for this valid workload.

The control is opt-in so it cannot silently replace the published Futhark
worker-count rows:

```text
python3 minimal/measure.py --futhark-ispc-scalarized \
  --instances 10000 --turns 20 --samples 5
```

This row is a source-level comparison, not Futhark's maximum possible result:
it intentionally uses one ISPC worker and remains synchronous at the
resident-loop boundary.

The fused reference controls use the same boundary as Mech's unchecked block:
each worker loads its assigned filters once, advances all turns locally, and
publishes one final state. Rust's packed SIMD control, Julia's eight-thread
SIMD control, and NumPy/Numba's `prange` control all expose this mode through
the fourth command-line argument (`fused` or `fused-fast`). The final state is
observable after the block; per-turn host observation requires the ordinary
synchronous mode. Raw measurements are retained in
`results/apple-m1-fused-reference-controls-2026-08-31.json`.

Rust checked mode uses the same strict contract as Mech: it snapshots the
publication boundary, rejects a block when any lane violates a predicate,
restores the complete prior state, and returns turn/instance/constraint fault
metadata. It does not selectively commit valid lanes from a partially invalid
SIMD group.

For example, the comparable controls are:

```text
JULIA_NUM_THREADS=8 julia --startup-file=no julia_simd_threads.jl 500000 40 unchecked fused
NUMBA_NUM_THREADS=8 python numpy_numba.py 500000 40 unchecked fused
rust-simd 500000 40 unchecked fused 8
```

The fourth argument selects the fused boundary; omit it for the observable
per-turn loop. NumPy/Numba also accepts `fused-fast` as an explicit unchecked
fast-math control, whose checksum is retained separately because it is not
strictly identical to the f32 result.
