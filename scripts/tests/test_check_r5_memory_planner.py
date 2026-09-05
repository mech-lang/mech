from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r5-memory-planner.py"
SPEC = importlib.util.spec_from_file_location("check_r5_memory_planner", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class R5MemoryPlannerCheckerTests(unittest.TestCase):
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

    def test_01_serializable_plan_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/core/src/memory_plan/model.rs",
            "\n#[derive(Serialize)] struct WireMemoryPlan;\n",
        )
        self.assert_failure(root, "derives serialization")

    def test_02_bytecode_plan_field_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/program/bytecode/writer.rs",
            "pub register_count: u32,",
            "pub register_count: u32,\n    pub memory: ProgramMemoryPlan,",
        )
        self.assert_failure(root, "BytecodeProgram carries R5 plan field")

    def test_03_program_artifact_plan_field_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/artifact/model.rs",
            "pub struct ProgramArtifact {",
            "pub struct ProgramArtifact {\n    pub memory: ProgramMemoryPlan,",
        )
        self.assert_failure(root, "ProgramArtifact carries R5 plan field")

    def test_04_native_build_plan_field_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/build/src/plan/model.rs",
            "pub struct NativeBuildPlan {",
            "pub struct NativeBuildPlan {\n    pub memory: ProgramMemoryPlan,",
        )
        self.assert_failure(root, "NativeBuildPlan carries R5 plan field")

    def test_05_gpu_execution_plan_field_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "hosts/gpu/src/execution_plan.rs",
            "pub struct GpuExecutionPlan {",
            "pub struct GpuExecutionPlan {\n    pub memory: ProgramMemoryPlan,",
        )
        self.assert_failure(root, "GpuExecutionPlan carries R5 plan field")

    def test_06_bound_call_memory_policy_field_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/specialization.rs",
            "pub struct BoundCall {",
            "pub struct BoundCall {\n    allocation: AllocationPlan,",
        )
        self.assert_failure(root, "BoundCall carries forbidden memory field")

    def test_07_core_planner_runtime_dependency_fails(self):
        root = self.fixture()
        self.append(root, "src/core/src/memory_plan/model.rs", "\nuse crate::ValueCell;\n")
        self.assert_failure(root, "core memory planner imports or names ValueCell")

    def test_08_hash_map_planner_fails(self):
        root = self.fixture()
        self.append(root, "src/compute/src/memory.rs", "\nuse std::collections::HashMap;\n")
        self.assert_failure(root, "nondeterministic HashMap")

    def test_09_pointer_derived_plan_identity_fails(self):
        root = self.fixture()
        self.append(root, "src/engine/src/memory_planner/program.rs", "\nfn bad() { as_ptr(); }\n")
        self.assert_failure(root, "pointer or cell identity")

    def test_10_factory_name_policy_fails(self):
        root = self.fixture()
        self.append(root, "src/engine/src/memory_planner/program.rs", "\nfn bad() { factory_name(); }\n")
        self.assert_failure(root, "runtime factory name")

    def test_11_open_implementation_memory_class_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_plan/implementation.rs",
            "pub enum ImplementationMemoryClass {",
            "pub enum ImplementationMemoryClass {\n    Unknown,",
        )
        self.assert_failure(root, "forbidden escape hatch Unknown")

    def test_12_factory_without_memory_class_fails(self):
        root = self.fixture()
        self.write(
            root,
            "machines/r5_probe.rs",
            "struct Bad; impl MechFunctionFactory for Bad { fn new() {} }\n",
        )
        self.assert_failure(root, "MechFunctionFactory omits implementation_memory_class")

    def test_13_specialized_function_without_call_plan_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/specialization.rs",
            "memory_plan: CallMemoryPlan,",
            "memory_plan: (),",
        )
        self.assert_failure(root, "SpecializedFunction omits CallMemoryPlan")

    def test_14_compiler_binding_without_sidecar_fails(self):
        root = self.fixture()
        path = "src/core/src/program/compiler/context.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.write(
            root,
            path,
            source.replace(
                ".push(self.current_node_memory_plan.clone())",
                ".push(None)",
            ),
        )
        self.assert_failure(root, "compiler executable binding omits memory sidecar")

    def test_15_direct_resident_arena_sizing_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/resident/general/mod.rs",
            "\nfn bad() { TypedResidentArena::allocate(ResidentArenaSizes::default()); }\n",
        )
        self.assert_failure(root, "resident arena sizing bypasses R5 plan")

    def test_16_unplanned_gpu_instruction_expansion_fails(self):
        root = self.fixture()
        path = "hosts/gpu/src/batched/mod.rs"
        source = (root / path).read_text(encoding="utf-8")
        self.write(
            root,
            path,
            source.replace("plan_scalar_instruction_expansion", "unchecked_scalar_expansion"),
        )
        self.assert_failure(root, "without expansion planning")

    def test_17_hardcoded_gpu_adapter_limit_fails(self):
        root = self.fixture()
        self.append(
            root,
            "hosts/gpu/src/memory.rs",
            "\nfn bad() { let _ = GpuMemoryLimits { max_buffer_size: 4096 }; }\n",
        )
        self.assert_failure(root, "hardcodes an adapter buffer limit")

    def test_18_second_resident_estimate_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/budget.rs",
            "demand: ResourceDemand,",
            "demand: ResourceDemand,\n    bytes: u64,",
        )
        self.assert_failure(root, "second Resident resource authority")

    def test_19_r6_allocator_concept_fails(self):
        root = self.fixture()
        self.append(root, "src/compute/src/memory.rs", "\nstruct AllocationHandle;\n")
        self.assert_failure(root, "R6 concept introduced during R5")

    def test_20_package_version_change_fails(self):
        root = self.fixture()
        self.replace(root, "Cargo.toml", 'version = "0.3.6"', 'version = "0.4.0"')
        self.assert_failure(root, "root package version changed")

    def test_21_incomplete_r5_status_fails(self):
        root = self.fixture()
        for relative in (
            "README.md",
            "docs/design/ROADMAP.mec",
            "docs/design/v0.4-endgame.md",
            "docs/design/type-memory-boundary.md",
            "docs/design/r4-type-system-cutover.md",
        ):
            path = root / relative
            source = path.read_text(encoding="utf-8")
            self.write(root, relative, source.replace("R5 Memory planner — complete", "R5 Memory planner — incomplete"))
        self.assert_failure(root, "does not mark R5 complete")

    def test_22_c64_layout_alias_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_plan/derive.rs",
            "Complex(FloatWidth::W64) => layouts.c64_slot,",
            "Complex(FloatWidth::W64) => return Err(MemoryPlanError::UnsupportedStorageLayout),",
        )
        self.assert_failure(root, "conflate C32 and C64")

    def test_23_program_identity_remap_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/program.rs",
            "fn remap_transaction(",
            "fn unchecked_transaction(",
        )
        self.assert_failure(root, "global identity/liveness closure")

    def test_24_turn_budget_recheck_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/turn.rs",
            "plan.budget_limits,",
            "Default::default(),",
        )
        self.assert_failure(root, "deferred budget closure")

    def test_incremental_progress_cannot_replace_target_limits(self):
        root = self.fixture()
        path = "src/engine/src/memory_planner/turn.rs"
        source = (root / path).read_text(encoding="utf-8")
        start = source.index("pub(crate) fn check_turn_planning_progress(")
        prefix, body = source[:start], source[start:]
        self.assertIn("plan.budget_limits,", body)
        self.write(root, path, prefix + body.replace("plan.budget_limits,", "Default::default(),", 1))
        self.assert_failure(root, "deferred budget closure")

    def test_25_resident_observation_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            "fn resident_value_observed_footprint",
            "fn copied_planned_footprint",
        )
        self.assert_failure(root, "copies planned footprints")

    def test_26_resident_payload_finalization_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            "fn finalize_resident_backing_footprints",
            "fn skip_resident_backing_footprints",
        )
        self.assert_failure(root, "live footprint lifecycle")

    def test_27_resident_call_identity_attachment_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/general/mod.rs",
            "attach_resident_call_memory_template",
            "attach_raw_resident_calls",
        )
        self.assert_failure(root, "bypass global identity")

    def test_28_deferred_call_demand_resolution_fails(self):
        root = self.fixture()
        path = "src/engine/src/memory_planner/turn.rs"
        source = (root / path).read_text().replace("resolve_deferred_call_memory", "lost_deferred_resolution")
        self.write(root, path, source)
        self.assert_failure(root, "turn planning omits deferred budget closure: resolve_deferred_call_memory")

    def test_29_turn_replacement_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/turn.rs",
            "place_allocations(&mut allocations)?",
            "Box::new([])",
        )
        self.assert_failure(root, "deferred budget closure")

    def test_30_zero_inner_product_admission_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "hosts/gpu/src/batched/mod.rs",
            "sum_products_work(left.columns)?",
            "left.columns",
        )
        self.assert_failure(root, "complete pre-allocation work")

    def test_31_external_contract_relabel_without_replan_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/artifact/compiler.rs",
            "replan_call_memory(memory_plan, binding)",
            "keep_stale_memory_plan(memory_plan, binding)",
        )
        self.assert_failure(root, "stale call memory plan")

    def test_32_published_outputs_collapsed_into_state_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/program.rs",
            "SlotRole::Output => PlannedValueClass::PublishedOutput,",
            "SlotRole::Output => PlannedValueClass::State,",
        )
        self.assert_failure(root, "collapses published outputs into state")

    def test_33_mixed_compute_single_target_projection_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/compute/src/memory.rs",
            "instantiate_program_memory_plan_with_target_overrides",
            "instantiate_program_memory_plan",
        )
        self.replace(
            root,
            "src/compute/src/memory.rs",
            "instantiate_program_memory_plan_with_target_overrides",
            "instantiate_program_memory_plan",
        )
        self.assert_failure(root, "mixed compute uses one target profile")

    def test_34_gpu_semantic_empty_program_plan_fails(self):
        root = self.fixture()
        self.append(
            root,
            "hosts/gpu/src/memory.rs",
            "\nfn stale() { let _ = ProgramMemoryPlan { values: Box::new([]) }; }\n",
        )
        self.assert_failure(root, "masquerades as a semantic ProgramMemoryPlan")

    def test_35_resident_synthetic_turn_plan_fails(self):
        root = self.fixture()
        self.append(
            root,
            "src/engine/src/resident/budget.rs",
            "\nfn stale() { let _ = TurnMemoryPlan { node: NodeId::new(0) }; }\n",
        )
        self.assert_failure(root, "manufactures a synthetic TurnMemoryPlan")

    def test_36_call_and_value_transaction_stage_duplication_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/program.rs",
            "fn transaction_stage_object(",
            "fn duplicate_transaction_stage(",
        )
        self.assert_failure(root, "global identity/liveness closure")

    def test_37_external_replan_cannot_lose_provisional_output_regions(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/memory_plan/derive.rs",
            "output_regions: request.regions.into(),",
            "forgotten_regions: request.regions.into(),",
        )
        self.assert_failure(root, "output_regions")

    def test_38_rmw_call_transaction_stage_cannot_be_dropped(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/memory_planner/turn.rs",
            ".chain(&call_transactions)",
            ".chain(&[])",
        )
        self.assert_failure(root, "loses remapped call transaction stages")

    def test_39_resident_binder_cannot_manufacture_execution_permit(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/numeric.rs",
            "return bound(\n            transpose_dense,",
            "admit_dense_transpose_layout(request.output.kind, request.output.shape)?;\n        return bound(\n            transpose_dense,",
        )
        self.assert_failure(root, "manufactures an execution permit")


    def test_41_scratch_must_have_physical_records(self):
        root = self.fixture()
        path = "src/core/src/memory_plan/derive.rs"
        source = (root / path).read_text().replace("derive_scratch_allocations(", "lost_scratch(")
        self.write(root, path, source)
        self.assert_failure(root, "implementation scratch has no physical allocation records")

    def test_42_aggregate_budget_evaluation_is_mandatory(self):
        root = self.fixture()
        path = "src/engine/src/memory_planner/program.rs"
        source = (root / path).read_text().replace("aggregate_budget_violations(", "lost_aggregate(")
        self.write(root, path, source)
        self.assert_failure(root, "omit aggregate budget evaluation")

    def test_43_stale_resident_turn_measurement_is_forbidden(self):
        root = self.fixture()
        self.append(root, "src/engine/src/memory_planner/turn.rs", "\nfn current_port_footprint() {}\n")
        self.assert_failure(root, "reconstructed from activation estimates")

    def test_44_live_measurements_must_keep_published_values(self):
        root = self.fixture()
        self.replace(root, "src/engine/src/resident/general/live.rs", "published_footprints", "lost_published")
        self.assert_failure(root, "live Resident measurement is incomplete: published_footprints")


if __name__ == "__main__":
    unittest.main()
