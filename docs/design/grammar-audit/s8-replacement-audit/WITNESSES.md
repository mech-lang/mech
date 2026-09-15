# Executable witnesses and evidence boundaries

Run the audit from its frozen-B-based branch. `run-probes.sh` defaults to a recording
run. `MECH_AUDIT_REQUIRE_PASS=1` turns a mismatch with the selected oracle into a
failing test. Current target rejection fixtures retain a second, positive milestone
oracle: select it with `MECH_AUDIT_REQUIRE_CAPABILITY=1`. A correct current rejection
does not satisfy that positive obligation or authorize a scope exclusion.
Visibility negatives require a planning error. Target capability negatives instead
check the exact activation rejection for both source and decoded artifacts;
pure artifact compilation is not required to activate a target. Production loader
acceptance separately checks rejection before installation or host effects.

```sh
# All source/bytecode probes and method/graph/catalog/transport observations.
./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh

# G02: current target rejection is correct (passes at the frozen baseline).
MECH_AUDIT_CASE=numeric-c32-add MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses

# The same case still lacks its positive capability (fails at the frozen baseline).
MECH_AUDIT_CASE=numeric-c32-add MECH_AUDIT_REQUIRE_PASS=1 \
  MECH_AUDIT_REQUIRE_CAPABILITY=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses

# G04: confirmed repeated-selection wrong result.
MECH_AUDIT_CASE=mixed-repeated MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses

# G16: confirmed transitive explicit-root stale second turn.
MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh ordered_transitive_explicit_root_witness

# G22: exact producer/retiring decoder incompatibility; not a browser smoke test.
MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh browser_document_payload_witness

# G21/G22: actual remaining authority in a separately checked-out C candidate.
python3 docs/design/grammar-audit/s8-replacement-audit/retirement-witness.py \
  /path/to/c-candidate --require-closed

# Inventory completeness and failure-owner accounting only.
python3 docs/design/grammar-audit/s8-replacement-audit/verify-inventory.py
```

Every semantic case has its specific executable command in
`semantic-obligations.tsv`. No selector is allowed to execute zero cases.
The wrapper accepts only the six exact audit test names, rejecting a misspelled
filter before Cargo. Each test emits the identity of its compiled fixture and
harness bytes; the recorder refuses logs whose execution identity differs from
the current inputs. It also requires the complete source/route/catalog/visibility
record census before replacing stored observations.
Use the recorded `expected-json` and the underlying fixture source to inspect the
oracle. `source-bytecode-equivalence-only` is explicitly an open O03 obligation.

G23, run in C at `e31d08260fac40bcf7982e854cc5d2623a606347`:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features \
  --features full_compiler,full_source,resident-artifact --lib --no-run
```

G24, run on the frozen B/audit baseline:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features \
  --features full_compiler,full_source,resident-artifact \
  --test canonical_phase_2i_semantic_certification
cargo +nightly-2026-03-03 test --locked -p mech-syntax --no-default-features \
  --features full --test canonical_phase_2i_certification \
  certification_evidence_uses_only_canonical_authorities
```

The six C consumer integration suites use `mech-runtime` with
`full_compiler,full_source,resident-routing-source,compute` and these test targets:
`canonical_config_profile`, `canonical_declaration_handoff`,
`canonical_document_outputs`, `canonical_document_render`, `canonical_source_index`,
`source_document`. Their 134 passing tests are adapter/retained-document evidence;
they do not exercise the real WASM document loader or all CLI/served integrations.

The selected existing engine suites use `full_compiler,full_source,resident-artifact`:
`canonical_boolean_match`, `canonical_document_state`, `canonical_source_collections`,
`canonical_source_completion`, `canonical_source_review`, `canonical_source_semantics`,
`canonical_source_structures` (179 passing tests). Separately use only `source` for
`canonical_source_profile` (one passing test). Run `canonical_s7_document_certification`
with `mech-syntax/full` for the 112 direct document syntax witnesses plus the 131-row
inventory check (four passing tests).

The actual product/backend qualification commands remain those in the frozen
consumer readiness rows and maintained CI workflow. A command or fixture name in
that inventory is a required obligation until an exact-head result is recorded;
this audit does not mark unexecuted commands passing.

## Exact value and binding boundary witnesses

The schema suite is strict and enters through `CapturedValueInput` and
`prepare_turn_values`, where schema admission is defined. Raw
`CapturedSignalInput` is already typed/physical and is not a public arbitrary
snapshot validator. Empty Dynamic storage is representable; its presence is not
an error oracle. See SCHEMA-BOUNDARY-RECONCILIATION.md for the discarded harness
mistakes and the independent expected values used by the corrected tests.

```sh
# 31 strict tests:26 positive scalar/structural binding cases plus 5 validation negatives.
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-runtime --no-default-features \
  --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_schema_audit -- --nocapture --test-threads=1

# G25: minimal foreign-schema constant binding bytecode failure.
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-runtime --no-default-features \
  --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_schema_audit exact_scalar_u8 -- --exact --nocapture

# G26: already-Dynamic identity changes during constant binding.
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-runtime --no-default-features \
  --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_schema_audit exact_generic_dynamic -- --exact --nocapture

# G03: same configured catalog, explicit negative/positive visibility pairs.
MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh source_visibility_witnesses
```

The final schema record credits 26 live source and 26 live bytecode phases,
25 constant-bound source phases and two constant-bound bytecode phases. All five
separate validation negatives pass. G25 has 23 manifestations; G26 has one.
These are two shared binding defects, not 24 new type families to implement.

`record-observations.py COMPLETE_LOG [TARGETED_RERUN_LOG ...]` merges serial
source audit records by exact record/case/route identity, validates the full
fixture census and records fixture/harness/log hashes. A targeted rerun cannot erase
another case. Original checkpoint evidence remains separate.
