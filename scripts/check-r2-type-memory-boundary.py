#!/usr/bin/env python3
"""Check the separation of type-memory contracts from physical storage and wire formats."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Pattern


sys.path.insert(0, str(Path(__file__).resolve().parent))
from rust_source import rust_code


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
WIRE_ROOTS = (
    "src/core/src/schema/encoding.rs", "src/core/src/operation_contract/encoding.rs",
    "src/core/src/program/bytecode", "src/bytecode", "src/engine/src/artifact", "src/abi",
)

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






def failures(root: Path) -> list[str]:
    root = root.resolve()
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
    serialization = re.compile(
        r"\b(?:encode|decode|serialize|deserialize|to_bytes|from_bytes|canonical_bytes)\w*\s*\("
        r"|\b(?:Serialize|Deserialize|serde)\b"
    )
    for relative, code in production:
        for match in re.finditer(r"\b(?:impl|fn)\b[^;{]*\{", code):
            end = _brace_end(code, match.end() - 1)
            item = code[match.start() : end + 1 if end is not None else match.end()]
            if wire_pattern.search(item) and serialization.search(item):
                found.append(f"{relative}: R2 serialization implementation outside the wire roots")
                break

    argument_source = sources["src/core/src/function/argument.rs"]
    alias = extract_item_body(argument_source, re.compile(r"\bfn\s+check_operation_output_alias\s*\([^)]*\)")) or ""
    _require(
        alias,
        r"\bsame_writable_storage\b",
        "operation alias checker does not use same_writable_storage",
        found,
    )
    for forbidden_alias in ("same_logical_cell", "same_cell", "reactive_cell_id", "CanonicalCellId", "ptr_eq"):
        if re.search(rf"\b{forbidden_alias}\b", alias):
            found.append(f"operation alias checker uses forbidden identity {forbidden_alias}")

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
