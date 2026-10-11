# Additive Uiua nbody denominator placement revision

These compact and optimized candidates derive from the canonical-only v2
sources, including Jupiter's corrected initial z coordinate
`-0.103622044471123109`. They change only line 13:

```uiua
≡≡(¤√ⁿ3)
≡≡¤ √ⁿ3
```

The original parenthesized body runs cube, square root, and Fix for each pair
cell. In the revision, cube and square root operate on the complete numeric
matrix, then nested Rows applies only Fix. The pinned runtime recognizes this
depth-aware Fix and inserts a singleton shape dimension without copying the
numeric data. The final denominator retains the same values, order, and shape.
`Norm ← ≡≡/+˙×` already uses native array squaring and reduction; it is unchanged.

Both sources retain 21 LOC. All five bodies, canonical constants, momentum
offset, directed-pair array, diagonal treatment, binary64 arithmetic, reduction
order, `dt=0.01`, requested steps, and two printed energies are preserved.
The compact form retains its original NewVelocity evaluations; the optimized
form retains its single stored NewVelocity per step. This change adds no host
helper or benchmark-specific runtime kernel.

Preparation and validation from this directory:

```sh
python3 prepare.py
python3 validate.py
python3 prepare_manifest.py
```

The manifest generator requires the canonical v2 manifest to be
`prepared-validated` and checks the hashes of its sources. Validation holds
`/tmp/mech-clbg-timing.lock` throughout all reference and runtime work. For each
form at `n=0,1,2,7,1000`, it checks the two production energies against an
independent canonical pairwise integrator, then compares every 30-component
position/velocity state against that form's canonical v2 source. Diagnostic
sources serialize explicit little-endian f64 bytes, preserving signed zero and
all bits. Final states and every executed step must match v2 bit for bit.
Raw positions, scaled velocities, masses, and the canonical initial state at
zero steps receive independent byte checks. The independent pairwise state
comparison allows absolute error `5e-11`; printed energies allow `1e-8`.

Seven generic positive matrix fixtures with shapes `1×1`, `2×3`, `3×2`, `5×5`,
`7×4`, `1×8`, and `9×1` compare the old and new denominator expressions,
including all values and the inserted final dimension. The evidence contains
10 production output checks, 10 complete-state comparisons, and 7 denominator
checks. Instrumented sources are separate from the production sources.

This package records no 50,000,000-step performance run. Original sources,
canonical v2 sources, frozen pins, and earlier evidence remain preserved.
Each candidate stays ineligible for a chart until explicit selection through a
new frozen audit and qualifying fresh full candidate/control measurements.
