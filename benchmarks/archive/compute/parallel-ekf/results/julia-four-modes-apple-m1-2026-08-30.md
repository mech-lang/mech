# Julia EKF four-mode comparison: Apple M1, 2026-08-30

This probe uses Julia `1.12.6` on the Apple M1 with 10,000 persistent filters,
five warmup turns, and 20 measured turns in each isolated process. The generic
source uses `Matrix{Float32}` and `mul!`; the fixed-shape source uses flat
column-major `Vector{Float32}` buffers and explicit 3x3/3x2 products. Both
sources accept `unchecked` or `checked` as their third argument.

The checked mode evaluates the same publication predicates as the Mech EKF:
all state and covariance values finite, covariance diagonal positive, and
covariance symmetric within `0.0001f0`. It writes state only after validation,
so a failed candidate leaves the previous state live. No faults occurred in
this nominal run.

Command shape:

```text
julia --startup-file=no julia_scalar.jl 10000 20 unchecked|checked
julia --startup-file=no julia_flat.jl   10000 20 unchecked|checked
```

Each row below is the raw throughput from five processes, in process order.
Every checksum was `2.682056074361449e6` and every measured fault count was 0.

| Julia implementation | Validation | Samples (million lane-turns/s) | Median |
| --- | --- | --- | ---: |
| Generic Matrix/`mul!` | unchecked | 3.161, 2.964, 3.167, 3.089, 3.079 | 3.089 |
| Generic Matrix/`mul!` | checked | 3.081, 3.171, 3.036, 3.074, 3.126 | 3.081 |
| Fixed-shape flat tuples | unchecked | 21.958, 21.817, 21.928, 22.009, 22.002 | 21.928 |
| Fixed-shape flat tuples | checked | 17.427, 18.982, 19.018, 19.001, 18.999 | 18.999 |

The fixed-shape implementation is therefore about 7.1x faster than the
generic unchecked translation and about 6.2x faster when carrying the same
validation and publication behavior as the checked Mech path. The raw Rust
control remains a separate unchecked ceiling; it is not a checked-language
comparison.
