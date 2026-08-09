from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER_PATH = ROOT / "scripts/check-program-artifact-contract.py"
SPEC = importlib.util.spec_from_file_location("c3_contract", CHECKER_PATH)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class ProgramArtifactContractTests(unittest.TestCase):
    def test_repository_contract_passes(self) -> None:
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_unapproved_runtime_identity_is_rejected(self) -> None:
        manifest = {
            "artifact_fields": ["revision"],
            "forbidden_artifact_tokens": ["CellId"],
        }
        source = "pub struct ProgramArtifact {\n revision: CellId,\n}\nimpl ProgramArtifact { pub fn revision(&self) {} }\n"
        failures = CHECKER.validate_model(source, manifest)
        self.assertTrue(any("CellId" in failure for failure in failures))

    def test_legacy_compatibility_source_is_rejected(self) -> None:
        source = (
            "pub fn compile_source_program() {}\n"
            "pub fn compile_executable_program_artifact(_: CompiledBytecode) {}\n"
            "type Input = LegacyCompiledGraph;"
        )
        failures = CHECKER.validate_source_compiler(source)
        self.assertTrue(any("LegacyCompiledGraph" in failure for failure in failures))

    def test_source_adapter_schema_guess_is_rejected(self) -> None:
        source = """
pub fn compile_source_program() {}
pub fn compile_executable_program_artifact(_: CompiledBytecode) {
    let schema = prior.map(|value| value.schema);
}
// CompiledInstructionRole register_kinds symbol_definitions return_register
// integrity_constraints runtime_entry_by_raw MissingRegisterKind
// MissingRegisterSource IntegrityConstraintSchemaMismatch
"""
        failures = CHECKER.validate_source_compiler(source)
        self.assertTrue(any("prior.map" in failure for failure in failures))

    def test_source_adapter_fallback_name_is_rejected(self) -> None:
        source = """
pub fn compile_source_program() {}
pub fn compile_executable_program_artifact(_: CompiledBytecode) {
    let name = format!("runtime-{function:016x}");
}
// CompiledInstructionRole register_kinds symbol_definitions return_register
// integrity_constraints runtime_entry_by_raw MissingRegisterKind
// MissingRegisterSource IntegrityConstraintSchemaMismatch
"""
        failures = CHECKER.validate_source_compiler(source)
        self.assertTrue(any("runtime-" in failure for failure in failures))

    def test_fabricated_legacy_nominal_path_is_rejected(self) -> None:
        source = """
pub fn compile_source_program() {}
pub fn compile_executable_program_artifact(_: CompiledBytecode) {}
impl LegacySemanticContext for CompilerLegacyContext {
    fn resolve_named_kind() { LegacyNamedKindUnresolved; }
    fn resolve_nominal() {
        LegacyNominalUnresolved;
        let path = "legacy".to_owned();
    }
}
// CompiledInstructionRole register_kinds symbol_definitions return_register
// integrity_constraints runtime_entry_by_raw MissingRegisterKind
// MissingRegisterSource IntegrityConstraintSchemaMismatch
"""
        failures = CHECKER.validate_source_compiler(source)
        self.assertTrue(any("fabricates durable nominal identity" in failure for failure in failures))

    def test_reified_kind_without_canonical_round_trip_is_rejected(self) -> None:
        source = """
pub fn from_canonical_bytes(canonical_bytes: Box<[u8]>) {
    decode_canonical_reified_kind(&canonical_bytes);
}
fn structurally_valid_noncanonical_dimensions_are_rejected() {}
"""
        failures = CHECKER.validate_reified_kind_canonicality(source)
        self.assertTrue(any("canonical_closed_kind_bytes" in failure for failure in failures))

    def test_public_finalized_artifact_field_is_rejected(self) -> None:
        manifest = {
            "artifact_fields": ["revision"],
            "forbidden_artifact_tokens": [],
        }
        source = "pub struct ProgramArtifact {\n pub revision: u64,\n}\nimpl ProgramArtifact { pub fn revision(&self) {} }\n"
        failures = CHECKER.validate_model(source, manifest)
        self.assertTrue(any("must be private" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
