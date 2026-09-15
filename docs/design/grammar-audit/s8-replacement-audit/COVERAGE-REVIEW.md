# Independent coverage review and concrete corrections

Reviewed frozen implementation `662d29b79`, candidate C `e31d08260`, and the audit
at `06a215882`. This review made no production changes and ran no Cargo commands.
The compiler/schema tables now contain responsibility-specific cells; they retain
explicit unexecuted cells and do not certify those cells by inventory membership.

## Corrections made in this review

- `compiler-methods.tsv` links all 36 public methods to 53 acceptance cells in
  `compiler-acceptance-cells.tsv`. Every cell gives a concrete source/value graph,
  expected result, owner, evidence status, and an exact test command where a test
  exists. Aliases share cells only after their delegation was inspected. Examples:
  P05 supplies real values and selected live ports; P13 checks all eight rooted
  canonical variants with a changed resolver revision; P18 records the transitive
  stale value; P20–P22 specify the missing transitive-provider, defining-function
  resource-context, and later-root rejection combinations.
- `frontend-apis.tsv` now distinguishes the 18 `CanonicalSourceFrontend` methods
  from six `CanonicalSourceProgram` input/artifact methods. F12 and F15–F19 specify
  previously hidden ordinal, binding remap, input-reference census, resource remap,
  external-contract error, and schema-override cells.
- `compiler-internal-entrypoints.tsv` accounts for three production entrances
  omitted by the public-method regex: `ProgramCompilerView::compile_interactive_source`,
  `compile_resolved_root`, and `compile_interactive_resolved_root`. The first is
  retired in C; its loader responsibility now enters a retained document. The
  resolved methods retain the existing runtime's compiler context.
- `schema-families.tsv` links all 20 variants to 64 cells in
  `schema-acceptance-cells.tsv`. It separates source construction, canonical `Value`
  input/binding, artifact codec, exact publication, and the narrower host adapter.
  QT-* specifies 17 exact scalar type/ValueData assertions; QG-* specifies nine
  structural snapshot rebinding cases. Their first run exposed harness mistakes.
  The corrected run passes all 26 live source/decoded publication cells; 23
  constant-bound codec failures deduplicate to G25 and one Dynamic identity
  failure to G26. Bool/String complete all routes and all five QN-* negatives pass.
  Id, Index, Enum, and ReifiedType have distinct Q30–Q33 cells.
- `src/runtime/tests/s8_replacement_schema_audit.rs` supplies 26 strict positive
  tests for QT-* and QG-*, plus five separate QN-* rejection tests. Each positive
  uses independent canonical snapshots, schema-checked Value admission, and
  source/decoded live and constant-bound publication over two turns. The five
  negatives test schema, nominal identity, extent, and key-schema rejection at
  Value::rebind, prepare_turn_values, and bind_input_constants. Raw physical
  CapturedSignalInput does not provide that semantic validation contract.
  `SCHEMA-BOUNDARY-RECONCILIATION.md` records why the original 26 failures are
  not 26 defects, and records the corrected run's two demonstrated root causes.
  Cargo was run by the coordinating agent, not this reviewer.
- `verify-acceptance-cells.py` checks links, schema inventory, and the existence of
  each cited test at its recorded implementation head. It passes. It does not run
  Rust or turn an existing test into an executed result. It also checks the
  executed schema-test source hash and the 31 case/phase/owner records.

## Qualification limits retained after reconciliation

