# Findings from corrective qualification

This supplement preserves the accepted `d25fbaad0` baseline and its original
25 groups. It records new demonstrated causes before implementation, rather
than silently widening an existing correction. Current declared scope is the
baseline plus the following one finding: 26 groups and 24 review boundaries.
These are ownership counts, not an effort estimate or a completeness guarantee.

## G27 — Index range resident prerequisite (proposed R24)

**Owner:** resident range cardinality, physical binding and execution. R03 owns
schema relocation; it does not own implementing new range kernels. Q31's Id-like
Index identity/binding and zero-index validation are qualified by R03, while
Q31's positive range endpoint subcase remains blocked by this prerequisite.

The canonical frontend and artifact codec admit `lo<ix>..hi<ix>` with detached
Index constants 1 and 3. Decoding succeeds. Activation returns
`InvalidDependency { node: NodeId(0) }`: `canonical_range_cardinality` calls
`snapshot_range_number`, whose match admits integer and floating-point ValueData
but omits Index. This is independent of G25 schema ordering and G26 wrapping.
The binder has already removed both live inputs; live-range shape rejection
cannot explain this constant-endpoint failure.

The strict positive test is
`src/runtime/tests/s8_recovery_index_range.rs::bound_index_range_has_exact_source_and_bytecode_values`
on the audit branch. It asserts exclusive output `[Index(1), Index(2)]` and
inclusive output `[Index(1), Index(2), Index(3)]`, with exact 1×N Index schema and
two turns through source and decoded artifacts. It intentionally fails until the
prerequisite is implemented; there is no success-on-rejection mode or ignored
positive test. This also corrects the initial exploratory fixture's exclusive
endpoint/column-orientation expectation before recording the final witness.

Run on the audit branch (or copy the one test file into a corrective checkout):

```sh
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features \
  --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_recovery_index_range \
  bound_index_range_has_exact_source_and_bytecode_values -- --exact
```

R24's finite acceptance must cover both range modes, explicit increments,
constant endpoints, portable Index bounds/overflow, and the existing rejection
of changing live endpoints. Index is not an arithmetic alias for u64. The
existing Index catalog/type contract must govern the physical implementation.
No production Index range implementation is included in R03 or this supplement.
