#!/usr/bin/env python3
"""Enforce the permanent R6 managed-memory runtime boundary."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "src/core/src/memory_runtime/mod.rs",
    "src/core/src/memory_runtime/identity.rs",
    "src/core/src/memory_runtime/domain.rs",
    "src/core/src/memory_runtime/allocation.rs",
    "src/core/src/memory_runtime/access.rs",
    "src/core/src/memory_runtime/payload.rs",
    "src/core/src/memory_runtime/transaction.rs",
    "src/core/src/memory_runtime/error.rs",
    "src/core/src/memory_plan/derive.rs",
    "src/core/src/memory_plan/model.rs",
    "src/core/src/execution.rs",
    "src/core/src/snapshot/encoding.rs",
    "src/core/src/snapshot/validation.rs",
    "include/project.js",
    "machines/string/src/lib.rs",
    "machines/matrix/src/transpose.rs",
    "machines/set/src/canonical.rs",
    "machines/set/src/operations/union.rs",
    "src/engine/src/memory_runtime/mod.rs",
    "src/engine/src/memory_runtime/realize.rs",
    "src/engine/src/memory_runtime/resident.rs",
    "src/engine/src/memory_planner/program.rs",
    "src/engine/src/memory_planner/resident.rs",
    "src/engine/src/resident/general/mod.rs",
    "src/engine/src/interpreter/mod.rs",
    "src/engine/src/literals.rs",
    "src/engine/src/structures.rs",
    "src/engine/src/intrinsics/define.rs",
    "src/engine/src/intrinsics/constructors.rs",
    "src/engine/src/intrinsics/table_ops.rs",
    "src/engine/src/intrinsics/access/mod.rs",
    "src/engine/src/intrinsics/access/matrix.rs",
    "src/engine/src/intrinsics/assign/mod.rs",
    "src/engine/src/intrinsics/assign/matrix.rs",
    "src/engine/src/intrinsics/horzcat.rs",
    "src/engine/src/intrinsics/vertcat.rs",
    "src/engine/src/function/external/resource_read.rs",
    "src/engine/src/function/external/host_call.rs",
    "src/engine/src/function/module.rs",
    "src/core/src/cell_binding.rs",
    "src/core/src/function/argument.rs",
    "src/core/src/function/mod.rs",
    "src/core/src/function/specialization.rs",
    "src/core/src/program/bytecode/constants/canonical.rs",
    "src/core/src/program/bytecode/writer.rs",
    "src/engine/src/artifact/model.rs",
    "src/build/src/plan/model.rs",
    "hosts/gpu/src/execution_plan.rs",
    "hosts/gpu/src/memory.rs",
    "hosts/gpu/src/native.rs",
    "hosts/gpu/src/batched/mod.rs",
    "include/browser-compute.js",
    "src/core/tests/r6_memory_runtime.rs",
    "src/core/tests/r6_memory_safety.rs",
    "src/engine/tests/r6_memory_runtime.rs",
    "src/stdlib/tests/r6_managed_functions.rs",
    "src/compute/tests/r6_memory_runtime.rs",
    "hosts/gpu/tests/r6_memory_runtime.rs",
    "scripts/check-r6-memory-runtime.py",
    "scripts/tests/test_check_r6_memory_runtime.py",
    "docs/design/r6-memory-runtime-cutover.md",
    "docs/design/r6-completion-status.md",
    "docs/design/r6-completion-status.md",
    ".github/workflows/ci.yml",
    ".github/workflows/ci-full.yml",
    ".github/ci/owners.toml",
)

AUTHORITIES = (
    "MemoryDomainId",
    "MemoryPlanRevision",
    "PublishedValueVersion",
    "RegionIncarnation",
    "AllocationHandle",
    "PlanObjectKey",
    "RuntimePlanView",
    "MemoryReservation",
    "RealizedMemoryPlan",
    "PlannedAllocator",
    "PlannedArenaElement",
    "PlannedArenaProjection",
    "ManagedPort",
    "PreparedCallAccess",
    "KernelMemoryFrame",
    "ManagedString",
    "ManagedSequence",
    "PreparedPublication",
    "PreparedDeviceSubmission",
    "RegisteredDeviceBuffer",
)

RUNTIME_RECEIPTS = (
    "MemoryDomain",
    "MemoryReservation",
    "RealizedMemoryPlan",
    "AllocationHandle",
    "PlanObjectKey",
    "PreparedCallAccess",
    "KernelMemoryFrame",
    "PreparedPublication",
    "DeviceSubmissionHold",
)

WIRE_TYPES = {
    "src/core/src/program/bytecode/writer.rs": "BytecodeProgram",
    "src/engine/src/artifact/model.rs": "ProgramArtifact",
    "src/build/src/plan/model.rs": "NativeBuildPlan",
    "hosts/gpu/src/execution_plan.rs": "GpuExecutionPlan",
}

RAW_LITERAL = re.compile(r'(?:br|rb|r)(?P<hashes>#{0,255})"')
CHAR_LITERAL = re.compile(
    r"(?:b)?'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|[^\r\n])|[^\\'\r\n])'"
)


def rust_code(source: str) -> str:
    """Blank comments and literals while preserving useful token boundaries."""
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
        if source.startswith("/*", index):
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
        character = CHAR_LITERAL.match(source, index)
        if character:
            blank(index, character.end())
            index = character.end()
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
        index += 1
    return "".join(output)


def balanced_body(source: str, declaration: str) -> str | None:
    match = re.search(
        rf"\b(?:pub\s+)?(?:struct|enum|trait)\s+{re.escape(declaration)}\b", source
    )
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


def function_bodies(source: str, name: str):
    """Yield balanced Rust function bodies from comment/literal-free source."""
    code = rust_code(source)
    pattern = re.compile(rf"\bfn\s+{re.escape(name)}\b[^{{;]*\{{")
    for match in pattern.finditer(code):
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


def brace_depth_at(source: str, offset: int) -> int:
    """Return the lexical Rust block depth at offset in literal-free source."""
    depth = 0
    for character in source[:offset]:
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
    return depth


def direct_match(pattern: str, source: str, *, depth: int = 0) -> re.Match[str] | None:
    """Find a match that executes directly at one lexical block depth."""
    for match in re.finditer(pattern, source):
        if brace_depth_at(source, match.start()) == depth:
            return match
    return None


def direct_matches(pattern: str, source: str, *, depth: int = 0) -> list[re.Match[str]]:
    """Return every match that executes directly at one lexical block depth."""
    return [
        match
        for match in re.finditer(pattern, source)
        if brace_depth_at(source, match.start()) == depth
    ]


def matched_block(source: str, match: re.Match[str] | None) -> str | None:
    """Return the balanced block opened by a matched Rust control expression."""
    if match is None:
        return None
    start = source.find("{", match.start(), match.end())
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


def matched_block_end(source: str, match: re.Match[str] | None) -> int | None:
    """Return the exclusive end offset of a matched balanced Rust block."""
    if match is None:
        return None
    start = source.find("{", match.start(), match.end())
    if start < 0:
        return None
    depth = 1
    for index in range(start + 1, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
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

    runtime_source = "\n".join(
        source
        for relative, source in sources.items()
        if "/memory_runtime/" in relative or relative == "hosts/gpu/src/memory.rs"
    )
    for authority in AUTHORITIES:
        if not re.search(rf"\b{re.escape(authority)}\b", runtime_source):
            found.append(f"required R6 runtime authority is missing: {authority}")

    # Runtime ownership is process-local and cannot leak into wire artifacts.
    for relative, source in rust_files(
        root, ("src/core/src/memory_runtime", "src/engine/src/memory_runtime")
    ):
        code = rust_code(source)
        if re.search(r"\b(?:Serialize|Deserialize)\b", code):
            found.append(f"{relative}: runtime ownership derives serialization")
    for relative, declaration in WIRE_TYPES.items():
        path = root / relative
        if not path.is_file():
            continue
        body = balanced_body(rust_code(path.read_text(encoding="utf-8")), declaration)
        if body is None:
            found.append(f"{relative}: {declaration} declaration is missing")
            continue
        for receipt in RUNTIME_RECEIPTS:
            if re.search(rf"\b{re.escape(receipt)}\b", body):
                found.append(f"{relative}: {declaration} serializes runtime receipt {receipt}")

    # R5 semantic/physical plan records remain free of runtime handles.
    specialization = root / "src/core/src/function/specialization.rs"
    if specialization.is_file():
        body = balanced_body(rust_code(specialization.read_text(encoding="utf-8")), "BoundCall")
        if body is not None:
            for receipt in RUNTIME_RECEIPTS:
                if re.search(rf"\b{re.escape(receipt)}\b", body):
                    found.append(f"BoundCall carries forbidden R6 runtime field {receipt}")

    domain = rust_code(sources.get("src/core/src/memory_runtime/domain.rs", ""))
    for operation in (
        "prepare_realization",
        "materialize",
        "prepare_call",
        "prepare_publication",
        "commit_publication",
        "abort_publication",
        "retire",
        "collect_retired",
        "close",
    ):
        if not re.search(rf"\bfn\s+{operation}\b", rust_code(runtime_source)):
            found.append(f"MemoryDomain is missing required operation {operation}")

    # Identity validation must remain explicit at every lease/binding boundary.
    access = rust_code(sources.get("src/core/src/memory_runtime/access.rs", ""))
    for token in (
        "InvalidPlanRevision",
        "WrongMemoryDomain",
        "StaleAllocationGeneration",
        "StaleRegionIncarnation",
    ):
        if token not in domain + access:
            found.append(f"runtime binding omits {token} validation")

    # Publication is one atomic record: value, shape, binding and semantic version.
    transaction = rust_code(sources.get("src/core/src/memory_runtime/transaction.rs", ""))
    for token in ("shape", "binding", "PublishedValueVersion"):
        if not re.search(rf"\b{token}\b", transaction, re.IGNORECASE):
            found.append(f"publication lifecycle omits {token}")

    cell_path = root / "src/core/src/cell_binding.rs"
    cell = rust_code(cell_path.read_text(encoding="utf-8")) if cell_path.is_file() else ""
    storage = balanced_body(cell, "CellStorageBinding")
    required_storage = ("ManagedHost", "ManagedCanonical", "ManagedDevice", "PinnedExternal")
    if storage is None or any(name not in storage for name in required_storage):
        found.append("ValueCell storage is not closed over ordinary managed host and payload bindings")
    if not re.search(r"\bfn\s+allocate_planned\b", cell):
        found.append("ValueCell lacks reservation-backed allocate_planned construction")
    canonical_storage = balanced_body(cell, "ManagedCanonicalCellStorage")
    if (
        canonical_storage is None
        or "RealizedMemoryPlan" not in canonical_storage
        or len(re.findall(r"\bPlanObjectKey\b", canonical_storage)) < 2
    ):
        found.append("managed canonical cells do not retain header and payload plan ownership")
    if not any(
        "admit_frozen_snapshot" in body and "into_retained_payload_ticket" in body
        for body in function_bodies(access, "stage_output_value")
    ):
        found.append("canonical output staging bypasses its admitted payload owner")
    admitted_builders = list(function_bodies(access, "with_admitted_canonical_output"))
    binary_builders = list(
        function_bodies(access, "with_admitted_canonical_binary_port_values")
    )
    if (
        not admitted_builders
        or "prepare_frozen_snapshot" not in admitted_builders[0]
        or "begin_construction" not in admitted_builders[0]
        or "build(self, &mut construction)" not in admitted_builders[0]
        or admitted_builders[0].index("begin_construction")
        > admitted_builders[0].index("build(self, &mut construction)")
        or not binary_builders
        or "with_admitted_canonical_output" not in binary_builders[0]
        or "prepare_frozen_snapshot" in binary_builders[0]
    ):
        found.append("maintained canonical construction does not admit before building")
    object_value_views = list(function_bodies(access, "with_object_value_view"))
    if not object_value_views or not re.search(
        r"initialization\s*\.\s*contains_region", object_value_views[0]
    ):
        found.append("read-capable object views can expose uninitialized managed storage")
    managed_element_checks = list(function_bodies(access, "validate_managed_element"))
    raw_byte_checks = list(function_bodies(access, "validate_raw_byte_object"))
    raw_mutators = list(function_bodies(access, "with_bytes_mut"))
    raw_prefix_mutators = list(function_bodies(access, "with_bytes_mut_prefix"))
    if (
        not managed_element_checks
        or "region.slot.is_none()" not in managed_element_checks[0]
        or "IntegerWidth::W8" not in managed_element_checks[0]
        or not raw_byte_checks
        or "region.slot.is_some()" not in raw_byte_checks[0]
        or not raw_mutators
        or "with_bytes_mut_inner(object, true" not in raw_mutators[0]
        or not raw_prefix_mutators
        or "validate_raw_byte_object" not in raw_prefix_mutators[0]
    ):
        found.append("typed Boolean storage can be exposed through raw byte access")
    payload = rust_code(sources.get("src/core/src/memory_runtime/payload.rs", ""))
    if (
        "PreparedFrozenSnapshotAdmission" not in payload
        or "FrozenSnapshotConstruction" not in payload
        or "try_vec_with_capacity" not in payload
        or "try_concatenate_string" not in payload
        or not any(
            "record_initialized" in body
            and "actual_retained_bytes" in body
            and "subtract(unused)" in body
            for body in function_bodies(payload, "complete")
        )
    ):
        found.append("canonical payload admission is not completed after valid construction")
    generic_builders = list(function_bodies(payload, "try_build_canonical_candidate_with"))
    if not generic_builders or "build(self)" not in generic_builders[0]:
        found.append("canonical candidate callback does not receive its construction authority")
    envelope = balanced_body(payload, "PayloadEnvelopeOwner")
    if (
        envelope is None
        or "block_capacity" not in envelope
        or "owner.block_capacity" not in payload
    ):
        found.append("payload node admission depends on allocator spare capacity")
    snapshot = rust_code(sources.get("src/core/src/snapshot/validation.rs", ""))
    finalizers = list(function_bodies(snapshot, "finalize_data"))
    finalized_values = list(function_bodies(snapshot, "finalized_value_with_construction"))
    admitted_finalizers = list(function_bodies(cell, "finalize_draft_with_construction"))
    if (
        "SnapshotConstructionAuthority" not in snapshot
        or not admitted_finalizers
        or "with_construction_authority(construction)" not in admitted_finalizers[0]
        or not finalizers
        or "context.try_vec_with_capacity" not in finalizers[0]
        or "context.try_boxed_str" not in finalizers[0]
        or "finalized_value_with_construction" not in snapshot
        or not finalized_values
        or finalized_values[0].count("context.try_arc(") < 2
        or "Arc::new(" in finalized_values[0]
    ):
        found.append("common canonical finalization bypasses construction authority")
    if (
        "shared_schemas: OnceCell<Arc<SchemaTable>>" not in snapshot
        or "self.shared_schemas.get()" not in snapshot
    ):
        found.append("recursive canonical values clone their schema owner repeatedly")
    if (
        "SchemaBody::Id => pack!(Id, Id)" not in snapshot
        or "ScalarSequenceElement::TableColumn" not in snapshot
        or "scalar_sequence_schema(&column.schema)" not in snapshot
    ):
        found.append("scalar matrix or table finalization bypasses packed construction")
    rebinds = list(function_bodies(snapshot, "rebind"))
    frozen_data = balanced_body(snapshot, "FrozenSnapshotData")
    frozen_storage = balanced_body(snapshot, "FrozenSnapshotStorage")
    if (
        "FrozenSnapshotData" not in snapshot
        or not rebinds
        or "return Ok(self.clone())" not in rebinds[0]
        or "root: self.root.clone()" not in snapshot
        or "schema_body_contains_dynamic" not in rebinds[0]
        or "schemas: Some(Arc::new(schemas.clone()))" not in rebinds[0]
    ):
        found.append("canonical snapshots do not preserve shared frozen ownership")
    if (
        frozen_data is None
        or "RetainedPayloadTicket" not in frozen_data
        or frozen_storage is None
        or "RetainedPayloadTicket" in frozen_storage
        or "has_retained_payload_ticket" not in snapshot
    ):
        found.append("frozen physical accounting is not attached to shared immutable data")

    region = balanced_body(domain, "RuntimeRegionRecord")
    retired = list(function_bodies(domain, "collect_retired"))
    if (
        region is None
        or "realization_owner" not in region
        or not retired
        or not re.search(r"state\s*\.\s*regions\s*\.\s*retain", retired[0])
        or not re.search(r"state\s*\.\s*realization_owners\s*\.\s*retain", retired[0])
    ):
        found.append("retired realizations retain historical region initialization metadata")
    replacement_paths = list(function_bodies(cell, "replace"))
    function_source = rust_code(sources.get("src/core/src/function/mod.rs", ""))
    promotion_paths = list(
        function_bodies(function_source, "promote_prepared_realization")
    )
    if (
        not any("collect_retired" in body for body in replacement_paths)
        or not promotion_paths
        or "collect_retired" not in promotion_paths[0]
    ):
        found.append("ordinary cold-path replacement or promotion omits retired collection")
    solve_publication_paths = [
        body
        for body in function_bodies(function_source, "solve_reactive_with")
        if "prepare_reactive_publication" in body
    ]
    if (
        not solve_publication_paths
        or "prepare_external_publication" not in solve_publication_paths[0]
        or not re.search(
            r"Err\s*\(\s*error\s*\).*?drop\s*\(\s*prepared\s*\)\s*;.*?collect_retired\s*\(\s*\).*?Err\s*\(\s*error\s*\)",
            solve_publication_paths[0],
            re.DOTALL,
        )
    ):
        found.append("late publication preparation failure omits retired collection")
    preparation_paths = list(
        function_bodies(function_source, "prepare_reactive_publication")
    )
    if (
        not preparation_paths
        or "let abandoned_candidate = candidate.is_some()" not in preparation_paths[0]
        or not re.search(
            r"drop\s*\(\s*candidate\s*\)\s*;\s*if\s+abandoned_candidate\s*\{\s*current\s*\.\s*domain\s*\.\s*collect_retired",
            preparation_paths[0],
            re.DOTALL,
        )
    ):
        found.append("unchanged invariant turn invokes cold reclamation without a candidate")
    register_paths = list(
        function_bodies(function_source, "commit_pending_registers_impl")
    )
    abandoned_batches = list(
        function_bodies(function_source, "collect_abandoned_function_publications")
    )
    if (
        not register_paths
        or register_paths[0].count("collect_abandoned_function_publications(staged,") < 3
        or not abandoned_batches
        or not re.search(
            r"drop\s*\(\s*prepared\s*\)\s*;.*?collect_retired",
            abandoned_batches[0],
            re.DOTALL,
        )
    ):
        found.append("failed register batch collects while staged candidates remain owned")
    if (
        not register_paths
        or "let failing_domain" not in register_paths[0]
        or "collect_abandoned_function_publications(staged, &[failing_domain])"
        not in register_paths[0]
        or not abandoned_batches
        or "additional_domains" not in abandoned_batches[0]
        or not re.search(
            r"for\s+domain\s+in\s+additional_domains\s*\{.*?domains\s*\.\s*push\s*\(\s*domain\s*\.\s*clone\s*\(\s*\)\s*\)\s*;",
            abandoned_batches[0],
            re.DOTALL,
        )
    ):
        found.append("failed register preparation omits its candidate domain")
    if "publication_shape" not in cell or not any(
        "publication_shape" in body and "publication_locked" in body
        for body in function_bodies(cell, "lock_publication")
    ):
        found.append("ready publication does not retain conflict-free shape authority")
    live_footprints = list(function_bodies(cell, "current_memory_footprint"))
    if (
        not live_footprints
        or "ManagedHostCellStorage" not in live_footprints[0]
        or "required_initialization_bytes" not in live_footprints[0]
        or live_footprints[0].index("ManagedHostCellStorage")
        > live_footprints[0].index("self.snapshot")
    ):
        found.append("managed-host footprint measurement materializes a semantic snapshot")
    if "return Ok(self.value.clone())" not in cell:
        found.append("ordinary managed canonical snapshots rebuild their frozen payload")

    # The maintained entry itself must require a frame. A second opt-in trait is
    # not a cutover because FunctionInstance can still invoke raw implementations.
    function_path = root / "src/core/src/function/mod.rs"
    function = rust_code(function_path.read_text(encoding="utf-8")) if function_path.is_file() else ""
    implementation = balanced_body(function, "MechFunctionImpl")
    if implementation is None or not re.search(r"\bfn\s+solve_managed\b", implementation):
        found.append("MechFunctionImpl does not require solve_managed")
    if implementation is not None and re.search(r"\bfn\s+solve_result(?:_with)?\b", implementation):
        found.append("MechFunctionImpl retains an unmanaged solve entry")
    for relative, source in rust_files(
        root, ("src/engine/src", "machines")
    ):
        if any(
            re.search(
                r"\.\s*(?:replace|replace_set|replace_set_drafts|replace_matrix_drafts)\s*\(",
                body,
            )
            for body in function_bodies(source, "solve_managed")
        ):
            found.append(
                f"{relative}: solve_managed bypasses frame-owned staged publication"
            )
    if "planned_output_footprints" not in function or "resolve_current_call_memory" not in function:
        found.append("payload-dependent calls do not refresh live and prospective footprints")
    publications = list(function_bodies(function, "prepare_reactive_publication"))
    transaction = rust_code(
        sources.get("src/core/src/memory_runtime/transaction.rs", "")
    )
    evidence = balanced_body(transaction, "CellPublicationEvidence")
    if (
        not publications
        or "snapshot_managed_host_data" in publications[0]
        or "CellPublicationEvidence::initialized_region" not in publications[0]
        or evidence is None
        or "InitializedManagedRegion" not in evidence
        or "FrozenValue" not in evidence
    ):
        found.append("fixed-width publication constructs a canonical evidence copy")
    prepared_calls = list(function_bodies(access, "prepare_function_call"))
    realization_preparation = list(function_bodies(function, "prepare"))
    if (
        not prepared_calls
        or "writable_outputs" not in prepared_calls[0]
        or not realization_preparation
        or "PayloadOutputPlanPolicy::PublishedInvariant" not in realization_preparation[0]
    ):
        found.append("published-invariant functions receive writable candidate authority")
    semantic_wrappers = list(function_bodies(function, "capture_external_output"))
    staged_external_wrappers = list(
        function_bodies(function, "stage_prepared_external_output")
    )
    if (
        len(semantic_wrappers) < 2
        or not any("self.function.capture_external_output" in body for body in semantic_wrappers)
        or len(staged_external_wrappers) < 2
        or not any(
            "self.function" in body and "stage_prepared_external_output" in body
            for body in staged_external_wrappers
        )
    ):
        found.append("semantic function wrappers discard external-result planning authority")
    binding_constructors = list(function_bodies(function, "new"))
    binding_constructors.extend(
        function_bodies(function, "new_with_managed_inputs")
    )
    if "validate_transaction_authority" not in function or not any(
        "validate_transaction_authority" in body for body in binding_constructors
    ):
        found.append("function binding does not validate transaction semantics")
    string_runtime = rust_code(sources.get("machines/string/src/lib.rs", ""))
    if (
        "canonical_concat_footprint" not in string_runtime
        or "with_admitted_canonical_binary_port_values" not in string_runtime
    ):
        found.append("String construction bypasses prospective payload admission")
    if re.search(r"if\s+matrix\s*\{\s*2\s*\}", string_runtime):
        found.append("String matrix footprint confuses rank with shape parameters")
    set_runtime = rust_code(sources.get("machines/set/src/canonical.rs", ""))
    set_union = rust_code(sources.get("machines/set/src/operations/union.rs", ""))
    if (
        "with_admitted_set" not in set_runtime
        or "with_admitted_canonical_output" not in set_runtime
        or "stage_output_value" in set_runtime
        or "with_admitted_set" not in set_union
        or "planned_output_footprints" not in set_union
    ):
        found.append("maintained set construction bypasses prospective payload admission")
    if (
        "PayloadOutputPlanPolicy" not in function
        or "output_policy == PayloadOutputPlanPolicy::Missing" not in function
    ):
        found.append("missing payload witness silently reuses published output authority")
    matrix_transpose = rust_code(sources.get("machines/matrix/src/transpose.rs", ""))
    if (
        "planned_output_footprint" not in matrix_transpose
        or "with_admitted_canonical_output" not in matrix_transpose
    ):
        found.append("String matrix transpose bypasses prospective payload admission")
    matrix_constructors = rust_code(
        sources.get("src/engine/src/intrinsics/constructors.rs", "")
    )
    if (
        "prospective_matrix_output_footprint" not in matrix_constructors
        or "canonical_matrix_output_requires_builder" not in matrix_constructors
        or len(re.findall(r"with_admitted_canonical_output", matrix_constructors)) < 3
    ):
        found.append("canonical matrix constructors bypass prospective payload admission")
    prospective_matrices = list(
        function_bodies(matrix_constructors, "prospective_matrix_output_footprint")
    )
    if not prospective_matrices or "output.current_memory_footprint" in prospective_matrices[0]:
        found.append("matrix candidate footprint includes the previous published payload")
    variable_definitions_source = rust_code(
        sources.get("src/engine/src/intrinsics/define.rs", "")
    )
    if "PayloadOutputPlanPolicy::PublishedInvariant" not in variable_definitions_source:
        found.append("frozen variable definitions lack explicit published-output authority")
    if "PayloadOutputPlanPolicy::PublishedInvariant" not in matrix_constructors:
        found.append("frozen set definitions lack explicit published-output authority")
    canonical_access_source = rust_code(
        sources.get("src/engine/src/intrinsics/access/mod.rs", "")
    )
    canonical_access_plans = list(
        function_bodies(canonical_access_source, "planned_output_footprints")
    )
    if (
        not canonical_access_plans
        or "prospective_repeated_sequence_memory_footprint" not in canonical_access_plans[0]
        or "canonical_indices" in canonical_access_plans[0]
    ):
        found.append("canonical access materializes selectors or omits payload witnesses")
    typed_access = rust_code(
        sources.get("src/engine/src/intrinsics/access/matrix.rs", "")
    )
    typed_assignment = rust_code(
        sources.get("src/engine/src/intrinsics/assign/matrix.rs", "")
    )
    if "planned_binary_output_footprint" not in typed_access or "stage_managed(frame)" not in typed_access:
        found.append("typed String matrix access bypasses prospective payload admission")
    if (
        "planned_selection_output_footprint" not in typed_assignment
        or "stage_managed(frame)" not in typed_assignment
    ):
        found.append("typed String matrix assignment bypasses prospective payload admission")
    for relative in (
        "src/engine/src/function/external/resource_read.rs",
        "src/engine/src/function/external/host_call.rs",
    ):
        external = rust_code(sources.get(relative, ""))
        prepared = list(function_bodies(external, "capture_external_output"))
        staged = list(function_bodies(external, "stage_prepared_external_output"))
        if (
            "prepared_result" in external
            or not prepared
            or not any(
                token in prepared[0]
                for token in ("read_resource", "invoke_host_function")
            )
            or not staged
            or "stage_output_value" not in staged[0]
        ):
            found.append(f"{relative}: external result is not captured once before replanning")
    prepared_publication = list(function_bodies(function, "prepare_reactive_publication"))
    if (
        "PreparedExternalResult" not in function
        or "ExternalMarshalling" not in function
        or not prepared_publication
        or "marshal_external_inputs" not in prepared_publication[0]
        or "capture_external_output" not in prepared_publication[0]
        or prepared_publication[0].index("marshal_external_inputs")
        > prepared_publication[0].index("capture_external_output")
    ):
        found.append("external provider invocation precedes admitted call-scoped marshalling")
    marshalling = list(function_bodies(function, "marshal_external_inputs"))
    planning = rust_code(sources.get("src/core/src/memory_plan/derive.rs", ""))
    marshalling_requirements = list(
        function_bodies(planning, "external_marshalling_input_bytes")
    )
    if (
        not marshalling
        or "ExternalMarshallingConstruction" not in marshalling[0]
        or "snapshot_for_external_marshalling" not in marshalling[0]
        or not marshalling_requirements
        or "canonical_snapshot_finalization_bytes" not in marshalling_requirements[0]
        or "ValueDataDraft" not in marshalling_requirements[0]
    ):
        found.append("external marshalling is not governed by canonical construction authority")
    external_snapshots = list(function_bodies(cell, "snapshot_for_external_marshalling"))
    if (
        not any("try_clone_for_external_marshalling" in body for body in external_snapshots)
        or "external_canonical_shape_clone_bytes(input)" not in marshalling_requirements[0]
        or "return Ok((0, 0))" in marshalling_requirements[0]
    ):
        found.append("external canonical metadata cloning bypasses marshalling authority")
    reactive_solve = list(function_bodies(function, "solve_reactive_with"))
    if (
        "PreparedLiveResourceBinding" not in function
        or "Box::try_new(commit)" not in rust_code(sources.get("src/core/src/execution.rs", ""))
        or not reactive_solve
        or "prepare_external_publication(services)" not in reactive_solve[0]
        or "external.commit()" not in reactive_solve[0]
        or reactive_solve[0].index("prepare_external_publication(services)")
        > reactive_solve[0].index("ready.commit()")
        or reactive_solve[0].index("external.commit()")
        < reactive_solve[0].index("ready.commit()")
        or "self.promote_prepared_realization(&mut prepared)" not in reactive_solve[0]
        or reactive_solve[0].index("external.commit()")
        < reactive_solve[0].index("self.promote_prepared_realization(&mut prepared)")
    ):
        found.append("external live binding is fallible after cell publication")
    instance = balanced_body(function, "FunctionInstance")
    binding = balanced_body(function, "ManagedFunctionBinding")
    realization = balanced_body(function, "ManagedCallRealization")
    if (
        instance is None
        or not re.search(r"\bmanaged\s*:\s*ManagedFunctionBinding\b", instance)
        or binding is None
        or "CallMemoryPlan" not in binding
        or realization is None
        or "MemoryDomain" not in realization
        or "PreparedCallAccess" not in realization
    ):
        found.append("FunctionInstance has no managed-domain execution authority")
    if not re.search(r"\bfn\s+solve_in_scope\b", function):
        found.append("FunctionInstance lacks its executor-owned managed-scope entry")
    if re.search(r"\bimpl\s+MechFunctionImpl\s+for\s+UserFunction\b", function):
        found.append("UserFunction retains a nested standalone execution wrapper")
    if not any(
        "transactions.len() != plan.outputs.len()" in body
        for body in function_bodies(access, "prepare_function_call")
    ):
        found.append("function access treats a missing transaction as write authority")
    undo_snapshot = balanced_body(access, "PreparedUndoSnapshot")
    if (
        "RetainedPublicationLease" not in access
        or undo_snapshot is None
        or "retained_lease" not in undo_snapshot
    ):
        found.append("undo publication does not retain its exclusive lease")
    if (
        "owns_lease" not in access
        or "workspace.leases[other].owns_lease = false" not in access
    ):
        found.append("repeated in-place input roles are not coalesced")
    undo_take = list(function_bodies(access, "take_undo_snapshot"))
    if not undo_take or "(None, None) if held.start == held.end" not in undo_take[0]:
        found.append("empty undo transactions require a fictitious physical lease")

    domain_code = rust_code(sources.get("src/core/src/memory_runtime/domain.rs", ""))
    arena_projection_paths = list(function_bodies(domain_code, "project_host_arena"))
    handle_filtered_projection = bool(
        arena_projection_paths
        and re.search(r"\.\s*handle\s*\(", arena_projection_paths[0])
    )
    if not any(
        "arena_projection_owner" in body and "leases.is_empty()" in body
        for body in arena_projection_paths
    ) or not any(
        "arena_projection_owner" in body for body in function_bodies(access, "acquire_call")
    ):
        found.append("resident arena projections and frame leases have separate authorities")
    if (
        not arena_projection_paths
        or "realized.arena_bindings.get(&arena)" not in arena_projection_paths[0]
        or "realized.binding(" in arena_projection_paths[0]
        or "T::supports_planned_slot(slot)" not in arena_projection_paths[0]
        or "Some(expected) if expected != slot" not in arena_projection_paths[0]
        or "region.arena != arena" not in arena_projection_paths[0]
        or handle_filtered_projection
        or "PlannedArenaProjection::<T>::validate_realized_layout" not in arena_projection_paths[0]
        or "region.handle == Some(handle)" not in arena_projection_paths[0]
        or "region.initialization.clear()" not in arena_projection_paths[0]
        or "arena: object.arena" not in domain_code
        or arena_projection_paths[0].find(
            "PlannedArenaProjection::<T>::validate_realized_layout"
        )
        > arena_projection_paths[0].find(
            "record.arena_projection_owner = Rc::downgrade"
        )
    ):
        found.append("resident arena projection is not bound to its complete planned slot")
    plan_validation_paths = list(function_bodies(domain_code, "validate_plan_view"))
    if (
        not plan_validation_paths
        or not re.search(
            r"members\s*\.\s*try_reserve_exact\s*\(\s*arena\s*\.\s*members\s*\.\s*len\s*\(\s*\)\s*\)",
            plan_validation_paths[0],
        )
        or "members.windows(2)" not in plan_validation_paths[0]
        or "collect::<BTreeSet" in plan_validation_paths[0]
    ):
        found.append("runtime plan member validation can allocate infallibly")
    member_index_paths = list(
        function_bodies(domain_code, "build_member_placement_index")
    )
    member_index_body = member_index_paths[0] if member_index_paths else ""
    member_index_methods = re.findall(
        r"\bmember_placements\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        member_index_body,
    )
    member_count_definition = direct_match(
        r"let\s+member_count\s*=\s*arena_members\s*\.\s*iter\s*\(\s*\)\s*"
        r"\.\s*try_fold\s*\(\s*0_usize\s*,\s*\|total\s*,\s*\(_\s*,\s*members\)\|\s*"
        r"\{\s*total\s*\.\s*checked_add\s*\(\s*members\s*\.\s*len\s*\(\s*\)\s*\)\s*\}\s*\)\s*"
        r"\.\s*ok_or\s*\(\s*MemoryRuntimeError\s*::\s*AccountingInvariantViolation\s*"
        r"\{[\s\S]*?\}\s*\)\s*\?\s*;",
        member_index_body,
    )
    member_index_constructor = direct_match(
        r"let\s+mut\s+member_placements\s*=\s*"
        r"Vec\s*::\s*<\s*\(\s*MemoryObjectId\s*,\s*MemoryArenaId\s*\)\s*>\s*"
        r"::\s*new\s*\(\s*\)\s*;",
        member_index_body,
    )
    member_index_reservation = direct_match(
        r"member_placements\s*\.\s*try_reserve_exact\s*"
        r"\(\s*member_count\s*\)\s*\.\s*map_err\s*"
        r"\(\s*\|_\|\s*MemoryRuntimeError\s*::\s*AllocationFailed\s*"
        r"\{[\s\S]*?\}\s*\)\s*\?\s*;",
        member_index_body,
    )
    member_flatten_loop = direct_match(
        r"for\s*\(\s*arena\s*,\s*members\s*\)\s*in\s*arena_members\s*\{",
        member_index_body,
    )
    member_flatten_body = matched_block(member_index_body, member_flatten_loop)
    member_flatten_end = matched_block_end(member_index_body, member_flatten_loop)
    member_flatten_extend = direct_match(
        r"member_placements\s*\.\s*extend\s*\(\s*members\s*\.\s*iter\s*\(\s*\)"
        r"\s*\.\s*map\s*\(\s*\|member\|\s*\(\s*\*member\s*,\s*\*arena\s*\)\s*\)"
        r"\s*\)\s*;",
        member_flatten_body or "",
    )
    member_index_sort = direct_match(
        r"member_placements\s*\.\s*sort_unstable\s*\(\s*\)\s*;",
        member_index_body,
    )
    member_duplicate_guard = direct_match(
        r"if\s+let\s+Some\s*\(\s*duplicate\s*\)\s*=\s*member_placements\s*"
        r"\.\s*windows\s*\(\s*2\s*\)\s*\.\s*find\s*"
        r"\(\s*\|pair\|\s*pair\[0\]\.0\s*==\s*pair\[1\]\.0\s*\)\s*"
        r"\.\s*map\s*\(\s*\|pair\|\s*pair\[0\]\.0\s*\)\s*\{",
        member_index_body,
    )
    member_duplicate_end = matched_block_end(member_index_body, member_duplicate_guard)
    member_index_freeze = direct_match(
        r"Ok\s*\(\s*member_placements\s*\.\s*into_boxed_slice\s*\(\s*\)\s*\)\s*$",
        member_index_body,
    )
    member_index_sequence = bool(
        member_count_definition
        and member_index_constructor
        and member_index_reservation
        and member_flatten_loop
        and member_flatten_end is not None
        and member_index_sort
        and member_duplicate_guard
        and member_duplicate_end is not None
        and member_index_freeze
        and not member_index_body[: member_count_definition.start()].strip()
        and not member_index_body[
            member_count_definition.end() : member_index_constructor.start()
        ].strip()
        and not member_index_body[
            member_index_constructor.end() : member_index_reservation.start()
        ].strip()
        and not member_index_body[
            member_index_reservation.end() : member_flatten_loop.start()
        ].strip()
        and not member_index_body[member_flatten_end : member_index_sort.start()].strip()
        and not member_index_body[
            member_index_sort.end() : member_duplicate_guard.start()
        ].strip()
        and not member_index_body[
            member_duplicate_end : member_index_freeze.start()
        ].strip()
    )
    allocation_loop = direct_match(
        r"for\s+allocation\s+in\s+view\s*\.\s*allocations\s*\{",
        plan_validation_paths[0] if plan_validation_paths else "",
    )
    allocation_loop_body = matched_block(
        plan_validation_paths[0] if plan_validation_paths else "", allocation_loop
    )
    member_index_search = direct_match(
        r"let\s+member_arena\s*=\s*member_placements\s*"
        r"\.\s*binary_search_by_key\s*\(\s*&allocation\s*\.\s*id\s*,\s*"
        r"\|\(member\s*,\s*_\)\|\s*\*member\s*\)\s*\.\s*ok\s*\(\s*\)\s*"
        r"\.\s*and_then\s*\(\s*\|index\|\s*member_placements\s*\.\s*get\s*"
        r"\(\s*index\s*\)\s*\)\s*\.\s*map\s*"
        r"\(\s*\|\(_\s*,\s*member_arena\)\|\s*\*member_arena\s*\)\s*;",
        allocation_loop_body or "",
    )
    member_index_call = direct_match(
        r"let\s+member_placements\s*=\s*build_member_placement_index\s*"
        r"\(\s*&arena_members\s*\)\s*\?\s*;",
        plan_validation_paths[0] if plan_validation_paths else "",
    )
    member_placement_guard = direct_match(
        r"if\s+member_arena\s*!=\s*Some\s*\(\s*arena\s*\.\s*id\s*\)\s*"
        r"\{\s*return\s+Err\s*\(\s*MemoryRuntimeError\s*::\s*InvalidLayout\s*"
        r"\{[\s\S]*?\}\s*\)\s*;\s*\}",
        allocation_loop_body or "",
    )
    validation_member_bindings = re.findall(
        r"\blet\s+(?:mut\s+)?member_placements\b",
        plan_validation_paths[0] if plan_validation_paths else "",
    )
    validation_member_methods = re.findall(
        r"\bmember_placements\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        plan_validation_paths[0] if plan_validation_paths else "",
    )
    member_arena_bindings = re.findall(
        r"\blet\s+(?:mut\s+)?member_arena\b", allocation_loop_body or ""
    )
    member_search_guard_sequence = bool(
        member_index_search
        and member_placement_guard
        and not (allocation_loop_body or "")[
            member_index_search.end() : member_placement_guard.start()
        ].strip()
    )
    if (
        not plan_validation_paths
        or not member_index_paths
        or member_index_methods
        != [
            "try_reserve_exact",
            "extend",
            "sort_unstable",
            "windows",
            "into_boxed_slice",
        ]
        or member_index_reservation is None
        or not member_index_sequence
        or member_flatten_loop is None
        or member_flatten_extend is None
        or (member_flatten_body or "").strip()
        != "member_placements.extend(members.iter().map(|member| (*member, *arena)));"
        or re.search(r"\blet\s+(?:mut\s+)?arena_members\b", member_index_body)
        or member_index_search is None
        or member_index_call is None
        or member_placement_guard is None
        or not member_search_guard_sequence
        or len(validation_member_bindings) != 1
        or len(member_arena_bindings) != 1
        or validation_member_methods != ["binary_search_by_key", "get"]
        or "memory object is listed by more than one arena"
        not in sources.get("src/core/src/memory_runtime/domain.rs", "")
    ):
        found.append("runtime plan member authority is not one-to-one")
    member_regression_paths = list(
        function_bodies(
            sources.get("src/core/tests/r6_memory_runtime.rs", ""),
            "plan_member_identity_validation_handles_many_zero_byte_objects_exactly",
        )
    )
    member_regression = member_regression_paths[0] if member_regression_paths else ""
    malformed_admission_is_atomic = bool(member_regression)
    for case in ("cross_listed", "wrong_arena", "nonadjacent"):
        ledger_captures = direct_matches(
            rf"let\s+{case}_ledger\s*=\s*{case}_domain\s*\.\s*"
            rf"ledger\s*\(\s*\)\s*;",
            member_regression,
        )
        rejected_admissions = direct_matches(
            rf"assert!\s*\(\s*matches!\s*\(\s*{case}_domain\s*\.\s*"
            rf"prepare_realization\s*\(",
            member_regression,
        )
        ledger_comparisons = direct_matches(
            rf"assert_eq!\s*\(\s*{case}_domain\s*\.\s*ledger\s*"
            rf"\(\s*\)\s*,\s*{case}_ledger\s*\)\s*;",
            member_regression,
        )
        ledger_capture = ledger_captures[0] if len(ledger_captures) == 1 else None
        rejected_admission = (
            rejected_admissions[0] if len(rejected_admissions) == 1 else None
        )
        ledger_comparison = (
            ledger_comparisons[0] if len(ledger_comparisons) == 1 else None
        )
        malformed_admission_is_atomic = malformed_admission_is_atomic and bool(
            ledger_capture
            and rejected_admission
            and ledger_comparison
            and ledger_capture.start()
            < rejected_admission.start()
            < ledger_comparison.start()
        )
    if not malformed_admission_is_atomic:
        found.append("malformed arena plans do not prove atomic admission")
    if not any(
        "*active != Some(key)" in body and "initialization.clear()" in body
        for body in function_bodies(domain_code, "enter_revision_point")
    ):
        found.append("reuse does not preserve initialization for one continuous owner lifetime")

    function_arguments = rust_code(
        sources.get("src/core/src/function/argument.rs", "")
    )
    for legacy_extractor in ("try_ref", "try_matrix", "try_copyable_matrix"):
        if re.search(rf"\bpub\s+fn\s+{legacy_extractor}\b", function_arguments):
            found.append(
                f"function ports expose legacy physical extractor {legacy_extractor}"
            )

    specialization_source = rust_code(
        sources.get("src/core/src/function/specialization.rs", "")
    )
    if re.search(
        r"has_managed_canonical_storage\s*\(\s*\).*?\|\|\s*matches!\s*\(\s*representation",
        specialization_source,
        re.DOTALL,
    ):
        found.append("numeric canonical matrices are forced out of typed managed storage")
    for legacy_extractor in ("try_ref", "try_matrix", "typed_cell"):
        if re.search(rf"\bpub\s+fn\s+{legacy_extractor}\b", specialization_source):
            found.append(
                f"source specialization exposes legacy physical extractor {legacy_extractor}"
            )
    if not re.search(
        r"\bValueCell\s*::\s*allocate_for_descriptor_in\s*\(",
        specialization_source,
    ):
        found.append("indirect runtime outputs can escape the invocation memory session")

    bytecode_constants = rust_code(
        sources.get("src/core/src/program/bytecode/constants/canonical.rs", "")
    )
    if re.search(r"\bValueCell\s*::\s*from_ref\s*\(", bytecode_constants):
        found.append("bytecode reconstruction installs pinned-external owned constants")
    if not re.search(r"\bMemoryDomain\s*::\s*new\s*\(", bytecode_constants):
        found.append("decoded constants do not share a managed bytecode session")

    interpreter = rust_code(sources.get("src/engine/src/interpreter/mod.rs", ""))
    literal = rust_code(sources.get("src/engine/src/literals.rs", ""))
    interpreter_body = balanced_body(interpreter, "Interpreter")
    if interpreter_body is None or not re.search(
        r"\bmemory_domain\s*:\s*MemoryDomain\b", interpreter_body
    ):
        found.append("Interpreter does not own one ordinary program memory session")
    if not re.search(r"\.\s*import_owned_in\s*\(\s*p\.memory_domain\s*\(\s*\)\s*\)", literal):
        found.append("source literals do not enter the interpreter memory session")
    conversion_staging = list(function_bodies(literal, "stage_conversion_output"))
    if (
        not conversion_staging
        or "snapshot_input_cell_with_construction" not in conversion_staging[0]
        or "try_rebuild_data_draft" not in conversion_staging[0]
        or "execute_fixed_conversion_plan" not in conversion_staging[0]
        or "execute_conversion_plan" in conversion_staging[0]
        or "from_resolved_descriptor_data" in conversion_staging[0]
    ):
        found.append("managed conversion escapes its frame-owned construction authority")
    fixed_conversion = list(function_bodies(access, "execute_fixed_conversion_plan"))
    canonical_c32 = list(function_bodies(access, "convert_canonical_c32_port_lanes"))
    if (
        not fixed_conversion
        or not any("K::C32 => canonical_c32_target!()" in body for body in fixed_conversion)
        or not canonical_c32
        or "snapshot_input_cell(input, 0)" not in canonical_c32[0]
        or "execute_scalar_conversion" not in canonical_c32[0]
        or "try_fill_column_major" not in canonical_c32[0]
    ):
        found.append("managed fixed conversion rejects canonical C32 sources")
    memory_model = rust_code(sources.get("src/core/src/memory_plan/model.rs", ""))
    memory_derive = rust_code(sources.get("src/core/src/memory_plan/derive.rs", ""))
    program_planner = rust_code(sources.get("src/engine/src/memory_planner/program.rs", ""))
    if (
        "ReservationOnlyWorkspace" not in memory_model
        or "construction_arena_for_space" not in memory_derive
        or "AllocationRole::ConstructionWorkspace" not in memory_derive
        or "ArenaBackingKind::ReservationOnlyWorkspace" not in domain
        or "ArenaBackingKind::ReservationOnlyWorkspace" not in program_planner
    ):
        found.append("canonical construction workspace retains duplicate contiguous backing")
    if (
        "allocation.role == AllocationRole::ConstructionWorkspace" not in domain
        or "construction != reservation_only" not in domain
    ):
        found.append("construction workspace role can enter a physical arena")

    table_ops = rust_code(sources.get("src/engine/src/intrinsics/table_ops.rs", ""))
    canonical_table = balanced_body(table_ops, "CanonicalTable")
    managed_join = [
        body
        for body in function_bodies(table_ops, "solve_managed")
        if "joined_table_data_with_construction" in body
    ]
    join_construction = list(
        function_bodies(table_ops, "joined_table_data_with_construction")
    )
    table_value = list(function_bodies(table_ops, "value"))
    sequence_draft = list(function_bodies(table_ops, "sequence_draft_at"))
    if (
        canonical_table is None
        or "snapshot: mech_core::Value" not in canonical_table
        or not managed_join
        or managed_join[0].count("CanonicalTable::from_borrowed") != 2
        or "canonical_data_draft" in managed_join[0]
        or "to_values" in managed_join[0]
        or not join_construction
        or not table_value
        or "sequence_draft_at" not in table_value[0]
        or not sequence_draft
        or "canonical_snapshot_data_draft" not in sequence_draft[0]
        or "lhs.value" not in join_construction[0]
        or "rhs.value" not in join_construction[0]
        or "try_finish_preallocated_with" not in join_construction[0]
    ):
        found.append("managed table join materializes input columns before construction admission")
    conversion_footprints = list(
        function_bodies(literal, "prospective_conversion_output_footprint")
    )
    if (
        not conversion_footprints
        or "conversion_string_payload_bound(&plan.step)" not in conversion_footprints[0]
        or "footprint.payload_bytes" not in conversion_footprints[0]
        or "footprint.encoded_bytes" not in conversion_footprints[0]
    ):
        found.append("conversion footprint ignores target payload expansion")

    structures = rust_code(sources.get("src/engine/src/structures.rs", ""))
    if (
        "snapshot_input_cell_with_construction" not in structures
        or "try_rebuild_tuple_values" not in structures
        or "try_rebuild_record_values" not in structures
        or "try_rebuild_table_values" not in structures
        or "try_rebuild_matrix_drafts_with" not in structures
    ):
        found.append("aggregate packs reread cells outside frame-owned construction")
    table_ops = rust_code(sources.get("src/engine/src/intrinsics/table_ops.rs", ""))
    join_solves = list(function_bodies(table_ops, "solve_managed"))
    if not any(
        "snapshot_input_cell_with_construction" in body
        and "joined_table_data_with_construction" in body
        and "try_rebuild_data_draft" in body
        for body in join_solves
    ) or "try_finish_preallocated_with" not in table_ops or table_ops.count("try_vec_with_capacity") < 5:
        found.append("table join constructs a detached logical cell during execution")

    module = rust_code(sources.get("src/engine/src/function/module.rs", ""))
    dynamic_resident = list(function_bodies(module, "dynamic_resident_execute"))
    if (
        not dynamic_resident
        or "candidate.as_mut_ptr()" not in dynamic_resident[0]
        or re.search(r"\bvec\s*!\s*\[", dynamic_resident[0])
        or ".to_vec(" in dynamic_resident[0]
    ):
        found.append("dynamic Resident module allocates private output-sized scratch")

    # A runtime factory may retain a stable logical cell, never a physical
    # `Ref`/matrix wrapper captured during binding.  Compiler-only compatibility
    # structs can remain while their frozen IDs are redirected, so enforce the
    # boundary specifically at every executable constructor.
    intrinsic_root = root / "src/engine/src/intrinsics"
    if intrinsic_root.is_dir():
        for relative, source in rust_files(root, ("src/engine/src/intrinsics",)):
            for body in function_bodies(source, "new_invocation"):
                if re.search(r"\.\s*(?:try_ref|try_copyable_matrix)\s*\(", body):
                    found.append(
                        f"{relative}: runtime factory retains physical input/output backing"
                    )
                    break

    # Frozen concrete concatenation factory identities remain registration
    # markers only. Their executable constructor is the shared managed
    # logical-port implementation; restoring a Ref/CopyMat payload or an
    # alternate executor on a marker recreates the legacy physical route.
    for relative in (
        "src/engine/src/intrinsics/horzcat.rs",
        "src/engine/src/intrinsics/vertcat.rs",
    ):
        code = rust_code(sources.get(relative, ""))
        if re.search(r"\b(?:Ref|CopyMat)\s*<", code):
            found.append(
                f"{relative}: concatenation factory marker owns legacy physical backing"
            )
        if re.search(r"\bimpl\s*<T>\s+MechFunction(?:Impl|Compiler)\s+for\b", code):
            found.append(
                f"{relative}: concatenation factory marker restores a parallel executor"
            )
        prefix = "horzcat" if relative.endswith("horzcat.rs") else "vertcat"
        allowed_macros = {
            f"{prefix}_single",
            f"{prefix}_two_args",
            f"{prefix}_three_args",
            f"{prefix}_four_args",
        }
        declared_macros = set(
            re.findall(rf"\bmacro_rules!\s+({prefix}_[A-Za-z0-9_]+)\b", code)
        )
        if declared_macros - allowed_macros:
            found.append(
                f"{relative}: concatenation marker retains a legacy element-copy body"
            )

    variable_definitions = rust_code(
        sources.get("src/engine/src/intrinsics/define.rs", "")
    )
    if re.search(r"\bRef\s*<", variable_definitions):
        found.append("variable-definition factory marker owns legacy physical backing")
    if re.search(
        r"\bimpl(?:\s*<[^>{}]*>)?\s+MechFunction(?:Impl|Compiler)\s+for\s+"
        r"(?:VariableDefine|\[<VariableDefine)",
        variable_definitions,
    ):
        found.append("variable-definition factory marker restores a parallel executor")

    # Resident lanes must project the realized R5 arenas themselves. A second
    # Box/Vec arena would make the domain ledger an accounting sidecar rather
    # than the storage used by execution.
    resident_relative = "src/engine/src/resident/general/mod.rs"
    resident = rust_code(sources.get(resident_relative, ""))
    lane = balanced_body(resident, "ResidentLane")
    reactive = balanced_body(resident, "ReactiveInstance")
    resident_lane_paths = list(function_bodies(resident, "resident_lane"))
    if lane is None or "PlannedArenaProjection" not in lane:
        found.append("Resident lanes do not project their realized R5 host arenas")
    if (
        not resident_lane_paths
        or "project_host_arena(memory.realized(), arena.id, len)"
        not in resident_lane_paths[0]
        or "arena.members.first()" in resident_lane_paths[0]
    ):
        found.append("Resident lanes use one member as whole-arena projection authority")
    resident_planner = rust_code(
        sources.get("src/engine/src/memory_planner/resident.rs", "")
    )
    effect_payload_paths = list(
        function_bodies(resident_planner, "plan_resident_effect_payload")
    )
    if (
        not effect_payload_paths
        or "slot: Some(resident_planned_slot(kind))" not in effect_payload_paths[0]
    ):
        found.append("Resident effect payload omits its typed arena slot")
    if reactive is None or not re.search(r"\bManagedProgramMemory\b", reactive):
        found.append("ReactiveInstance does not retain its managed program realization")
    if re.search(r"\b[A-Za-z_][A-Za-z0-9_]*\.state\.clone\s*\(", resident):
        found.append("Resident state migration clones an unplanned arena")

    allocation = rust_code(sources.get("src/core/src/memory_runtime/allocation.rs", ""))
    if re.search(r"\bpub(?:\([^)]*\))?\s+struct\s+PlannedHostArenaAllocator\b", allocation):
        found.append("planned host arena allocator escapes its sealed projection boundary")

    # Native GPU storage and accepted submissions share one ownership
    # lifecycle. Accounting-only attachment remains a test seam only.
    gpu_relative = "hosts/gpu/src/memory.rs"
    gpu = rust_code(sources.get(gpu_relative, ""))
    registered = balanced_body(gpu, "RegisteredDeviceBuffer")
    if registered is None or "wgpu::Buffer" not in registered or "DeviceAllocationOwner" not in registered:
        found.append("registered GPU buffers do not couple the buffer and allocation owner")
    planned_gpu = balanced_body(gpu, "PlannedGpuExecution")
    if planned_gpu is None or "writable_state_objects" not in planned_gpu:
        found.append("GPU write accounting does not distinguish double-buffer bind groups")
    gpu_source = sources.get(gpu_relative, "")
    if not re.search(
        r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*pub\s*\(\s*crate\s*\)\s*fn\s+attach_device_allocation\b",
        gpu_source,
    ):
        found.append("accounting-only GPU allocation attachment is available in production")
    gpu_production = "\n".join(
        rust_code(source)
        for relative, source in rust_files(root, ("hosts/gpu/src",))
        if not relative.endswith("/tests.rs")
    )
    if re.search(r"\.\s*(?:track|track_at)\s*\(", gpu_production):
        found.append("GPU submission can register ownership after backend submission")
    if re.search(
        r"debug_assert!\s*\(\s*[^,;]*\.\s*(?:release|complete)\s*\(",
        gpu_production,
    ):
        found.append("GPU cleanup is hidden inside debug_assert")
    for relative in ("hosts/gpu/src/native.rs", "hosts/gpu/src/batched/mod.rs"):
        backend = rust_code(sources.get(relative, ""))
        if not re.search(r"\.\s*writable_state_objects\s*\(\s*group\s*\)", backend):
            found.append(f"{relative}: completed writes ignore the submitted state bind group")

    browser = sources.get("include/browser-compute.js", "")
    if "queue.onSubmittedWorkDone()" not in browser:
        found.append("browser GPU ownership is not retained through queue completion")
    if "Promise.allSettled" not in browser:
        found.append("browser GPU mapping cleanup can skip siblings after rejection")
    project = sources.get("include/project.js", "")
    completion = "await globalThis.MechBrowserCompute.awaitSmokeTargetCompletion(target);"
    passed = "root.dataset.mechGpuSmoke = 'passed';"
    if (
        completion not in project
        or passed not in project
        or project.index(completion) > project.index(passed)
        or "async function awaitSmokeTargetCompletion(target)" not in browser
        or "await Promise.resolve(completion);" not in browser
        or "throw target.bridgeFailure;" not in browser
    ):
        found.append("browser compute smoke can pass before its final submission completes")

    # The execution-owned core may use unsafe only in the sealed allocation and
    # typed-view modules. Policy, identity, payload, and transaction code stay safe.
    for relative, source in rust_files(root, ("src/core/src/memory_runtime",)):
        if relative.endswith(("/allocation.rs", "/access.rs", "/payload.rs")):
            continue
        if re.search(r"\bunsafe\b", rust_code(source)):
            found.append(f"{relative}: unsafe escapes the sealed allocation/access boundary")

    # Production may not hide an incomplete cutover behind a feature or legacy
    # fallback, nor accept the R5 placeholder capacity disposition.
    production = tuple(
        relative
        for relative in ("src/core/src", "src/engine/src", "src/runtime/src", "src/compute/src", "hosts/gpu/src")
        if (root / relative).exists()
    )
    for relative, source in rust_files(root, production):
        code = rust_code(source)
        if re.search(
            r"\b(?:legacy_unmanaged|unmanaged_fallback|disable_managed_memory)\b",
            code + "\n" + source,
        ):
            found.append(f"{relative}: production exposes an unmanaged-memory bypass")
        if "CapacityDeferredToR6" in code and not relative.endswith(
            ("/memory_plan/model.rs", "/memory_planner/audit.rs")
        ):
            found.append(f"{relative}: production accepts CapacityDeferredToR6 after cutover")

    docs = sources.get("docs/design/r6-memory-runtime-cutover.md", "")
    if "Status: implementation in progress" not in docs and "Status: complete" not in docs:
        found.append("R6 design status is missing")

    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", nargs="?", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if found:
        print("R6 memory-runtime contract violations:", file=sys.stderr)
        for item in found:
            print(f"- {item}", file=sys.stderr)
        return 1
    print("R6 memory-runtime contract is satisfied.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
