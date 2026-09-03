from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r2-type-memory-boundary.py"
SPEC = importlib.util.spec_from_file_location("check_r2_type_memory_boundary", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


MEMORY_MOD = """
mod type_contract;
mod storage_capability;
mod operation_requirement;
pub use self::type_contract::*;
pub use self::storage_capability::*;
pub use self::operation_requirement::*;
"""
STORAGE = """
pub fn check_schema_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    storage: &StorageCapabilityDescriptor,
) {}
pub(crate) fn check_resolved_type_storage_compatibility() {}
"""
OPERATION = """
pub struct PortMemoryRequirement {
    access: AccessMode,
    delivery: DeliveryMode,
    construction: OutputConstruction,
    alias: AliasPolicy,
    change_detection: ChangeDetectionPolicy,
}
pub struct OperationMemoryRequirements;
pub enum PortStorageCompatibilityError { SemanticAddressingUnsupported }
fn check_semantic_addressing() {}
pub fn check_port_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    requirement: &PortMemoryRequirement,
    storage: &StorageCapabilityDescriptor,
) {}
impl OperationContractDeclaration {
    pub fn memory_requirements(&self, input_count: usize) {
        self.inputs.resolve(input_count);
    }
}
"""
CELL = """
pub(crate) trait ErasedCellStorage {
    fn capabilities(&self);
    fn same_storage(&self);
    fn detached_clone(&self);
}
pub(crate) struct DetachedCellStorage {
    identity: CanonicalCellId,
    storage: Rc<dyn ErasedCellStorage>,
}
impl ValueCell {
    pub fn type_memory_contract(&self) {}
    pub fn resolved_type_memory_contract(&self) {}
    pub fn storage_capabilities(&self) {}
    pub fn validate_storage_contract(&self) { check_schema_storage_compatibility(); }
    pub fn same_logical_cell(&self) {}
    pub fn same_storage(&self, other: &Self) -> bool { true }
    pub fn same_cell(&self, other: &Self) -> bool { self.same_storage(other) }
}
"""
ARGUMENT = """
impl FunctionInvocation {
    pub fn check_operation_memory_contract(&self) {}
    fn check_operation_output_alias(&self, input: &ValueCell) {
        self.output.same_storage(input);
    }
}
fn check_invocation_cell_requirement() {
    check_port_storage_compatibility();
}
"""
CONFORMANCE = "\n".join(CHECKER.CONFORMANCE + (
    "SemanticAddressingUnsupported", "DynamicAxisUnsupported",
    "same_logical_cell", "same_storage", "snapshot_eq",
))
DESIGN = """Status: R2 complete
shadow-only RowDVector DVector DynamicAxisUnsupported R3 R4 R5 R6
"""
README = """The canonical value-system cutover, R1 contract closure, and R2 type-memory
boundary are complete. Package 0.3.6.
"""
ROADMAP = "Type–memory boundary: complete\nNext endgame phase: R3\nPackage 0.3.6\n"
ENDGAME = "## R2 closure\nPackage 0.3.6\n"
R1 = "python3 scripts/check-r1-compatibility-closure.py"
R2 = "python3 scripts/check-r2-type-memory-boundary.py"
UNIT = "scripts/tests/test_check_r2_type_memory_boundary.py"
R2_SUITE = "cargo +nightly-2026-03-03 test --locked -p mech-core --all-features --test type_memory_boundary"
CI = f"""  static-contracts:
    run: |
      {R1}
      {R2}
      python3 -m unittest {UNIT}
"""
FULL = CI.replace("static-contracts", "architecture-contracts") + f"      {R2_SUITE}\n"
OWNERS = '[owners.architecture-contracts]\npaths = ["scripts/check-r2-type-memory-boundary.py", "' + UNIT + '", "' + '", "'.join(CHECKER.R2_DOCS) + '"]\n'


class R2TypeMemoryBoundaryTests(unittest.TestCase):
    def fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        files = {
            "src/core/src/lib.rs": "pub mod memory_contract;\npub(crate) mod runtime_storage;\n",
            "src/core/src/memory_contract/mod.rs": MEMORY_MOD,
            "src/core/src/memory_contract/type_contract.rs": "pub struct Contract;\n",
            "src/core/src/memory_contract/storage_capability.rs": STORAGE,
            "src/core/src/memory_contract/operation_requirement.rs": OPERATION,
            "src/core/src/runtime_storage.rs": "pub(crate) struct FunctionValueRepresentation;\n",
            "src/core/src/schema/mod.rs": "impl Schema { pub fn type_memory_contract(&self) {} pub fn resolved_type_memory_contract(&self) {} }\n",
            "src/core/src/cell_binding.rs": CELL,
            "src/core/src/function/argument.rs": ARGUMENT,
            "src/core/tests/type_memory_boundary.rs": CONFORMANCE,
            "docs/design/type-memory-boundary.md": DESIGN,
            "docs/design/ROADMAP.mec": ROADMAP,
            "docs/design/v0.4-endgame.md": ENDGAME,
            "README.md": README,
            ".github/workflows/ci.yml": CI,
            ".github/workflows/ci-full.yml": FULL,
            ".github/ci/owners.toml": OWNERS,
        }
        for relative, source in files.items():
            self.write(root, relative, source)
        return root

    @staticmethod
    def write(root: Path, relative: str, source: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")

    def replace(self, root: Path, relative: str, old: str, new: str) -> None:
        path = root / relative
        self.write(root, relative, path.read_text(encoding="utf-8").replace(old, new))

    def assert_failure(self, root: Path, diagnostic: str) -> None:
        self.assertTrue(any(diagnostic in item for item in CHECKER.failures(root)), CHECKER.failures(root))

    def test_01_canonical_fixture_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_02_missing_required_module_fails(self):
        root = self.fixture()
        (root / "src/core/src/memory_contract/type_contract.rs").unlink()
        self.assert_failure(root, "required file is missing")

    def test_03_public_runtime_storage_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/lib.rs", "pub(crate) mod runtime_storage", "pub mod runtime_storage")
        self.assert_failure(root, "runtime_storage must not be public")

    def test_04_transitional_representation_leak_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "struct FunctionValueRepresentation;\n")
        self.assert_failure(root, "forbidden memory-contract identifier")
        root = self.fixture()
        self.replace(root, "src/core/src/schema/mod.rs", "pub fn type_memory_contract(&self) {}", "pub fn type_memory_contract(&self) { let _: FunctionValueRepresentation; }")
        self.assert_failure(root, "Schema type-memory projection")

    def test_05_cell_binding_or_value_cell_leak_fails(self):
        for identifier in ("CellBinding", "ValueCell"):
            with self.subTest(identifier=identifier):
                root = self.fixture()
                self.write(root, "src/core/src/memory_contract/type_contract.rs", f"struct Uses({identifier});\n")
                self.assert_failure(root, identifier)

    def test_06_target_specific_identifier_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "struct GpuMemory;\n")
        self.assert_failure(root, "target-specific")

    def test_07_serialize_derive_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "#[derive(Serialize)] struct Contract;\n")
        self.assert_failure(root, "Serialize")

    def test_08_serde_import_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "use serde::Serialize;\n")
        self.assert_failure(root, "serde")

    def test_09_repr_or_unsafe_fails(self):
        for source, diagnostic in (("#[repr(C)] struct Contract;", "repr"), ("#[repr(transparent)] struct Contract(u8);", "repr"), ("#[repr(packed)] struct Contract { value: u8 }", "repr"), ("unsafe fn leak() {}", "unsafe")):
            with self.subTest(source=source):
                root = self.fixture()
                self.write(root, "src/core/src/memory_contract/type_contract.rs", source)
                self.assert_failure(root, diagnostic)

    def test_10_physical_layout_field_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "struct Contract { byte_offset: u64 }\n")
        self.assert_failure(root, "physical-layout field")

    def test_11_physical_plan_public_name_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "pub struct MemoryLayout;\n")
        self.assert_failure(root, "physical-plan public name")

    def test_12_r2_identifier_in_schema_encoding_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/schema/encoding.rs", "fn encode(_: TypeMemoryContract) {}\n")
        self.assert_failure(root, "wire-format code")
        for source in ("impl TypeMemoryContract { fn encode(&self) {} }", "fn decode_contract(_: PortMemoryRequirement) {}"):
            root = self.fixture()
            self.write(root, "src/core/src/r2_wire.rs", source)
            self.assert_failure(root, "serialization implementation")

    def test_13_r2_identifier_in_bytecode_artifact_or_abi_fails(self):
        for relative in ("src/bytecode/src/lib.rs", "src/engine/src/artifact/model.rs", "src/abi/src/lib.rs"):
            with self.subTest(relative=relative):
                root = self.fixture()
                self.write(root, relative, "fn encode(_: PortMemoryRequirement) {}\n")
                self.assert_failure(root, "wire-format code")

    def test_14_public_low_level_checker_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/memory_contract/storage_capability.rs", "pub(crate) fn check_resolved", "pub fn check_resolved")
        self.assert_failure(root, "low-level resolved checker")

    def test_15_missing_safe_schema_api_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/memory_contract/storage_capability.rs", "pub fn check_schema_storage_compatibility", "fn check_schema_storage_compatibility")
        self.assert_failure(root, "safe schema-bound")

    def test_16_missing_semantic_addressing_stage_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/memory_contract/operation_requirement.rs", "fn check_semantic_addressing", "fn renamed")
        self.assert_failure(root, "semantic-addressing stage")

    def test_17_revived_erased_logical_id_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/cell_binding.rs", "fn capabilities", "fn logical_cell_id(&self); fn capabilities")
        self.assert_failure(root, "revived logical_cell_id")
        for declaration in ("pub struct PhysicalStorageId;", "pub fn storage_identity() {}", "pub fn backing_pointer() {}"):
            root = self.fixture()
            self.write(root, "src/core/src/probe.rs", declaration)
            self.assert_failure(root, "public physical-storage identity")

    def test_18_same_cell_must_delegate_exactly(self):
        root = self.fixture()
        self.replace(root, "src/core/src/cell_binding.rs", "self.same_storage(other)", "true")
        self.assert_failure(root, "delegate exactly")

    def test_19_detached_storage_needs_identity(self):
        root = self.fixture()
        self.replace(root, "src/core/src/cell_binding.rs", "identity: CanonicalCellId,", "")
        self.assert_failure(root, "explicit CanonicalCellId")

    def test_20_production_storage_validation_call_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn probe(cell: &ValueCell) { cell.validate_storage_contract(); }\n")
        self.assert_failure(root, "production call")
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn probe() { check_schema_storage_compatibility(); }\n")
        self.assert_failure(root, "unauthorized production use of check_schema_storage_compatibility")

    def test_21_production_invocation_validation_call_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn probe(value: &FunctionInvocation) { value.check_operation_memory_contract(); }\n")
        self.assert_failure(root, "production call")

    def test_22_other_port_checker_call_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn probe() { check_port_storage_compatibility(); }\n")
        self.assert_failure(root, "unauthorized production use")

    def test_23_cfg_test_calls_are_allowed(self):
        for attribute in ("#[cfg(test)]", '#[cfg(all(test, feature = "x"))]'):
            with self.subTest(attribute=attribute):
                root = self.fixture()
                source = f"{attribute} mod tests {{ fn probe(c: &ValueCell) {{ c.validate_storage_contract(); }} }}\n"
                self.write(root, "src/core/src/probe.rs", source)
                self.assertEqual(CHECKER.failures(root), [])

    def test_24_integration_test_calls_are_allowed(self):
        root = self.fixture()
        self.write(root, "src/core/tests/probe.rs", "fn probe(c: &ValueCell) { c.check_operation_memory_contract(); }\n")
        self.assertEqual(CHECKER.failures(root), [])

    def test_25_alias_logical_identity_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/function/argument.rs", "same_storage", "same_logical_cell")
        self.assert_failure(root, "forbidden identity same_logical_cell")

    def test_26_alias_same_cell_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/function/argument.rs", "same_storage", "same_cell")
        self.assert_failure(root, "forbidden identity same_cell")

    def test_27_missing_conformance_marker_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/tests/type_memory_boundary.rs", CHECKER.CONFORMANCE[0], "missing")
        self.assert_failure(root, "conformance suite is missing")

    def test_28_missing_vector_transition_docs_fail(self):
        for marker in ("RowDVector", "DVector"):
            with self.subTest(marker=marker):
                root = self.fixture()
                self.replace(root, "docs/design/type-memory-boundary.md", marker, "missing")
                self.assert_failure(root, "type-memory documentation")

    def test_29_stale_r2_release_blocker_fails(self):
        root = self.fixture()
        path = "docs/design/v0.4-endgame.md"
        stale = "The R2 boundary has not yet separated semantic identity from physical storage identity."
        self.write(root, path, ENDGAME + stale)
        self.assert_failure(root, "stale R2 release blocker")

    def test_30_missing_normal_ci_invocation_fails(self):
        root = self.fixture()
        self.replace(root, ".github/workflows/ci.yml", R2, "")
        self.assert_failure(root, "R1/R2 architecture checker sequence")

    def test_31_missing_full_ci_invocation_fails(self):
        root = self.fixture()
        self.replace(root, ".github/workflows/ci-full.yml", R2, "")
        self.assert_failure(root, "R1/R2 architecture checker sequence")
        root = self.fixture()
        self.replace(root, ".github/workflows/ci-full.yml", R2_SUITE, "")
        self.assert_failure(root, "R2 conformance target")

    def test_32_incorrect_checker_order_fails(self):
        root = self.fixture()
        self.write(root, ".github/workflows/ci.yml", CI.replace(f"{R1}\n      {R2}", f"{R2}\n      {R1}"))
        self.assert_failure(root, "before the R1 checker")

    def test_33_missing_checker_unit_invocation_fails(self):
        root = self.fixture()
        self.replace(root, ".github/workflows/ci.yml", UNIT, "missing.py")
        self.assert_failure(root, "checker unit tests")
        root = self.fixture()
        self.write(root, ".github/workflows/ci.yml", CI + "    continue-on-error: true\n")
        self.assert_failure(root, "waives the R2 architecture gate")

    def test_34_missing_owner_entry_fails(self):
        for path in (UNIT,) + CHECKER.R2_DOCS:
            root = self.fixture()
            self.replace(root, ".github/ci/owners.toml", path, "missing")
            self.assert_failure(root, "architecture owner entry")

    def test_35_comment_forbidden_identifier_is_ignored(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "// ValueCell\nstruct Contract;\n")
        self.assertEqual(CHECKER.failures(root), [])

    def test_36_normal_string_forbidden_identifier_is_ignored(self):
        for literal in ('"ValueCell"', 'b"ValueCell"'):
            with self.subTest(literal=literal):
                root = self.fixture()
                self.write(root, "src/core/src/memory_contract/type_contract.rs", f"const NOTE: &[u8] = {literal};\n")
                self.assertEqual(CHECKER.failures(root), [])
        self.assertEqual(CHECKER.rust_code("'{' b'}' '\\n'").strip(), "")
        self.assertIn("'a", CHECKER.rust_code("fn value<'a>(item: &'a str) {}"))

    def test_37_raw_string_forbidden_identifier_is_ignored(self):
        for literal in ('r###"ValueCell"###', 'br###"ValueCell"###'):
            with self.subTest(literal=literal):
                root = self.fixture()
                self.write(root, "src/core/src/memory_contract/type_contract.rs", f"const NOTE: &[u8] = {literal};\n")
                self.assertEqual(CHECKER.failures(root), [])

    def test_38_nested_block_comments_are_stripped(self):
        root = self.fixture()
        self.write(root, "src/core/src/memory_contract/type_contract.rs", "/* outer /* ValueCell */ unsafe */ struct Contract;\n")
        self.assertEqual(CHECKER.failures(root), [])


if __name__ == "__main__":
    unittest.main()
