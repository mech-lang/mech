# Final-cutover test disposition

This is the complete test census for PR11 relative to the immutable PR10 base
`fe2e71425c78cae913ef3b01f622f72bceb7438c`. The audit compared every changed
Rust file's `#[test]` count and then reviewed every removed `#[cfg(test)]`
module. A lower count is permitted only when the old case was compatibility
specific or its permanent oracle is named below.

## Mixed machine files rewritten canonically

These files contained useful canonical behavior beside retired constructors.
Their tests now construct `FunctionInvocation`, `ValueCell`, and exact matrix
backings directly; no legacy helper was restored.

| Mixed suite | Canonical oracle retained in PR11 |
| --- | --- |
| `machines/combinatorics/src/n_choose_k.rs` | Exact scalar and matrix ports, dynamic matrix extents, input/output identity, invalid-selection atomicity, and typed rollback. |
| `machines/compare/src/eq.rs` | Exact scalar/fixed/dynamic ports, canonical atom/table snapshots, identity, shape restoration, arity, and type rejection. |
| `machines/compare/src/seq.rs` | Canonical arbitrary-value equality, schema-distinct payloads, exact Boolean output identity, and checkpointing. |
| `machines/logic/src/lib.rs` | Unary/binary Boolean ports, fixed/dynamic storage, identity, rollback, wrong type, and wrong layout. |
| `machines/math/src/op_assign/port_tests.rs` | All scalar and indexed operation families, whole/indexed matrix forms, aliasing, staged commits, Boolean/usize selectors, row/all selection, overflow, atomicity, dynamic shape restoration, and exact backing identity. |
| `machines/math/src/ops/add.rs` | Checked arithmetic and fixed/dynamic typed-state restoration remain local; source/catalog/native routing is covered by the math catalog and native-linkage gates. |
| `machines/math/src/ops/{negate,pow}.rs` and `machines/math/src/trig/atan2.rs` | Exact canonical ports, checked failure atomicity, typed state, and the restored mixed-type `PowRational` factory. |
| `machines/matrix/src/{dot,matmul,solve,transpose}.rs` | Scalar/fixed/dynamic exact ports, storage rejection, dimension/singularity atomicity, nonnumeric transpose, identity, shape restoration, and rollback. |
| `machines/range/src/port_tests.rs` | All four range families, exact port rejection, arity, dynamic extent, identity, and rollback. |
| `machines/set/src/port_tests.rs` | Dynamic set schemas, union/insert/remove/powerset/product, identity, and rollback. |
| `machines/stats/src/{sum_row,sum_column}.rs` | Fixed/dynamic orientation, exact storage, identity, arity/type rejection, checked sums, dynamic shape restoration, and rollback. |
| `machines/string/src/concat.rs` | Scalar/source/fixed/dynamic exact ports, vector-versus-row broadcast orientation, identity, shape restoration, and rollback. |

The reduced catalog-local counts in
`machines/{combinatorics,math,range}/src/catalog.rs` are intentional: their
legacy `FunctionArgs` validator cases were replaced by canonical contract tests
in the operation suites above. Frozen source surfaces, IDs, exposure, linkage,
and catalog uniqueness remain asserted in the retained catalog tests and
architecture checks. The one removed scene-provider assertion duplicated the
canonical provider-output cases still present in `hosts/scene/tests/provider.rs`.

## Mixed-production declaration census

The same base/head audit compared every top-level public declaration in changed
machine files, independently of the test census. It found and restored the one
canonical production survivor, `PowRational`. After restoration, every remaining
removed declaration belongs to one of two retired classes:

- legacy value-dependent validators:
  `validate_n_choose_k_{scalar,matrix}_contract` and the four
  `validate_range_*` functions; their canonical validators remain in the
  respective catalogs;
- `FunctionSpecializer` implementations and helpers for legacy op-assignment:
  `AddAssignMath`, the `*AssignRange*`/`*AssignValue` specializers, and
  `add_assign_math_fxn`; the current canonical specializers remain registered.

The operation macros were audited separately because generated factories do
not appear as top-level declarations in source text. Current factory/catalog
enumeration and owner tests confirm that the canonical generated families are
still present.

