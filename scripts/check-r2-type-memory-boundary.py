#!/usr/bin/env python3
"""Enforce the permanent R2 type-memory boundary and its R4 authority use."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Pattern


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "src/core/src/lib.rs",
    "src/core/src/memory_contract/mod.rs",
    "src/core/src/memory_contract/type_contract.rs",
    "src/core/src/memory_contract/storage_capability.rs",
    "src/core/src/memory_contract/operation_requirement.rs",
    "src/core/src/runtime_storage.rs",
    "src/core/src/schema/mod.rs",
    "src/core/src/cell_binding.rs",
    "src/core/src/function/argument.rs",
    "src/core/tests/type_memory_boundary.rs",
    "docs/design/type-memory-boundary.md",
    "docs/design/ROADMAP.mec",
    "docs/design/v0.4-endgame.md",
    "README.md",
    ".github/workflows/ci.yml",
    ".github/workflows/ci-full.yml",
    ".github/ci/owners.toml",
)
R2_IDENTIFIERS = (
    "TypeMemoryContract", "ResolvedTypeMemoryContract", "StorageCapabilityDescriptor",
    "StorageTopology", "StorageExtentCapability", "StorageCompatibilityError",
    "SchemaStorageCompatibilityError", "PortMemoryRequirement",
    "OperationMemoryRequirements", "PortStorageCompatibilityError",
    "OwnershipRequirement", "AddressingRequirement", "PublicationRequirement",
)
TRANSITIONAL = ("FunctionValueRepresentation", "FunctionRuntimeType", "FunctionMatrixRepresentation",
    "FunctionMatrixStoragePattern", "FunctionMatrixElement")
R2_DOCS = ("README.md", "docs/design/type-memory-boundary.md", "docs/design/ROADMAP.mec",
    "docs/design/v0.4-endgame.md")
WIRE_ROOTS = (
    "src/core/src/schema/encoding.rs", "src/core/src/operation_contract/encoding.rs",
    "src/core/src/program/bytecode", "src/bytecode", "src/engine/src/artifact", "src/abi",
)
CONFORMANCE = (
    "canonical_storage_is_universal_mechanics_not_universal_semantics",
    "semantic_addressing_precedes_backing_addressing",
    "exact_backings_preserve_kind_extent_and_evolution",
    "port_capability_failures_remain_structured",
    "declared_requirements_preserve_delivery_without_target_policy",
    "shadow_invocation_validation_is_complete_and_pure",
    "logical_value_cell_and_storage_identity_are_distinct",
    "inferred_vector_fixed_axis_mismatches_remain_owned_by_r4",
    "r2_analysis_is_deterministic_non_mutating_and_non_serialized",
)
RAW_LITERAL = re.compile(r'(?:br|rb|r)(?P<hashes>#{0,255})"')


def rust_code(source: str) -> str:
    """Blank Rust comments and literals while retaining code and newlines."""
    out = list(source)
    size = len(source)

    def blank(start: int, end: int) -> None:
        for index in range(start, end):
            if out[index] not in "\r\n":
                out[index] = " "

    index = 0
    while index < size:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < size and depth:
                if source.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif source.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            blank(index, end)
            index = end
            continue
        raw = RAW_LITERAL.match(source, index)
        if raw:
            delimiter = '"' + raw.group("hashes")
            end = source.find(delimiter, raw.end())
            end = size if end < 0 else end + len(delimiter)
            blank(index, end)
            index = end
            continue
        prefix = 1 if source.startswith(('b"', "b'"), index) else 0
        quote_index = index + prefix
        if quote_index < size and source[quote_index] == '"':
            end, escaped = quote_index + 1, False
            while end < size:
                char = source[end]
                end += 1
                if char == '"' and not escaped:
                    break
                escaped = char == "\\" and not escaped
                if char != "\\":
                    escaped = False
            blank(index, end)
            index = end
            continue
        if quote_index < size and source[quote_index] == "'":
            value = quote_index + 1
            end = value + 1
            if value < size and source[value] == "\\":
                if source.startswith("\\x", value):
                    end = value + 4
                elif source.startswith("\\u{", value):
                    close = source.find("}", value + 3)
                    end = size if close < 0 else close + 1
                else:
                    end = value + 2
            if end < size and source[end] == "'":
                blank(index, end + 1)
                index = end + 1
                continue
        index += 1
    return "".join(out)


def line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def _brace_end(code: str, opening: int) -> int | None:
    depth = 0
    for index in range(opening, len(code)):
        if code[index] == "{":
            depth += 1
        elif code[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    return None


def extract_item_body(source: str, item_pattern: Pattern[str]) -> str | None:
    code = rust_code(source)
    match = item_pattern.search(code)
    if match is None:
        return None
    opening = code.find("{", match.end())
    if opening < 0:
        return None
    end = _brace_end(code, opening)
    return None if end is None else code[opening + 1 : end]


def strip_cfg_test_modules(source: str) -> str:
    code = rust_code(source)
    out = list(code)
    cursor = 0
    while True:
        start = code.find("#[", cursor)
        if start < 0:
            break
        close = code.find("]", start + 2)
        if close < 0:
            break
        attribute = code[start : close + 1]
        cursor = close + 1
        if not re.search(r"\bcfg\b", attribute) or not re.search(r"\btest\b", attribute):
            continue
        module = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", code[close + 1 :])
        if module is None:
            continue
        opening = code.find("{", close + 1, close + 1 + module.end())
        end = _brace_end(code, opening)
        if end is None:
            break
        for index in range(start, end + 1):
            if out[index] not in "\r\n":
                out[index] = " "
        cursor = end + 1
    return "".join(out)


def _read(root: Path, relative: str, found: list[str]) -> str:
    path = root / relative
    if not path.is_file():
        found.append(f"required file is missing: {relative}")
        return ""
    return path.read_text(encoding="utf-8")


def _require(source: str, pattern: str, diagnostic: str, found: list[str]) -> None:
    if re.search(pattern, source, re.MULTILINE | re.DOTALL) is None:
        found.append(diagnostic)


def _rust_files(root: Path, relative: str):
    path = root / relative
    if path.is_file() and path.suffix == ".rs":
        yield path
    elif path.is_dir():
        yield from path.rglob("*.rs")


def _declared_item_bodies(source: str):
    code = rust_code(source)
    pattern = re.compile(r"\b(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+\w+")
    for match in pattern.finditer(code):
        opening = code.find("{", match.end())
        semicolon = code.find(";", match.end())
        if opening < 0 or 0 <= semicolon < opening:
            continue
        end = _brace_end(code, opening)
        if end is not None:
            yield code[opening + 1 : end]


def _job(source: str, name: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)", source)
    return "" if match is None else match.group(1)


def _step_containing(block: str, marker: str) -> str:
    offset = block.find(marker)
    if offset < 0:
        return ""
    start = block.rfind("\n      - ", 0, offset)
    end = block.find("\n      - ", offset)
    return block[start if start >= 0 else 0 : end if end >= 0 else len(block)]


def failures(root: Path) -> list[str]:
    root = root.resolve()
    r4_active = (root / "scripts/check-r4-type-cutover.py").is_file()
    found: list[str] = []
    sources = {relative: _read(root, relative, found) for relative in REQUIRED}
    lib = rust_code(sources["src/core/src/lib.rs"])
    _require(lib, r"\bpub\s+mod\s+memory_contract\s*;", "memory_contract is not public", found)
    _require(lib, r"\bpub\(crate\)\s+mod\s+runtime_storage\s*;", "runtime_storage is not crate-private", found)
    if re.search(r"\bpub\s+mod\s+runtime_storage\s*;", lib):
        found.append("runtime_storage must not be public")

    module = rust_code(sources["src/core/src/memory_contract/mod.rs"])
    for name in ("type_contract", "storage_capability", "operation_requirement"):
        _require(module, rf"\bmod\s+{name}\s*;", f"memory_contract does not declare {name}", found)
        _require(module, rf"\bpub\s+use\s+(?:self::)?{name}::\*\s*;", f"memory_contract does not publicly re-export {name}", found)

    memory_files = list(_rust_files(root, "src/core/src/memory_contract"))
    forbidden = TRANSITIONAL + ("CanonicalCellId", "CellBinding",
        "ErasedCellStorage", "ValueCell", "Ref", "Rc", "Arc", "unsafe", "Serialize", "Deserialize", "serde")
    target = re.compile(r"\b(?:Resident|Gpu|GPU|Native|Wasm|WebAssembly)\w*\b")
    fields = ("byte_offset", "byte_offsets", "offset_bytes", "byte_size", "size_bytes", "alignment",
        "align", "stride", "strides", "capacity", "allocator", "arena", "pool", "allocation",
        "allocation_class", "pointer", "ptr", "address", "reference_count", "ref_count",
        "owner_count", "placement", "device", "backend", "reuse", "reuse_class", "copy_on_write",
        "cow", "lifetime_interval")
    physical_names = ("Layout", "Offset", "Alignment", "Stride", "Allocation", "Allocator", "Arena",
        "Pool", "Pointer", "Placement", "ReusePlan", "LifetimePlan")
    for path in memory_files:
        relative = path.relative_to(root).as_posix()
        code = rust_code(path.read_text(encoding="utf-8"))
        for identifier in forbidden:
            match = re.search(rf"\b{identifier}\b", code)
            if match:
                found.append(f"{relative}:{line_number(code, match.start())}: forbidden memory-contract identifier {identifier}")
        if re.search(r"#\s*\[\s*repr\s*\(", code):
            found.append(f"{relative}: forbidden repr memory-contract declaration")
        for match in target.finditer(code):
            found.append(f"{relative}:{line_number(code, match.start())}: target-specific memory-contract identifier {match.group()}")
        declared = "\n".join(_declared_item_bodies(code))
        for field in fields:
            if re.search(rf"\b{field}\s*:", declared):
                found.append(f"{relative}: physical-layout field {field}")
        for match in re.finditer(r"\bpub(?:\([^)]*\))?\s+(?:struct|enum|type|fn)\s+(\w+)", code):
            if any(term in match.group(1) for term in physical_names):
                found.append(f"{relative}: physical-plan public name {match.group(1)}")
        if path.name in ("encoding.rs", "codec.rs", "serde.rs"):
            found.append(f"memory_contract contains wire-format file {relative}")

    wire_pattern = re.compile(r"\b(?:" + "|".join(R2_IDENTIFIERS) + r")\b")
    for relative in WIRE_ROOTS:
        for path in _rust_files(root, relative):
            code = rust_code(path.read_text(encoding="utf-8"))
            match = wire_pattern.search(code)
            if match:
                found.append(f"{path.relative_to(root).as_posix()}: R2 identifier leaks into wire-format code: {match.group()}")

    production: list[tuple[str, str]] = []
    for base in ("src", "machines", "hosts"):
        for path in _rust_files(root, base):
            relative = path.relative_to(root).as_posix()
            if "tests" not in path.relative_to(root).parts:
                production.append((relative, strip_cfg_test_modules(path.read_text(encoding="utf-8"))))
    serialization = re.compile(r"\b(?:encode|decode|serialize|deserialize|to_bytes|from_bytes|canonical_bytes)\w*\b|\b(?:Serialize|Deserialize|serde)\b")
    for relative, code in production:
        for match in re.finditer(r"\b(?:impl|fn)\b[^;{]*\{", code):
            end = _brace_end(code, match.end() - 1)
            item = code[match.start() : end + 1 if end is not None else match.end()]
            if wire_pattern.search(item) and serialization.search(item):
                found.append(f"{relative}: R2 serialization implementation outside the wire roots")
                break

    schema = extract_item_body(sources["src/core/src/schema/mod.rs"], re.compile(r"\bimpl\s+Schema\b")) or ""
    for method in ("type_memory_contract", "resolved_type_memory_contract"):
        _require(schema, rf"\bpub\s+fn\s+{method}\s*\(", f"Schema::{method} is missing", found)
    if any(re.search(rf"\b{identifier}\b", schema) for identifier in TRANSITIONAL):
        found.append("Schema type-memory projection uses a transitional runtime representation")
    storage = rust_code(sources["src/core/src/memory_contract/storage_capability.rs"])
    safe_signature = r"pub\s+fn\s+check_schema_storage_compatibility\s*\(\s*schema\s*:\s*&Schema\s*,\s*shape\s*:\s*&ShapeInstance"
    _require(storage, safe_signature, "safe schema-bound storage checker is missing", found)
    _require(storage, r"pub\(crate\)\s+fn\s+check_resolved_type_storage_compatibility", "low-level resolved checker is not crate-private", found)
    if re.search(r"(?<!\))\bpub\s+fn\s+check_resolved_type_storage_compatibility", storage):
        found.append("low-level resolved checker is public")
    operation_source = sources["src/core/src/memory_contract/operation_requirement.rs"]
    operation = rust_code(operation_source)
    port_signature = (r"pub\s+fn\s+check_port_storage_compatibility\s*\(\s*schema\s*:\s*&Schema\s*,"
        r"\s*shape\s*:\s*&ShapeInstance\s*,\s*requirement\s*:\s*&PortMemoryRequirement\s*,"
        r"\s*storage\s*:\s*&StorageCapabilityDescriptor")
    _require(operation, port_signature, "public port checker does not accept the complete compatibility triangle", found)
    for marker in ("SemanticAddressingUnsupported", "check_semantic_addressing"):
        _require(operation, rf"\b{marker}\b", f"semantic-addressing stage lost {marker}", found)

    cell_source = sources["src/core/src/cell_binding.rs"]
    cell_code = rust_code(cell_source)
    public_item = re.compile(r"\bpub(?:\([^)]*\))?\s+(?:struct|enum|type|fn)\s+(\w+)")
    for relative, code in production:
        for match in public_item.finditer(code):
            words = {word.lower() for word in re.findall(r"[A-Z]+(?=[A-Z][a-z]|$)|[A-Z]?[a-z]+", match.group(1))}
            if words & {"storage", "physical", "backing"} and words & {"id", "identity", "token", "key", "pointer", "ptr", "address"}:
                found.append(f"{relative}: public physical-storage identity API {match.group(1)}")
    erased = extract_item_body(cell_source, re.compile(r"\btrait\s+ErasedCellStorage\b")) or ""
    for method in ("capabilities", "same_storage", "detached_clone"):
        _require(erased, rf"\bfn\s+{method}\b", f"ErasedCellStorage lost {method}", found)
    if re.search(r"\blogical_cell_id\b", erased):
        found.append("ErasedCellStorage revived logical_cell_id")
    detached = extract_item_body(cell_source, re.compile(r"\bstruct\s+DetachedCellStorage\b")) or ""
    _require(detached, r"\bidentity\s*:\s*CanonicalCellId\b", "DetachedCellStorage lacks explicit CanonicalCellId", found)
    _require(detached, r"\bstorage\s*:\s*Rc\s*<\s*dyn\s+ErasedCellStorage\s*>", "DetachedCellStorage lacks erased storage", found)
    value_cell = extract_item_body(cell_source, re.compile(r"\bimpl\s+ValueCell\b")) or ""
    for method in ("type_memory_contract", "resolved_type_memory_contract", "storage_capabilities",
        "validate_storage_contract", "same_logical_cell", "same_storage", "same_cell"):
        _require(value_cell, rf"\bpub\s+fn\s+{method}\s*\(", f"ValueCell::{method} is missing", found)
    same_cell = extract_item_body(value_cell, re.compile(r"\bfn\s+same_cell\s*\([^)]*\)"))
    normalized = re.sub(r"\s+", "", same_cell or "")
    if normalized not in ("self.same_storage(other)", "self.same_storage(other);", "returnself.same_storage(other);"):
        found.append("same_cell must delegate exactly to same_storage")
    validation = extract_item_body(value_cell, re.compile(r"\bfn\s+validate_storage_contract\s*\([^)]*\)")) or ""
    if r4_active:
        _require(validation, r"\bvalidate_storage_compatibility\b", "ValueCell authoritative validation bypasses the safe storage bridge", found)
        bridge = extract_item_body(cell_source, re.compile(r"\bfn\s+validate_storage_compatibility\s*\([^)]*\)")) or ""
        _require(bridge, r"\bcheck_schema_storage_compatibility\b", "ValueCell safe storage bridge bypasses the schema checker", found)
    else:
        _require(validation, r"\bcheck_schema_storage_compatibility\b", "ValueCell shadow validation bypasses the safe schema checker", found)

    for marker in ("PortMemoryRequirement", "OperationMemoryRequirements"):
        _require(operation, rf"\b{marker}\b", f"operation requirements lost {marker}", found)
    declaration_impl = extract_item_body(operation_source, re.compile(r"\bimpl\s+OperationContractDeclaration\b")) or ""
    _require(declaration_impl, r"\bpub\s+fn\s+memory_requirements\s*\(", "OperationContractDeclaration::memory_requirements is missing", found)
    requirements = extract_item_body(declaration_impl, re.compile(r"\bfn\s+memory_requirements\s*\([^)]*\)")) or ""
    _require(requirements, r"self\s*\.\s*inputs\s*\.\s*resolve\s*\(\s*input_count\s*\)", "memory_requirements bypasses InputPortLayout::resolve", found)
    port = extract_item_body(operation_source, re.compile(r"\bstruct\s+PortMemoryRequirement\b")) or ""
    for policy in ("AccessMode", "DeliveryMode", "OutputConstruction", "AliasPolicy", "ChangeDetectionPolicy"):
        _require(port, rf"\b{policy}\b", f"derived port requirement lost {policy}", found)

    r4_call_allowance = {
        ("validate_storage_contract", "src/core/src/cell_binding.rs"): 5,
        ("check_operation_memory_contract", "src/core/src/function/catalog.rs"): 1,
        ("check_operation_memory_contract", "src/core/src/function/specialization.rs"): 2,
    }
    for method, expected_definitions in (("validate_storage_contract", 1), ("check_operation_memory_contract", 1)):
        definitions = sum(len(re.findall(rf"\bfn\s+{method}\s*\(", code)) for _, code in production)
        if definitions != expected_definitions:
            found.append(f"shadow method {method} must have exactly one production definition")
        for relative, code in production:
            calls = len(re.findall(rf"(?:\.|::)\s*{method}\b", code))
            allowed_calls = r4_call_allowance.get((method, relative), 0) if r4_active else 0
            if calls > allowed_calls:
                found.append(f"{relative}: production call or function-item reference to {method}")
    for relative, code in production:
        for match in re.finditer(r"\bcheck_port_storage_compatibility\b", code):
            allowed = relative == "src/core/src/memory_contract/operation_requirement.rs" or (
                relative == "src/core/src/function/argument.rs" and code[max(0, match.start() - 1000):match.start()].rfind("fn check_invocation_cell_requirement") >= 0
            )
            if not allowed:
                found.append(f"{relative}: unauthorized production use of check_port_storage_compatibility")
        schema_checks = len(re.findall(r"\bcheck_schema_storage_compatibility\b", code))
        allowed_schema_checks = len(re.findall(r"\bfn\s+check_schema_storage_compatibility\b", code)) \
            if relative == "src/core/src/memory_contract/storage_capability.rs" else 0
        if r4_active:
            allowed_schema_checks += {
                "src/core/src/cell_binding.rs": 1,
                "src/core/src/function/catalog.rs": 2,
                "src/core/src/function/specialization.rs": 1,
            }.get(relative, 0)
        elif relative == "src/core/src/cell_binding.rs":
            allowed_schema_checks += validation.count("check_schema_storage_compatibility")
        if schema_checks > allowed_schema_checks:
            found.append(f"{relative}: unauthorized production use of check_schema_storage_compatibility")

    argument_source = sources["src/core/src/function/argument.rs"]
    alias = extract_item_body(argument_source, re.compile(r"\bfn\s+check_operation_output_alias\s*\([^)]*\)")) or ""
    _require(alias, r"\bsame_storage\b", "operation alias checker does not use same_storage", found)
    for forbidden_alias in ("same_logical_cell", "same_cell", "reactive_cell_id", "CanonicalCellId", "ptr_eq"):
        if re.search(rf"\b{forbidden_alias}\b", alias):
            found.append(f"operation alias checker uses forbidden identity {forbidden_alias}")

    conformance = sources["src/core/tests/type_memory_boundary.rs"]
    expected_conformance = tuple(
        "inferred_vector_fixed_axes_are_authoritative_after_r4"
        if r4_active and marker == "inferred_vector_fixed_axis_mismatches_remain_owned_by_r4"
        else marker
        for marker in CONFORMANCE
    )
    for marker in expected_conformance + ("SemanticAddressingUnsupported", "DynamicAxisUnsupported", "same_logical_cell", "same_storage", "snapshot_eq"):
        if marker not in conformance:
            found.append(f"conformance suite is missing marker {marker}")
    design = sources["docs/design/type-memory-boundary.md"]
    design_markers = (
        ("Status: R2 complete", "authoritative", "RowDVector", "DVector", "R3", "R4", "R5", "R6")
        if r4_active else
        ("Status: R2 complete", "shadow-only", "RowDVector", "DVector", "DynamicAxisUnsupported", "R3", "R4", "R5", "R6")
    )
    for marker in design_markers:
        if marker not in design:
            found.append(f"type-memory documentation is missing {marker}")
    readme = re.sub(r"\s+", " ", sources["README.md"])
    sentence = "The canonical value-system cutover, R1 contract closure, and R2 type-memory boundary are complete."
    if sentence not in readme:
        found.append("README does not mark R2 complete")
    roadmap = sources["docs/design/ROADMAP.mec"]
    roadmap_markers = (
        ("Type–memory boundary: complete", "R4 authority cutover are complete", "R5 is")
        if r4_active else
        ("Type–memory boundary: complete", "Next endgame phase: R3")
    )
    for marker in roadmap_markers:
        if marker not in roadmap:
            found.append(f"ROADMAP is missing {marker}")
    endgame = re.sub(r"\s+", " ", sources["docs/design/v0.4-endgame.md"])
    if "## R2 closure" not in endgame:
        found.append("v0.4 endgame is missing R2 closure")
    stale = "The R2 boundary has not yet separated semantic identity from physical storage identity."
    if stale in endgame:
        found.append("v0.4 endgame retains stale R2 release blocker")
    for relative in ("README.md", "docs/design/ROADMAP.mec", "docs/design/v0.4-endgame.md"):
        if "0.3.6" not in sources[relative]:
            found.append(f"{relative} lost package version 0.3.6")

    r1, r2, unit = ("python3 scripts/check-r1-compatibility-closure.py",
        "python3 scripts/check-r2-type-memory-boundary.py", "scripts/tests/test_check_r2_type_memory_boundary.py")
    for relative, job_name in ((".github/workflows/ci.yml", "static-contracts"), (".github/workflows/ci-full.yml", "architecture-contracts")):
        block = _job(sources[relative], job_name)
        if not block:
            found.append(f"{relative} is missing {job_name}")
            continue
        if r1 not in block or r2 not in block:
            found.append(f"{relative} is missing the R1/R2 architecture checker sequence")
        elif block.index(r2) < block.index(r1):
            found.append(f"{relative} runs the R2 checker before the R1 checker")
        if unit not in block:
            found.append(f"{relative} is missing the R2 checker unit tests")
        if any("continue-on-error" in _step_containing(block, marker) for marker in (r2, unit)):
            found.append(f"{relative} waives the R2 architecture gate")
        if relative.endswith("ci-full.yml"):
            normalized_block = re.sub(r"\s+", " ", block)
            if not re.search(r"cargo \+nightly-2026-03-03 test .*--all-features .*--test type_memory_boundary", normalized_block):
                found.append("Full CI does not execute the R2 conformance target with all features")
    owner_match = re.search(r"(?ms)^\[owners\.architecture-contracts\]\n(.*?)(?=^\[owners\.|\Z)", sources[".github/ci/owners.toml"])
    owners = "" if owner_match is None else owner_match.group(1)
    for path in ("scripts/check-r2-type-memory-boundary.py", "scripts/tests/test_check_r2_type_memory_boundary.py") + R2_DOCS:
        if path not in owners:
            found.append(f"architecture owner entry is missing {path}")
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if not found:
        print("R2 type-memory boundary contract passed")
        return 0
    print("R2 type-memory boundary contract failed:", file=sys.stderr)
    for failure in found:
        print(f"  {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
