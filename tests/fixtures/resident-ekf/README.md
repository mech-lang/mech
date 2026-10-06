# Resident EKF fixture

This directory owns the deterministic input trace and numerical oracle used by
resident EKF tests and benchmarks.

Verify the committed bytes with:

```text
python3 scripts/generate-resident-ekf-fixture.py --check
```

## EKF v1 workload

The fixture contains a 4,096-turn mobile-robot EKF episode with `f64`
values and column-major matrices. The trace can be shared by benchmarks containing multiple independent EKFs.

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

`inverse_2x2` is the closed-form solve using
`det(S) = S00*S11 - S01*S10`. Equivalent implementations must not substitute a dynamic LU solve or
omit the Joseph covariance update.

Each candidate is rejected before publication unless all state and covariance
values are finite, `q > 1e-12`, `abs(det(S)) > 1e-12`, every covariance
diagonal is positive, and covariance symmetry error is at most `1e-10`.
Numerical agreement is authoritative when
`abs(actual - expected) <= 1e-10 + 1e-10*abs(expected)` for every state and
covariance element on every turn. The committed quantized SHA-256 trajectory
hash rounds every value to a signed `1e-10` integer and encodes it little
endian; it is diagnostic, not a replacement for tolerance checks.
