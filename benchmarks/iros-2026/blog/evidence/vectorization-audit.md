# Maintained EKF and scene source: array-expression audit

This audit tests source simplifications without changing the live sources,
archived benchmark fixtures, or recorded measurements. It is not a performance
measurement. Candidates below were compiled and executed with the current built
WebAssembly runtime, including the elementwise `abs` lowering added for the
vectorized symmetry tolerance.

## Live bearing-only EKF

Seven candidates were tested independently against
`blog/source/ekf.mec`: 256 filters, 40 turns with varying velocity and per-lane
bearings, prediction-only intervals, and a landmark switch. Every candidate
compiled, generated a WGSL manifest, and produced zero differences in the f32
state bits of `μ` and `Σ`. A subsequent NaN in lane 129 produced the same reported
fault and preserved the candidate's entire accepted state. This audit generated
WGSL but did not execute these candidate variants on a GPU.

The two absolute-bound candidates and the absolute-symmetry candidate also passed
27 nonfinite-input comparisons: NaN, +Infinity, and -Infinity injected separately
into bearing, velocity, and landmark inputs. Reported faults, whole-state rollback,
and recovery matched the original. A separate 2×3 bound-reduction test matched
acceptance and state for ±0, the smallest ±f32 subnormals, ±f32::MAX, NaN, and both
infinities. Nonfinite values were rejected; all six finite boundary values passed.

### 1. Replace basis-vector arithmetic with selection

The strongest cleanup is removing arithmetic that only selects components:

| Current | Candidate |
| --- | --- |
| `θ := μ · et` | `θ := μ[3]` |
| `δ := m - J ** μ̄` | `δ := m - μ̄[1..=2]` |
| `δx := δ · [1f32 0f32]'` | `δx := δ[1]` |
| `δy := δ · [0f32 1f32]'` | `δy := δ[2]` |
| `ẑ := atan2(δy, δx) - (μ̄ · et)` | `ẑ := atan2(δy, δx) - μ̄[3]` |
| `xyz := [μ₊ · ex, μ₊ · ey, μ₊ · et]'` | Delete this reconstruction; check `μ₊` directly. |

These changes remove `J`, `ex`, `ey`, and `et`. Current lowering supports the
index and range selections. The original matrix product `q := δ · δ` computes a
genuine inner product and should stay.

### 2. Apply finite bounds to whole arrays

The existing two-sided tests can each become one reduction:

```mech
finμ := all(abs(μ₊) <= fmax)
finraw := all(abs(Σraw) <= fmax)
finΣ := all(abs(Σ₊) <= fmax)
finite-candidate! := finμ && finraw && finΣ
```

The tested candidate applied the first test to the existing `xyz`; combining it
with the independently tested removal of `xyz` is straightforward but should
receive the final integrated regression run. `abs` accepts scalar, vector, and
matrix inputs after the bounded lowering fix. NaNs and infinities fail these
comparisons, as they do in the two-sided form.

The following single matrix reduction also compiled and passed the same checks:

```mech
finite-candidate! := all(abs([μ₊ Σraw Σ₊]) <= fmax)
```

It concatenates a 3×1 mean with two 3×3 covariances into a 3×7 matrix. This is
shorter, but the three named checks above make their purposes easier to identify
and avoid introducing a container solely for validation.

### 3. Reduce the symmetry residual directly

```mech
-- Current
symmetric-covariance! := all(ΔΣ <= τ) && all(ΔΣ >= -τ)
-- Candidate
symmetric-covariance! := all(abs(ΔΣ) <= τ)
```

This preserves the same elementwise tolerance rule without duplicating its
positive and negative comparisons. Array `abs`, comparison, and `all` lowering
are supported.

### 4. Broadcast common arithmetic within small vector expressions

Both of these individually tested candidates preserve the operation order of
their components:

```mech
-- Current
μ̄ := μ + [d * cosθ, d * sinθ, w * Δt]'
H := [δy / q, (0f32 - δx) / q, -1f32]
-- Candidates
μ̄ := μ + [d * [cosθ sinθ], w * Δt]'
H := [[δy (0f32 - δx)] / q, -1f32]
```

Nested concatenation and scalar broadcasting are already supported. The source
is more compact; this audit does not establish a speed improvement. Explicit
Jacobians `G` and `V` still express distinct matrix entries and do not need to be
hidden behind additional constructions simply to eliminate scalar literals.

### 5. Combine paired diagonal checks if desired

```mech
positive-covariance! := all([Σraw[[1 5 9]] Σ₊[[1 5 9]]] > 0f32)
```

This tested candidate combines the raw and published diagonals into a 3×2 matrix.
It preserves checking both. Do not drop the published-diagonal check merely
because the formulas usually give identical diagonal values: halving very small
subnormal values can round to zero.

## Maintained range-and-bearing example

`examples/ekf/localization.mec`, section 5.3.2, still expands three constraints
component by component. With `logic/all` imported, they can be written as:

```mech
finite-state! := all(abs(μ-next) <= finite-limit)
positive-covariance! := all(Σ-next[[1 5 9]] > 0f32)
symmetric-covariance! := all(abs(Σ-next[[4 7 8]] - Σ-next[[2 3 6]]) <= 0.0001<f32>)
```

The extracted compute region compiled and generated WGSL with these three changes
together. Its complete `filter-sample` output matched the original over 256 lanes
and 40 changing-control turns, including zero-visibility intervals. Rejection
parity was not separately exercised for this example during this audit.

## Live scene

The corresponding distance formulas can use inner products:

```mech
camera-distance := sqrt(camera-offset · camera-offset)
display-camera-distance := sqrt(display-camera-offset · display-camera-offset)
```

The coordinated scene check found exact equality of six numeric exports and the
entire scene snapshot over 40 variable-control turns with 256 lanes. The 35
manually enumerated grid rows are a separate table-generation opportunity;
appropriate table-construction syntax and compiler support still need inspection.

## Numerical boundaries and follow-up

Keep the stabilized average as two half-scaled matrices:

```mech
Σ₊ := Σraw * 0.5<f32> + (Σraw') * 0.5<f32>
```

Replacing it with `(Σraw + Σraw') * 0.5<f32>` changes rounding and can overflow
before scaling. Similarly, factoring `ρ` outside the tolerance's sum changes
floating-point operation order. Neither change is justified by shorter source.

Exact agreement on these test packets is not a proof of bitwise equivalence for
all inputs. Replacing basis-vector products with selection can affect signed
zero and the propagation of nonfinite intermediate components. Before adopting
the combined cleanup, repeat integrated CPU/GPU acceptance, rollback, and
recovery tests, including nonfinite inputs. The archived source and its timing
and source-size claims should remain separate from these maintained examples.

## Audit inputs

SHA-256 values at the time of the exploratory checks:

- Live EKF: `69480e5b46a4da7b9391755dc2a40a5e45a3e39e0899351910c83e5352893689`
- Maintained localization example: `a054bafc623c793a2ffd8fe257952c754db621f155db7d56f2d0a0a39bafd5e0`
- Built WASM: `ba2fe6330931fdeb32f0417dfd903185cee392d1bb4bbae026876415168ba880`