## Core function and bytecode suites

| Removed or reduced suite | Permanent disposition |
| --- | --- |
| `src/core/src/function/{argument,catalog,contract,specialization}.rs` | Exact port/layout/alias/shape behavior lives in `src/core/src/function/tests/catalog.rs`, the owner-machine suites above, and compile-fail port tests. Legacy construction and projection cases were deleted. |
| `src/core/src/function/tests/{checkpoint,solve}.rs` | Restored in place against `FunctionInstance`, `FunctionInvocation`, `ValueCell`, and `CanonicalCellId`. All 33 checkpoint/scheduler tests retain their original names and assertions. |
| `src/core/src/function/tests/{dependencies,reactive_plan,register_commit,registry}.rs` | The register-commit suite retains all eleven permanent commit-boundary assertions canonically: stage-before-commit, plan-order deduplication, stage-error atomicity, missing/non-register rejection, overlapping-output rejection, staged-output agreement, ordered dirty cells, no downstream execution, unsupported-staging rejection, and the empty no-op. Dependency tests separately preserve metadata and scope-arity error identity plus plan/index atomicity. Only assertions whose oracle was the retired value projection were deleted. |
| `src/core/src/program/bytecode/{runtime_contracts,tests}.rs` | Canonical bytecode constant, type, malformed-input, round-trip, and deterministic fixture coverage remains; only legacy-value decoder/materializer cases were removed. |
| `src/core/src/program/compiler/context.rs` and `symbol_table.rs` | Canonical register/cell planning is exercised by compiler, engine planning, artifact, and bytecode owner suites; universal-reference identity tests were removed. |
| `src/core/src/state_journal/tests/hashed_cycles.rs` | `state_journal/tests/{canonical,exact_minimal}.rs` cover canonical identity, deduplication, rewind/replay, and sealing. Universal aggregate graph cycles were compatibility-only. |
| `src/core/src/value.rs`, `src/core/src/kind.rs`, and `src/core/src/legacy_adapter/**` | Deleted as representation/conversion tests for types that no longer exist. |
| `src/core/tests/{legacy_kind_adapter_contract,legacy_snapshot_adapter_contract}.rs` | Deleted as adapter-only integration suites. Their permanent boundary is the retired-surface absence contract. |

`src/core/tests/stdlib_macro_hygiene.rs` is a new external-consumer regression:
it expands the relocated exported macros without importing `paste`, including
the generic family and native unary/binary installer paths.

## Engine, runtime, and public behavior

| Removed or reduced suite | Permanent disposition |
| --- | --- |
| `src/engine/src/activation/tests/{captures,dispatch,exhaustiveness,guards,registers,registration,rollback,support}.rs` | Universal-value fixtures were deleted. Their permanent assertions are split between canonical capture/state tests in `activation/{captures,mod}.rs` and source-level dispatch, guard, exhaustiveness, register, rollback, and topology tests in `statements/tests/activation_scope.rs`; the assertion groups are enumerated below. |
| `src/engine/src/expressions/tests/variables.rs` | Restored in place with all nine original tests using canonical cells, snapshots, representations, and live-resource identity. |
| `src/engine/src/function/{external/mod.rs,mod.rs}` | Canonical external-function planning, schema rebinding, output identity, and errors remain in engine/runtime external tests; legacy output projection cases were deleted. |
| `src/engine/src/interpreter/tests/checkpoint.rs` | Restored in place with all five original checkpoint tests using canonical specializers, cells, snapshots, and handle identity. |
| `src/engine/src/program/bytecode_plan_topology_tests.rs` | Ordinary-source parity lives in `program/compiler_planning.rs`; malformed canonical sidecars and decoded artifacts live in `engine/tests/program_artifact_contract.rs`. |
| reduced `src/engine/src/program/compiler_planning.rs` tests | The retained canonical cases cover direct cells, schemas, typed wrappers, resource/host values, constants, and artifacts. Removed cases asserted legacy wrapper order/projection. |
| `src/engine/src/statements/tests/op_assign.rs` | Six scheduler/register tests are restored in place canonically. `src/stdlib/tests/source_op_assign_contract.rs` preserves the machine-catalog source path for indexed add-assignment and asserts `[2,:]`, `[1..=2,:]`, index-vector/all, and logical-mask/all selectors. Machine-local port tests continue to cover the underlying scalar/range/index/mask/row-all factories. |
| reduced `src/engine/src/{integrity.rs,intrinsics/horzcat.rs}` tests | Canonical integrity/resource and concatenation cases remain; removed assertions inspected universal-value projections. |
| `src/runtime/src/runtime/program/external/value_adapter_tests.rs` | Deleted as adapter-only. Canonical external values remain covered by `external/tests.rs` and runtime program tests. |
| one reduced `tests/mech_repl.rs` assertion | Public REPL source/output behavior remains; only universal-value formatting was removed. |

