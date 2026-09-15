# S8 replacement audit — inspection checkpoint

Implementation is frozen at S8B `662d29b79df8ab05a25bbadb941a689fd5bd5aae`.
This branch contains audit tests, fixtures, and observations only. It is an
in-progress inspection handoff, not the completed replacement-gap audit or a
request to resume implementation. The uncommitted numeric implementation in the
original S8B worktree is excluded.

## Reproduce

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 \
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_gap_audit -- --nocapture
```

To make a particular semantic observation an executable failing witness:

```sh
MECH_AUDIT_CASE=mixed-repeated MECH_AUDIT_REQUIRE_PASS=1 \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 \
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_gap_audit semantic_replacement_witnesses -- --nocapture
```

## Evidence and interpretation

- 364 semantic fixture observations; 294 pass and 70 report a failure stage.
- Four observational test functions complete successfully. The default harness
  intentionally records failures without failing the aggregate test. This is
  **not** a green implementation result.
- The harness also probes 18 compiler entry routes, an ordered transitive explicit
  root graph, and the configured source export census (120 exports).
- The ordered graph witness observes a stale second-turn value (`1`, expected `2`).
- Semantic probes exercise source and decoded bytecode over two turns. Fixtures
  with explicit expected outputs check those values; other fixtures only check
  successful execution and equality between those two paths.
- Raw semantic failures are **not** deduplicated gap counts. Fixture mistakes and
  intentionally unsupported operation/type combinations still require
  classification. In particular, matrix sum expected orientation, unsigned
  subtraction domains, Bessel argument types, and underscore-containing export
  spellings must be reconciled before interpreting those observations as defects.

## Remaining audit deliverables

Finish the closed inventories for the 80 Phase 2I semantic rules, 131 document
rules, 17 scalar kinds, structural schema families, configured source exports,
all ProgramCompiler entry points, and all 27 frozen production consumer contracts.
Produce the deduplicated gap register with owners, executable witnesses and
acceptance conditions, followed by concrete stacked PR scopes. No production
implementation resumes during that work.

The deleted-parser qualification candidate is separately identified by
`e31d08260fac40bcf7982e854cc5d2623a606347`. Its runtime suite reports 713 passing
and two failing tests; its engine library test build still fails on retired parser
references. Its isolated browser project build passes, but that does not establish
that the browser document transport has completed cutover.
