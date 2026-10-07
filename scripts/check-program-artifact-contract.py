#!/usr/bin/env python3
"""Enforce the ProgramArtifact and bytecode-v1 semantic boundary."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/architecture/program-artifact/boundary.json"


def struct_body(source: str, name: str) -> str | None:
    match = re.search(rf"pub struct {re.escape(name)}\s*\{{(?P<body>.*?)\n\}}", source, re.S)
    return None if match is None else match.group("body")


def declared_fields(body: str) -> list[str]:
    return re.findall(r"^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:", body, re.M)


def public_fields(body: str) -> list[str]:
    return re.findall(r"^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", body, re.M)


def function_body(source: str, signature: str) -> str | None:
    start = source.find(signature)
    if start < 0:
        return None
    opening = source.find("{", start + len(signature))
    if opening < 0:
        return None
    depth = 0
    for index in range(opening, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[opening + 1 : index]
    return None


def validate_model(source: str, manifest: dict[str, object]) -> list[str]:
    failures: list[str] = []
    body = struct_body(source, "ProgramArtifact")
    if body is None:
        return ["ProgramArtifact declaration is missing"]
    durable_fields = list(manifest["artifact_fields"])
    if "requirements: super::ApplicationRequirementTable" in body:
        durable_fields.insert(durable_fields.index("contracts") + 1, "requirements")
    sidecars = list(manifest.get("artifact_internal_sidecars", []))
    expected = durable_fields + [sidecar["field"] for sidecar in sidecars]
    actual = declared_fields(body)
    if actual != expected:
        failures.append(f"ProgramArtifact fields changed: expected {expected}, found {actual}")
    if public_fields(body):
        failures.append("finalized ProgramArtifact fields must be private")
    for field in durable_fields:
        if re.search(rf"pub\s+(?:const\s+)?fn\s+{re.escape(field)}\s*\(&self\)", source) is None:
            failures.append(f"ProgramArtifact read-only accessor {field}() is missing")
    for sidecar in sidecars:
        accessor = sidecar["read_accessor"]
        if re.search(rf"pub\s+(?:const\s+)?fn\s+{re.escape(accessor)}\s*\(", source) is None:
            failures.append(
                f"ProgramArtifact internal sidecar accessor {accessor}() is missing"
            )
    if re.search(r"pub\s+(?:const\s+)?fn\s+\w+_mut\s*\(", source):
        failures.append("finalized ProgramArtifact exposes a mutable accessor")
    for token in manifest["forbidden_artifact_tokens"]:
        if token in body:
            failures.append(f"ProgramArtifact contains forbidden runtime token {token}")
    prefix = source[: source.index("pub struct ProgramArtifact")]
    derive = prefix.rsplit("#[derive", 1)[-1]
    if "Deserialize" in derive:
        failures.append("ProgramArtifact must not derive unchecked Deserialize")
    node = struct_body(source, "NodeDeclaration") or ""
    operation = struct_body(source, "OperationNodeBody") or ""
    if "requirements" in durable_fields and not (
        "body: ExecutableNodeBody" in node
        and "Operation(OperationNodeBody)" in source
        and "requirement: Option<ApplicationRequirementId>" in operation
    ):
        failures.append("resident external artifact requirement table lacks per-node requirement identity")
    for token in manifest["forbidden_artifact_tokens"]:
        if token in node or token in operation:
            failures.append(f"NodeDeclaration contains forbidden runtime token {token}")
    return failures


def validate_source_compiler(source: str) -> list[str]:
    failures: list[str] = []
    for required in (
        "pub fn compile_source_program",
        "pub fn compile_executable_program_artifact",
        "CompiledBytecode",
    ):
        if required not in source:
            failures.append(f"actual source compiler artifact adapter is missing {required}")
    for token in ("LegacyCompiledGraph", "artifact_from_legacy_graph"):
        if token in source:
            failures.append(f"source compiler retains forbidden compatibility token {token}")
    for required in (
        "CompiledInstructionRole",
        "register_schemas",
        "symbol_definitions",
        "return_register",
        "integrity_constraints",
        "runtime_entry_by_raw",
        "MissingRegisterKind",
        "MissingRegisterSource",
        "IntegrityConstraintSchemaMismatch",
    ):
        if required not in source:
            failures.append(f"source compiler semantic sidecar is missing {required}")

    adapter = function_body(source, "pub fn compile_executable_program_artifact")
    if adapter is None:
        failures.append("source compiler artifact adapter body is missing")
        return failures
    for obsolete in (
        'format!("input-{constant_index}")',
        'format!("runtime-{function:016x}")',
        'format!("host-{requirement}")',
        'format!("resource-{requirement}")',
        "unwrap_or(CompiledNodeKind::Combinational)",
        "constraints: Box::new([])",
        "prior.map(|value| value.schema)",
        "inputs.iter().find_map",
        "RuntimeFunctionId::from_raw",
    ):
        if obsolete in adapter:
            failures.append(f"source compiler adapter retains obsolete semantic guess {obsolete}")

    for retired in ("LegacySemanticContext", "CompilerLegacyContext"):
        if retired in source:
            failures.append(f"source compiler retains retired semantic boundary {retired}")
    return failures


def validate_reified_kind_canonicality(source: str) -> list[str]:
    failures: list[str] = []
    constructor = function_body(source, "pub fn from_canonical_bytes")
    if constructor is None:
        return ["ReifiedKind canonical byte constructor is missing"]
    for required in (
        "decode_canonical_reified_kind",
        "canonical_closed_kind_bytes",
        "reencoded.as_ref() != canonical_bytes.as_ref()",
    ):
        if required not in constructor:
            failures.append(
                f"ReifiedKind canonical byte constructor does not prove canonicality with {required}"
            )
    if "structurally_valid_noncanonical_dimensions_are_rejected" not in source:
        failures.append("ReifiedKind noncanonical dimension regression is missing")
    return failures


def validate_bytecode_sections(source: str, required: list[str]) -> list[str]:
    failures = [f"bytecode v1 section {section} is missing" for section in required if section not in source]
    if "BYTECODE_VERSION: u16 = 1" not in source:
        failures.append("bytecode v1 version declaration changed")
    return failures


def validate_canonical_compilation_product(program: str, runtime_compiler: str) -> list[str]:
    """Follow the normal retained-source path to its exact immutable artifact."""
    failures: list[str] = []
    constructor = function_body(program, "pub fn from_canonical_artifact(")
    if constructor is None:
        failures.append("normal compiler path is missing the immutable canonical artifact product constructor")
    else:
        compact = re.sub(r"\s+", "", constructor)
        if "letbytecode=encode_program_artifact_bytecode_v1(&artifact)" not in compact:
            failures.append("canonical compilation product does not encode the exact supplied artifact")
        if "Ok(Self{artifact,bytecode," not in compact:
            failures.append("canonical compilation product does not retain the supplied artifact and its encoded bytes")

    document = function_body(runtime_compiler, "pub(crate) fn compile_document(")
    compact_document = re.sub(r"\s+", "", document or "")
    if "letartifact=self.canonical_document_artifact(document)?;" not in compact_document:
        failures.append("normal document compilation does not obtain the canonical document artifact")
    if "ProgramCompilationProduct::from_canonical_artifact(artifact)" not in compact_document:
        failures.append("normal document compilation does not forward its exact canonical artifact to the product")

    artifact = function_body(runtime_compiler, "fn canonical_document_artifact(")
    if "self.canonical_document_artifact_with_projection(document,false)" not in re.sub(r"\s+", "", artifact or ""):
        failures.append("normal document artifact path does not select the retained canonical preparation owner")
    preparation = function_body(runtime_compiler, "fn canonical_document_artifact_with_projection(")
    compact_preparation = re.sub(r"\s+", "", preparation or "")
    for required in (
        "CanonicalSourceFrontend::compile_document_with_catalog_and_resources",
        "CanonicalSourceFrontend::compile_interactive_document_with_catalog_and_resources",
        "Arc::clone(&self.function_catalog)",
        "program.compile_artifact_with_external_contracts(&ResidentExternalContractResolver::new(self.resources,))",
    ):
        if required not in compact_preparation:
            failures.append(f"canonical document artifact preparation is missing {required}")
    return failures


def validate_ordinary_source_proof(source: str) -> list[str]:
    proof = function_body(
        source, "fn canonical_documents_emit_complete_equivalent_bytecode_artifacts()"
    )
    if proof is None:
        return ["ordinary-source artifact proof is missing its canonical document test"]
    failures: list[str] = []
    for required in (
        "include_str!",
        "scalar-alias.mec",
        "state-register.mec",
        "matrix-literal.mec",
        "comparison-output.mec",
        "integrity-constraint.mec",
        "compiled(source).compile_artifact()",
        "encode_program_artifact_bytecode_v1(&artifact)",
        "encode_program_artifact_bytecode_v1(&repeated)",
        "decode_program_artifact_bytecode_v1(&bytecode)",
        "encode_program_artifact_bytecode_v1(&decoded)",
        "assert_eq!(bytecode, reencoded)",
        "assert_eq!(artifact.revision(), repeated.revision())",
        "assert_eq!(artifact.revision(), decoded.revision())",
    ):
        if required not in proof:
            failures.append(f"ordinary-source artifact proof is missing {required}")
    compact = re.sub(r"\s+", "", proof)
    for field in (
        "requirements", "compute_regions", "source_nominal_declarations", "contracts", "inputs",
        "slots", "bindings", "outputs", "constraints", "nodes",
    ):
        assertion = f"assert_eq!(artifact.{field}(), decoded.{field}())"
        if re.sub(r"\s+", "", assertion) not in compact:
            failures.append(f"ordinary-source artifact proof is missing {assertion}")
    for required in (
        "assert_eq!(artifact.schemas().len(), decoded.schemas().len())",
        "for (left, right) in artifact.schemas().entries().zip(decoded.schemas().entries())",
        "assert_eq!(left.key(), right.key())",
        "assert_eq!(left.canonical_bytes(), right.canonical_bytes())",
        "assert_eq!(artifact.constants().len(), decoded.constants().len())",
        "for raw in 0..artifact.constants().len()",
        "artifact.constants().get(id).unwrap().canonical_snapshot_bytes(artifact.schemas()).unwrap()",
        "decoded.constants().get(id).unwrap().canonical_snapshot_bytes(decoded.schemas()).unwrap()",
        "assert_eq!(artifact_constant, decoded_constant)",
    ):
        if re.sub(r"\s+", "", required) not in compact:
            failures.append(f"ordinary-source artifact proof is missing {required}")
    if proof.count("compiled(source).compile_artifact()") < 2:
        failures.append("ordinary-source artifact proof must compile each fixture independently twice")
    if re.search(
        r"assert_eq!\(\s*bytecode,\s*mech_engine::encode_program_artifact_bytecode_v1"
        r"\(&repeated\)\.unwrap\(\)\s*\);",
        proof,
    ) is None:
        failures.append("ordinary-source artifact proof must compare independently compiled bytes")
    return failures


def run(root: Path = ROOT) -> list[str]:
    manifest = json.loads((root / MANIFEST.relative_to(ROOT)).read_text())
    failures: list[str] = []
    model = (root / "src/engine/src/artifact/model.rs").read_text()
    compiler = (root / "src/engine/src/artifact/compiler.rs").read_text()
    bytecode = (root / "src/engine/src/artifact/bytecode.rs").read_text()
    sections = (root / "src/core/src/program/bytecode/section.rs").read_text()
    sections += (root / "src/core/src/program/bytecode/header.rs").read_text()
    test = (root / "src/engine/tests/program_artifact_contract.rs").read_text()
    program = (root / "src/engine/src/program/compiler_planning.rs").read_text()
    runtime_compiler = (root / "src/runtime/src/runtime/program/compiler.rs").read_text()
    encoding = (root / "src/engine/src/artifact/encoding.rs").read_text()
    snapshot_data = (root / "src/core/src/snapshot/data.rs").read_text()
    failures.extend(validate_model(model, manifest))
    failures.extend(validate_source_compiler(compiler))
    failures.extend(validate_reified_kind_canonicality(snapshot_data))
    failures.extend(validate_bytecode_sections(sections, manifest["bytecode_sections"]))
    for required in (
        "encode_program_artifact_bytecode_v1",
        "decode_program_artifact_bytecode_v1",
        "decode_program_artifact_sections",
        "write_bytecode_with_artifact",
        "ProgramArtifactDraft",
    ):
        if required not in bytecode:
            failures.append(f"typed bytecode artifact path is missing {required}")
    for forbidden in ("LegacyValue", "LegacyCompiledGraph", "artifact_from_legacy_graph"):
        if forbidden in bytecode:
            failures.append(f"bytecode decoder retains forbidden compatibility token {forbidden}")
    failures.extend(validate_canonical_compilation_product(program, runtime_compiler))
    source_proof = (root / "src/engine/tests/canonical_document_state.rs").read_text()
    failures.extend(validate_ordinary_source_proof(source_proof))
    for required in (
        "IntegrityConstraintSchemaMismatch",
        "CompiledTypeBindingMismatch",
    ):
        if required not in test:
            failures.append(f"malformed artifact regression proof is missing {required}")
    if 'b"mech-program-v1\\0"' not in encoding:
        failures.append("ProgramRevision domain separator changed")
    return failures


def main() -> int:
    failures = run()
    if failures:
        for failure in failures:
            print(f"ProgramArtifact contract failure: {failure}", file=sys.stderr)
        return 1
    print("ProgramArtifact and bytecode-v1 boundary: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
