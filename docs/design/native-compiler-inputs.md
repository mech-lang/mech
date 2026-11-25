# Native compiler input compatibility

This inventory covers the canonical native scalar, iteration and Program entry
points, the compact diagnostic and its Cranelift fork. It records the
Reconciled source snapshot at `5b9f252fc7e775bf8e4d850ca368da581e9f5386`.
The fixed recipe sets 233 of the indexed native values. The compiler driver
separately owns `MECH_COMPILE_OVERLAP_SOURCE_CATALOG`, an OS-presence switch for
overlapping catalog construction with parsing. Its fallback is synchronous
catalog construction if the worker cannot start.

## Ownership and interpretation

The canonical Program and its catalog provide source identity, checked layouts,
effects and proofs. The managed route also accepts explicit finite compile and
runtime limits. The legacy native entry snapshots process values. The compact
diagnostic can receive an explicitly captured configuration instead. Its
unstable benchmark options authorize only the selection their existing proof
and applicability checks admit.

Preparation retains the immutable owner through root and helper lowering.
Phase jobs carry an immutable snapshot and a configured Cranelift ISA.
Coordinator-owned accounting, IR construction and ordered publication retain
their original ordering. Each independently built object and each retry has
separate mutable accounting. The sparse guard choice formerly cached for the
process now belongs to the compilation while its exact-value test stays the
same.

OS presence accepts any set value, including empty and non-Unicode input. OS
mode comparisons preserve exact matches. Unicode reads treat unset values and
non-Unicode values according to the original consumer. Settings can be inactive
under a target, observe mode, proof, phase name, selected recipe or feature.
The snapshot stores raw settings and profile read outcomes. Consumers keep
their original checks and parse only on the route that would have parsed
before. The input fingerprint includes raw local path bytes. The benchmark
recipe supplies a separately comparable, path-relative configuration identity.

### Compatibility cases with distinct defaults or errors

| Setting | Interpretation |
| --- | --- |
| `MECH_COMPACT_COMPILE_THREADS` | Parse a Unicode unsigned integer, use zero when absent or invalid, and cap at four. Worker publication follows the existing ordered tickets. |
| `MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2` | On AArch64 with runtime observation disabled, accept only the Unicode strings `4` to `7`. Other present Unicode values return the existing alignment error. |
| `MECH_COMPACT_DIRECT_CONTROLLER_WORDS`, `MECH_COMPACT_DIRECT_CONTINUATION_WORDS` | When the respective preparation route asks for a limit, absent or non-Unicode means zero, a malformed unsigned integer is an error, and values above 40 reject. The controller route first requires enough candidates to share. |
| `MECH_COMPACT_ROUTE_SITES` | Parse comma-separated unsigned source sites, skipping empty parts. Invalid numbers and sites outside the plan reject. Absence or non-Unicode uses the empty set. |
| `MECH_COMPACT_COMPLETE_PROOF_HEADERS` | Parse comma-separated `u32` values only when the proof inventory is selected. Invalid members reject. |
| `MECH_COMPACT_REFUSE_GENERAL_VECTORS` | Parse a comma-separated unsigned header set or use the empty set on an unavailable Unicode value. A malformed member rejects. A nonempty set requires the unobserved compact entry. |
| `MECH_COMPACT_TRAVERSAL_SKIP_HEADERS` | A refusal-region route parses the comma-separated unsigned header set and rejects malformed members. Other region kinds use an empty set. |
| `MECH_COMPACT_FIXED_TRANSFORMS` | Enable only exact listed headers on the existing Apple arm64 eligible route. Unparseable members do not match. |
| `MECH_COMPACT_PHASE_SELECT_LIMITS` | Read `instructions:selects` as two `usize` values. Missing or malformed input keeps the default selector limits. |
| `MECH_GROUP_PACKED_HISTOGRAM_VISITS` | Use four if the existing unsigned parse fails or the Unicode value is unavailable. The grouped histogram retains its applicability checks. |
| `MECH_COMPOSE_IMMUTABLE_COST_CODE_PAGE_SHIFT` | Require a Unicode `u8` in `4..=12`. Otherwise retain the dense representation. Packed storage still has to save bytes. |
| `MECH_COMPACT_PHASE_BLOCK_LAYOUT`, `MECH_COMPACT_PHASE_REGISTER_WEIGHTS` | Read the first exact `symbol@path` match in a Unicode list delimited by literal `;`. Entries without `@` do not match. A selected path uses captured bytes or the captured read error. Layout and register applicability remain independent. |
| `MECH_COMPACT_ROOT_TAIL_PROFILE` | Capture the raw OS path. The checked root reads its captured bytes only on the existing branch-zero route and validates the target extent and padding. |
| `MECH_COMPACT_FP_SPILL_SLOTS` | On entry to its phase consumer, each Unicode `phase@u8` member must parse. Matching symbols and `all` can select a slot count. |
| `MECH_ARM64_REUSE_I8_DEFINITIONS`, `MECH_ARM64_REUSE_I32_ARITHMETIC` | Unicode modes select the named load or compare, or the named add, sub or mul. `all` and `1` select all classes of the respective input. Other values leave them disabled. |
| `MECH_ARM64_INSERT_MASKED_FIELDS` | `1`, `wide` and `wide-copy` select the original narrow, wide and separated-destination paths. Other values keep ordinary lowering. |
| `MECH_INLINE_OWNER_RECORDS` | Runtime support is compiled with this value. Absent means 1. The implemented values are 1, 8, 12 and 16. Other values fail runtime compilation. The benchmark receipt records the runtime identity. |

