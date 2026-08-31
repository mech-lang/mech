#!/usr/bin/env python3
"""Enforce the focused C4 operation-contract architecture boundary."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/architecture/operation-contract/c4-boundary.json"
C4_FINAL_COMMIT = "33298522331d40960175427052ce363bb5e424df"
CONTRACT_SOURCES = (
    "src/core/src/operation_contract/declaration.rs",
    "src/core/src/operation_contract/resolved.rs",
    "src/core/src/operation_contract/validation.rs",
    "src/core/src/operation_contract/encoding.rs",
    "src/core/src/semantic_identity.rs",
)


def read(root: Path, path: str) -> str:
    return (root / path).read_text()


def read_at(root: Path, commit: str, path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{commit}:{path}"],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def type_is_declared(source: str, name: str) -> bool:
    return (
        re.search(rf"\b(?:struct|enum)\s+{re.escape(name)}\b", source) is not None
        or f"artifact_id!({name})" in source
    )


def named_block(source: str, kind: str, name: str) -> str | None:
    match = re.search(rf"\b{kind}\s+{re.escape(name)}\b[^{{]*{{", source)
    if match is None:
        return None
    depth = 1
    index = match.end()
    while index < len(source) and depth:
        depth += (source[index] == "{") - (source[index] == "}")
        index += 1
    return source[match.start():index] if depth == 0 else None


def validate_contract_sources(source: str, required_types: list[str]) -> list[str]:
    failures = [name for name in required_types if not type_is_declared(source, name)]
    errors = [f"missing frozen C4 contract type {name}" for name in failures]
    resolved = named_block(source, "enum", "ResolvedOperationContract") or ""
    declared = named_block(source, "struct", "DeclaredOperationContract") or ""
    portable = resolved + declared
    for token in (
        "RuntimeFunctionContract",
        "LegacyValue",
        "RuntimeFunctionId",
        "fn(",
        "dyn Fn",
        "FunctionPointer",
        "BufferStrategy",
        "StorageStrategy",
        "KernelId",
        "SelectedKernel",
        "capacity:",
        "stride:",
        "page_size:",
        "chunk_size:",
    ):
        if token in portable:
            errors.append(f"resolved artifact contract contains forbidden runtime/physical token {token}")
    return errors


def validate_table_boundary(source: str) -> list[str]:
    errors: list[str] = []
    declaration = re.search(
        r"((?:#\[[^\]]+\]\s*)*)pub struct OperationContractTable\b", source
    )
    if declaration is None:
        return ["missing finalized OperationContractTable declaration"]
    if "Deserialize" in declaration.group(1):
        errors.append("OperationContractTable must not derive Deserialize")

    implementation = named_block(source, "impl", "OperationContractTable") or ""
    if re.search(r"\bpub\s+fn\s+empty\s*\(\s*\)\s*->\s*Self\b", implementation) is None:
        errors.append("OperationContractTable lost public empty() constructor")
    unchecked_constructor = "from_" + "entries_unchecked"
    if re.search(
        rf"\bpub\(super\)\s+const\s+fn\s+{unchecked_constructor}\b", implementation
    ) is None:
        errors.append("OperationContractTable unchecked constructor must be pub(super)")

    raw_entry_constructor = "from_" + "canonical_entries"
    if re.search(rf"\b{raw_entry_constructor}\b", source):
        errors.append("OperationContractTable exposes a forbidden canonical raw-entry constructor")
    if re.search(
        rf"\bpub(?:\(crate\))?\s+(?:const\s+)?fn\s+{unchecked_constructor}\b", source
    ):
        errors.append("OperationContractTable unchecked constructor has public or pub(crate) visibility")
    if source.count(unchecked_constructor) != 2:
        errors.append("unchecked table construction must appear only in the table implementation and decoder")
    decoder_use = f"Self::{unchecked_constructor}(entries.into_boxed_slice())"
    if decoder_use not in source:
        errors.append("canonical decoder lost its sole unchecked table construction")
    return errors


def validate_semantic_guards(validation: str) -> list[str]:
    errors: list[str] = []
    for marker in ("AliasSchemaMismatch", "EffectOutputUnsupported", '"alias.input"'):
        if marker not in validation:
            errors.append(f"operation-contract validation lost semantic guard {marker}")
    if "value.trim() != value" not in validation:
        errors.append("shape-contract reference validation lost the canonical whitespace guard")
    if re.search(r"value\.contains\(\[\s*'\\0'\s*,\s*'/'\s*,\s*'\\\\'\s*\]\)", validation) is None:
        errors.append("shape-contract reference validation lost NUL, slash, or backslash rejection")
    return errors


def validate_artifact_fields(model: str, fields: dict[str, str]) -> list[str]:
    errors: list[str] = []
    for name, field in fields.items():
        block = named_block(model, "struct", name)
        if block is None:
            errors.append(f"missing artifact declaration {name}")
            continue
        field_name, field_type = field.split(":", 1)
        pattern = rf"\b{re.escape(field_name.strip())}\s*:\s*{re.escape(field_type.strip())}\b"
        if re.search(pattern, block) is None:
            errors.append(f"{name} lost required C4 field {field}")
    return errors


def validate_bytecode(section: str, reader: str, expected: str) -> list[str]:
    errors: list[str] = []
    for required in (
        "BYTECODE_SECTION_COUNT: usize = 18",
        "BYTECODE_CONTENT_OFFSET: u64 = 640",
        expected,
    ):
        if required not in section:
            errors.append(f"bytecode v1 lost C4 contract framing {required}")
    if expected not in reader:
        errors.append("bytecode v1 reader does not retain the operation-contract section")
    return errors


def validate_representatives(root: Path, representatives: list[dict[str, object]]) -> list[str]:
    errors: list[str] = []
    for representative in representatives:
        path = str(representative["path"])
        source = read(root, path)
        for marker in representative["markers"]:
            if str(marker) not in source:
                errors.append(f"representative declaration {path} lost {marker}")
    return errors


def validate_representatives_at(
    root: Path, commit: str, representatives: list[dict[str, object]]
) -> list[str]:
    errors: list[str] = []
    for representative in representatives:
        path = str(representative["path"])
        source = read_at(root, commit, path)
        for marker in representative["markers"]:
            if str(marker) not in source:
                errors.append(f"representative declaration {path} lost {marker}")
    return errors


def validate_canonical_contract_policy(compiler: str, fixture: str) -> list[str]:
    errors: list[str] = []
    if "MissingOperationContract" not in compiler:
        errors.append("canonical contract enforcement lost MissingOperationContract")
    if re.search(r"declaration\s*\.\s*as_ref\(\)", compiler) is None:
        errors.append("canonical contract enforcement lost declaration.as_ref()")
    test = named_block(
        fixture,
        "fn",
        "synthetic_ekf_contract_fixture_is_fully_declared_and_round_trips_contract_ids",
    ) or ""
    for marker in ("ResolvedOperationContract::Declared", ".all("):
        if marker not in test:
            errors.append(f"synthetic fully-declared artifact proof lost {marker}")
    return errors


def changed_protected_paths(
    root: Path, base: str, protected: list[str], head: str = "HEAD"
) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", base, head, "--", *protected],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def run(root: Path = ROOT) -> list[str]:
    manifest = json.loads((root / MANIFEST.relative_to(ROOT)).read_text())
    errors: list[str] = []
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", C4_FINAL_COMMIT, "HEAD"],
        cwd=root,
        text=True,
        capture_output=True,
    )
    if ancestor.returncode != 0:
        errors.append(f"C4 final commit {C4_FINAL_COMMIT} must be an ancestor of HEAD")
    contract_source = "\n".join(read(root, path) for path in CONTRACT_SOURCES)
    errors.extend(validate_contract_sources(contract_source, manifest["contract_types"]))
    errors.extend(validate_table_boundary(contract_source))
    errors.extend(validate_semantic_guards(
        read(root, "src/core/src/operation_contract/validation.rs")
    ))
    model = read(root, "src/engine/src/artifact/model.rs")
    errors.extend(validate_artifact_fields(model, manifest["artifact_contract_fields"]))
    section = read(root, "src/core/src/program/bytecode/section.rs")
    reader = read(root, "src/core/src/program/bytecode/reader.rs")
    errors.extend(validate_bytecode(section, reader, manifest["bytecode_section"]))
    errors.extend(
        validate_representatives_at(
            root, C4_FINAL_COMMIT, manifest["representative_declarations"]
        )
    )
    errors.extend(validate_canonical_contract_policy(
        read(root, "src/engine/src/artifact/compiler.rs"),
        read(root, "src/engine/tests/program_artifact_contract.rs"),
    ))
    changed = changed_protected_paths(
        root,
        manifest["base_commit"],
        manifest["protected_execution_paths"],
        C4_FINAL_COMMIT,
    )
    unapproved = [path for path in changed if path not in manifest["allowed_protected_changes"]]
    if unapproved:
        errors.append("C4 changes an unapproved production execution path: " + ", ".join(unapproved))
    diff = subprocess.run(
        [
            "git",
            "diff",
            manifest["base_commit"],
            C4_FINAL_COMMIT,
            "--",
            "src",
            "machines",
            "hosts",
        ],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    ).stdout
    additions = "\n".join(line[1:] for line in diff.splitlines() if line.startswith("+") and not line.startswith("+++"))
    if re.search(r"bytecode[ _-]?v2", additions, re.IGNORECASE):
        errors.append("C4 introduces forbidden bytecode-v2 work")
    return errors


def main() -> int:
    failures = run()
    if failures:
        for failure in failures:
            print(f"C4 contract failure: {failure}", file=sys.stderr)
        return 1
    print("C4 operation-contract checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
