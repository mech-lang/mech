# E7 — Retained interactive lifecycle extraction

E7 copies the frozen retained-session lifecycle and its projection-refresh
dependency closure from S8B `662d29b79df8ab05a25bbadb941a689fd5bd5aae`
onto E6 `ef30e7e81`. Only four production/test files change. This document,
`e7-symbols.json` and `verify-e7.py` remain on the audit branch.

| File | Copied closure |
| --- | --- |
| `src/runtime/src/interactive.rs` | Complete frozen file: retained-document activation, finalized-stream submission, accepted/candidate document lifecycle, replacement/reset/clear, canonical mutation ownership, and all existing lifecycle tests. |
| `src/engine/src/resident/general/execution.rs` | Complete frozen file; the remaining delta is exactly the three `refresh_output_projections` hunks: artifact argument, revision/published-state candidate checks, and candidate copying during refresh. |
| `src/runtime/src/runtime/program/query.rs` | Complete frozen file; the remaining delta passes `candidate_artifact` to the new refresh signature. E5's output-identity helper remains unchanged. |
| `src/runtime/src/runtime/program/tests.rs` | The complete existing `canonical_interactive_uses_configured_resource_planning` regression, previously deferred because it requires the retained-session API. |

The engine derives already-published candidates from the accepted artifact's
state-writer identity operation so projection refresh does not repeat a migrated
state transition. Its signature, implementation and caller move together. Source
inspection finds one refresh method definition and one production caller; both
now match frozen B. The lifecycle depends on E5's interactive-document compiler
and the existing canonical document/stream APIs, not deferred mixed compilation.

The copied lifecycle tests cover definition/assignment clearing, same-line and
tuple ownership, inactive mutations, invariant names, pending selections,
finalized-only streaming admission, retained state across turns/replacement,
matrix projection and failed candidates, migrated-epoch projections, and result
identity around presentation fences. The program test adds configured resource
authority and verifies that a denied candidate preserves accepted state.

All copied spans and removals have exact byte positions, lengths and SHA-256
hashes in `e7-symbols.json`. The checker reconstructs the four resulting files
from E6 plus those frozen spans and rejects any other tracked delta:

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e7.py /private/tmp/mech-syntax-s8e7-interactive
git diff --check
rustup run nightly-2026-03-03 rustfmt --check --edition 2024 --config skip_children=true \
  src/runtime/src/interactive.rs \
  src/engine/src/resident/general/execution.rs \
  src/runtime/src/runtime/program/{query,tests}.rs
```

These checks passed. No Cargo was run by the extraction agent; parent owns
serialized validation. Suggested intermediate-head validation:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib interactive::tests::
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib canonical_interactive_uses_configured_resource_planning
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute --lib
```

These are proposed checks, not claimed test results. Any frozen implementation
failure remains an audit obligation; this extraction introduces no repair.