The host report flag stays outside the legacy code-affecting observation modes.
Those modes keep their existing meaning, including any argument, instrumentation
and selection effects. A diagnostic can therefore have code effects even when
its main purpose is observation. Construction counters come from a separate
instrumented run. Current experimental switches remain implementation details.

## Consumer index

The key identifies the implementation or observation it selects. The owner
column names the source file and function for the precise conditional
interpretation, including where a value is consumed more than once. Native
paths are relative to `hosts/gpu/src/native_artifact`. The `cranelift/` prefix
identifies paths under `vendor/cranelift-codegen/src`.

**Runtime** identifies an input that changes the required compiled runtime
support or entry representation. **Implementation** covers scheduling,
organization or code layout. **Profitability** covers proof-gated selection.
**Diagnostic** covers observations, optional probes and fixture paths. A
superseded classification requires evidence that a consumer or experiment was
retired. Absence from the winning recipe alone does not establish retirement.

| Setting | Classification | Legacy read | Owner |
| --- | --- | --- | --- |
| `MECH_ADJACENT_ADDRESSES` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_APPEND_FORWARD_REPORT` | Diagnostic | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_ARM64_CCMP_BRANCH_SELECT` | Profitability | OS presence | `cranelift/isa/aarch64/lower/isle/mod.rs` · `lower_conjoined_select` |
| `MECH_ARM64_CCMP_SHARED_BOOLEAN` | Profitability | OS presence | `program/codegen/shared_boolean_tests.rs` · `check` / `cranelift/isa/aarch64/lower/isle/mod.rs` · `lower_disjoined_boolean` |
| `MECH_ARM64_FOLD_SHIFTED_EXTENSIONS` | Profitability | OS equal to `"1"` | `cranelift/isa/aarch64/lower/isle/narrow_packs.rs` · `lower` |
| `MECH_ARM64_INSERT_MASKED_FIELDS` | Profitability | OS value interpreted at consumer | `cranelift/isa/aarch64/lower/isle/bit_fields.rs` · `lower` |
| `MECH_ARM64_LOOP_LIVE_RANGES` | Profitability | OS presence | `cranelift/isa/aarch64/inst/mod.rs` · `preallocation_rewrite_enabled` |
| `MECH_ARM64_PACK_NARROW_HALVES` | Profitability | OS equal to `"1"` | `cranelift/isa/aarch64/lower/isle/narrow_packs.rs` · `lower` |
| `MECH_ARM64_REUSE_I32_ARITHMETIC` | Profitability | Unicode value interpreted at consumer | `program/codegen/cfg_cleanup/i32_arithmetic_extensions_tests.rs` · `arithmetic_word_extensions_remove_only_the_proven_add_extension_move` / `cranelift/isa/aarch64/lower/isle/i32_extensions.rs` · `lower` |
| `MECH_ARM64_REUSE_I32_SHIFTS` | Profitability | OS equal to `"1"` | `cranelift/isa/aarch64/lower/isle/i32_extensions.rs` · `lower` |
| `MECH_ARM64_REUSE_I8_BOOLEAN_DEFINITIONS` | Profitability | Unicode value interpreted at consumer | `cranelift/isa/aarch64/lower/isle/byte_extensions.rs` · `lower` |
| `MECH_ARM64_REUSE_I8_DEFINITIONS` | Profitability | Unicode value interpreted at consumer | `program/codegen/cfg_cleanup/byte_definition_extensions_tests.rs` · `byte_definition_extensions_remove_only_the_proven_compare_extension` / `cranelift/isa/aarch64/lower/isle/byte_extensions.rs` · `lower` |
| `MECH_ARM64_REUSE_PREFETCH_BASES` | Profitability | Unicode equality with `"1"` | `cranelift/context.rs` · `optimize` |
| `MECH_ARM64_SUBTRACT_BRANCH` | Profitability | OS presence | `cranelift/isa/aarch64/inst/mod.rs` · `preallocation_branch_rewrite_enabled` / `cranelift/isa/aarch64/inst/mod.rs` · `preallocation_rewrite_branch` |
| `MECH_ARM64_SUBTRACT_IMMEDIATE_BRANCH` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` / `cranelift/isa/aarch64/inst/mod.rs` · `preallocation_branch_rewrite_enabled` / `cranelift/isa/aarch64/inst/mod.rs` · `preallocation_rewrite_branch` |
| `MECH_BOOLEAN_EXTENSION_PROBE` | Diagnostic | OS value interpreted at consumer | `program/codegen/byte_boolean_tests.rs` · `exercise` |
| `MECH_BORROWED_DICTIONARY_SOURCE` | Diagnostic | Unicode value interpreted at consumer | `program/prepare/borrowed_dictionary_values.rs` · `borrowed_dictionary_values_actual_source_selects_two_generic_seed_key_sets` / `program/prepare/fixed_key_loops.rs` · `fixed_key_native_loops_actual_source_has_two_complete_acyclic_bodies` |
| `MECH_BORROW_COMPACT_READ_COPIES` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_BOUNDED_BYTE_HISTOGRAM` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_CARRY_REDUCTION_BOUND` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_CHECKED_APPEND_RUN` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_CHECK_GROUPED_LITERAL_BYTES` | Profitability | OS presence | `program/codegen/append_run.rs` · `optimize_inner` |
| `MECH_COALESCE_EQUAL_JOIN_ARGUMENTS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_COMPACT_ADMIT_CARRIED_HALFWORDS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_ADMIT_DECREASING_INDICES` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_ALIGN_REGION_OUTPUTS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_BIND_REFUSAL_LITERALS` | Profitability | OS value interpreted at consumer | `program/compact/checked_traversals.rs` · `share` |
| `MECH_COMPACT_BIND_REGION_INPUT_ROLES` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_BIND_REGION_LITERALS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_BOUNDED_PRICE_BLEND` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_BOUNDED_U32_TREE` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_BRANCH_ON_ZERO` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_CARRY_BRANCH_OUTCOMES` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_CCMP_LITERAL_CONSTANTS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_CHECKED_BYTE_SCATTER` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_CHECKED_PHASE_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_CHECKED_TRAVERSALS_COLD_LAYOUT` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_COALESCE_IDENTICAL_ARGUMENTS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_COALESCE_PARALLEL_PHIS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_COMPILE_THREADS` | Implementation | Unicode number at consumer | `program/compiler.rs` · `emit_prepared_compact_object` |
| `MECH_COMPACT_COMPLETE_PROOF_HEADERS` | Diagnostic | Unicode number at consumer | `program/compiler.rs` · `compact_native_layout_for_diagnostic` |
| `MECH_COMPACT_CONTINUATION_REGION_REPORT` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_CONTINUATION_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_DEFINE_TERMINAL_ENTRY_FIRST` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_DELAY_IMMUTABLE_WORDS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_DENSE_BRANCH_ON_ZERO` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_DENSE_NO_GROWTH_ONLY` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_DIRECT_CONTINUATION_WORDS` | Runtime | Unicode integer with conditional error | `program/compact/checked_traversals.rs` · `direct_input_limit` |
| `MECH_COMPACT_DIRECT_CONTROLLER_WORDS` | Runtime | Unicode integer with conditional error | `program/compact/checked_traversals.rs` · `direct_input_limit` |
| `MECH_COMPACT_DIRECT_ENTRY_LITERALS` | Profitability | OS presence | `program/compiler.rs` · `emit_compact_closed_object_with_phase_observer_for_diagnostic` |
| `MECH_COMPACT_DIRECT_RUNTIME_CALLS` | Runtime | Unicode value interpreted at consumer | `program/compiler.rs` · `emit_prepared_compact_object` |
| `MECH_COMPACT_DIRECT_SCALAR_HOMES` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_DIRECT_SCALAR_HOME_CENSUS` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_DUMP_CONTROLLER_MATCHES` | Diagnostic | OS directory | `program/compact/checked_traversals.rs` · `prepare_bodies` |
| `MECH_COMPACT_DUMP_REFUSAL_MATCHES` | Diagnostic | OS directory | `program/compact/checked_traversals.rs` · `prepare_bodies` |
| `MECH_COMPACT_EARLY_REGION_PREVIEW` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_with_pool` |
| `MECH_COMPACT_EARLY_REGION_REPORT` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_with_pool` |
| `MECH_COMPACT_EARLY_SOURCE_REGION` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_EMPTY_SOURCE_ROUTES` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_EXPORTS_FIRST` | Implementation | OS presence | `program/compiler.rs` · `emit_prepared_compact_object` |
| `MECH_COMPACT_FAILURE_DIR` | Diagnostic | OS presence / OS value interpreted at consumer | `program/compact/phase_function.rs` · `general_ir_observed` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_FINISH_CFG_CLEANUP` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FINISH_CFG_REPORT` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FIXED_PHASE_PACKET` | Profitability | OS value interpreted at consumer | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_FIXED_TRANSFORMS` | Profitability | Unicode number at consumer | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_FOLD_CHECKED_ADDS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FOLD_FRESH_OWNER_COUNTS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FOLD_INVERTED_CSET` | Profitability | OS value interpreted at consumer | `program/compiler.rs` · `emit_compact_closed_object_with_phase_observer_for_diagnostic` |
| `MECH_COMPACT_FOLD_PHASE_CHECKED_ARITHMETIC` | Profitability | Unicode symbol list or exact `all` | `program/compact/phase_function.rs` · `phase_selected` |
| `MECH_COMPACT_FOLD_REDUNDANT_ZERO` | Profitability | OS presence | `program/compiler.rs` · `emit_compact_closed_object_with_phase_observer_for_diagnostic` |
| `MECH_COMPACT_FOLD_SINGLE_USE_CSET` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_FOLD_UNIT_BRANCHES` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FORWARD_ARGUMENT_JUMPS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_FORWARD_PHASE_ROUTES` | Profitability | Unicode symbol list or exact `all` | `program/compact/phase_function.rs` · `phase_selected` |
| `MECH_COMPACT_FORWARD_PHASE_SPILLS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_FORWARD_SPILL_RELOADS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_FORWARD_VIEW_GEOMETRY` | Runtime | OS presence | `program/compact/arrays.rs` · `declare` |
| `MECH_COMPACT_FORWARD_WORD_ORDER_RESULT` | Profitability | OS presence | `program/codegen/cfg_cleanup/compared_bytes/word_order.rs` · `fold_word_order` |
| `MECH_COMPACT_FP_SPILL_KEEP_PAIRS` | Profitability | OS presence | `program/compact/phase_layout.rs` · `apply_fp_spill_cache` |
| `MECH_COMPACT_FP_SPILL_SLOTS` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_layout.rs` · `apply_fp_spill_cache` |
| `MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2` | Implementation | Unicode value interpreted at consumer | `program/compiler.rs` · `emit_compact_closed_object_with_phase_observer_for_diagnostic` |
| `MECH_COMPACT_FUSE_WRITABLE_GROWTH` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_GROUP_CODE_PRICES` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_GROUP_FOUR_LITERAL_APPENDS` | Profitability | OS presence | `program/codegen/append_run.rs` · `optimize_inner` |
| `MECH_COMPACT_GROUP_RETIREMENTS` | Runtime | OS presence | `program/compact/arrays.rs` · `declare` |
| `MECH_COMPACT_GUARD_DENSE_BEFORE_PACKET` | Profitability | OS presence | `program/compact/phases.rs` · `emit` |
| `MECH_COMPACT_HOIST_INDEXED_LOAD_BASES` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_HOIST_PREFETCH_BASES` | Profitability | Unicode equality with `"1"` | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_IMMUTABLE_DEFAULTED_READS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_INDEX_LOOP_LIVE_OUTS` | Profitability | OS presence | `program/codegen/cfg_cleanup/threaded_edges/mod.rs` · `optimize` |
| `MECH_COMPACT_INLINE_SHORT_BYTE_APPENDS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_INLINE_SINGLE_CALLS` | Profitability | OS presence / OS value interpreted at consumer | `program/compact/object.rs` · `new` / `program/compact/object.rs` · `inline_private_helpers` / `program/compact/object.rs` · `define_function_with_control_plane` / `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_INLINE_SYMBOLS` | Profitability | Unicode value interpreted at consumer | `program/compact/object.rs` · `once_called_helpers` / `program/compact/object.rs` · `define_function_with_control_plane` / `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_KEEP_REFUSED_PHASE_CODE` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_KEEP_SHARED_TERMINAL_RETURNS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `plan` |
| `MECH_COMPACT_LATE_PHASE_SELECTS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_LAYOUT_CONTROLLER_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_LAYOUT_REGIONS_LAST` | Implementation | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compiler.rs` · `emit_prepared_compact_object` |
| `MECH_COMPACT_LAYOUT_REGION_REPORT` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_LITERAL_ALLOCATION_FROM_FRAME` | Profitability | OS presence | `program/compact/release_stubs.rs` · `prepare_invariant` |
| `MECH_COMPACT_LITERAL_ALLOCATION_SETUP` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_LITERAL_EDGE_ARGUMENTS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_NATIVE_GAP_SORT` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_NONZERO_U32_PACK` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_NORMALIZE_COMPOSED_ADDRESSES` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_NORMALIZE_INDEXED_ADDRESSES` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_OBSERVE_CHECKED_TRAVERSALS` | Diagnostic | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_OBSERVE_CONTINUATIONS` | Diagnostic | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_OBSERVE_LAYOUT_REGIONS` | Diagnostic | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_ORDER_REGION_CAPTURES` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_PACK_LARGE_REFUSALS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `share` |
| `MECH_COMPACT_PACK_LE_SCALAR_STORES` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_PACK_PHASE_INPUTS` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_PACK_WRITABLE_STORAGE` | Runtime | OS presence | `program/compact/arrays.rs` · `declare` |
| `MECH_COMPACT_PAD_REGION_OUTPUTS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_PAIR_REGISTER_SPILLS` | Profitability | OS value interpreted at consumer | `program/compiler.rs` · `emit_compact_closed_object_with_phase_observer_for_diagnostic` |
| `MECH_COMPACT_PAIR_STACK_WORDS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_PAIR_WORDS` | Profitability | OS value interpreted at consumer | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compact/emit.rs` · `define_entry` / `program/compact/phase_bodies.rs` · `prepare_body` |
| `MECH_COMPACT_PHASE_BLOCK_LAYOUT` | Implementation | Unicode value interpreted at consumer | `program/compact/phase_layout.rs` · `profiles` |
| `MECH_COMPACT_PHASE_BRANCH_ON_ZERO` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_PHASE_CONTROLLER_WINDOWS` | Diagnostic | OS presence / Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `general_ir_observed` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_PHASE_REGISTER_WEIGHTS` | Implementation | Unicode value interpreted at consumer | `program/compact/phase_layout.rs` · `apply_register_weights` |
| `MECH_COMPACT_PHASE_SELECTS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_PHASE_SELECT_LIMITS` | Profitability | Unicode number at consumer | `program/compact/phase_function.rs` · `select_phase_choices` |
| `MECH_COMPACT_PREFER_PRESERVED_REGS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_PREPARE_DENSE_FIRST` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_PREPARE_LAYOUT_REGION_IR` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` |
| `MECH_COMPACT_PREPARE_REFUSAL_IR` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` |
| `MECH_COMPACT_PREPARE_REFUSED_VECTOR_ONLY` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_PROFILE_PHASE_CALLS` | Diagnostic | OS presence | `program/compact/phases.rs` · `emit` / `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_PROOF_INVENTORY` | Diagnostic | OS presence | `program/compiler.rs` · `compact_native_layout_for_diagnostic` |
| `MECH_COMPACT_PROVE_BACKING_OWNERS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_PROVE_NONOWNER_FIELDS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_PROVE_UNSIGNED_EXTREMA` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_REBASE_BOUNDED_COMPOSED_INDICES` | Profitability | Unicode symbol list or exact `all` | `program/compact/phase_function.rs` · `phase_selected` |
| `MECH_COMPACT_REFINE_COALESCED_ARGUMENTS` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_REFINE_IMMUTABLE_WORDS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_REFINE_PHASE_PREDICATES` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_REFINE_SELECTED_VALUES` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_REFUSE_GENERAL_VECTORS` | Profitability | Unicode number at consumer | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_REFUSE_PHASE_GROWTH` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_REGION_REGISTER_INPUTS` | Runtime | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_REGION_REGISTER_OUTPUTS` | Runtime | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_REGISTER_BACKING` | Runtime | OS presence | `program/compact/arrays.rs` · `declare` |
| `MECH_COMPACT_REMATERIALIZE_IMMUTABLE_READS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_RETURN_REGION_ERROR_ORIGIN` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `from_environment` |
| `MECH_COMPACT_REUSE_CENSUS` | Diagnostic | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_REUSE_OVERWRITTEN_RESULT` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_REUSE_POINTWISE_SOURCE` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_REUSE_WORD_MISMATCH_ORDER` | Profitability | Unicode symbol list or exact `all` | `program/compact/phase_function.rs` · `phase_selected` |
| `MECH_COMPACT_ROOT_FLOW_PASSES` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_ROOT_PROOFS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_ROOT_SELECTS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_ROOT_TAIL_PROFILE` | Implementation | OS value interpreted at consumer | `program/compact/object.rs` · `define_function_with_control_plane` |
| `MECH_COMPACT_ROUTE_DUMP` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_ROUTE_SITES` | Diagnostic | Unicode number at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_ROUTE_SOURCELOCS` | Diagnostic | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SCHEDULE_IMMUTABLE_WORDS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_SEEDED_HISTOGRAM` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_SELECT_DEFAULTED_LOADS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SELECT_PHASE_DEFAULTED_LOADS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_SELECT_PHASE_OUTPUT_WORDS` | Profitability | OS value interpreted at consumer | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_SHARE_BACKING_SETUP` | Profitability | Unicode value interpreted at consumer | `program/compact/release_stubs.rs` · `prepare_invariant` |
| `MECH_COMPACT_SHARE_CAPACITY_GROWTH` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_CHECKED_TRAVERSALS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_COLD_EPILOGUES` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_COLD_TAILS` | Implementation | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_CONTROLLER_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_FRAME_COPIES` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_INDEXED_STORES` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_MATRIX_WORDS` | Profitability | OS presence | `program/compact/emit.rs` · `define_with_pool` |
| `MECH_COMPACT_SHARE_NUMERIC_ERROR_TAILS` | Profitability | OS presence / Unicode value interpreted at consumer | `program/codegen/numeric_compact.rs` · `emit` / `program/compact/phase_function.rs` · `refused_general_ssa_unobserved` |
| `MECH_COMPACT_SHARE_OWNER_RETIREMENT` | Profitability | OS value interpreted at consumer | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_PHASE_BODIES` | Implementation | OS presence | `program/compiler.rs` · `emit_prepared_compact_object` |
| `MECH_COMPACT_SHARE_PHASE_RELOCATIONS` | Implementation | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_SHARE_RELEASE_SETUP` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_SOURCE_CALL_PACKETS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHARE_STATIC_ERROR_STATUS` | Profitability | OS presence | `program/compact/checked_traversals.rs` · `prepare_bodies` / `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SHORT_APPEND_SPARE_STORE` | Profitability | OS equal to `"8-direct"` | `program/compact/checked_byte_append.rs` · `emit` |
| `MECH_COMPACT_SHORT_BYTE_APPENDS` | Profitability | OS equal to `"wide"` / OS presence | `program/compact/checked_byte_append.rs` · `recognize` / `program/compact/checked_traversals.rs` · `prepare_bodies` |
| `MECH_COMPACT_SHORT_EQUALITY_PHASE` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_COMPACT_SKIP_EMPTY_JUMPS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_COMPACT_SORT_SNAPSHOT_SCRATCH` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPACT_SOURCE_BLOCK_REPORT` | Diagnostic | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SOURCE_CONTINUATION_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SOURCE_CONTROLLER_WINDOWS` | Profitability | Unicode value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_SOURCE_OUTPUT_SCRATCH` | Profitability | OS value interpreted at consumer | `program/compact/early_units.rs` · `derive` |
| `MECH_COMPACT_SOURCE_OWNER_HOMES` | Profitability | OS presence | `program/compact/early_units.rs` · `derive` |
| `MECH_COMPACT_SPECIALIZE_TAGGED_STORE_TAILS` | Profitability | Unicode symbol list or exact `all` | `program/compact/phase_function.rs` · `phase_selected` |
| `MECH_COMPACT_TRAVERSAL_SKIP_HEADERS` | Profitability | Unicode number at consumer | `program/compact/checked_traversals.rs` · `share_families` |
| `MECH_COMPACT_TRIM_RETIREMENTS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_VECTOR_FACTS` | Profitability | OS value interpreted at consumer | `program/compact/emit.rs` · `define_entry` |
| `MECH_COMPACT_VERIFY_CLEANUP_ONCE` | Profitability | OS presence | `program/codegen/cfg_cleanup/mod.rs` · `optimize_final_batched_routes` |
| `MECH_COMPACT_WORD_MISMATCH_EXTRACT` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPOSE_FOLD_ADMITTED_COST_MINIMUM` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_COMPOSE_IMMUTABLE_COST_CODES` | Profitability | OS presence | `program/codegen/cfg_cleanup/composed_reads.rs` · `plan` / `program/compact/immutable_arrays.rs` · `literal` |
| `MECH_COMPOSE_IMMUTABLE_COST_CODE_PAGE_SHIFT` | Profitability | Unicode number at consumer | `program/codegen/cfg_cleanup/composed_code_pages.rs` · `selected_shift` |
| `MECH_COMPOSE_READONLY_LOOKUPS` | Profitability | OS presence | `program/compact/emit.rs` · `define_with_pool` |
| `MECH_COMPOSE_SPLIT_COST_LENGTH` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_CURRENT_PREDICATES` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_DEFER_EXIT_PACKET_WORDS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_DIRECT_ITEM_NO_GROWTH` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `emit` |
| `MECH_EMIT_REDUCTION_FALLTHROUGH` | Profitability | OS presence | `program/codegen/cfg_cleanup/reduction_rotation.rs` · `apply` |
| `MECH_EQUAL_COMPARE_STRATEGY` | Profitability | Unicode value interpreted at consumer | `program/codegen/wide_equal.rs` · `optimize` |
| `MECH_FIRST_PACKED_REVERSE_READ` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_FOLD_NARROW_PACKED_WORDS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_FORWARD_PREVIOUS_APPEND` | Profitability | OS equal to `"seed"` / OS presence | `program/codegen/append_previous.rs` · `apply` / `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_FORWARD_PREVIOUS_CLEANUP` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_FORWARD_PREVIOUS_NARROW` | Profitability | OS presence | `program/codegen/append_previous.rs` · `apply` |
| `MECH_FORWARD_SELECTED_CONTINUATIONS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_FORWARD_ZERO_TRIP_REDUCTIONS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_FP_SPILL_PROBE` | Diagnostic | OS value interpreted at consumer | `program/codegen/fp_spill_tests.rs` · `fp_spills_preserve_bits_loops_aliases_guards_and_call_clobbers` |
| `MECH_GROUPED_LITERAL_PREFIX_SPAN` | Profitability | OS presence | `program/codegen/append_run.rs` · `optimize_inner` |
| `MECH_GROUP_PACKED_HISTOGRAM` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_ADMITTED_PREFIX_READS` | Profitability | OS presence | `program/codegen/cfg_cleanup/grouped_histogram.rs` · `specialize_single_literals` |
| `MECH_GROUP_PACKED_HISTOGRAM_BYTE_KEYS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_FOLD_FIELDS` | Profitability | Unicode value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_MIXED_DISPATCH` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_MIXED_IF_EITHER` | Profitability | OS presence | `program/codegen/cfg_cleanup/grouped_histogram.rs` · `guard_entry` |
| `MECH_GROUP_PACKED_HISTOGRAM_PAIR_VECTOR_CHUNKS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_PROGRESSIVE` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_STREAM_CHUNKS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_VECTOR_CHUNKS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_VISITS` | Profitability | Unicode number at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GROUP_PACKED_HISTOGRAM_WAIT_FOR_RUN` | Profitability | OS presence | `program/codegen/cfg_cleanup/grouped_histogram_chunks.rs` · `emit` |
| `MECH_GROUP_PACKED_HISTOGRAM_WIDE_KEYS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_GUARD_SPARSE_ROUNDS` | Implementation | Unicode equal to `"1"` | `program/compact/guard_state.rs` · `use_sparse_rounds` |
| `MECH_HISTOGRAM_ENTRY_COUNTER_CAPACITY` | Profitability | OS value interpreted at consumer | `program/compact/phase_histogram.rs` · `prepare` |
| `MECH_HISTOGRAM_GROUP_BYTE_VISITS` | Profitability | OS value interpreted at consumer | `program/compact/phase_histogram.rs` · `prepare` |
| `MECH_HISTOGRAM_VECTOR_COUNTER_SCAN` | Profitability | OS presence | `program/compact/phase_histogram.rs` · `prepare` |
| `MECH_IDENTITY_ROUTE_FORWARDING` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_INLINE_OWNER_RECORDS` | Runtime | Compile-time text | `compact_runtime/owner_storage.rs` |
| `MECH_INSPECT_COMPOSED_READS` | Diagnostic | OS presence | `program/codegen/cfg_cleanup/composed_reads.rs` · `plan` |
| `MECH_INSPECT_FP_SPILL_CACHE` | Diagnostic | OS value interpreted at consumer | `cranelift/isa/aarch64/mod.rs` · `compile_function` |
| `MECH_INSPECT_NUMERIC_SITE_BLOCKS` | Diagnostic | OS value interpreted at consumer | `program/codegen/numeric_compact.rs` · `emit` |
| `MECH_INSPECT_OPTIMIZED_PHASE_CLIF` | Diagnostic | OS presence / OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_INSPECT_OVERWRITE_PROOF` | Diagnostic | OS presence | `program/compact/overwrite_reuse.rs` · `derive` |
| `MECH_INSPECT_PHASE_CLIF` | Diagnostic | OS presence / OS value interpreted at consumer | `program/codegen/append_run.rs` · `optimize_inner` / `program/codegen/cfg_cleanup/mod.rs` · `coalesce_equal_join_arguments` / `program/codegen/wide_equal.rs` · `optimize_with_strategy` / `program/compact/phase_function.rs` · `general_ir_observed` / `program/compact/phase_function.rs` · `prepare_impl` / `program/compact/phase_function.rs` · `optimize_phase` / `program/compact/phase_histogram.rs` · `prepare` |
| `MECH_INSPECT_PHASE_REGISTER_PROFILE` | Diagnostic | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` / `program/compact/phase_seed.rs` · `define_seeded_histograms` |
| `MECH_INSPECT_PHASE_VCODE` | Diagnostic | OS presence / OS value interpreted at consumer | `program/compact/phase_function.rs` · `general_ir_observed` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_INSPECT_WORD_MISMATCH_ORDER` | Diagnostic | OS presence | `program/codegen/cfg_cleanup/compared_bytes/mod.rs` · `trace` / `program/codegen/cfg_cleanup/compared_bytes/word_order.rs` · `fold_word_order` |
| `MECH_MATRIX_KEY_BENCH_INPUT` | Diagnostic | Required Unicode fixture path | `program/runtime/matrix_map_keys_actual_tests.rs` · `actual fixture reader` |
| `MECH_MATRIX_KEY_BENCH_ORACLE` | Diagnostic | Required Unicode fixture path | `program/runtime/matrix_map_keys_actual_tests.rs` · `actual fixture reader` |
| `MECH_MATRIX_KEY_BENCH_SOURCE` | Diagnostic | Unicode value interpreted at consumer | `program/prepare/borrowed_map_keys.rs` · `borrowed_map_keys_actual_source_pins_snapshot_across_original_handle_retirement` / `program/prepare/dictionary_seed_frames.rs` · `dictionary_seed_frames_actual_source_has_two_generic_nonempty_seed_packs` / `program/prepare/generic_dictionary.rs` · `generic_dictionary_actual_source_has_two_generic_nonempty_seed_candidates` / `program/prepare/matrix_map_keys/tests.rs` · `matrix_map_keys_selects_actual_current_related_post_source` |
| `MECH_MOVE_PHASE_INPUT_PACKET` | Profitability | OS presence or Unicode `only:` list | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_NARROW_MASKED_ARITHMETIC` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_NARROW_ORDERED_ITEM` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` / `program/codegen/ordered_reduction/emit.rs` · `emit_item` |
| `MECH_NARROW_PACK_PROBE` | Diagnostic | OS value interpreted at consumer | `program/codegen/narrow_pack_tests.rs` · `exercise` / `program/codegen/narrow_pack_tests.rs` · `masked_field_inserts_preserve_values_effects_and_fallbacks` |
| `MECH_NARROW_PRIVATE_ARITHMETIC` | Profitability | OS equal to `"after-prefetch"` / OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` / `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_NORMALIZE_DEFAULTED_READ_SETUP` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_NORMALIZE_SELECTED_LOOP_TEST` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_ORDERED_CAPTURE_SPANS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_ORDERED_ENCODE_WINNER_WITH_OFFSET` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_FIRST_REVERSE_FROM_END` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_HORIZONTAL_MINIMUM` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `minimum_splat` |
| `MECH_ORDERED_LAZY_REVERSE_CURSOR` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_PACKED_ACCUMULATORS` | Profitability | OS value interpreted at consumer | `program/codegen/ordered_reduction/emit.rs` · `emit` / `program/codegen/ordered_reduction/vector_loop.rs` · `selected` |
| `MECH_ORDERED_PACKED_COST_INDEX` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` / `program/codegen/ordered_reduction/vector_loop.rs` · `packed_enabled` |
| `MECH_ORDERED_PACKED_EIGHT_ENTRY` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_PACKED_HEAD_GROUP` | Profitability | OS presence | `program/codegen/ordered_reduction/vector_loop.rs` · `selected` |
| `MECH_ORDERED_PACKED_WIDE_ITEM` | Profitability | OS presence | `program/codegen/ordered_reduction/vector_loop.rs` · `selected` |
| `MECH_ORDERED_PACK_WINNER_AT_EXIT` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_PAIRED_CONTINUATION` | Profitability | OS presence / OS value interpreted at consumer | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_PREFER_PRESERVED_REGS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_ORDERED_REUSE_FIRST_ORDINAL` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_REVERSE_LOW_WORDS` | Profitability | OS presence | `program/codegen/ordered_reduction/cost.rs` · `read_at` |
| `MECH_ORDERED_SHORT_NARROW_ORDINAL` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_SKIP_UNCHANGED_VECTOR_WINNER` | Profitability | OS value interpreted at consumer | `program/codegen/ordered_reduction/vector_loop.rs` · `emit` |
| `MECH_ORDERED_VECTOR_CARRY_LANES` | Profitability | OS presence | `program/codegen/ordered_reduction/vector_loop.rs` · `lanes` / `program/codegen/ordered_reduction/vector_loop.rs` · `selected` |
| `MECH_ORDERED_VECTOR_CARRY_POINTERS` | Profitability | OS presence / OS value interpreted at consumer | `program/codegen/ordered_reduction/vector_loop.rs` · `packed_enabled` / `program/codegen/ordered_reduction/vector_loop.rs` · `lanes` |
| `MECH_ORDERED_VECTOR_HOIST_COST_WORDS` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_ORDERED_VECTOR_LATCH_TEST` | Profitability | OS presence | `program/codegen/ordered_reduction/vector_loop.rs` · `lanes` / `program/codegen/ordered_reduction/vector_loop.rs` · `selected` |
| `MECH_PACK_PRIVATE_HALFWORD_READS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PACK_PRIVATE_HALFWORD_STORES` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PAIR_BIASED_APPENDS` | Profitability | OS presence | `program/codegen/append_run.rs` · `optimize_inner` / `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_PAIR_CHECKED_APPENDS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_PAIR_REBASE_LITERAL_COSTS` | Profitability | OS presence | `program/codegen/append_run.rs` · `optimize_inner` |
| `MECH_PHASE_ASSERTION_FLOORS` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `terminal_fallbacks` / `program/codegen/numeric_compact.rs` · `emit` / `program/codegen/numeric_phase_bounds.rs` · `derive` |
| `MECH_PHASE_INVARIANT_VIEWS` | Profitability | OS presence | `program/compact/phases.rs` · `define` / `program/compact/private_phase.rs` · `derive` |
| `MECH_PHASE_OWNER_CENSUS` | Diagnostic | OS presence | `program/compact/phases.rs` · `emit` |
| `MECH_PHASE_OWNER_MOVE_CENSUS` | Diagnostic | OS presence | `program/compact/phases.rs` · `summary` / `program/compact/phases.rs` · `emit` |
| `MECH_PHASE_OWNER_TRANSFER` | Profitability | OS presence | `program/compact/phases.rs` · `word_effects` / `program/compact/phases.rs` · `summary` / `program/compact/phases.rs` · `emit` |
| `MECH_PHASE_VECTOR_FAST` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_PREFETCH_PRIVATE_WRITES` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `emit` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PREFETCH_SECOND_PRIVATE_WRITE` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PRIVATE_INDEXED_CHAINS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_PRIVATE_NESTED_PHASES` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_PRIVATE_READ_INTERVALS` | Profitability | OS presence | `program/compact/private_chains.rs` · `derive` |
| `MECH_PRIVATE_VECTOR_PREDICATES` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_PRIVATE_VECTOR_PROVE_UNSIGNED_EXTREMA` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_PRIVATE_VECTOR_WIDTHS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_PROBE_WORD_BEFORE_VECTOR` | Profitability | OS presence | `program/codegen/wide_equal.rs` · `optimize_with_strategy` |
| `MECH_PROFILE_COMPILE_JOBS` | Diagnostic | OS presence | `program/compact/compile_jobs.rs` · `drop` |
| `MECH_PROVE_BOUNDED_UNSIGNED_INCREMENTS` | Profitability | OS presence | `program/codegen/cfg_cleanup/selected_increments/mod.rs` · `optimize` |
| `MECH_PROVE_PACKED_BASE_SPANS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_PROVE_SCALAR_SPANS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_QUALIFY_PHASE_INPUT_PACKET` | Profitability | OS presence or Unicode `only:` list | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_REDUCTION_COUNTDOWN` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_ADMITTED_BITWISE_INDEX_RANGES` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_ADMITTED_INDEX_RANGES` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` / `program/compact/phases.rs` · `define` |
| `MECH_REUSE_ADMITTED_INPUT_SPANS` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_REUSE_ADMITTED_INPUT_WINDOWS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_ADMITTED_INPUT_WORD_LENGTHS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_ADMITTED_VIEW_LENGTHS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_COMPARED_BYTES` | Profitability | OS equal to `"proved"` / OS presence | `program/codegen/numeric_compact.rs` · `emit` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_REUSE_COMPLEMENTARY_PREDICATES` | Profitability | OS presence | `program/codegen/cfg_cleanup/current_predicates/relations/mod.rs` · `fold` |
| `MECH_REUSE_DOMINATING_WINDOW_FACTS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_REUSE_EQUALITY_PREFIX` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_REUSE_SELECTED_READ_BOUND` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_REUSE_SHIFTED_EQUALITY` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `emit` |
| `MECH_REUSE_VECTOR_MISMATCH` | Profitability | OS presence | `program/codegen/wide_equal.rs` · `optimize_with_strategy` |
| `MECH_REVERSE_REDUCTION_CURSOR` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_SCHEDULE_INDEPENDENT_PRIVATE_READS` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `emit` / `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_SHARE_CLAMP_DECISIONS` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_SHARE_REPEATED_BOUNDS` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_SHIFT_EXTENSION_PROBE` | Diagnostic | OS value interpreted at consumer | `program/codegen/low_word_tests.rs` · `exercise` |
| `MECH_SHORT_CIRCUIT_COMPARISON_BRANCHES` | Profitability | OS presence | `program/compact/phase_function.rs` · `prepare_impl` |
| `MECH_SINGLETON_ORDERED_REDUCTION` | Profitability | OS presence | `program/codegen/ordered_reduction/emit.rs` · `emit` |
| `MECH_SPLIT_APPEND_DESCRIPTORS` | Profitability | OS value interpreted at consumer | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_TERMINAL_BYTE_APPENDS` | Profitability | OS presence | `program/compact/emit.rs` · `define_entry` |
| `MECH_THREAD_AFTER_GROWTH_ISOLATION` | Profitability | OS presence | `program/compact/phase_function.rs` · `optimize_phase` |
| `MECH_TRAVERSAL_FLOOR_DIAGNOSTIC` | Diagnostic | OS presence | `program/codegen/numeric_traversal.rs` · `analyze` / `program/compact/phase_function.rs` · `refused_general_ssa_unobserved` |
| `MECH_TRAVERSAL_WRITE_BOUNDS` | Profitability | OS presence | `program/codegen/numeric_compact.rs` · `emit` |
| `MECH_WHOLE_PHASE_CAPACITY` | Profitability | OS presence | `program/compact/phases.rs` · `define` |
| `MECH_WIDENING_REDUCTION` | Profitability | OS presence | `program/compact/widening_reduction.rs` · `derive` |
| `MECH_WIDE_ANY_BITS` | Profitability | OS presence | `program/codegen/wide_equal.rs` · `any_bits` |
