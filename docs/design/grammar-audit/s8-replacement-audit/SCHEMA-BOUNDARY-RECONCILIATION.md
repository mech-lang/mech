# Exact schema witness boundary reconciliation

The first strict run of `s8_replacement_schema_audit` reported 0 passing and 26
failing tests. That result exposed incorrect audit expectations and must not be
counted as 26 replacement defects. Production remains frozen at `662d29b79`.
The original and corrected runs are distinguished below; no production changes were made.

| Original failure | Contract evidence | Audit correction and owner |
| --- | --- | --- |
| Bool and F64 fail on their first positive `ResidentValueRef::Snapshot` capture | `CapturedSignalInput` carries a physical `ResidentValueRef`; `install_inputs` copies it into the declared physical region (`src/engine/src/resident/general/execution.rs:1010`). Bool/F64 use native lanes. `prepare_turn_values` checks declared schema identity and writes canonical values through the adapter (`execution.rs:737`, `execution.rs:1056`). | Audit harness error. Positive witnesses first rebind independently owned snapshots to the declared schema, retaining foreign payload schemas with `extend_preserving_ids`, then use `CapturedValueInput`. No production fix is proposed. |
| Most other families accept an incompatible raw Snapshot capture | The raw capture route checks counts, slots, and physical layout. The canonical Value route additionally checks schema ID, schema key, and shape before adaptation (`execution.rs:1080`). The production external coordinator constructs `CapturedValueInput` and calls `prepare_turn_values` (`src/runtime/src/runtime/program/external/coordinator.rs:625`). | Audit harness assigned a semantic guarantee to the wrong layer. Five separate QN-* tests exercise actual schema-aware admission and constant-binding rejection, including preservation of output and epoch. This run does not establish a production input-validation defect. |
| Dynamic expects finalization of `Dynamic(None)` to reject | `DynamicValue` explicitly documents an empty composite-reconstruction placeholder (`src/core/src/snapshot/data.rs:224`); finalization intentionally retains None (`src/core/src/snapshot/validation.rs:3160`). | Audit expectation error. Remove the invented finalization rejection. QG-Dynamic supplies materialized payloads and verifies their exact nested identities; placeholder escape is not alleged from draft acceptance alone. |

The raw capture API is a lower-level engine interface. Its checks are not an
alternative authority for canonical Value schema admission. The existing product
coordinator enters the checked Value path. The source inspection therefore does
not justify adding a security or replacement gap solely because a manually
constructed raw capture can violate the higher-level schema convention.

The corrected positive suite contains 17 scalar and nine structural tests. Each
has an independent expected schema and value, exercises live inputs twice for
direct and decoded artifacts, and separately binds the first value into a
zero-input canonical program for direct and decoded publication. Phase markers
report the completed live and bound routes. Negative expectations occur in five
separate tests, so they cannot prevent positive witnesses from running.

The five negative cells are QN-scalar-schema, QN-native-scalar-schema,
QN-atom-identity, QN-matrix-extent, and QN-map-key. Each checks `Value::rebind`
(the documented equivalent-schema boundary), `prepare_turn_values` (checked
resident admission), and `CanonicalSourceProgram::bind_input_constants`
(canonical binding). These are representative responsibility cells, not an
assertion that every combination of every composite has been exhaustively run.

A separate implementation hypothesis identified during harness correction was: `bind_input_constants` unconditionally constructs a Dynamic wrapper
for a Dynamic input declaration (`src/engine/src/source_semantics/frontend.rs:359`).
QG-Dynamic supplies an already Dynamic value and requires exactly one wrapper.
The corrected execution below confirms that this belongs to canonical constant
binding, not to all scalar or structural families.

## Corrected run: two deduplicated implementation defects

The corrected `s8-audit-schema-qualified-boundaries.log` run executed 31 tests:
seven passed and 24 failed. All 26 positive families passed both source and
decoded live publication on two turns. Bool and String also passed both
constant-bound routes; all five independent rejection tests passed.

The 24 remaining failures have two distinct demonstrated causes:

| Gap | Owning responsibility | Minimal executable witness | Demonstrated cause and finite acceptance |
| --- | --- | --- | --- |
| G25 | Engine canonical dependency-value binding and artifact schema remapping | `s8_replacement_schema_audit::exact_scalar_u8`; source `answer := signal<u8>`; bind U8(1) from a separately constructed table that also owns Id | `bind_input_constants` imports the whole detached schema table (`source_semantics/frontend.rs:338`) through append-only `extend_preserving_ids` (`core/src/schema/table.rs:458`). The artifact compiler clones that table unchanged (`engine/src/artifact/compiler.rs:571`), and the encoder emits its current ID order (`engine/src/artifact/bytecode.rs:1231`). The decoder rebuilds sorted canonical IDs and rejects any remap (`bytecode.rs:1256`). The witness fails `NonCanonicalSchemaId { expected: 0, found: 1 }`; 23 scalar/structural cells share this cause. Acceptance is successful direct and decoded constant-bound publication with exact schema/data identity despite foreign-table additions, with all schema-bearing references remapped consistently if IDs change. |
| G26 | Engine canonical dependency-value binding, Dynamic identity preservation | `s8_replacement_schema_audit::exact_generic_dynamic`; materialized Dynamic(Tuple(Bool(true),F64(1))) bound to a Dynamic input | The Dynamic target branch wraps every supplied value (`source_semantics/frontend.rs:359`), including an already Dynamic value. The inner schema lookup selects Dynamic itself and then constructs `Dynamic(Some(Dynamic(Some(payload))))`. Live source and bytecode routes pass; bound artifact encoding/decoding succeeds, but its first direct output has the wrong exact canonical value hash. Acceptance preserves the existing single Dynamic wrapper and nested schema/value identity, while still boxing a genuinely concrete input once. |

The G25 ordering mechanism is independent of any numeric operation or supported
overload. `Schema::canonical_bytes` writes body length before body bytes
(`core/src/schema/encoding.rs:59`), and `SchemaTableBuilder::finish` sorts those
bytes (`core/src/schema/table.rs:99`). Imported Id has a one-byte body. It sorts
before the three-byte integer/float bodies and larger compound bodies, but
append-only extension places it after the existing table. Bool and String also
have one-byte bodies with tags below Id, explaining why their bound codec tests
pass. This concrete ordering dependency explains the 23 repeated failures;
it does not justify 23 separate repair tickets.

G26 cannot be a manifestation of G25: its bound artifact already decodes, its
failure is an independently expected published value identity, and the source
explicitly adds the extra wrapper. Conversely, G25's minimal U8 witness has no
Dynamic data and fails before any decoded execution.

The final `s8-audit-schema-final.log` run executes `bound-source` before invoking
the decoder. It records 31 strict tests: seven pass and 24 fail. All 26 live
source and all 26 live bytecode phases pass. Twenty-five bound-source phases
pass; Dynamic fails exact identity on its first bound-source turn. Bool and
String complete bound-bytecode publication; 23 other bound-bytecode decodes fail
under G25. Each completed publication phase checks two turns. The five separate
schema-aware rejection tests all pass.

`schema-observations.json` records the exact phases per cell, strict result,
production baseline, audit head, executed audit-test SHA-256, log hashes and
reproduction command. Audit changes were uncommitted at execution, so the test
hash is necessary alongside the audit commit. Production implementation and
source fixtures remain unchanged. These results do not close compiler F15's
multi-input/reference-remapping obligation or P29's host-adapter census.
