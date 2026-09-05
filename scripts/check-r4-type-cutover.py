#!/usr/bin/env python3
"""Enforce the permanent R4 semantic-type authority cutover."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "src/core/src/type_system/resolved_value.rs",
    "src/core/src/function/specialization.rs",
    "src/core/src/function/catalog.rs",
    "src/core/src/cell_binding.rs",
    "src/core/src/program/compiler/context.rs",
    "src/core/src/function/resident.rs",
    "src/engine/src/function/resolver.rs",
    "src/engine/src/artifact/compiler.rs",
    "src/engine/src/program/compiler_planning.rs",
    "src/engine/src/resident/general/mod.rs",
    "src/build/src/analysis/bytecode.rs",
    "src/build/src/plan/model.rs",
    "src/core/tests/r4_type_cutover.rs",
    "src/engine/tests/r4_type_cutover.rs",
    "src/stdlib/tests/r4_type_cutover.rs",
    "docs/design/r4-type-system-cutover.md",
    "docs/design/type-system-v1.md",
    "docs/design/type-memory-boundary.md",
    "docs/design/ROADMAP.mec",
    "docs/design/v0.4-endgame.md",
    ".github/workflows/ci.yml",
    ".github/workflows/ci-full.yml",
)
PRESENCE = {
    "src/core/src/type_system/resolved_value.rs": (
        "ResolvedValueDescriptor", "ResolvedOutputSchemaRule",
    ),
    "src/core/src/function/specialization.rs": (
        "ResolvedOperationDescriptor", "BoundCall", "BoundImplementationId", "ArtifactOperation",
    ),
    "src/core/src/function/catalog.rs": (
        "RuntimeFamilyId", "RuntimeBindingSelector",
        "bind_resolved_invocation", "runtime_entries_for_binding",
    ),
    "src/core/src/cell_binding.rs": (
        "allocate_for_descriptor", "validate_descriptor",
    ),
    "src/core/src/program/compiler/context.rs": (
        "instruction_type_bindings", "register_type_descriptors",
    ),
    "src/build/src/analysis/bytecode.rs": (
        "validate_bound_call_for_target", "instruction_type_bindings",
    ),
}
FORBIDDEN = (
    "with_resolved_output_type",
    "representation_supports_resolved_type",
    "runtime_output_rank",
    "inferred_output_dimensions",
    "bind_runtime_factory_derived_output",
    "entry.name.starts_with(name_prefix)",
    "SpecializedFunction::into_instance",
    "MechFunctionFactory::OUTPUT_SCHEMA_RULE",
    "fallback_operation_memory_contract",
    "RuntimeOperationContractAuthority::ImplementationDeclared",
    "default_for_representation",
    "RuntimeFunctionId::from_name(&canonical_operation)",
    "bind_physical_invocation_unchecked",
    "RuntimeFamilyId::from_name(&name)",
    "begin_plan_node_with_semantics",
    "begin_plan_node_with_contract",
)
MAINTAINED_RUNTIME_ROOTS = (
    "src/stdlib", "machines/combinatorics", "machines/compare", "machines/logic",
    "machines/math", "machines/matrix", "machines/range", "machines/set",
    "machines/stats", "machines/string",
)
TEST_ONLY_ALLOWLIST = {"src/stdlib/tests/r4_type_cutover.rs"}


def production_rust_files(root: Path):
    for base in ("src", "machines"):
        directory = root / base
        if not directory.exists():
            continue
        for path in directory.rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            if "/tests/" in relative or relative.endswith("/tests.rs"):
                continue
            yield relative, path.read_text(encoding="utf-8")


def failures(root: Path) -> list[str]:
    root = root.resolve()
    found: list[str] = []
    sources: dict[str, str] = {}
    for relative in REQUIRED:
        path = root / relative
        if not path.is_file():
            found.append(f"required file is missing: {relative}")
            sources[relative] = ""
        else:
            sources[relative] = path.read_text(encoding="utf-8")

    for relative, names in PRESENCE.items():
        source = sources.get(relative, "")
        for name in names:
            if not re.search(rf"\b{re.escape(name)}\b", source):
                found.append(f"{relative}: required R4 authority {name} is missing")

    for relative, source in production_rust_files(root):
        for forbidden in FORBIDDEN:
            if forbidden in source:
                found.append(f"{relative}: retired R4 path remains: {forbidden}")
        if relative != "src/core/src/cell_binding.rs" and re.search(
            r"\ballocate_backing_for_representation\s*\(", source
        ):
            found.append(f"{relative}: bypasses descriptor-directed allocation")

    for relative in MAINTAINED_RUNTIME_ROOTS:
        path = root / relative
        if not path.exists():
            continue
        for rust in path.rglob("*.rs"):
            if rust.relative_to(root).as_posix() in TEST_ONLY_ALLOWLIST:
                continue
            if "CompilerResolved(" in rust.read_text(encoding="utf-8"):
                found.append(
                    f"{rust.relative_to(root).as_posix()}: maintained runtime uses CompilerResolved"
                )

    type_system = root / "src/core/src/type_system"
    if type_system.exists():
        for rust in type_system.rglob("*.rs"):
            if "FunctionValueRepresentation" in rust.read_text(encoding="utf-8"):
                found.append(
                    f"{rust.relative_to(root).as_posix()}: pure type system imports physical representation"
                )

    for relative in ("machines", "src/engine/src"):
        path = root / relative
        if path.exists():
            for rust in path.rglob("*.rs"):
                if "/tests/" in rust.as_posix():
                    continue
                if "allocate_backing_for_representation" in rust.read_text(encoding="utf-8"):
                    found.append(
                        f"{rust.relative_to(root).as_posix()}: source path allocates from representation"
                    )

    engine_source = root / "src/engine/src"
    if engine_source.exists():
        for rust in engine_source.rglob("*.rs"):
            if ".register_instance(" in rust.read_text(encoding="utf-8"):
                found.append(
                    f"{rust.relative_to(root).as_posix()}: executable plan node drops BoundCall"
                )

    for relative, source in production_rust_files(root):
        for declaration in re.finditer(
            r"declare_native_runtime_factory!\s*\{(?P<body>.*?)\n\s*\}",
            source,
            re.DOTALL,
        ):
            body = declaration.group("body")
            if "operations:" not in body and "compiler_family:" not in body:
                found.append(
                    f"{relative}: native runtime factory omits explicit operation/family authority"
                )

    compiler = sources.get("src/engine/src/artifact/compiler.rs", "")
    if re.search(r"let Some\(binding\) = binding else \{\s*continue;", compiler):
        found.append("artifact compiler still treats executable type bindings as optional")
    if "input_registers.len() != binding.inputs().len()" not in compiler:
        found.append("artifact compiler does not reject semantic input arity mismatches")
    for independent in (
        "implementation().semantic_operation_name()",
        "implementation().semantic_operation_contract()",
        "step.semantic_operation_name()",
        "step.semantic_operation_contract()",
    ):
        if independent in compiler:
            found.append(
                "artifact compiler obtains semantic authority independently of BoundCall"
            )

    compiler_planning = sources.get("src/engine/src/program/compiler_planning.rs", "")
    if "begin_plan_node_with_type_binding" not in compiler_planning:
        found.append("compiler planning does not require a BoundCall for executable nodes")
    if "step.semantic_operation_name()" in compiler_planning or (
        "step.semantic_operation_contract()" in compiler_planning
    ):
        found.append("compiler planning supplies semantic sidecars independently of BoundCall")

    compiler_context = sources.get("src/core/src/program/compiler/context.rs", "")
    if "type_binding: &crate::BoundCall" not in compiler_context or (
        "type_binding.operation_descriptor()" not in compiler_context
    ):
        found.append("compiler context does not derive operation semantics from BoundCall")

    specialization = sources.get("src/core/src/function/specialization.rs", "")
    if specialization.count("operation: ResolvedOperationDescriptor") < 2:
        found.append("ResolvedCall and BoundCall do not share one operation descriptor")
    if "OperationId::from_name(&canonical_name) != id" not in specialization:
        found.append("operation descriptors do not validate canonical-name identity")
    if "instance.implementation().semantic_operation_contract()" in specialization:
        found.append("instance certification trusts an implementation semantic contract")

    resident = sources.get("src/engine/src/resident/general/mod.rs", "")
    if "BoundCall::artifact_operation" not in resident:
        found.append("resident activation does not bind the actual artifact operation")

    native = sources.get("src/build/src/analysis/bytecode.rs", "")
    if "validate_bound_call_for_target(binding, ExecutionTarget::Native)" not in native:
        found.append("native planning does not consume and validate semantic type bindings")

    catalog = sources.get("src/core/src/function/catalog.rs", "")
    if "F::declared_operation_contract()" not in catalog:
        found.append("fixed runtime registration does not require a preconstruction contract")
    if catalog.find("check_operation_memory_contract(operation_contract)") > catalog.find(
        "construct_validated_physical_invocation(invocation)"
    ):
        found.append("operation-memory validation occurs after factory construction")
    if "FunctionCatalogDuplicateRuntimeCapability" not in catalog:
        found.append("catalog does not reject identical operation/target/signature candidates")

    docs = "\n".join(
        sources.get(path, "")
        for path in (
            "docs/design/type-system-v1.md",
            "docs/design/type-memory-boundary.md",
            "docs/design/ROADMAP.mec",
            "docs/design/v0.4-endgame.md",
        )
    ).lower()
    if "shadow-only" in docs or "shadow mode" in docs:
        found.append("documentation still describes R2 compatibility as shadow-only")

    for workflow in (".github/workflows/ci.yml", ".github/workflows/ci-full.yml"):
        source = sources.get(workflow, "")
        if "python3 scripts/check-r4-type-cutover.py" not in source:
            found.append(f"{workflow}: does not run the R4 checker")
        if "scripts/tests/test_check_r4_type_cutover.py" not in source:
            found.append(f"{workflow}: does not run the R4 checker tests")

    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if found:
        print("R4 type cutover check failed:", file=sys.stderr)
        for failure in found:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("R4 type cutover check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
