from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r6-memory-runtime.py"
SPEC = importlib.util.spec_from_file_location("check_r6_memory_runtime", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class R6MemoryRuntimeCheckerTests(unittest.TestCase):
    def fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in CHECKER.REQUIRED:
            source = REPOSITORY / relative
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        return root

    @staticmethod
    def write(root: Path, relative: str, source: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")

    def append(self, root: Path, relative: str, source: str) -> None:
        path = root / relative
        self.write(root, relative, path.read_text(encoding="utf-8") + source)

    def replace(self, root: Path, relative: str, old: str, new: str) -> None:
        path = root / relative
        source = path.read_text(encoding="utf-8")
        self.assertIn(old, source)
        self.write(root, relative, source.replace(old, new, 1))

    def assert_failure(self, root: Path, expected: str) -> None:
        found = CHECKER.failures(root)
        self.assertTrue(any(expected in item for item in found), found)

    def test_00_repository_fixture_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_01_runtime_receipt_serialization_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "\n#[derive(Serialize)] struct SerializedMemoryDomain;\n",
        )
        self.assert_failure(root, "runtime ownership derives serialization")

    def test_02_wire_runtime_handle_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/program/bytecode/writer.rs",
            "pub register_count: u32,",
            "pub register_count: u32,\n    pub allocation: AllocationHandle,",
        )
        self.assert_failure(root, "BytecodeProgram serializes runtime receipt")

    def test_03_bound_call_runtime_handle_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/specialization.rs",
            "pub struct BoundCall {",
            "pub struct BoundCall {\n    runtime: AllocationHandle,",
        )
        self.assert_failure(root, "BoundCall carries forbidden R6 runtime field")

    def test_04_realization_reservation_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "pub fn prepare_realization(",
            "pub fn unchecked_realization(",
        )
        self.assert_failure(root, "missing required operation prepare_realization")

    def test_05_plan_revision_validation_cannot_be_removed(self):
        root = self.fixture()
        for relative in (
            "src/core/src/memory_runtime/domain.rs",
            "src/core/src/memory_runtime/access.rs",
        ):
            path = root / relative
            self.write(
                root,
                relative,
                path.read_text(encoding="utf-8").replace(
                    "InvalidPlanRevision", "IgnoredPlanRevision"
                ),
            )
        self.assert_failure(root, "omits InvalidPlanRevision validation")

    def test_06_publication_binding_cannot_be_removed(self):
        root = self.fixture()
        path = "src/core/src/memory_runtime/transaction.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.write(root, path, source.replace("binding", "placement"))
        self.assert_failure(root, "publication lifecycle omits binding")

    def test_07_managed_cell_storage_cannot_be_removed(self):
        root = self.fixture()
        path = "src/core/src/cell_binding.rs"
        source = (root / path).read_text(encoding="utf-8")
        source = source.replace("ManagedHost {", "PinnedHost {")
        source = source.replace("ManagedCanonical {", "PinnedCanonical {")
        self.write(root, path, source)
        self.assert_failure(root, "ValueCell storage is not closed")

    def test_07b_function_instance_managed_authority_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "managed: ManagedFunctionBinding,",
            "managed: RemovedManagedFunctionBinding,",
        )
        self.assert_failure(root, "FunctionInstance has no managed-domain execution authority")

    def test_08_planned_cell_allocation_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/cell_binding.rs",
            "fn allocate_planned(",
            "fn allocate_unplanned(",
        )
        self.assert_failure(root, "lacks reservation-backed allocate_planned")

    def test_09_managed_function_entry_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "fn solve_managed(",
            "fn solve_unmanaged(",
        )
        self.assert_failure(root, "does not require solve_managed")

    def test_09b_missing_payload_witness_cannot_fall_back_to_published_output(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "output_policy == PayloadOutputPlanPolicy::Missing",
            "false",
        )
        self.assert_failure(root, "missing payload witness silently reuses")

    def test_09c_set_builder_cannot_allocate_then_adopt(self):
        root = self.fixture()
        self.replace(
            root,
            "machines/set/src/canonical.rs",
            "with_admitted_canonical_output",
            "stage_output_value",
        )
        self.assert_failure(root, "set construction bypasses prospective payload admission")

    def test_10_raw_function_solve_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "pub trait MechFunctionImpl {",
            "pub trait MechFunctionImpl {\n    fn solve_result(&self) -> MResult<()>;",
        )
        self.assert_failure(root, "retains an unmanaged solve entry")

    def test_11_unsafe_policy_code_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/memory_runtime/transaction.rs",
            "\nfn bypass() { unsafe { core::hint::unreachable_unchecked() } }\n",
        )
        self.assert_failure(root, "unsafe escapes the sealed allocation/access boundary")

    def test_12_production_legacy_fallback_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/memory_runtime/realize.rs",
            "\nfn legacy_unmanaged() {}\n",
        )
        self.assert_failure(root, "unmanaged-memory bypass")

    def test_13_deferred_capacity_acceptance_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/memory_runtime/realize.rs",
            "\nfn accept() { let _ = CapacityDeferredToR6; }\n",
        )
        self.assert_failure(root, "accepts CapacityDeferredToR6")

    def test_14_production_bypass_flag_fails(self):
        root = self.fixture()
        self.append(
            root,
            "hosts/gpu/src/memory.rs",
            "\n#[cfg(feature = \"disable_managed_memory\")] fn bypass() {}\n",
        )
        self.assert_failure(root, "unmanaged-memory bypass")

    def test_15_resident_parallel_arena_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            "Managed(PlannedArenaProjection<T>),",
            "Managed(Box<[T]>),",
        )
        self.assert_failure(root, "do not project their realized R5 host arenas")

    def test_16_resident_realization_owner_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            "_managed_memory: ManagedProgramMemory,",
            "_managed_memory: RemovedManagedProgramMemory,",
        )
        self.assert_failure(root, "does not retain its managed program realization")

    def test_17_resident_state_clone_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/resident/general/mod.rs",
            "\nfn bypass(instance: &ReactiveInstance) { let _ = instance.state.clone(); }\n",
        )
        self.assert_failure(root, "state migration clones an unplanned arena")

    def test_18_arena_allocator_cannot_be_public(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/allocation.rs",
            "struct PlannedHostArenaAllocator {",
            "pub struct PlannedHostArenaAllocator {",
        )
        self.assert_failure(root, "allocator escapes its sealed projection boundary")

    def test_19_gpu_accounting_only_attachment_cannot_be_production(self):
        root = self.fixture()
        self.replace(
            root,
            "hosts/gpu/src/memory.rs",
            "#[cfg(test)]\n    pub(crate) fn attach_device_allocation(",
            "pub(crate) fn attach_device_allocation(",
        )
        self.assert_failure(root, "attachment is available in production")

    def test_20_gpu_post_submit_registration_fails(self):
        root = self.fixture()
        self.append(
            root,
            "hosts/gpu/src/native.rs",
            "\nfn after_submit(tracker: &mut Tracker, hold: Hold) { tracker.track(hold); }\n",
        )
        self.assert_failure(root, "register ownership after backend submission")

    def test_21_cleanup_inside_debug_assert_fails(self):
        root = self.fixture()
        self.append(
            root,
            "hosts/gpu/src/native.rs",
            "\nfn drop_only_in_debug(owner: &mut Owner) { debug_assert!(owner.release().is_ok()); }\n",
        )
        self.assert_failure(root, "cleanup is hidden inside debug_assert")

    def test_22_browser_queue_completion_cannot_be_removed(self):
        root = self.fixture()
        path = "include/browser-compute.js"
        source = (root / path).read_text(encoding="utf-8")
        self.write(root, path, source.replace("queue.onSubmittedWorkDone()", "queue.workWasQueued()"))
        self.assert_failure(root, "not retained through queue completion")

    def test_23_browser_mapping_cleanup_must_settle_every_sibling(self):
        root = self.fixture()
        path = "include/browser-compute.js"
        source = (root / path).read_text(encoding="utf-8")
        self.write(root, path, source.replace("Promise.allSettled", "Promise.all"))
        self.assert_failure(root, "cleanup can skip siblings")

    def test_24_runtime_factory_cannot_capture_physical_backing(self):
        root = self.fixture()
        self.write(
            root,
            "src/engine/src/intrinsics/constructors.rs",
            """
fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
    let _output = invocation.output().try_ref()?;
    unreachable!()
}
""",
        )
        self.assert_failure(root, "runtime factory retains physical input/output backing")

    def test_25_executor_owned_scope_entry_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "pub fn solve_in_scope(",
            "pub fn solve_outside_scope(",
        )
        self.assert_failure(root, "lacks its executor-owned managed-scope entry")

    def test_26_user_function_cannot_restore_nested_standalone_execution(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/function/mod.rs",
            "\nimpl MechFunctionImpl for UserFunction { fn solve_managed(&self, _: &mut KernelMemoryFrame<'_>, _: &mut dyn MechExecutionServices) -> MResult<ReactiveSolveStatus> { unreachable!() } }\n",
        )
        self.assert_failure(root, "retains a nested standalone execution wrapper")

    def test_27_indirect_output_cannot_leave_the_invocation_session(self):
        root = self.fixture()
        path = "src/core/src/function/specialization.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.assertIn("ValueCell::allocate_for_descriptor_in", source)
        self.write(
            root,
            path,
            source.replace(
                "ValueCell::allocate_for_descriptor_in",
                "ValueCell::allocate_for_descriptor",
            ),
        )
        self.assert_failure(root, "outputs can escape the invocation memory session")

    def test_28_interpreter_program_session_cannot_be_removed(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/interpreter/mod.rs",
            "memory_domain: MemoryDomain,",
            "removed_memory_domain: MemoryDomain,",
        )
        self.assert_failure(root, "does not own one ordinary program memory session")

    def test_29_source_literals_cannot_create_per_value_sessions(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/literals.rs",
            ".import_owned_in(p.memory_domain())",
            ".import_owned_in(&MemoryDomain::new().unwrap())",
        )
        self.assert_failure(root, "literals do not enter the interpreter memory session")

    def test_30_concatenation_marker_cannot_own_physical_backing(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/intrinsics/horzcat.rs",
            "\nstruct RestoredLegacy<T> { input: Ref<T> }\n",
        )
        self.assert_failure(root, "factory marker owns legacy physical backing")

    def test_31_concatenation_marker_cannot_restore_parallel_execution(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/intrinsics/vertcat.rs",
            "\nimpl<T> MechFunctionImpl for RestoredLegacy<T> {}\n",
        )
        self.assert_failure(root, "factory marker restores a parallel executor")

    def test_32_variable_definition_marker_cannot_own_physical_backing(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/intrinsics/define.rs",
            "\nstruct VariableDefineLegacy<T> { value: Ref<T> }\n",
        )
        self.assert_failure(root, "variable-definition factory marker owns legacy physical backing")

    def test_33_variable_definition_marker_cannot_restore_parallel_execution(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/intrinsics/define.rs",
            "\nimpl<T> MechFunctionImpl for VariableDefineLegacy<T> {}\n",
        )
        self.assert_failure(root, "variable-definition factory marker restores a parallel executor")

    def test_34_function_port_cannot_restore_physical_extraction(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/function/argument.rs",
            "\npub fn try_ref<T>() {}\n",
        )
        self.assert_failure(root, "function ports expose legacy physical extractor try_ref")

    def test_35_specialization_cannot_restore_physical_cell_construction(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/function/specialization.rs",
            "\npub fn typed_cell<T>() {}\n",
        )
        self.assert_failure(root, "source specialization exposes legacy physical extractor typed_cell")

    def test_36_bytecode_constants_cannot_restore_pinned_external_backing(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/program/bytecode/constants/canonical.rs",
            "\nfn restore_pinned_constant() { ValueCell::from_ref(); }\n",
        )
        self.assert_failure(root, "bytecode reconstruction installs pinned-external")

    def test_37_bytecode_constants_must_share_a_managed_session(self):
        root = self.fixture()
        path = "src/core/src/program/bytecode/constants/canonical.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.assertIn("MemoryDomain::new()", source)
        self.write(root, path, source.replace("MemoryDomain::new()", "removed_domain()"))
        self.assert_failure(root, "decoded constants do not share a managed bytecode session")

    def test_38_concatenation_marker_cannot_restore_element_copy_body(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/intrinsics/vertcat.rs",
            "\nmacro_rules! vertcat_v2v2 { () => {}; }\n",
        )
        self.assert_failure(root, "concatenation marker retains a legacy element-copy body")

    def test_39_gpu_write_accounting_cannot_collapse_double_buffers(self):
        root = self.fixture()
        self.replace(
            root,
            "hosts/gpu/src/memory.rs",
            "writable_state_objects: [Box<[(MemoryObjectId, u64)]>; 2],",
            "removed_state_objects: [Box<[(MemoryObjectId, u64)]>; 2],",
        )
        self.assert_failure(root, "does not distinguish double-buffer bind groups")

    def test_40_gpu_backend_must_report_the_submitted_state_group(self):
        root = self.fixture()
        path = "hosts/gpu/src/native.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.assertIn(".writable_state_objects(group)", source)
        self.write(
            root,
            path,
            source.replace(".writable_state_objects(group)", ".writable_device_objects()"),
        )
        self.assert_failure(root, "completed writes ignore the submitted state bind group")

    def test_41_canonical_cell_must_retain_its_payload_plan(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/cell_binding.rs",
            "    payload: crate::PlanObjectKey,",
            "    removed_payload: u64,",
        )
        self.assert_failure(root, "do not retain header and payload plan ownership")

    def test_42_canonical_stage_must_admit_the_frozen_payload(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "admit_frozen_snapshot",
            "accept_unplanned_snapshot",
        )
        self.assert_failure(root, "staging bypasses its admitted payload owner")

    def test_43_missing_transaction_cannot_authorize_an_output_write(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "plan.transactions.len() != plan.outputs.len()",
            "false",
        )
        self.assert_failure(root, "missing transaction as write authority")

    def test_44_projection_must_conflict_with_frame_leases(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "record.arena_projection_owner.upgrade().is_some()",
            "false",
        )
        self.assert_failure(root, "projections and frame leases have separate authorities")

    def test_45_continuous_reuse_owner_must_keep_initialization(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "if *active != Some(key) {",
            "if true {",
        )
        self.assert_failure(root, "does not preserve initialization")

    def test_46_ready_publication_must_not_reborrow_published_shape(self):
        root = self.fixture()
        path = "src/core/src/cell_binding.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.write(root, path, source.replace("publication_shape", "removed_shape"))
        self.assert_failure(root, "conflict-free shape authority")

    def test_47_canonical_builder_must_admit_before_construction(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "allocator.prepare_frozen_snapshot",
            "allocator.accept_after_build",
        )
        self.assert_failure(root, "does not admit before building")

    def test_48_payload_calls_must_refresh_same_shape_footprints(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "resolve_current_call_memory",
            "reuse_shape_only_plan",
        )
        self.assert_failure(root, "do not refresh live and prospective footprints")

    def test_49_snapshot_rebind_must_keep_frozen_ownership(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "return Ok(self.clone());",
            "return Ok(rebuild_without_owner());",
        )
        self.assert_failure(root, "do not preserve shared frozen ownership")

    def test_50_payload_nodes_use_declared_not_allocator_capacity(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/payload.rs",
            "block_capacity: usize,",
            "incidental_capacity: usize,",
        )
        self.assert_failure(root, "allocator spare capacity")

    def test_51_function_binding_validates_transaction_semantics(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "validate_transaction_authority(&plan)?;",
            "validate_transaction_arity(&plan)?;",
        )
        self.assert_failure(root, "does not validate transaction semantics")

    def test_52_undo_publication_retains_the_exclusive_lease(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "retained_lease: Option<RetainedPublicationLease>,",
            "dropped_lease: Option<RetainedPublicationLease>,",
        )
        self.assert_failure(root, "does not retain its exclusive lease")

    def test_53_repeated_in_place_roles_are_coalesced(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "workspace.leases[other].owns_lease = false;",
            "workspace.leases[other].owns_lease = true;",
        )
        self.assert_failure(root, "are not coalesced")

    def test_54_string_transpose_requires_prospective_admission(self):
        root = self.fixture()
        self.replace(
            root,
            "machines/matrix/src/transpose.rs",
            "with_admitted_canonical_output",
            "stage_unplanned_canonical_output",
        )
        self.assert_failure(root, "String matrix transpose bypasses prospective")

    def test_55_canonical_matrix_constructors_require_prospective_admission(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/intrinsics/constructors.rs",
            "with_admitted_canonical_output",
            "stage_unplanned_canonical_output",
        )
        self.assert_failure(root, "canonical matrix constructors bypass prospective")

    def test_56_managed_host_footprint_cannot_snapshot_payload(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/cell_binding.rs",
            "if let Some(managed) = storage.as_any().downcast_ref::<ManagedHostCellStorage>()",
            "if false",
        )
        self.assert_failure(root, "footprint measurement materializes a semantic snapshot")

    def test_57_ordinary_canonical_snapshot_must_share_frozen_root(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/cell_binding.rs",
            "return Ok(self.value.clone());",
            "return self.value.rebuild();",
        )
        self.assert_failure(root, "ordinary managed canonical snapshots rebuild")

    def test_58_matrix_candidate_cannot_include_previous_output(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/intrinsics/constructors.rs",
            "let mut footprint = CurrentMemoryFootprint {",
            "let mut footprint = output.current_memory_footprint()?; /* CurrentMemoryFootprint { */",
        )
        self.assert_failure(root, "matrix candidate footprint includes the previous")

    def test_59_canonical_access_cannot_materialize_selector_plan(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/intrinsics/access/mod.rs",
            "prospective_repeated_sequence_memory_footprint",
            "canonical_indices",
        )
        self.assert_failure(root, "canonical access materializes selectors")

    def test_60_external_resource_result_must_be_captured_before_replanning(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/function/external/resource_read.rs",
            "fn capture_external_output(",
            "fn capture_after_execution(",
        )
        self.assert_failure(root, "external result is not captured once")

    def test_61_frozen_definition_requires_explicit_policy(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/intrinsics/define.rs",
            "PayloadOutputPlanPolicy::PublishedInvariant",
            "PayloadOutputPlanPolicy::Missing",
        )
        self.assert_failure(root, "frozen variable definitions lack explicit")

    def test_62_fixed_publication_cannot_snapshot_for_evidence(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "CellPublicationEvidence::initialized_region(shape.clone())",
            "snapshot_managed_host_data(&frame, object, self.output().representation())?",
        )
        self.assert_failure(root, "fixed-width publication constructs a canonical evidence copy")

    def test_63_published_invariant_cannot_receive_output_write_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "output_policy != PayloadOutputPlanPolicy::PublishedInvariant",
            "true",
        )
        self.assert_failure(root, "published-invariant functions receive writable")

    def test_64_canonical_builder_must_receive_construction_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "build(self, &mut construction)",
            "build(self)",
        )
        self.assert_failure(root, "canonical construction does not admit before building")

    def test_65_binary_canonical_builder_must_delegate_to_shared_path(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "self.with_admitted_canonical_output(",
            "self.with_separate_binary_admission(",
        )
        self.assert_failure(root, "canonical construction does not admit before building")

    def test_66_external_marshalling_must_precede_provider_capture(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "let arguments = preliminary.marshal_external_inputs(&self.managed_inputs)?;",
            "let arguments = unadmitted_external_inputs(&self.managed_inputs)?;",
        )
        self.assert_failure(root, "provider invocation precedes admitted call-scoped marshalling")

    def test_67_external_result_cannot_live_in_implementation_state(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/function/external/host_call.rs",
            "\nstruct StaleExternalState { prepared_result: Option<Value> }\n",
        )
        self.assert_failure(root, "external result is not captured once")

    def test_68_common_finalizer_must_consume_construction_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/cell_binding.rs",
            "SnapshotValidationContext::new(schemas).with_construction_authority(construction)",
            "SnapshotValidationContext::new(schemas)",
        )
        self.assert_failure(root, "common canonical finalization bypasses construction authority")

    def test_69_external_marshalling_must_use_its_finite_construction_token(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "input.snapshot_for_external_marshalling(&construction)?",
            "input.snapshot()?",
        )
        self.assert_failure(
            root,
            "external marshalling is not governed by canonical construction authority",
        )

    def test_70_live_binding_must_be_prepared_before_publication(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "prepare_external_publication(services)",
            "prepare_external_publication_after_commit(services)",
        )
        self.assert_failure(root, "external live binding is fallible after cell publication")

    def test_71_live_binding_token_allocation_must_be_fallible(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/execution.rs",
            "Box::try_new(commit)",
            "Box::new(commit)",
        )
        self.assert_failure(root, "external live binding is fallible after cell publication")

    def test_72_live_binding_install_must_follow_realization_promotion(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "                self.promote_prepared_realization(&mut prepared)?;\n                if prepared.external {",
            "                if prepared.external {",
        )
        self.assert_failure(root, "external live binding is fallible after cell publication")

    def test_73_finalizer_data_root_must_use_construction_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "let data = context.try_arc(FrozenSnapshotData {",
            "let data = Arc::new(FrozenSnapshotData {",
        )
        self.assert_failure(root, "common canonical finalization bypasses")

    def test_74_finalizer_storage_root_must_use_construction_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "let root = context.try_arc(FrozenSnapshotStorage {",
            "let root = Arc::new(FrozenSnapshotStorage {",
        )
        self.assert_failure(root, "common canonical finalization bypasses")

    def test_75_recursive_values_must_share_one_schema_owner(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "self.shared_schemas.get()",
            "None",
        )
        self.assert_failure(root, "clone their schema owner repeatedly")

    def test_76_scalar_id_matrices_must_use_packed_finalization(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "SchemaBody::Id => pack!(Id, Id)",
            "SchemaBody::Id => unreachable!()",
        )
        self.assert_failure(root, "scalar matrix or table finalization bypasses packed")

    def test_77_external_canonical_shape_clone_must_be_planned(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_plan/derive.rs",
            "external_canonical_shape_clone_bytes(input)?",
            "0",
        )
        self.assert_failure(root, "external canonical metadata cloning bypasses")

    def test_78_browser_smoke_must_observe_final_completion_before_pass(self):
        root = self.fixture()
        self.replace(
            root,
            "include/project.js",
            "await globalThis.MechBrowserCompute.awaitSmokeTargetCompletion(target);",
            "await globalThis.MechBrowserCompute.awaitSmokeTargetCompletion(target).catch(() => {});",
        )
        self.assert_failure(root, "browser compute smoke can pass before")

    def test_79_managed_solve_cannot_publish_through_a_logical_cell(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/structures.rs",
            """
impl MechFunctionImpl for Bypass {
    fn solve_managed(&self, _frame: &mut KernelMemoryFrame<'_>, _services: &mut dyn MechExecutionServices) -> MResult<ReactiveSolveStatus> {
        self.output.replace(&self.next_value()?)?;
        Ok(ReactiveSolveStatus::Changed)
    }
}
""",
        )
        self.assert_failure(root, "solve_managed bypasses frame-owned staged publication")

    def test_80_retired_region_metadata_must_be_collected(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            ".regions\n            .retain(|_, region| region.realization_owner.strong_count() != 0);",
            ".regions\n            .iter().for_each(|_| {});",
        )
        self.assert_failure(root, "retain historical region initialization metadata")

    def test_81_frozen_ticket_must_follow_shared_data(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/snapshot/validation.rs",
            "ownership: SharedOwnershipCell<crate::RetainedPayloadTicket>,",
            "ownership_removed: (),",
        )
        self.assert_failure(root, "accounting is not attached to shared immutable data")

    def test_82_conversion_cannot_restore_detached_runtime_cell(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/literals.rs",
            "construction.try_rebuild_data_draft(output, converted)",
            "execute_conversion_plan(source, target, plan)?.snapshot()",
        )
        self.assert_failure(root, "conversion escapes its frame-owned construction authority")

    def test_83_dynamic_module_cannot_restore_private_output_scratch(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/function/module.rs",
            "let changed = match state.kernel {",
            "let next = vec![0.0; candidate.len()];\n    let changed = match state.kernel {",
        )
        self.assert_failure(root, "module allocates private output-sized scratch")

    def test_84_conversion_footprint_must_include_target_payload(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/literals.rs",
            "conversion_string_payload_bound(&plan.step)",
            "None",
        )
        self.assert_failure(root, "conversion footprint ignores target payload expansion")

    def test_85_boolean_storage_cannot_regain_raw_byte_access(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "&& region.slot.is_none();",
            "&& matches!(region.slot, None | Some(crate::PlannedSlotKind::FixedScalar(_)));",
        )
        self.assert_failure(root, "typed Boolean storage can be exposed through raw byte access")

    def test_86_ordinary_promotion_must_collect_retired_realizations(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "fn promote_prepared_realization(",
            "fn promote_prepared_realization_without_collection(",
        )
        self.assert_failure(root, "ordinary cold-path replacement or promotion omits retired collection")

    def test_87_construction_workspace_cannot_regain_contiguous_backing(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_plan/model.rs",
            "ReservationOnlyWorkspace,",
            "",
        )
        self.assert_failure(root, "canonical construction workspace retains duplicate contiguous backing")

    def test_88_fixed_conversion_cannot_restore_snapshot_materialization(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/literals.rs",
            "frame.execute_fixed_conversion_plan(source, output, plan)",
            "frame.stage_output_value(output, output.snapshot()?)",
        )
        self.assert_failure(root, "conversion escapes its frame-owned construction authority")

    def test_89_late_publication_failure_must_collect_abandoned_realization(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "drop(prepared);\n                domain.collect_retired()",
            "drop(prepared);\n                Ok(0).map(|_| 0)",
        )
        self.assert_failure(root, "late publication preparation failure omits retired collection")

    def test_90_fixed_conversion_must_accept_canonical_c32_sources(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/access.rs",
            "K::C32 => canonical_c32_target!(),",
            "K::C32 => Err(crate::MechError::new(\n                crate::ConversionExecutionError::ConversionExecutionUnsupported,\n                None,\n            )\n            .with_compiler_loc()),",
        )
        self.assert_failure(root, "managed fixed conversion rejects canonical C32 sources")

    def test_91_unchanged_invariant_turn_cannot_collect_without_a_candidate(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "if abandoned_candidate {\n                        current.domain.collect_retired()",
            "if true {\n                        current.domain.collect_retired()",
        )
        self.assert_failure(
            root,
            "unchanged invariant turn invokes cold reclamation without a candidate",
        )

    def test_92_failed_register_batch_must_drop_candidates_before_collection(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "drop(prepared);\n    for domain in domains",
            "for domain in domains",
        )
        self.assert_failure(
            root,
            "failed register batch collects while staged candidates remain owned",
        )

    def test_93_construction_role_cannot_regain_physical_arena_backing(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "if construction != reservation_only {",
            "if false {",
        )
        self.assert_failure(root, "construction workspace role can enter a physical arena")

    def test_94_managed_join_cannot_rematerialize_inputs_before_admission(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/intrinsics/table_ops.rs",
            "CanonicalTable::from_borrowed(&self.lhs_fields, lhs)",
            "CanonicalTable::from_value(self.lhs.cell(), &lhs)",
        )
        self.assert_failure(
            root,
            "managed table join materializes input columns before construction admission",
        )

    def test_95_failed_register_preparation_must_collect_its_own_domain(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "collect_abandoned_function_publications(staged, &[failing_domain])",
            "collect_abandoned_function_publications(staged, &[])",
        )
        self.assert_failure(
            root,
            "failed register preparation omits its candidate domain",
        )

    def test_96_failed_register_domain_must_enter_collection_set(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/mod.rs",
            "            domains.push(domain.clone());\n",
            "",
        )
        self.assert_failure(
            root,
            "failed register preparation omits its candidate domain",
        )

    def test_97_resident_projection_must_validate_its_planned_slot(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "if !T::supports_planned_slot(slot) {",
            "if false {",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_98_resident_projection_must_use_direct_arena_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "realized.arena_bindings.get(&arena)",
            "realized.arena_bindings.values().next()",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_99_resident_projection_must_revoke_managed_initialization(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "            region.initialization.clear();\n",
            "",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_100_plan_member_validation_must_reserve_fallibly(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "            .try_reserve_exact(arena.members.len())",
            "            .reserve_exact(arena.members.len())",
        )
        self.assert_failure(
            root,
            "runtime plan member validation can allocate infallibly",
        )

    def test_101_resident_lane_cannot_use_one_member_as_arena_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            ".project_host_arena(memory.realized(), arena.id, len)",
            ".project_host_arena(memory.realized(), arena.members[0], len)",
        )
        self.assert_failure(
            root,
            "Resident lanes use one member as whole-arena projection authority",
        )

    def test_102_resident_effect_payload_must_retain_its_typed_slot(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/resident.rs",
            "slot: Some(resident_planned_slot(kind)),",
            "slot: None,",
        )
        self.assert_failure(
            root,
            "Resident effect payload omits its typed arena slot",
        )

    def test_103_resident_projection_must_require_one_exact_slot_kind(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "Some(expected) if expected != slot =>",
            "Some(expected) if T::supports_planned_slot(expected) && false =>",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_104_projection_layout_validation_must_precede_revocation(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            """            PlannedArenaProjection::<T>::validate_realized_layout(
                block_bytes,
                block_alignment,
                len,
            )?;
            record.arena_projection_owner = Rc::downgrade(&projection_owner);""",
            """            record.arena_projection_owner = Rc::downgrade(&projection_owner);
            PlannedArenaProjection::<T>::validate_realized_layout(
                block_bytes,
                block_alignment,
                len,
            )?;""",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_105_empty_arena_members_must_retain_slot_authority(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "                        arena: object.arena,\n",
            "",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_106_projection_cannot_filter_members_by_physical_handle(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "            for (key, binding) in bindings.iter() {\n                let region = state",
            """            for (key, binding) in bindings.iter() {
                if binding
                    .handle()
                    != Some(handle)
                {
                    continue;
                }
                let region = state""",
        )
        self.assert_failure(
            root,
            "resident arena projection is not bound to its complete planned slot",
        )

    def test_107_plan_membership_must_be_one_to_one(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            """    if let Some(duplicate) = member_placements
        .windows(2)
        .find(|pair| pair[0].0 == pair[1].0)""",
            """    if let Some(duplicate) = member_placements
        .windows(2)
        .find(|_| false)""",
        )
        self.assert_failure(
            root,
            "runtime plan member authority is not one-to-one",
        )

    def test_108_flat_member_index_must_reserve_complete_capacity(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            ".try_reserve_exact(member_count)",
            ".try_reserve_exact(0)",
        )
        self.assert_failure(
            root,
            "runtime plan member authority is not one-to-one",
        )

    def test_109_declared_arena_comparison_cannot_be_disabled(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "if member_arena != Some(arena.id) {",
            "if false && member_arena != Some(arena.id) {",
        )
        self.assert_failure(
            root,
            "runtime plan member authority is not one-to-one",
        )

    def test_110_flat_member_index_must_be_sorted_before_validation(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_runtime/domain.rs",
            "    member_placements.sort_unstable();\n",
            "",
        )
        self.assert_failure(
            root,
            "runtime plan member authority is not one-to-one",
        )


if __name__ == "__main__":
    unittest.main()
