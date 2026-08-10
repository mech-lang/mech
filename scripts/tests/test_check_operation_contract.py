from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER_PATH = ROOT / "scripts/check-operation-contract.py"
SPEC = importlib.util.spec_from_file_location("c4_contract", CHECKER_PATH)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)

UNCHECKED_CONSTRUCTOR = "from_" + "entries_unchecked"
VALID_TABLE_SOURCE = """
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContractTable { entries: Box<[ResolvedOperationContract]> }
impl OperationContractTable {
    pub fn empty() -> Self { todo!() }
    pub(super) const fn UNCHECKED_CONSTRUCTOR_TOKEN(
        entries: Box<[ResolvedOperationContract]>,
    ) -> Self { Self { entries } }
}
impl OperationContractTable {
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let entries = decode(bytes)?;
        Ok(Self::UNCHECKED_CONSTRUCTOR_TOKEN(entries.into_boxed_slice()))
    }
}
""".replace("UNCHECKED_CONSTRUCTOR_TOKEN", UNCHECKED_CONSTRUCTOR)

VALID_SEMANTIC_GUARDS = r"""
AliasSchemaMismatch EffectOutputUnsupported "alias.input"
value.trim() != value
value.contains(['\0', '/', '\\'])
"""


class OperationContractCheckerTests(unittest.TestCase):
    def test_repository_contract_passes(self) -> None:
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_missing_contract_type_is_rejected(self) -> None:
        failures = CHECKER.validate_contract_sources(
            "pub struct OperationContractId(u32);", ["OperationContractId", "AccessMode"]
        )
        self.assertTrue(any("AccessMode" in failure for failure in failures))

    def test_runtime_function_pointer_in_resolved_contract_is_rejected(self) -> None:
        source = """
pub enum ResolvedOperationContract { Declared(DeclaredOperationContract) }
pub struct DeclaredOperationContract { validator: fn() }
"""
        failures = CHECKER.validate_contract_sources(
            source, ["ResolvedOperationContract", "DeclaredOperationContract"]
        )
        self.assertTrue(any("fn(" in failure for failure in failures))

    def test_storage_strategy_in_resolved_contract_is_rejected(self) -> None:
        source = """
pub enum ResolvedOperationContract { Declared(DeclaredOperationContract) }
pub struct DeclaredOperationContract { storage: StorageStrategy }
"""
        failures = CHECKER.validate_contract_sources(
            source, ["ResolvedOperationContract", "DeclaredOperationContract"]
        )
        self.assertTrue(any("StorageStrategy" in failure for failure in failures))

    def test_operation_contract_table_deserialize_is_rejected(self) -> None:
        source = VALID_TABLE_SOURCE.replace("derive(Serialize)", "derive(Serialize, Deserialize)")
        failures = CHECKER.validate_table_boundary(source)
        self.assertTrue(any("Deserialize" in failure for failure in failures))

    def test_operation_contract_table_requires_empty_constructor(self) -> None:
        source = VALID_TABLE_SOURCE.replace("pub fn empty()", "fn empty()")
        failures = CHECKER.validate_table_boundary(source)
        self.assertTrue(any("empty()" in failure for failure in failures))

    def test_public_raw_entry_constructor_is_rejected(self) -> None:
        raw_entry_constructor = "from_" + "canonical_entries"
        source = VALID_TABLE_SOURCE + f"\npub const fn {raw_entry_constructor}() {{}}\n"
        failures = CHECKER.validate_table_boundary(source)
        self.assertTrue(any("raw-entry constructor" in failure for failure in failures))

    def test_unchecked_constructor_requires_pub_super_visibility(self) -> None:
        for visibility in ("pub", "pub(crate)"):
            source = VALID_TABLE_SOURCE.replace("pub(super)", visibility)
            failures = CHECKER.validate_table_boundary(source)
            self.assertTrue(any("pub(super)" in failure for failure in failures))

    def test_alias_schema_mismatch_guard_is_required(self) -> None:
        failures = CHECKER.validate_semantic_guards(
            VALID_SEMANTIC_GUARDS.replace("AliasSchemaMismatch", "")
        )
        self.assertTrue(any("AliasSchemaMismatch" in failure for failure in failures))

    def test_effect_output_guard_is_required(self) -> None:
        failures = CHECKER.validate_semantic_guards(
            VALID_SEMANTIC_GUARDS.replace("EffectOutputUnsupported", "")
        )
        self.assertTrue(any("EffectOutputUnsupported" in failure for failure in failures))

    def test_canonical_reference_separator_and_whitespace_guards_are_required(self) -> None:
        for source in (
            VALID_SEMANTIC_GUARDS.replace("value.trim() != value", ""),
            VALID_SEMANTIC_GUARDS.replace("value.contains(['\\0', '/', '\\\\'])", ""),
        ):
            failures = CHECKER.validate_semantic_guards(source)
            self.assertTrue(any("shape-contract reference" in failure for failure in failures))

    def test_node_without_contract_is_rejected(self) -> None:
        model = "pub struct NodeDeclaration { pub node: NodeId }"
        failures = CHECKER.validate_artifact_fields(
            model, {"NodeDeclaration": "contract: OperationContractId"}
        )
        self.assertTrue(any("NodeDeclaration" in failure for failure in failures))

    def test_bytecode_contract_section_is_required(self) -> None:
        failures = CHECKER.validate_bytecode(
            "BYTECODE_SECTION_COUNT: usize = 18; BYTECODE_CONTENT_OFFSET: u64 = 640;",
            "",
            "ArtifactOperationContracts",
        )
        self.assertTrue(any("operation-contract" in failure for failure in failures))

    def test_synthetic_fully_declared_assertion_is_required(self) -> None:
        validation = "IntegrityConstraintContractInvalid ResolvedOperationContract::LegacyOpaque ExternalInteraction::Pure"
        fixture = "fn ekf_contract_fixture_is_fully_declared_and_round_trips_contract_ids() {}"
        failures = CHECKER.validate_opaque_policy(validation, fixture)
        self.assertTrue(any("synthetic fully-declared artifact" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
