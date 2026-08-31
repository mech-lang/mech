#!/usr/bin/env python3
"""Enforce the C3 ProgramArtifact and bytecode-v1 semantic boundary."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/architecture/program-artifact/c3-boundary.json"
C3_FINAL_COMMIT = "15d06dd6ad1b19d874c7c512dd92acfd367fd45d"


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
    expected = list(manifest["artifact_fields"])
    if "requirements: super::ApplicationRequirementTable" in body:
        expected.insert(expected.index("contracts") + 1, "requirements")
    actual = declared_fields(body)
    if actual != expected:
        failures.append(f"ProgramArtifact fields changed: expected {expected}, found {actual}")
    if public_fields(body):
        failures.append("finalized ProgramArtifact fields must be private")
    for field in expected:
        if re.search(rf"pub\s+(?:const\s+)?fn\s+{re.escape(field)}\s*\(&self\)", source) is None:
            failures.append(f"ProgramArtifact read-only accessor {field}() is missing")
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
    if "requirements" in expected and "requirement: Option<ApplicationRequirementId>" not in node:
        failures.append("D3 artifact requirement table lacks per-node requirement identity")
    for token in manifest["forbidden_artifact_tokens"]:
        if token in node:
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


def changed_protected_paths(
    root: Path, base: str, paths: list[str], head: str = "HEAD"
) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", base, head, "--", *paths],
        cwd=root,
        check=True,
        text=True,
        capture_output=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def run(root: Path = ROOT) -> list[str]:
    manifest = json.loads((root / MANIFEST.relative_to(ROOT)).read_text())
    failures: list[str] = []
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", C3_FINAL_COMMIT, "HEAD"],
        cwd=root,
        text=True,
        capture_output=True,
    )
    if ancestor.returncode != 0:
        failures.append(f"C3 final commit {C3_FINAL_COMMIT} must be an ancestor of HEAD")
    historical_manifest_process = subprocess.run(
        [
            "git",
            "show",
            f"{C3_FINAL_COMMIT}:tests/architecture/program-artifact/c3-boundary.json",
        ],
        cwd=root,
        text=True,
        capture_output=True,
    )
    if historical_manifest_process.returncode != 0:
        failures.append("unable to read the frozen C3 boundary manifest")
        historical_manifest = manifest
    else:
        historical_manifest = json.loads(historical_manifest_process.stdout)
    model = (root / "src/engine/src/artifact/model.rs").read_text()
    compiler = (root / "src/engine/src/artifact/compiler.rs").read_text()
    bytecode = (root / "src/engine/src/artifact/bytecode.rs").read_text()
    sections = (root / "src/core/src/program/bytecode/section.rs").read_text()
    sections += (root / "src/core/src/program/bytecode/header.rs").read_text()
    test = (root / "src/engine/tests/program_artifact_contract.rs").read_text()
    program = (root / "src/engine/src/program/compiler_planning.rs").read_text()
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
    for required in (
        "compile_program_product",
        "compile_executable_program_artifact",
        "encode_program_artifact_sections",
        "write_bytecode_with_artifact",
    ):
        if required not in program:
            failures.append(f"normal compiler path is missing {required}")
    for required in (
        "include_str!",
        "plan_source_for_test",
        "compile_program_product",
        "ParsedProgram::from_bytes",
        "decode_program_artifact_sections",
        "artifact_a.revision()",
        "artifact_b.revision()",
        "comparison-output.mec",
        "integrity-constraint.mec",
        "artifact_a.constraints()",
    ):
        if required not in program:
            failures.append(f"ordinary-source artifact proof is missing {required}")
    for required in (
        "IntegrityConstraintSchemaMismatch",
        "MissingRegisterKind",
        "MissingRegisterSource",
    ):
        if required not in test:
            failures.append(f"malformed artifact regression proof is missing {required}")
    if 'b"mech-program-v1\\0"' not in encoding:
        failures.append("ProgramRevision domain separator changed")
    changed = changed_protected_paths(
        root,
        historical_manifest["base_commit"],
        historical_manifest["protected_execution_paths"],
        C3_FINAL_COMMIT,
    )
    changed = [
        path
        for path in changed
        if path not in historical_manifest["allowed_protected_changes"]
    ]
    if changed:
        failures.append("C3 routes execution through the artifact or changes production execution: " + ", ".join(changed))
    production_uses = subprocess.run(
        [
            "git",
            "grep",
            "-n",
            "-E",
            "crate::artifact::ProgramArtifact|artifact::ProgramArtifact",
            C3_FINAL_COMMIT,
            "--",
            "src/engine/src",
            "src/runtime/src",
        ],
        cwd=root,
        text=True,
        capture_output=True,
    )
    if production_uses.returncode not in (0, 1):
        failures.append("unable to scan production ProgramArtifact uses")
    elif production_uses.stdout.strip():
        failures.append("ProgramArtifact is routed into production execution: " + production_uses.stdout.strip())
    return failures


def main() -> int:
    failures = run()
    if failures:
        for failure in failures:
            print(f"C3 contract failure: {failure}", file=sys.stderr)
        return 1
    print("C3 ProgramArtifact and bytecode-v1 boundary: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