## Migration-only test infrastructure

The function-system baseline fixture and the Python migration inventory,
growth, destination, and C2-boundary suites were deleted with their temporary
contracts. Their permanent replacements are:

- `retired-public-surface-v1.json`, the inventory derived from every public
  declaration in the deleted core files at the PR10 base;
- `scripts/check-no-retired-value-system.py`, which rejects every retired
  topology, module, symbol, conversion entry point, and the retired semantic
  `Kind` declaration while allowing the parser AST's distinct `nodes::Kind`;
- `scripts/tests/test_check_no_retired_value_system.py`, which injects a
  negative fixture for every inventory class and entry;
- the retained canonical encoding, schema, snapshot, resident, execution,
  bytecode-v1, unsafe-boundary, and warning-policy contracts.

The old fixture's 17-case `specialization-cases.json` is not migration-only.
`src/stdlib/tests/specialization_contract.rs` now consumes it directly through
canonical `SpecializationInvocation` cells and verifies every operation ID,
input/output kind and shape, and selected runtime factory name/ID. The
function-system source gate executes that integration test.

## Assertion-level mixed-suite audit

The second review found that the original file-level census was not precise
enough. The following groups record the exact permanent assertions that moved
out of wholesale-deleted engine suites. Names not listed here were restored in
place under their original names as described above.

### Activation capture storage

`activation/captures.rs::canonical_capture_slots_cover_scalar_tuple_record_and_matrix_schemas`
constructs every enabled scalar family plus tuple, record, and matrix slots.
`canonical_capture_slot_preserves_identity_across_repeated_updates` separately
checks two updates through one cell identity. Together they replace:

- `activation_capture_slot_supports_all_enabled_scalar_kinds`
- `activation_capture_slot_preserves_identity_across_updates`
- `activation_capture_slots_support_enabled_composite_value_kinds`
- `activation_pattern_capture_storage_identity_is_stable`

`activation/captures.rs::canonical_capture_slots_support_dynamic_set_map_and_table_extents`
preserves dynamic aggregate payload, schema, and identity assertions from the
composite capture cases. `capture_batch_preflight_is_atomic`,
`selected_and_unselected_capture_gates_commit_atomically`, and
`patterns::tests::failed_compiled_match_returns_no_bindings_and_cannot_mutate_sink`
cover the preflight, selected-arm, unselected-arm, mismatch, and failed-match
boundaries formerly asserted by:

- `activation_capture_commit_validates_every_binding_before_mutation`
- `activation_capture_gate_validates_entire_commit_before_mutation_or_pulse`
- `activation_failed_repeated_binding_leaves_proposed_and_committed_unchanged`
- `activation_non_selected_composite_capture_keeps_last_committed_value`
- `activation_capture_slot_rejects_kind_mismatch`

The source-level tests
`patterned_activation_permanent_pattern_forms_dispatch_without_topology_growth`,
`patterned_activation_captures_tuple_values_without_growing_the_plan`, and
`patterned_activation_captures_do_not_leak_and_restore_outer_bindings` retain
the atom/enum payload, whole-composite visibility, tuple element access,
array-rest payload, capture leak, and capture-shadowing oracles formerly named:

- `activation_atom_capture_accepts_a_new_atom_value`
- `activation_pattern_enum_payload_capture_is_available`
- `activation_only_selected_arm_commits_matching_captures`
- `activation_whole_composite_capture_is_stable_and_visible_to_the_body`
- `activation_whole_tuple_capture_keeps_element_access_attached`
- `activation_array_rest_capture_preserves_kind_payload_and_identity`
- `activation_pattern_capture_does_not_leak`
- `activation_pattern_capture_shadows_and_restores_outer_symbol`

