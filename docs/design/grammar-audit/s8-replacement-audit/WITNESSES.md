# Executable witnesses and evidence boundaries

Run the audit from its frozen-B-based branch. `run-probes.sh` defaults to a recording
run. `MECH_AUDIT_REQUIRE_PASS=1` turns observed failures into failing tests. For
contract-decision rows, that switch is a reproducer of missing successful behavior,
not authorization to implement a currently unsupported operation. After the target
contract decision, negative acceptance must assert the precise planning error and
that no host effect occurred; an activation failure is not sufficient.

```sh
# All source/bytecode probes and method/graph/catalog/transport observations.
./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh

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
