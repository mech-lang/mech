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
    "src/core/src/snapshot/encoding.rs",
    "src/core/src/snapshot/validation.rs",
    "machines/string/src/lib.rs",
    "machines/matrix/src/transpose.rs",
    "machines/set/src/canonical.rs",
    "machines/set/src/operations/union.rs",
    "src/engine/src/memory_runtime/mod.rs",
    "src/engine/src/memory_runtime/realize.rs",
    "src/engine/src/memory_runtime/resident.rs",
    "src/engine/src/resident/general/mod.rs",
    "src/engine/src/interpreter/mod.rs",
    "src/engine/src/literals.rs",
    "src/engine/src/intrinsics/define.rs",
    "src/engine/src/intrinsics/constructors.rs",
    "src/engine/src/intrinsics/horzcat.rs",
    "src/engine/src/intrinsics/vertcat.rs",
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
    admitted_builders.extend(
        function_bodies(access, "with_admitted_canonical_binary_port_values")
    )
    if not admitted_builders or not all(
        "prepare_frozen_snapshot" in body
        and "build(" in body
        and body.index("prepare_frozen_snapshot") < body.index("build(")
        for body in admitted_builders
    ):
        found.append("maintained canonical construction does not admit before building")
    object_value_views = list(function_bodies(access, "with_object_value_view"))
    if not object_value_views or not re.search(
        r"initialization\s*\.\s*contains_region", object_value_views[0]
    ):
        found.append("read-capable object views can expose uninitialized managed storage")
    payload = rust_code(sources.get("src/core/src/memory_runtime/payload.rs", ""))
    if "PreparedFrozenSnapshotAdmission" not in payload or not any(
        "record_initialized" in body
        for body in function_bodies(payload, "complete")
    ):
        found.append("canonical payload admission is not completed after valid construction")
    envelope = balanced_body(payload, "PayloadEnvelopeOwner")
    if (
        envelope is None
        or "block_capacity" not in envelope
        or "owner.block_capacity" not in payload
    ):
        found.append("payload node admission depends on allocator spare capacity")
    snapshot = rust_code(sources.get("src/core/src/snapshot/validation.rs", ""))
    rebinds = list(function_bodies(snapshot, "rebind"))
    if (
        "FrozenSnapshotData" not in snapshot
        or not rebinds
        or "return Ok(self.clone())" not in rebinds[0]
        or "data: self.root.data.clone()" not in snapshot
        or "schema_body_contains_dynamic" not in rebinds[0]
        or "schemas: Some(Arc::new(schemas.clone()))" not in rebinds[0]
    ):
        found.append("canonical snapshots do not preserve shared frozen ownership")
    if "publication_shape" not in cell or not any(
        "publication_shape" in body and "publication_locked" in body
        for body in function_bodies(cell, "lock_publication")
    ):
        found.append("ready publication does not retain conflict-free shape authority")

    # The maintained entry itself must require a frame. A second opt-in trait is
    # not a cutover because FunctionInstance can still invoke raw implementations.
    function_path = root / "src/core/src/function/mod.rs"
    function = rust_code(function_path.read_text(encoding="utf-8")) if function_path.is_file() else ""
    implementation = balanced_body(function, "MechFunctionImpl")
    if implementation is None or not re.search(r"\bfn\s+solve_managed\b", implementation):
        found.append("MechFunctionImpl does not require solve_managed")
    if implementation is not None and re.search(r"\bfn\s+solve_result(?:_with)?\b", implementation):
        found.append("MechFunctionImpl retains an unmanaged solve entry")
    if "planned_output_footprints" not in function or "resolve_current_call_memory" not in function:
        found.append("payload-dependent calls do not refresh live and prospective footprints")
    if "validate_transaction_authority" not in function or not any(
        "validate_transaction_authority" in body
        for body in function_bodies(function, "new")
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
    if not any(
        "arena_projection_owner" in body and "leases.is_empty()" in body
        for body in function_bodies(domain_code, "project_host_arena")
    ) or not any(
        "arena_projection_owner" in body for body in function_bodies(access, "acquire_call")
    ):
        found.append("resident arena projections and frame leases have separate authorities")
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
    if lane is None or "PlannedArenaProjection" not in lane:
        found.append("Resident lanes do not project their realized R5 host arenas")
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