`activation/mod.rs::activation_dispatch_nodes_checkpoint_every_hidden_canonical_state_cell`
is the exact canonical replacement for
`activation_transaction_state_exposes_hidden_mutable_cells`.

### Activation matching, dispatch, and guards

`statements/tests/activation_scope.rs::patterned_activation_permanent_pattern_forms_dispatch_without_topology_growth`
runs every case twice and checks the selected output, dormant unselected body
nodes, equal-packet redispatch, and stable topology. Its cases cover enum and
payload-free variants, nested tuples, equal and unequal repeated bindings,
typed literals, pattern-expression shadowing, prefix/suffix spread, and nested
rest capture:

- `activation_pattern_selects_pressed_released_and_wildcard`
- `activation_pattern_enum_arms_compile_independent_of_initial_variant`
- `activation_pattern_equal_packets_dispatch_repeatedly`
- `activation_pattern_unselected_arm_nodes_do_not_execute`
- `activation_pattern_switching_arms_does_not_grow_plan`
- `activation_pattern_matches_payload_free_enum_variant`
- `activation_pattern_atom_tagged_tuple_selects_arm`
- `activation_pattern_atom_tuple_arms_compile_independent_of_initial_tag`
- `activation_typed_literal_pattern_uses_shared_value_matching`
- `activation_pattern_atom_tagged_tuple_captures_payload`
- `activation_pattern_tuple_captures_elements`
- `activation_pattern_nested_tuple_captures_elements`
- `activation_pattern_repeated_capture_requires_equal_values`
- `activation_array_pattern_supports_prefix_suffix_and_anonymous_spread`
- `activation_array_rest_segment_accepts_nested_array_pattern`
- `activation_pattern_repeated_capture_kind_mismatch_uses_canonical_error`
- `activation_pattern_expression_uses_outer_symbol_when_capture_name_collides`

The four sampling/scope assertions have dedicated canonical tests rather than
being inferred from that dispatch matrix:

- `patterned_activation_samples_pattern_expressions_only_on_trigger`
- `patterned_activation_samples_current_user_function_output_on_trigger`
- `patterned_activation_captures_do_not_leak_and_restore_outer_bindings`
- `patterned_activation_arm_definitions_do_not_leak_between_arms`

`patterned_activation_guards_dispatch_in_order_without_growing_the_plan` and
`patterned_activation_unselected_guard_skips_body_errors_atomically` preserve
source order, repeated packets, dormant unselected bodies, and failed-body
rollback. `patterned_activation_rejects_eager_guard_control_flow_atomically`
and `patterned_activation_rejects_unsafe_extensions_before_specialization`
cover guard purity, non-Boolean rollback, eager nested/user dispatch, and the
pre-specialization safety boundary. The permanent exhaustiveness cases,
including the fixed-matrix irrefutable form, are asserted by
`patterned_activation_exhaustiveness_rules_are_preserved_canonically`.

Concretely, those targets replace all ten former guard tests:

- `activation_guards_fall_through_in_source_order_and_commit_only_the_selected_arm`
- `activation_guard_outer_dependencies_are_sampled_until_the_next_trigger`
- `activation_guard_user_function_refreshes_on_each_matching_trigger`
- `activation_guard_initialization_commits_the_first_eligible_arm_without_pulsing_a_body`
- `activation_guard_equal_packets_dispatch_again_without_changing_topology`
- `activation_guard_rejects_unsafe_extension_before_specialization`
- `activation_guard_rejects_eager_nested_match_control_flow`
- `activation_guard_rejects_user_function_pattern_dispatch_that_cannot_refresh_statically`
- `activation_guard_capture_shadows_outer_name_while_pattern_expression_keeps_outer_name`
- `activation_guard_composite_rest_proposal_commits_only_when_the_guard_passes`

### Activation registers, registration, and rollback

The canonical activation-scope tests retain the former register and
registration assertions as follows:

