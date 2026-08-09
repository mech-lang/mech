# Runtime Gate B resident-EKF efficacy contract

Gate B is an efficacy gate for the private resident candidate-epoch design. It
does not route production turns through a resident executor. The frozen base is
`integration/value-executor-v0.4` at
`437f6c6c636d9818729597342165dfc9af5eb4a7`.

## EKF v1 workload

Every lane executes the same 4,096-turn mobile-robot EKF episode with `f64`
values and column-major matrices. The primary workload contains one EKF; the
scaled controls contain 8 and 64 independent EKFs.

```text
dt       = 0.05
landmark = [25.0, -10.0]
x0       = [2.0, 1.0, 0.15]
P0       = [1.0, 0.0, 0.0
            0.0, 1.0, 0.0
            0.0, 0.0, 0.05]
Q        = [0.04,   0.0
             0.0, 0.0025]
R        = [0.25,    0.0
             0.0, 0.0009]
```

For state `x = [px, py, theta]`, input `[v, omega, z_range,
z_bearing]`, `c = cos(theta)`, and `s = sin(theta)`, prediction is:

```text
G  = [1  0  -v*s*dt; 0  1  v*c*dt; 0  0  1]
V  = [c*dt  0; s*dt  0; 0  dt]
x- = [px + v*c*dt, py + v*s*dt, theta + omega*dt]
P- = G*P*G' + V*Q*V'
```

Measurement correction is:

```text
dx = landmark.x - x-.x
dy = landmark.y - x-.y
q  = dx^2 + dy^2
r  = sqrt(q)
h  = [r, atan2(dy, dx) - x-.theta]
H  = [-dx/r  -dy/r   0; dy/q  -dx/q  -1]
S  = H*P-*H' + R
K  = P-*H'*inverse_2x2(S)
innovation = [z_range - h.range, z_bearing - h.bearing]
x' = x- + K*innovation
A  = I - K*H
P' = A*P-*A' + K*R*K'
P' = 0.5*(P' + P'')
```

`inverse_2x2` is the frozen closed-form solve using
`det(S) = S00*S11 - S01*S10`. No lane may substitute a dynamic LU solve or
omit the Joseph covariance update.

Each candidate is rejected before publication unless all state and covariance
values are finite, `q > 1e-12`, `abs(det(S)) > 1e-12`, every covariance
diagonal is positive, and covariance symmetry error is at most `1e-10`.
Numerical agreement is authoritative when
`abs(actual - expected) <= 1e-10 + 1e-10*abs(expected)` for every state and
covariance element on every turn. The committed quantized SHA-256 trajectory
hash rounds every value to a signed `1e-10` integer and encodes it little
endian; it is diagnostic, not a replacement for tolerance checks.

The committed little-endian trace is generated once by
`scripts/generate-gate-b-ekf-trace.py`. Its manifest records the trace SHA-256,
the first and last eight rows, the final reference state, the authoritative
tolerance, and the diagnostic trajectory hash. The trace guarantees that the
bearing innovation never crosses the `+/-pi` discontinuity. Freshness checks
verify the exact frozen fixture bytes; they do not regenerate
platform-dependent transcendental results.

## B0 control lanes

- `rust-kernel`: direct preallocated Rust EKF mathematics and state update.
- `rust-epoch`: inactive candidate output, the same integrity checks, fixed
  receipt preparation, one release-store publication, and Gate A retained
  append. Admission permits are reserved outside timing.
- `numpy-persistent`: one persistent Python process, one BLAS thread (including
  BLIS, OpenBLAS, MKL, or Accelerate),
  Fortran-contiguous `float64` state, trace loaded once, preallocated scratch,
  and internal `perf_counter_ns` timing.
- `julia-persistent`: an informational Julia 1.12 control using the same frozen
  trace and equations, one Julia thread, preallocated column-major scratch, and
  garbage collection left enabled. It is not used by the frozen Gate B
  pass/fail calculation.
- `mech-legacy-atomic`: source activated once, one four-update host-input
  packet per turn, the same EKF kernel hosted as a transaction-journaled Mech
  function output, and the ordinary atomic reactive-turn path.

Timed regions begin immediately before input lookup or host-input application
and end after direct state update, retained append, NumPy state update, or the
ordinary atomic turn respectively. Fixture construction, state reset,
admission reservation, process startup, trace loading, output serialization,
and correctness checking are outside timing.

Every instance is replayed against the per-turn oracle outside timing. Rust
lanes hash their observed replay trajectory rather than copying the reference
hash. Legacy structural counters are collected in a separate probe-enabled,
untimed process; the legacy Criterion timing binary does not enable Gate A
probes.

All controls use the same equations, operation order, input trace, initial
state, episode length, integrity conditions, and tolerance. Correctness work
may not occur inside only one timed lane, and input conversion may not move
across only one timing boundary.

## Full-write control

The full-write control owns two activation-sized 64-by-64 `f64` buffers and
computes every element each turn:

```text
next[i] = current[i] * 1.000001 + coefficient[i] * input_scalar
```

The raw epoch control must report zero candidate seed bytes, zero published
buffer-copy bytes, one release-store publication, zero steady-state
allocations, and an unchanged published-buffer hash after a forced abort.

## B0 stop condition

B0 records median and p95 nanoseconds per turn, allocations and bytes per turn,
correctness, and the quantized hash for every control. Before resident work may
begin, the primary-size control must have a positive denominator:

```text
T_mech-legacy-atomic - T_rust-epoch > 0
```

A non-positive denominator is a hard B0 stop. It is not repaired by beginning
resident implementation or changing the Gate A feature boundary.

## Julia comparison

Run the standalone Julia control with:

```sh
JULIA_NUM_THREADS=1 julia --startup-file=no \
  benchmarks/runtime/gate-b/julia/ekf_v1.jl --samples 9
```

The timed region contains only retained EKF episodes. Workspace construction,
trace loading, reset, compilation warmup, and validation are outside timing.
The runner reports bytes allocated and GC time for the median sample. It checks
the final state and covariance against the manifest tolerance and freezes a
diagnostic Julia trajectory hash. As with NumPy, the hash can differ from the
scalar oracle because sub-tolerance floating-point ordering can cross a
`1e-10` quantization boundary.

Apple M1 results from 2026-08-08 using Julia 1.12.6:

| Instances | Episode median | Per turn | Allocated | GC time |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 1.016 ms | 248 ns | 0 bytes | 0 ms |
| 8 | 8.139 ms | 1.987 us | 0 bytes | 0 ms |
| 64 | 65.086 ms | 15.890 us | 0 bytes | 0 ms |

The committed B0 NumPy control measured 20.868 us per turn at one instance on
the same hardware. Julia is about 84x faster here because its compiled small
matrix loops avoid sending a long sequence of 2-by-2 and 3-by-3 operations
through NumPy's Python/API dispatch boundary. Raw data is in
`julia/RESULTS_APPLE_M1_2026-08-08.csv`.

The resident turn-shell follow-up, including a Time Profiler flame graph and
layer subtraction, is recorded in
[`profiles/PROFILE_APPLE_M1_2026-08-09.md`](profiles/PROFILE_APPLE_M1_2026-08-09.md).
