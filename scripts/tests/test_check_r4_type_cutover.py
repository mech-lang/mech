from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r4-type-cutover.py"
SPEC = importlib.util.spec_from_file_location("check_r4_type_cutover", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class R4TypeCutoverCheckerTests(unittest.TestCase):
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

    def assert_failure(self, root: Path, expected: str) -> None:
        found = CHECKER.failures(root)
        self.assertTrue(any(expected in failure for failure in found), found)

    def test_repository_fixture_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_missing_authority_file_fails(self):
        root = self.fixture()
        (root / "src/core/src/type_system/resolved_value.rs").unlink()
        self.assert_failure(root, "required file is missing")

    def test_retired_factory_probe_fails(self):
        root = self.fixture()
        path = root / "src/core/src/function/specialization.rs"
        path.write_text(path.read_text() + "\nfn runtime_output_rank() {}\n")
        self.assert_failure(root, "retired R4 path remains")


    def test_maintained_compiler_resolved_runtime_fails(self):
        root = self.fixture()
        path = root / "machines/math/src/r4_probe.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fn bad() { CompilerResolved(RuntimeFamilyId::from_raw(1)); }\n")
        self.assert_failure(root, "maintained runtime uses CompilerResolved")


    def test_fallback_operation_contract_fails(self):
        root = self.fixture()
        path = root / "src/core/src/function/catalog.rs"
        path.write_text(path.read_text() + "\nfn fallback_operation_memory_contract() {}\n")
        self.assert_failure(root, "retired R4 path remains")

    def test_postconstruction_operation_contract_fails(self):
        root = self.fixture()
        path = root / "src/core/src/function/catalog.rs"
        path.write_text(
            path.read_text()
            + "\nfn bad() { RuntimeOperationContractAuthority::ImplementationDeclared; }\n"
        )
        self.assert_failure(root, "retired R4 path remains")

    def test_fabricated_resident_runtime_id_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        path.write_text(
            path.read_text() + "\nfn bad() { RuntimeFunctionId::from_name(&canonical_operation); }\n"
        )
        self.assert_failure(root, "retired R4 path remains")

    def test_optional_executable_binding_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/artifact/compiler.rs"
        path.write_text(
            path.read_text()
            + "\nfn bad(binding: Option<BoundCall>) { let Some(binding) = binding else { continue; }; }\n"
        )
        self.assert_failure(root, "type bindings as optional")

    def test_native_semantic_validation_omission_fails(self):
        root = self.fixture()
        path = root / "src/build/src/analysis/bytecode.rs"
        path.write_text(
            path.read_text().replace(
                "validate_bound_call_for_target(binding, ExecutionTarget::Native)",
                "runtime_entry(binding.runtime_function().unwrap())",
                1,
            )
        )
        self.assert_failure(root, "native planning does not consume")

    def test_independent_compiler_semantic_sidecar_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/program/compiler_planning.rs"
        path.write_text(path.read_text() + "\nfn bad(step: &Step) { step.semantic_operation_name(); }\n")
        self.assert_failure(root, "semantic sidecars independently")

    def test_independent_resident_semantic_sidecar_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        path.write_text(path.read_text() + "\nfn bad(step: &Step) { step.semantic_operation_name(); }\n")
        self.assert_failure(root, "semantic sidecars independently")

    def test_resident_binding_must_use_the_actual_artifact_operation(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        self.assertIn("BoundCall::artifact_operation(", source)
        path.write_text(source.replace("BoundCall::artifact_operation(", "unchecked_call(", 1))
        self.assert_failure(root, "resident activation does not bind the actual artifact operation")

    def test_resident_binding_must_use_the_actual_semantic_contract(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        self.assertIn("ResolvedOperationDescriptor::from_resolved_contract(", source)
        path.write_text(source.replace("ResolvedOperationDescriptor::from_resolved_contract(", "ResolvedOperationDescriptor::from_name(", 1))
        self.assert_failure(root, "semantic descriptors from the actual artifact contract and layouts")

    def test_resident_binding_must_use_actual_output_shape(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        body = CHECKER.rust_function_body(source, "bind_resident_operation")
        self.assertIn("output_layout.shape_instance.clone()", body)
        path.write_text(source.replace(body, body.replace("output_layout.shape_instance.clone()", "ShapeInstance::scalar()", 1), 1))
        self.assert_failure(root, "semantic descriptors from the actual artifact contract and layouts")

    def test_resident_memory_plan_must_consume_selected_bound_call(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        body = CHECKER.rust_function_body(source, "bind_resident_operation")
        self.assertIn("&resident_context.bound_call,", body)
        path.write_text(source.replace(body, body.replace("&resident_context.bound_call,", "&unrelated_bound_call,", 1), 1))
        self.assert_failure(root, "call-memory planning does not consume the selected BoundCall")

    def test_resident_kernel_must_retain_selected_bound_call(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        self.assertIn(".with_bound_call(resident_context.bound_call)", source)
        path.write_text(source.replace(".with_bound_call(resident_context.bound_call)", ".with_unchecked_metadata()", 1))
        self.assert_failure(root, "resident executable kernel does not retain its selected BoundCall")

    def test_selected_kernel_call_is_not_merely_a_sidecar_argument(self):
        root = self.fixture()
        path = root / "src/core/src/function/resident.rs"
        source = path.read_text()
        self.assertIn("self.bound_call = Some(bound_call)", source)
        path.write_text(source.replace("self.bound_call = Some(bound_call)", "drop(bound_call)", 1))
        self.assert_failure(root, "retain and expose the selected semantic BoundCall")

    def test_executable_node_must_retain_bound_kernel(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/general/mod.rs"
        source = path.read_text()
        self.assertIn("pub kernel: BoundResidentKernel", source)
        path.write_text(source.replace("pub kernel: BoundResidentKernel", "pub kernel: UnboundKernel", 1))
        self.assert_failure(root, "executable node does not retain the bound kernel")

    def test_implementation_contract_authority_fails(self):
        root = self.fixture()
        path = root / "src/core/src/function/specialization.rs"
        path.write_text(
            path.read_text()
            + "\nfn bad(instance: &FunctionInstance) { instance.implementation().semantic_operation_contract(); }\n"
        )
        self.assert_failure(root, "certification trusts an implementation")

    def test_duplicate_runtime_capability_guard_omission_fails(self):
        root = self.fixture()
        path = root / "src/core/src/function/catalog.rs"
        path.write_text(path.read_text().replace("FunctionCatalogDuplicateRuntimeCapability", "RemovedDuplicateGuard"))
        self.assert_failure(root, "identical operation/target/signature")

    def test_unbound_executable_plan_registration_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/program/compiler_planning.rs"
        path.write_text(path.read_text() + "\nfn bad(plan: &Plan, instance: FunctionInstance) { plan.register_instance(instance); }\n")
        self.assert_failure(root, "executable plan node drops BoundCall")


if __name__ == "__main__":
    unittest.main()
