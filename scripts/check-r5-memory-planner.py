#!/usr/bin/env python3
"""Enforce the permanent R5 deterministic memory-planner boundary."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "src/core/src/memory_plan/mod.rs",
    "src/core/src/memory_plan/model.rs",
    "src/core/src/memory_plan/target.rs",
    "src/core/src/memory_plan/derive.rs",
    "src/core/src/memory_plan/implementation.rs",
    "src/core/src/memory_plan/budget.rs",
    "src/core/src/memory_plan/error.rs",
    "src/core/src/program/bytecode/writer.rs",
    "src/core/src/program/compiler/context.rs",
    "src/core/src/function/catalog.rs",
    "src/core/src/function/specialization.rs",
    "src/engine/src/artifact/model.rs",
    "src/engine/src/artifact/compiler.rs",
    "src/engine/src/memory_planner/mod.rs",
    "src/engine/src/memory_planner/program.rs",
    "src/engine/src/memory_planner/turn.rs",
    "src/engine/src/memory_planner/resident.rs",
    "src/engine/src/memory_planner/audit.rs",
    "src/engine/src/resident/budget.rs",
    "src/engine/src/resident/general/execution.rs",
    "src/engine/src/resident/general/live.rs",
    "src/engine/src/resident/general/mod.rs",
    "src/engine/src/resident/matrix_literal.rs",
    "src/engine/src/resident/numeric.rs",
    "src/compute/src/memory.rs",
    "src/compute/src/program.rs",
    "hosts/gpu/src/execution_plan.rs",
    "hosts/gpu/src/memory.rs",
    "hosts/gpu/src/batched/mod.rs",
    "src/build/src/plan/model.rs",
    "src/core/tests/r5_memory_plan.rs",
    "src/engine/tests/r5_memory_plan.rs",
    "src/stdlib/tests/r5_memory_contract.rs",
    "src/compute/tests/r5_memory_plan.rs",
    "hosts/gpu/tests/r5_memory_plan.rs",
    "scripts/check-r5-memory-planner.py",
    "scripts/tests/test_check_r5_memory_planner.py",
    "docs/design/r5-memory-planner.md",
    "docs/design/type-memory-boundary.md",
    "docs/design/r4-type-system-cutover.md",
    "docs/design/ROADMAP.mec",
    "docs/design/v0.4-endgame.md",
    "README.md",
    ".github/ci/owners.toml",
    ".github/workflows/ci.yml",
    ".github/workflows/ci-full.yml",
    "Cargo.toml",
)

PLAN_AUTHORITIES = (
    "ProgramMemoryPlanTemplate",
    "ProgramMemoryPlan",
    "TurnMemoryPlan",
    "CallMemoryPlan",
    "ValueLayoutPlan",
    "CapacityRequirement",
    "TargetMemoryProfile",
    "ImplementationMemoryClass",
    "MemoryLifetime",
    "AliasDecision",
    "ReuseGroupId",
    "TransactionRequirement",
    "TransferPlan",
    "MemoryPlanAuditReport",
    "plan_call_memory",
    "plan_program_memory_template",
    "instantiate_program_memory_plan",
    "plan_turn_memory",
    "GpuBackingMemoryPlan",
    "plan_scalar_instruction_expansion",
)

PLANNER_ROOTS = (
    "src/core/src/memory_plan",
    "src/engine/src/memory_planner",
    "src/compute/src/memory.rs",
    "hosts/gpu/src/memory.rs",
)
PRODUCTION_ROOTS = (
    "src/core/src",
    "src/engine/src",
    "src/compute/src",
    "src/stdlib/src",
    "hosts/gpu/src",
    "machines",
)
WIRE_TYPES = {
    "src/core/src/program/bytecode/writer.rs": "BytecodeProgram",
    "src/engine/src/artifact/model.rs": "ProgramArtifact",
    "src/build/src/plan/model.rs": "NativeBuildPlan",
    "hosts/gpu/src/execution_plan.rs": "GpuExecutionPlan",
}
R5_PLAN_NAMES = (
    "ProgramMemoryPlanTemplate",
    "ProgramMemoryPlan",
    "TurnMemoryPlan",
    "CallMemoryPlan",
    "ValueLayoutPlan",
    "AllocationPlan",
    "ArenaPlan",
    "TransferPlan",
)
CORE_PLANNER_FORBIDDEN = (
    "ValueCell",
    "ProgramArtifact",
    "wgpu",
    "nalgebra",
    "Rc",
    "RefCell",
)
R6_FORBIDDEN = (
    "AllocationHandle",
    "AllocatorPool",
    "ArenaPool",
    "FreeList",
    "reclaim_allocation",
    "copy_on_write_backing",
    "replace_value_cell_backing",
)
RAW_LITERAL = re.compile(r'(?:br|rb|r)(?P<hashes>#{0,255})"')


def rust_code(source: str) -> str:
    """Blank comments and literals while retaining code coordinates."""
    output = list(source)
    size = len(source)

    def blank(start: int, end: int) -> None:
        for offset in range(start, end):
            if output[offset] not in "\r\n":
                output[offset] = " "

    index = 0
    while index < size:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        elif source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < size and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
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
        quote = index + prefix
        if quote < size and source[quote] == '"':
            end, escaped = quote + 1, False
            while end < size:
                character = source[end]
                end += 1
                if character == '"' and not escaped:
                    break
                escaped = character == "\\" and not escaped
                if character != "\\":
                    escaped = False
            blank(index, end)
            index = end
            continue
        if quote < size and source[quote] == "'":
            value, end = quote + 1, quote + 2
            if value < size and source[value] == "\\":
                end = value + (4 if source.startswith("\\x", value) else 2)
            if end < size and source[end] == "'":
                blank(index, end + 1)
                index = end + 1
                continue
        index += 1
    return "".join(output)


def balanced_body(source: str, declaration: str) -> str | None:
    match = re.search(rf"\b(?:pub\s+)?(?:struct|enum)\s+{re.escape(declaration)}\b", source)
    if match is None:
        return None
    start = source.find("{", match.end())
    if start < 0:
        return None
    depth = 1
    for index in range(start + 1, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1 : index]
    return None


def rust_files(root: Path, entries: tuple[str, ...]):
    seen: set[Path] = set()
    for entry in entries:
        path = root / entry
        candidates = path.rglob("*.rs") if path.is_dir() else (path,)
        for candidate in candidates:
            if candidate.is_file() and candidate not in seen:
                seen.add(candidate)
                yield candidate.relative_to(root).as_posix(), candidate.read_text(encoding="utf-8")


def impl_blocks(source: str, trait: str):
    code = rust_code(source)
    for match in re.finditer(rf"\bimpl\b[^{{;]*\b{re.escape(trait)}\b[^{{;]*\{{", code):
        start = code.find("{", match.start())
        depth = 1
        for index in range(start + 1, len(code)):
            if code[index] == "{":
                depth += 1
            elif code[index] == "}":
                depth -= 1
                if depth == 0:
                    yield code[start + 1 : index]
                    break


def function_bodies(source: str, prefix: str):
    code = rust_code(source)
    for match in re.finditer(rf"\bfn\s+(?P<name>{re.escape(prefix)}\w*)\b[^{{;]*\{{", code):
        start = code.find("{", match.start())
        depth = 1
        for index in range(start + 1, len(code)):
            if code[index] == "{":
                depth += 1
            elif code[index] == "}":
                depth -= 1
                if depth == 0:
                    yield match.group("name"), code[start + 1 : index]
                    break


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

    all_required = "\n".join(
        source
        for relative, source in sources.items()
        if relative.startswith(("src/", "hosts/")) and "/tests/" not in relative
    )
    for authority in PLAN_AUTHORITIES:
        if not re.search(rf"\b{re.escape(authority)}\b", all_required):
            found.append(f"required R5 authority is missing: {authority}")

    # 1. Plans are process-local and non-wire.
    for relative, source in rust_files(root, ("src/core/src/memory_plan", "src/engine/src/memory_planner")):
        code = rust_code(source)
        if re.search(r"\b(?:Serialize|Deserialize)\b", code):
            found.append(f"{relative}: R5 plan derives serialization")

    # 2-5. Existing wire/build schemas may not acquire plan fields.
    for relative, declaration in WIRE_TYPES.items():
        body = balanced_body(rust_code(sources.get(relative, "")), declaration)
        if body is None:
            found.append(f"{relative}: {declaration} declaration is missing")
            continue
        for plan_name in R5_PLAN_NAMES:
            if re.search(rf"\b{re.escape(plan_name)}\b", body):
                found.append(f"{relative}: {declaration} carries R5 plan field {plan_name}")

    # 6. BoundCall remains semantic/physical selection, not memory policy.
    bound = balanced_body(
        rust_code(sources.get("src/core/src/function/specialization.rs", "")), "BoundCall"
    )
    for field in ("layout", "capacity", "lifetime", "allocation", "reuse", "transaction", "transfer"):
        if bound is not None and re.search(rf"\b{field}\w*\s*:", bound, re.IGNORECASE):
            found.append(f"BoundCall carries forbidden memory field {field}")

    # 7. Shared vocabulary stays below runtime/backing implementations.
    for relative, source in rust_files(root, ("src/core/src/memory_plan",)):
        code = rust_code(source)
        for identifier in CORE_PLANNER_FORBIDDEN:
            if re.search(rf"\b{re.escape(identifier)}\b", code):
                found.append(f"{relative}: core memory planner imports or names {identifier}")

    # 8-10. Deterministic inputs only.
    for relative, source in rust_files(root, PLANNER_ROOTS):
        code = rust_code(source)
        if re.search(r"\bHashMap\b", code):
            found.append(f"{relative}: R5 planner uses nondeterministic HashMap")
        if re.search(r"\b(?:as_ptr|from_ptr|pointer_identity|CanonicalCellId|cell_identity)\b", code):
            found.append(f"{relative}: plan identity derives from pointer or cell identity")
        if re.search(r"\b(?:factory_name|runtime_factory_name)\b", code):
            found.append(f"{relative}: runtime factory name is used as memory policy")

    # 11. Implementation classes are a closed vocabulary.
    implementation = rust_code(sources.get("src/core/src/memory_plan/implementation.rs", ""))
    class_body = balanced_body(implementation, "ImplementationMemoryClass")
    for variant in ("Custom", "Unknown", "Opaque"):
        if class_body is not None and re.search(rf"\b{variant}\b", class_body):
            found.append(f"ImplementationMemoryClass has forbidden escape hatch {variant}")

    # 12. Every concrete runtime implementation declares its memory class.
    for relative, source in rust_files(root, PRODUCTION_ROOTS):
        for body in impl_blocks(source, "MechFunctionFactory"):
            if "implementation_memory_class" not in body:
                found.append(f"{relative}: MechFunctionFactory omits implementation_memory_class")

    # 13. Specialized functions cannot drop their call plan.
    specialization = rust_code(sources.get("src/core/src/function/specialization.rs", ""))
    specialized = balanced_body(specialization, "SpecializedFunction") or ""
    if not re.search(r"\bmemory_plan\s*:\s*CallMemoryPlan\b", specialized):
        found.append("SpecializedFunction omits CallMemoryPlan")
    if not re.search(r"fn\s+new\s*\([^)]*memory_plan\s*:\s*CallMemoryPlan", specialization, re.DOTALL):
        found.append("production SpecializedFunction constructor omits CallMemoryPlan")

    # 14. Executable compiler bindings retain the matching non-wire sidecar.
    context = sources.get("src/core/src/program/compiler/context.rs", "")
    for required in (
        "instruction_memory_plans",
        "instruction_memory_plans.len() != self.instructions.len()",
        ".push(self.current_node_memory_plan.clone())",
    ):
        if required not in context:
            found.append(f"compiler executable binding omits memory sidecar authority: {required}")

    # 15. Resident sizing is projected from a checked plan only.
    resident = rust_code(sources.get("src/engine/src/resident/general/mod.rs", ""))
    for forbidden in (
        "TypedResidentArena::allocate(",
        "ResidentArenaSizes::from_artifact",
        "ResidentArenaSizes::from_dimensions",
    ):
        if forbidden in resident:
            found.append(f"resident arena sizing bypasses R5 plan: {forbidden}")

    # 16. GPU Cartesian expansion is admitted before appending.
    batched = rust_code(sources.get("hosts/gpu/src/batched/mod.rs", ""))
    if "plan_scalar_instruction_expansion(" not in batched or "reserve_scalar_instructions(" not in batched:
        found.append("GPU scalar instructions can append without expansion planning")
    reserve = re.search(
        r"fn\s+reserve_scalar_instructions\b(?P<body>.*?)(?:\n\s*fn\s+emit\b)",
        batched,
        re.DOTALL,
    )
    if reserve is None or reserve.group("body").find("plan_scalar_instruction_expansion") > reserve.group("body").find("try_reserve_exact"):
        found.append("GPU instruction reserve occurs before scalar-expansion validation")
    for required in (
        "scalar_instruction_work",
        "sum_products_work(left.elements())",
        "sum_products_work(left.columns)",
    ):
        if required not in batched:
            found.append(f"GPU reduction expansion omits complete pre-allocation work: {required}")

    # 17. Adapter limits remain external facts.
    gpu_memory = rust_code(sources.get("hosts/gpu/src/memory.rs", ""))
    if re.search(r"\bmax_(?:buffer_size|storage_buffer_binding_size)\s*:\s*[1-9][0-9_]*", gpu_memory):
        found.append("GPU memory planner hardcodes an adapter buffer limit")

    # 18. Resident compatibility accounting has exactly one shared demand.
    budget = rust_code(sources.get("src/engine/src/resident/budget.rs", ""))
    estimate = balanced_body(budget, "KernelCostEstimate") or ""
    fields = re.findall(r"\b\w+\s*:\s*[^,]+", estimate)
    if fields != ["demand: ResourceDemand"]:
        found.append("KernelCostEstimate remains a second Resident resource authority")

    # 19. Review-closure invariants remain centralized and fail closed.
    target = rust_code(sources.get("src/core/src/memory_plan/target.rs", ""))
    derive = rust_code(sources.get("src/core/src/memory_plan/derive.rs", ""))
    if "c32_slot" not in target or "Complex64Bits" not in target or "Complex(FloatWidth::W64) => layouts.c64_slot" not in derive:
        found.append("target primitive layouts conflate C32 and C64 storage")
    program = rust_code(sources.get("src/engine/src/memory_planner/program.rs", ""))
    for required in (
        "CallSiteMemoryTemplate",
        "instruction_nodes",
        "fn remap_transaction",
        "validate_global_object_namespace",
        "safe_destructive_alias",
        "attach_resident_call_memory_template",
        "PlannedValueClass::PublishedOutput",
        "instantiate_program_memory_plan_with_target_overrides",
        "fn transaction_stage_object",
        "remap_call_allocations(node, call, &object_map",
    ):
        if required not in program:
            found.append(f"program planning omits global identity/liveness closure: {required}")
    if not re.search(
        r"SlotRole\s*::\s*Output\s*=>\s*PlannedValueClass\s*::\s*PublishedOutput",
        program,
    ):
        found.append("program planning collapses published outputs into state")
    turn = rust_code(sources.get("src/engine/src/memory_planner/turn.rs", ""))
    for required in (
        "resolved_footprints",
        "resolve_deferred_call_memory",
        "place_allocations(&mut allocations)",
        "apply_observed_turn_demand",
        "grow_transaction_family",
        "grow_transaction_stage_total",
        "arenas",
        "evaluate_memory_budget",
        "budget_limits",
    ):
        if required not in turn:
            found.append(f"turn planning omits deferred budget closure: {required}")
    for name in ("plan_turn_memory", "apply_observed_turn_demand", "check_turn_planning_progress"):
        body = dict(function_bodies(turn, name)).get(name, "")
        if not re.search(r"evaluate_memory_budget\([\s\S]*?plan\.budget_limits,\s*\)", body):
            found.append(f"turn planning omits deferred budget closure: {name} plan.budget_limits")
    if not re.search(
        r"let\s+call_transactions\b.*?\.chain\s*\(\s*&call_transactions\s*\)",
        turn,
        re.DOTALL,
    ):
        found.append("turn planning loses remapped call transaction stages for RMW writers")
    for required in (
        "input_witnesses",
        "output_witnesses",
        "output_regions",
        "resolve_deferred_call_demand",
        "canonical_footprint_demand",
        "semantic_hash_comparison_work",
    ):
        if required not in derive:
            found.append(f"core call-demand resolution is incomplete: {required}")
    if "output_regions: request.regions.into()" not in derive:
        found.append("core call-demand resolution is incomplete: output_regions")
    if "derive_scratch_allocations(" not in derive or "AllocationRole::OrderedIndex" not in derive:
        found.append("implementation scratch has no physical allocation records")
    if program.count("aggregate_budget_violations(") < 3:
        found.append("program and Resident attachment omit aggregate budget evaluation")
    if "current_port_footprint(" in turn:
        found.append("resident turn facts are reconstructed from activation estimates")
    live = rust_code(sources.get("src/engine/src/resident/general/live.rs", "").split("#[cfg(test)]", 1)[0])
    for required in ("visit_canonical_data_work", "published_footprints", "selected_region", "value.capacity()"):
        if required not in live:
            found.append(f"live Resident measurement is incomplete: {required}")
    if resident.count("attach_resident_call_memory_template") < 2:
        found.append("resident call plans bypass global identity and placement")
    resident_execution = rust_code(
        sources.get("src/engine/src/resident/general/execution.rs", "")
    )
    budget = rust_code(sources.get("src/engine/src/resident/budget.rs", ""))
    if "plan_current_resident_turn" not in resident_execution:
        found.append("resident execution bypasses real turn-plan authority: plan_current_resident_turn")
    if "with_resident_turn_plan" not in resident_execution or "with_resident_turn_plan" not in budget:
        found.append("resident execution bypasses real turn-plan authority: with_resident_turn_plan")
    if re.search(r"\bTurnMemoryPlan\s*\{", budget):
        found.append("resident budget manufactures a synthetic TurnMemoryPlan")
    for relative in (
        "src/engine/src/resident/matrix_literal.rs",
        "src/engine/src/resident/numeric.rs",
    ):
        for name, body in function_bodies(sources.get(relative, ""), "bind_"):
            if "PreparedKernel" in body or re.search(r"\badmit_\w*\s*\(", body):
                found.append(
                    f"{relative}: Resident binder {name} manufactures an execution permit"
                )

    compiler = rust_code(sources.get("src/engine/src/artifact/compiler.rs", ""))
    if "replan_call_memory(memory_plan, binding)" not in compiler:
        found.append("resolved external contracts retain a stale call memory plan")
    if re.search(r"memory_plan\s*\.\s*bound_call\s*=", compiler):
        found.append("resolved external contracts relabel rather than replan memory")

    compute_memory = rust_code(sources.get("src/compute/src/memory.rs", ""))
    for required in (
        "TargetMemoryProfile::current_native_host()",
        "instantiate_program_memory_plan_with_target_overrides",
    ):
        if required not in compute_memory:
            found.append(f"mixed compute uses one target profile for every space: {required}")
    if compute_memory.count("instantiate_program_memory_plan_with_target_overrides") < 2:
        found.append("mixed compute uses one target profile for every space: target override call")
    if re.search(r"\bProgramMemoryPlan\s*\{", gpu_memory):
        found.append("GPU backing projection masquerades as a semantic ProgramMemoryPlan")
    if "GpuBackingMemoryPlan" not in gpu_memory:
        found.append("GPU backing projection is not explicitly subordinate")
    resident_planner = rust_code(sources.get("src/engine/src/memory_planner/resident.rs", ""))
    if "input.footprint.payload_bytes" not in resident_planner:
        found.append("resident projection ignores concrete retained footprints")
    for required in (
        "finalize_resident_current_footprints",
        "payload_offsets",
    ):
        if required not in resident_planner:
            found.append(f"resident payload finalization is incomplete: {required}")
    if "fn resident_value_observed_footprint" not in resident:
        found.append("resident audit copies planned footprints instead of observing backings")
    if resident.count("finalize_resident_backing_footprints") < 2:
        found.append(
            "resident live footprint lifecycle is incomplete: "
            "finalize_resident_backing_footprints"
        )
    if "resident_state_buffer(allocation)" not in resident:
        found.append(
            "resident live footprint lifecycle is incomplete: "
            "resident_state_buffer(allocation)"
        )

    # 20. R6 backing and allocator concepts do not appear in production R5 code.
    for relative, source in rust_files(root, PRODUCTION_ROOTS):
        code = rust_code(source)
        for identifier in R6_FORBIDDEN:
            if re.search(rf"\b{re.escape(identifier)}\b", code):
                found.append(f"{relative}: R6 concept introduced during R5: {identifier}")

    # 21. Package versions remain on the existing release line.
    cargo = sources.get("Cargo.toml", "")
    if not re.search(r"(?m)^version\s*=\s*\"0\.3\.6\"\s*$", cargo):
        found.append("root package version changed during R5")
    if not re.search(r"(?m)^mech-core\s*=\s*\{\s*version\s*=\s*\"0\.3\.5\"", cargo):
        found.append("workspace component versions changed during R5")

    # 22. Final status and workflow ownership are permanent.
    status_sources = "\n".join(
        sources.get(path, "")
        for path in (
            "README.md",
            "docs/design/ROADMAP.mec",
            "docs/design/v0.4-endgame.md",
            "docs/design/type-memory-boundary.md",
            "docs/design/r4-type-system-cutover.md",
        )
    )
    if "R5 Memory planner — complete" not in status_sources or "R6 Memory runtime cutover — next" not in status_sources:
        found.append("documentation does not mark R5 complete and R6 next")
    if re.search(r"(?i)\b(?:TODO[^\n]*R5|R5[^\n]*(?:incomplete|follow-up|required later))\b", status_sources):
        found.append("documentation leaves required R5 work incomplete")
    owners = sources.get(".github/ci/owners.toml", "")
    for owner in ("[owners.mech-compute]", "[owners.mech-gpu]"):
        if owner not in owners:
            found.append(f"R5 owner registration is missing: {owner}")
    for workflow in (".github/workflows/ci.yml", ".github/workflows/ci-full.yml"):
        source = sources.get(workflow, "")
        if "python3 scripts/check-r5-memory-planner.py" not in source:
            found.append(f"{workflow}: does not run the R5 checker")
        if "scripts/tests/test_check_r5_memory_planner.py" not in source:
            found.append(f"{workflow}: does not run the R5 checker mutation suite")
    full = sources.get(".github/workflows/ci-full.yml", "")
    if "name: R5 memory planner" not in full:
        found.append("Full CI omits the R5 memory planner job")

    return sorted(set(found))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if found:
        print("R5 memory planner check failed:", file=sys.stderr)
        for failure in found:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("R5 memory planner check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
