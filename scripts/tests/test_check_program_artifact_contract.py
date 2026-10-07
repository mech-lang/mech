from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER_PATH = ROOT / "scripts/check-program-artifact-contract.py"
SPEC = importlib.util.spec_from_file_location("program_artifact_contract", CHECKER_PATH)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class ProgramArtifactContractTests(unittest.TestCase):
    def test_repository_contract_passes(self) -> None:
        self.assertEqual(CHECKER.run(ROOT), [])

    def canonical_product_sources(self) -> tuple[str, str]:
        return (
            (ROOT / "src/engine/src/program/compiler_planning.rs").read_text(),
            (ROOT / "src/runtime/src/runtime/program/compiler.rs").read_text(),
        )

    def test_canonical_product_encodes_the_actual_artifact(self) -> None:
        program, runtime = self.canonical_product_sources()
        self.assertIn("encode_program_artifact_bytecode_v1(&artifact)", program)
        changed = program.replace("encode_program_artifact_bytecode_v1(&artifact)", "encode_program_artifact_bytecode_v1(&other_artifact)", 1)
        failures = CHECKER.validate_canonical_compilation_product(changed, runtime)
        self.assertTrue(any("encode the exact supplied artifact" in failure for failure in failures), failures)

    def test_canonical_product_requires_the_real_encoder(self) -> None:
        program, runtime = self.canonical_product_sources()
        changed = program.replace("encode_program_artifact_bytecode_v1(&artifact)", "Ok(Vec::new())", 1)
        failures = CHECKER.validate_canonical_compilation_product(changed, runtime)
        self.assertTrue(any("encode the exact supplied artifact" in failure for failure in failures), failures)

    def test_canonical_product_retains_the_actual_artifact(self) -> None:
        program, runtime = self.canonical_product_sources()
        constructor = CHECKER.function_body(program, "pub fn from_canonical_artifact(")
        self.assertIsNotNone(constructor)
        self.assertIn("            artifact,", constructor)
        changed = program.replace(constructor, constructor.replace("            artifact,", "            artifact: other_artifact,", 1), 1)
        failures = CHECKER.validate_canonical_compilation_product(changed, runtime)
        self.assertTrue(any("retain the supplied artifact" in failure for failure in failures), failures)

    def test_document_compilation_forwards_the_actual_artifact(self) -> None:
        program, runtime = self.canonical_product_sources()
        document = CHECKER.function_body(runtime, "pub(crate) fn compile_document(")
        self.assertIsNotNone(document)
        changed = runtime.replace(document, document.replace("from_canonical_artifact(artifact)", "from_canonical_artifact(other_artifact)", 1), 1)
        failures = CHECKER.validate_canonical_compilation_product(program, changed)
        self.assertTrue(any("forward its exact canonical artifact" in failure for failure in failures), failures)

    def test_document_compilation_requires_canonical_preparation(self) -> None:
        program, runtime = self.canonical_product_sources()
        document = CHECKER.function_body(runtime, "pub(crate) fn compile_document(")
        self.assertIsNotNone(document)
        changed = runtime.replace(document, document.replace("self.canonical_document_artifact(document)", "self.unchecked_document_artifact(document)", 1), 1)
        failures = CHECKER.validate_canonical_compilation_product(program, changed)
        self.assertTrue(any("obtain the canonical document artifact" in failure for failure in failures), failures)

    def test_document_compilation_retains_the_selected_function_catalog(self) -> None:
        program, runtime = self.canonical_product_sources()
        preparation = CHECKER.function_body(runtime, "fn canonical_document_artifact_with_projection(")
        self.assertIsNotNone(preparation)
        self.assertIn("Arc::clone(&self.function_catalog)", preparation)
        changed = runtime.replace(preparation, preparation.replace("Arc::clone(&self.function_catalog)", "default_catalog()", 1), 1)
        failures = CHECKER.validate_canonical_compilation_product(program, changed)
        self.assertTrue(any("Arc::clone(&self.function_catalog)" in failure for failure in failures), failures)

    def test_document_compilation_retains_external_contract_resolution(self) -> None:
        program, runtime = self.canonical_product_sources()
        preparation = CHECKER.function_body(runtime, "fn canonical_document_artifact_with_projection(")
        self.assertIsNotNone(preparation)
        changed = runtime.replace(preparation, preparation.replace("compile_artifact_with_external_contracts", "compile_artifact", 1), 1)
        failures = CHECKER.validate_canonical_compilation_product(program, changed)
        self.assertTrue(any("compile_artifact_with_external_contracts" in failure for failure in failures), failures)

    def test_tokens_outside_the_product_constructor_do_not_satisfy_the_guard(self) -> None:
        program, runtime = self.canonical_product_sources()
        changed = program.replace("pub fn from_canonical_artifact(", "pub fn unchecked_product(", 1)
        failures = CHECKER.validate_canonical_compilation_product(changed, runtime)
        self.assertTrue(any("missing the immutable canonical artifact product constructor" in failure for failure in failures), failures)

    def test_canonical_source_proof_rejects_missing_field_assertions(self) -> None:
        source = (ROOT / "src/engine/tests/canonical_document_state.rs").read_text()
        for field in (
            "requirements", "compute_regions", "source_nominal_declarations", "contracts", "inputs",
            "slots", "bindings", "outputs", "constraints", "nodes",
        ):
            assertion = f"assert_eq!(artifact.{field}(), decoded.{field}());"
            with self.subTest(field=field):
                self.assertTrue(CHECKER.validate_ordinary_source_proof(source.replace(assertion, "")))
        for assertion in (
            "assert_eq!(artifact.schemas().len(), decoded.schemas().len());",
            "assert_eq!(left.key(), right.key());",
            "assert_eq!(left.canonical_bytes(), right.canonical_bytes());",
            "assert_eq!(artifact.constants().len(), decoded.constants().len());",
            "assert_eq!(artifact_constant, decoded_constant);",
        ):
            with self.subTest(assertion=assertion):
                self.assertTrue(CHECKER.validate_ordinary_source_proof(source.replace(assertion, "")))

    def test_canonical_source_proof_requires_independent_compilation(self) -> None:
        source = (ROOT / "src/engine/tests/canonical_document_state.rs").read_text()
        changed = source.replace("let repeated = compiled(source).compile_artifact().unwrap();", "")
        self.assertTrue(any("independently twice" in failure for failure in CHECKER.validate_ordinary_source_proof(changed)))

    def test_source_proof_tokens_outside_the_test_do_not_satisfy_the_guard(self) -> None:
        source = (ROOT / "src/engine/tests/canonical_document_state.rs").read_text()
        changed = source.replace(
            "fn canonical_documents_emit_complete_equivalent_bytecode_artifacts()",
            "fn renamed_artifact_test()",
        )
        self.assertTrue(CHECKER.validate_ordinary_source_proof(changed))

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

    def test_retired_legacy_semantic_context_is_rejected(self) -> None:
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
        self.assertTrue(any("retains retired semantic boundary" in failure for failure in failures))

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

    def test_declared_nonwire_sidecar_is_accepted(self) -> None:
        manifest = {
            "artifact_fields": ["revision"],
            "artifact_internal_sidecars": [
                {"field": "shape_hints", "read_accessor": "shape_hint"}
            ],
            "forbidden_artifact_tokens": [],
        }
        source = """
pub struct ProgramArtifact {
    revision: u64,
    shape_hints: Vec<u64>,
}
impl ProgramArtifact {
    pub fn revision(&self) {}
    pub fn shape_hint(&self, slot: usize) { let _ = slot; }
}
"""
        self.assertEqual(CHECKER.validate_model(source, manifest), [])

    def test_unlisted_nonwire_sidecar_is_rejected(self) -> None:
        manifest = {
            "artifact_fields": ["revision"],
            "forbidden_artifact_tokens": [],
        }
        source = """
pub struct ProgramArtifact {
    revision: u64,
    shape_hints: Vec<u64>,
}
impl ProgramArtifact { pub fn revision(&self) {} }
"""
        failures = CHECKER.validate_model(source, manifest)
        self.assertTrue(any("fields changed" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