| Priority | Qualification limit | Evidence and required boundary |
| --- | --- | --- |
| P1 | Enumerate catalog kind/layout cells, not only signature rows | The revised catalog-acceptance-cells.tsv specifies 480 candidate rows across 120 names and 34 shared families, with concrete kind/layout domains, recipes and oracles. All candidate-specific executions remain unrecorded. Export-level observations cannot certify which competing overload resolved; retain actual candidate-identification assertions at execution. |
| P1 | Resolve the host input/return asymmetry explicitly | `src/runtime/src/input.rs:18` has 20 `RuntimeHostInputValue` input variants. Its `from_numeric_value` at line190 accepts only F32/F64 and 2-D F32/F64 matrices. `execute_named_canonical_outputs` uses that conversion for defaults/static returns. P29 now records this asymmetry; neither all 20 input forms nor all `SchemaBody` variants imply a generic returned-default contract. Any broadening needs an explicit product-contract owner. |
| P1 | Preserve the existing fixed-population mask contract when deciding G17 | `canonical_source_structures::composed_live_mask_shapes_require_facts_and_reject_extent_changes_atomically` supplies population1, permits changing mask positions, and rejects population0/2 atomically. Q09 captures it. The no-facts failing audit case does not by itself establish a variable-population read requirement. The no-facts observation alone does not prove a compiler omission. General computed-mask admission still has an independent positive [2,3] witness and no accepted S8 exclusion; TARGET-REJECTION-RECONCILIATION.md separates that unavailable capability from correct current missing-fact rejection. |
| P1 | Keep composite-control ownership precise | `canonical_source_completion::structured_patterns_read_live_components_and_reject_partial_matches` already executes tuple/array destructuring into primitive variables. `match_publishes_the_selected_compound_result_across_turns` already publishes tuple arm results. G06 concerns retained composite binding/yield storage; G09 concerns structural match scrutinees. These existing capabilities must not be reimplemented or presented as absent. |
| P2 | Do not credit predicate/codec observations with exact type publication | Most `scalar-*` audit cases publish Bool comparisons. Q01's `bytecode_v1_round_trips_every_c2_snapshot_family` constructs 19 schema families as constants but has no inputs, nodes or outputs, and Dynamic is absent. QT-* distinguishes exact live type identity; Q02/Q03 carry the stronger existing Dynamic publication evidence. |
| P2 | Do not infer options identity coverage from supplied options | `canonical_resolved_and_rooted_interactive_compilation_preserve_revision_and_symbols` supplies options but asserts revision content and symbol projection, not module-identity changes. P14 defines a finite one-field-at-a-time module-identity check. Do not require inert options to alter executable artifact revision without the module builder contract saying so. |
| P2 | Correct the native sidecar claim | `canonical_native_sidecars_cover_every_encoded_instruction` actually asserts a nonempty artifact graph and an empty retired instruction list with three empty sidecar vectors. P03 now states that exact result. It is not source-span or nonzero native instruction coverage. |
| P2 | Link production contracts to actual boundary runs | The revised consumer ledger has 54 cells across 27 contracts: 24 recorded passes, five partial, nine untested and 16 blocked. Existing adapter evidence is credited only for its inspected path. G22 still blocks actual browser bootstrap/REPL loading; complete configured application and rejection witnesses remain explicit. Native helper success cannot certify browser publication. |

## Extraction review requirements

The current E1–E11 extraction separates accumulated responsibilities. All eleven
heads are recorded in extraction-results.json; E1–E8 have completed named Cargo
checks, and E9–E11 retain unrun Cargo qualification. Every one of the 91 frozen
paths has its actual slice owners and patch hashes. The final E11 tree exactly
matches frozen B without path exclusions. That proves source provenance, not
implementation completeness or final distribution qualification. Shared `frontend.rs`, `document_lowering.rs`,
`compiler.rs`, `program/tests.rs`, Cargo feature wiring, and intrinsic catalog
changes carry cross-boundary dependencies. For each extracted head, retain:

1. The exact moved hunks and the earlier prerequisite exposing each new API/type.
2. A minimal nonzero compile/test profile appropriate to the extracted responsibility.
3. An exact final tree comparison against frozen B; no production differences are
   permitted merely to make extraction easier.

The scope cells are acceptance work, not a claim that all 24 gap groups are one
root cause each or that the proposed corrective PRs have already been approved.
Substantial G14/G15/G19/G20 semantic decisions still require their canonical
language/target contract. No production fix is authorized by this review alone.