- `activation_scope_registers_commit_atomically` and
  `activation_scope_register_commit_does_not_reactivate_body` cover selected
  register batches, unselected-arm dormancy, equal-trigger transitions, and
  at-most-once register commits.
- `activation_scope_rejects_whole_assignment_to_trigger`,
  `activation_scope_rejects_operator_assignment_to_trigger`, and
  `patterned_activation_rejects_alias_and_indexed_writes_atomically` cover
  direct, alias, plain-indexed, and indexed-op trigger/write rejection.
- `activation_scope_external_inputs_are_sampled`,
  `activation_scope_samples_latest_external_value`, and
  `activation_scope_ignores_external_value_change` cover sampled live inputs
  and aliases.
- `patterned_activation_preflight_and_elaboration_fail_atomically` and
  `activation_scope_failed_elaboration_clears_registration_state` cover every
  former symbol-table, dictionary, plan, heterogeneous-repeat preflight,
  nested-activation, context-declaration, and elaboration rollback assertion.

### Artifact topology

The former `bytecode_plan_topology_tests.rs` assertions are split by semantic
owner rather than removed. This table records the assertion-level mapping:

| Former topology test | Canonical owner |
| --- | --- |
| `ordinary_mech_sources_emit_equivalent_program_artifacts_in_bytecode_v1` | Restored under the same name in `program/compiler_planning.rs`. |
| `generic_source_matrix_literals_fold_without_legacy_artifact_nodes`, `heterogeneous_source_matrix_literal_fails_structurally` | `program_artifact_contract::{matrix_literal_resolution_is_homogeneous_and_structured,matrix_literal_resolution_canonicalizes_optional_elements,compiled_matrix_sidecars_fold_static_literals_through_canonical_ir}`. |
| `frozen_v1_compatibility_product_preserves_implementation_operation_ids` | `program_artifact_contract::representative_source_and_bytecode_routes_produce_identical_artifacts`, the bytecode-v1 fixture gate, and the new 17-case specialization contract. |
| `composite_return_materialization_has_semantic_node_metadata`, `immutable_composite_definitions_remain_reactive_packs` | `compiler_planning::source_composites_and_mutable_state_keep_exact_artifact_semantics` plus `core::program::bytecode::tests::composite_pack_round_trips_and_reconstructs_from_child_registers`. |
| `ordinary_source_artifacts_preserve_exact_semantics` | The restored ordinary-source parity test plus `program_artifact_contract::representative_source_and_bytecode_routes_produce_identical_artifacts`. |
| `mutable_matrix_state_retains_its_declaration_time_initializer`, `equal_interned_constants_keep_distinct_register_roles`, `multiple_full_state_writers_fail_closed` | `compiler_planning::{source_composites_and_mutable_state_keep_exact_artifact_semantics,source_artifact_rejects_multiple_full_state_writers}`. |
| `composite_helpers_and_mutable_metadata_without_a_declaration_do_not_become_state` | The retired compiler-metadata fixture was deleted; the permanent state-marker rule remains in `program_artifact_contract::{state_slots_are_initialized_and_break_feedback_cycles,constants_remain_sources_and_outputs_receive_only_publication_slots}`. |
| `collection_schemas_use_actual_element_cardinality` | PR10 deliberately replaced frozen cardinality with dynamic extents; `cell_binding` dynamic-extent tests and the set/range port suites assert the permanent schema, identity, and rollback rules. |
| `compiler_sidecar_resolves_declared_contracts_into_the_artifact`, `catalog_contract_fills_an_empty_specialized_function_sidecar`, `malformed_compiled_sidecars_fail_closed` | `program_artifact_contract::{malformed_compiled_scalar_metadata_fails_closed,compiled_matrix_sidecar_disagreement_fails_before_lowering,artifact_bytecode_rejects_malformed_operation_contract_semantics_first,decoded_artifact_sections_revalidate_structure_and_limits}`. |
| `pseudo_destination_effects_preserve_the_node_and_every_input` | `document_outputs::effect_only_context_sends_are_not_program_values`, artifact requirement round trips, and bytecode resource intent/delivery validation. |

Bytecode-v1 deterministic topology and malformed wire inputs additionally
remain in `core/program/bytecode/tests.rs` and the 20-fixture bytecode-v1
contract.
