# Uiua fannkuch additive native-array revision

These candidates retain the original complete factoradic rank enumeration,
1024-permutation blocks, canonical signed checksum and maximum. Every permutation
is materialized and each required prefix reversal is performed in Uiua. There is
no new runtime, host helper, benchmark executor, permutation cache or input-specific
shortcut. Original sources and frozen manifests are not modified.

The original optimized source tables `FlipIndex`, a user function containing a
switch, at every permutation-by-position cell. This expression does not match
Uiua's single-primitive native table dispatch. The replacements make two native
primitive tables and use ordinary broadcast arithmetic:

`index = column + (column <= first) * (first - 2 * column)`.

For a lane with first value zero, this is the identity permutation; its count
remains unchanged. All other lanes reverse exactly their first `first + 1` values.

- `sources/compact.ua`, 20 LOC: primitive comparison/subtraction tables followed
  by the original row gathers.
- `sources/fast.ua`, 21 LOC: also adds `lane * N` to each index, flattens the index
  and permutation arrays, performs one native select, and restores the shape.
  This removes the per-row gather-function dispatch as well as per-cell dispatch.

The existing original 20-LOC scalar compact program has no `FlipIndex` table: its
`Flip` already uses ordinary prefix take/reverse. Its source and evidence remain
available. The new compact form is a bounded array alternative, not evidence of
being faster or shorter before full measurements.

Run `python3 prepare.py`, then `python3 validate.py`, then
`python3 prepare_manifest.py`. The validator uses the shared benchmark mutex and
only complete sizes 1 through 8. It independently enumerates permutations in
canonical order, validates both outputs and every recorded prefix-reversal state,
including finished lanes and 1024-block tails. Generic index/gather probes at
width 12 contain at most 1024 arbitrary rows; they do **not** enumerate 12! ranks.
The validator's instrumented sources are never timed.

All independent bindings and declarations occupy separate lines. No compressed
size or packed-binding LOC reduction is used. Full canonical n=12 measurements
and fresh unchanged Mech controls are required before chart admission.
