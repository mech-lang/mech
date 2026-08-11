#!/usr/bin/env python3
"""Enforce the D0 ProgramArtifact-to-resident activation boundary."""

from __future__ import annotations

import copy
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_DIR = ROOT / "tests/architecture/resident-activation"
BOUNDARY_PATH = CONTRACT_DIR / "d0-boundary.json"
PROJECTION_PATH = CONTRACT_DIR / "d0-migration-projection.json"
GENERATOR_PATH = ROOT / "scripts/generate-resident-activation-contract.py"
D0_C4_SEMANTIC_BASE = "33298522331d40960175427052ce363bb5e424df"
D0_PR_BASE = "a9d06ee20ceb03d56c3ba465b726aa4a69427af8"
D0_GATE_B_IMPLEMENTATION = "b9ee8d1be7633f8b434947b748a810843bbd2144"
D0_FINAL_COMMIT = "a9422eff9908e967e0537f7ec1fa56e7bd05eb8d"
D0_ALLOWED_CHANGES = (
    ".github/workflows/ci.yml",
    "docs/design/index.mec",
    "docs/design/program-artifact-resident-activation.md",
    "scripts/check-resident-activation-contract.py",
    "scripts/generate-resident-activation-contract.py",
    "scripts/tests/test_check_resident_activation_contract.py",
    "scripts/tests/test_generate_resident_activation_contract.py",
    "scripts/tests/fixtures/resident-activation/",
    "src/engine/tests/resident_activation_contract.rs",
    "tests/architecture/value-system/current-inventory.json",
    "tests/architecture/resident-activation/",
)
D0_CURRENT_INVENTORY_BLOB = "5b5fd877143cba1d7945d850405a45975930e6f4"
INVENTORY_PATH = "tests/architecture/value-system/current-inventory.json"
D0_TEST_SOURCE = "src/engine/tests/resident_activation_contract.rs"
D0_INVENTORY_TARGET = {
    "kinds": ["test"],
    "name": "resident_activation_contract",
    "reachable_rust_files": [D0_TEST_SOURCE],
    "root": D0_TEST_SOURCE,
}
EXPECTED_PUBLICATION_CONTRACT = {
    "store_count": 1,
    "writer_ordering": "Release",
    "reader_ordering": "Acquire",
    "abort_preserves_published_epoch": True,
    "ordered_steps": [
        "reserve",
        "begin",
        "execute",
        "validate",
        "summary",
        "prepare",
        "publish",
        "append",
    ],
    "capacity_reserved_before_execution": True,
    "candidate_executes_before_receipt_preparation": True,
    "candidate_summary_before_receipt_preparation": True,
    "receipt_prepared_before_publish": True,
    "append_after_publish": "infallible",
}

LEGACY_TOKENS = (
    "LegacyValue",
    "ValRef",
    "MutableReference",
    "ReactiveCellId",
    "ValueStateJournal",
    "ReactiveTurnJournal",
    "RuntimeExecutionTransaction",
    "RuntimeTransaction",
    "transaction_state_values",
    "commit_runtime",
    "RefCell",
    "Rc",
)
POINTER_PATTERNS = {
    "as_ptr": re.compile(r"\.as_ptr\s*\("),
    "as_mut_ptr": re.compile(r"\.as_mut_ptr\s*\("),
    "addr": re.compile(r"\baddr\b"),
    "expose_addr": re.compile(r"\bexpose_addr\b"),
    "from_exposed_addr": re.compile(r"\bfrom_exposed_addr\b"),
}
HOT_FUNCTION_PARTS = (
    "begin_candidate",
    "execute",
    "run_turn",
    "publish",
    "solve",
    "schedule",
)
HOT_TOKENS = (
    "SchemaTable",
    "OperationContractTable",
    "ProgramArtifact",
    "canonical_bytes",
    "value_hash",
    "key_hash",
    "snapshot_from_legacy",
    "legacy_from_snapshot",
)
PROGRAM_ARTIFACT_DECLARATION = re.compile(
    r"(?m)^\s*(?P<visibility>pub(?:\([^)]*\))?\s+)?struct\s+ProgramArtifact\s*\{"
)


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def command(args: list[str], root: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=root, check=False, text=True, capture_output=True)


def changed_paths(root: Path, base: str, head: str = "HEAD") -> list[str]:
    result = command(["git", "diff", "--name-only", base, head], root)
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "unable to enumerate D0 changed paths")
    return [line for line in result.stdout.splitlines() if line]


def path_is_allowed(path: str, allowed: list[str]) -> bool:
    return any(path == entry or (entry.endswith("/") and path.startswith(entry)) for entry in allowed)


def validate_changed_paths(paths: list[str], allowed: list[str]) -> list[str]:
    unexpected = sorted(path for path in paths if not path_is_allowed(path, allowed))
    if not unexpected:
        return []
    return ["changed path is outside the exact D0 allowlist: " + ", ".join(unexpected)]


def validate_commit_topology(root: Path) -> list[str]:
    failures = []
    ancestor = command(
        ["git", "merge-base", "--is-ancestor", D0_FINAL_COMMIT, "HEAD"], root
    )
    if ancestor.returncode != 0:
        failures.append(f"D0 final commit {D0_FINAL_COMMIT} must be an ancestor of HEAD")
    count = command(
        ["git", "rev-list", "--count", f"{D0_PR_BASE}..{D0_FINAL_COMMIT}"], root
    )
    if count.returncode != 0 or count.stdout.strip() != "5":
        detail = count.stderr.strip() or count.stdout.strip() or "unavailable"
        failures.append(f"D0 final must contain exactly five commits after D0_PR_BASE; found {detail}")
    parent = command(["git", "rev-parse", f"{D0_FINAL_COMMIT}~5"], root)
    if parent.returncode != 0 or parent.stdout.strip() != D0_PR_BASE:
        actual = parent.stdout.strip() or parent.stderr.strip() or "unavailable"
        failures.append(f"D0_FINAL_COMMIT~5 must equal D0_PR_BASE {D0_PR_BASE}; found {actual}")
    semantic_ancestor = command(
        ["git", "merge-base", "--is-ancestor", D0_C4_SEMANTIC_BASE, D0_PR_BASE], root
    )
    if semantic_ancestor.returncode != 0:
        failures.append("D0_C4_SEMANTIC_BASE must be an ancestor of D0_PR_BASE")
    semantic_diff = command(
        ["git", "diff", "--quiet", D0_C4_SEMANTIC_BASE, D0_PR_BASE], root
    )
    if semantic_diff.returncode != 0:
        failures.append("D0_C4_SEMANTIC_BASE and D0_PR_BASE must have identical trees")
    return failures


def validate_boundary_policy(boundary: dict) -> list[str]:
    failures = []
    if boundary.get("base_commit") != D0_C4_SEMANTIC_BASE:
        failures.append(
            f"D0 boundary base_commit must remain independently pinned to {D0_C4_SEMANTIC_BASE}"
        )
    if boundary.get("pr_base_commit") != D0_PR_BASE:
        failures.append(
            f"D0 boundary pr_base_commit must remain independently pinned to {D0_PR_BASE}"
        )
    if boundary.get("allowed_changes") != list(D0_ALLOWED_CHANGES):
        failures.append("D0 boundary allowed_changes differs from the independently pinned allowlist")
    if boundary.get("publication_contract") != EXPECTED_PUBLICATION_CONTRACT:
        failures.append("D0 publication_contract differs from the exact reserve/execute/prepare/publish/append sequence")
    return failures


def expected_inventory_after_d0(baseline: dict) -> dict:
    expected = copy.deepcopy(baseline)
    engine = next(
        fixture
        for fixture in expected["auxiliary_cargo_fixtures"]
        if fixture["manifest"] == "src/engine/Cargo.toml"
    )
    engine["targets"].append(copy.deepcopy(D0_INVENTORY_TARGET))
    engine["targets"].sort(key=lambda target: target["name"])
    expected["enumerated_rust_files"].append(D0_TEST_SOURCE)
    expected["enumerated_rust_files"].sort()
    package_counts = {"mech": 912, "mech-engine": 143}
    for package in expected["workspace_packages"]:
        if package["name"] not in package_counts:
            continue
        baseline_count = package_counts[package["name"]]
        if package["rust_file_count"] != baseline_count:
            raise ValueError(
                f"D0 inventory baseline for {package['name']} is {package['rust_file_count']}, expected {baseline_count}"
            )
        package["rust_file_count"] = baseline_count + 1
    return expected


def validate_inventory_documents(baseline: dict, current: dict) -> list[str]:
    try:
        expected = expected_inventory_after_d0(baseline)
    except (KeyError, StopIteration, ValueError) as error:
        return [f"unable to construct the frozen D0 inventory delta: {error}"]
    if current == expected:
        return []
    return [
        "current-inventory.json exceeds the frozen D0 delta: exactly one resident_activation_contract target, one enumerated Rust source, the mech 912→913 count, and the mech-engine 143→144 count are permitted; every legacy occurrence must remain unchanged"
    ]


def validate_inventory_delta(root: Path, base: str, current_commit: str = "HEAD") -> list[str]:
    baseline_source = git_source(root, base, INVENTORY_PATH)
    if not baseline_source:
        return [f"unable to read {INVENTORY_PATH} at pinned D0 base {base}"]
    try:
        baseline = json.loads(baseline_source)
        current = json.loads(git_source(root, current_commit, INVENTORY_PATH))
    except (json.JSONDecodeError, OSError) as error:
        return [f"unable to read D0 inventory documents: {error}"]
    return validate_inventory_documents(baseline, current)


def validate_inventory_blob_id(actual: str) -> list[str]:
    if actual == D0_CURRENT_INVENTORY_BLOB:
        return []
    return [
        f"current-inventory.json blob must remain {D0_CURRENT_INVENTORY_BLOB}; found {actual or 'unavailable'}"
    ]


def validate_inventory_blob(root: Path, commit: str = "HEAD") -> list[str]:
    result = command(["git", "rev-parse", f"{commit}:{INVENTORY_PATH}"], root)
    if result.returncode != 0:
        return [result.stderr.strip() or "unable to hash current-inventory.json"]
    return validate_inventory_blob_id(result.stdout.strip())


def production_rust_sources(root: Path) -> dict[str, str]:
    sources = {}
    for path in sorted(root.glob("src/*/src/**/*.rs")):
        relative = path.relative_to(root).as_posix()
        sources[relative] = path.read_text(encoding="utf-8")
    return sources


def production_rust_sources_at_commit(root: Path, commit: str) -> dict[str, str]:
    listing = command(["git", "ls-tree", "-r", "--name-only", commit, "src"], root)
    if listing.returncode != 0:
        return {}
    return {
        path: git_source(root, commit, path)
        for path in listing.stdout.splitlines()
        if re.fullmatch(r"src/[^/]+/src/.+\.rs", path)
    }


def validate_artifact_authority(sources: dict[str, str]) -> list[str]:
    declarations = []
    for path, source in sources.items():
        for match in PROGRAM_ARTIFACT_DECLARATION.finditer(source):
            declarations.append((path, (match.group("visibility") or "").strip()))
    expected_paths = {
        "src/engine/src/artifact/model.rs",
        "src/engine/src/resident/artifact.rs",
    }
    actual_paths = {path for path, _ in declarations}
    failures = []
    if len(declarations) != 2 or actual_paths != expected_paths:
        failures.append(
            "ProgramArtifact authority must contain exactly the finalized public artifact and the private Gate B control; found "
            + ", ".join(f"{path} ({visibility or 'private'})" for path, visibility in declarations)
        )
        return failures
    visibility = dict(declarations)
    if visibility["src/engine/src/artifact/model.rs"] != "pub":
        failures.append("finalized mech_engine::ProgramArtifact is no longer public authority")
    private_visibility = visibility["src/engine/src/resident/artifact.rs"]
    if private_visibility == "pub":
        failures.append("private Gate B ProgramArtifact became publicly visible")
    if private_visibility not in ("", "pub(crate)", "pub(super)", "pub(self)"):
        failures.append("private Gate B ProgramArtifact has an unapproved visibility")
    return failures


def resident_sources(root: Path) -> dict[str, str]:
    directory = root / "src/engine/src/resident"
    return {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(directory.rglob("*.rs"))
    }


def git_source(root: Path, base: str, path: str) -> str:
    result = command(["git", "show", f"{base}:{path}"], root)
    return result.stdout if result.returncode == 0 else ""


def token_count(source: str, token: str) -> int:
    return len(re.findall(rf"\b{re.escape(token)}\b", source))


def validate_new_legacy_dependencies(
    current: dict[str, str], baseline: dict[str, str]
) -> list[str]:
    failures = []
    for path, source in current.items():
        before = baseline.get(path, "")
        for token in LEGACY_TOKENS:
            growth = token_count(source, token) - token_count(before, token)
            if growth > 0:
                failures.append(f"resident module adds {growth} new {token} reference(s) in {path}")
    return failures


def matching_brace(source: str, opening: int) -> int | None:
    depth = 0
    for offset in range(opening, len(source)):
        if source[offset] == "{":
            depth += 1
        elif source[offset] == "}":
            depth -= 1
            if depth == 0:
                return offset
    return None


def function_spans(source: str) -> list[tuple[str, int, int, int]]:
    spans = []
    for match in re.finditer(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)[^{}]*\{", source):
        opening = source.find("{", match.start())
        closing = matching_brace(source, opening)
        if closing is not None:
            spans.append((match.group(1), match.start(), opening, closing + 1))
    return spans


def enclosing_function(source: str, offset: int) -> tuple[str, int, int, int] | None:
    candidates = [span for span in function_spans(source) if span[2] <= offset < span[3]]
    return max(candidates, key=lambda span: span[2], default=None)


def validate_pointer_identity(sources: dict[str, str]) -> list[str]:
    failures = []
    helper_specs = {
        ("src/engine/src/resident/arena.rs", "buffer_addresses"),
        ("src/engine/src/resident/program_execution.rs", "version_addresses_for_d1_test"),
    }
    helpers_seen = set()
    for path, source in sources.items():
        for token, pattern in POINTER_PATTERNS.items():
            for match in pattern.finditer(source):
                function = enclosing_function(source, match.start())
                allowed = False
                if token == "as_ptr" and function is not None:
                    name, start, _, end = function
                    attributes = source[max(0, start - 128) : start]
                    helper = (path, name)
                    if helper in helper_specs and "#[cfg(test)]" in attributes:
                        allowed = True
                        helpers_seen.add(helper)
                        helper_source = source[start:end]
                        if len(POINTER_PATTERNS["as_ptr"].findall(helper_source)) != 4:
                            failures.append(
                                f"test-only {name} helper must contain exactly four as_ptr calls"
                            )
                if not allowed:
                    location = "outside a function" if function is None else f"in {function[0]}"
                    failures.append(f"resident pointer identity token {token} appears in {path} {location}")
    missing_helpers = helper_specs.difference(helpers_seen)
    for path, name in sorted(missing_helpers):
        failures.append(f"frozen #[cfg(test)] {path}::{name} pointer helper is missing")
    return sorted(set(failures))


def hot_token_counts(source: str) -> dict[tuple[str, str], int]:
    counts = {}
    for name, _, opening, closing in function_spans(source):
        if not any(part in name for part in HOT_FUNCTION_PARTS):
            continue
        body = source[opening:closing]
        for token in HOT_TOKENS:
            count = token_count(body, token)
            if count:
                counts[(name, token)] = counts.get((name, token), 0) + count
    return counts


def validate_hot_turn_boundary(
    current: dict[str, str], baseline: dict[str, str]
) -> list[str]:
    failures = []
    for path, source in current.items():
        current_counts = hot_token_counts(source)
        base_counts = hot_token_counts(baseline.get(path, ""))
        for (function, token), count in current_counts.items():
            growth = count - base_counts.get((function, token), 0)
            if growth > 0:
                failures.append(f"resident hot function {function} adds {token} lookup in {path}")
    return failures


def load_generator(root: Path = ROOT):
    path = root / GENERATOR_PATH.relative_to(ROOT)
    spec = importlib.util.spec_from_file_location("d0_resident_generator", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load D0 generator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_source_contract(source: str, workload: dict, root: Path = ROOT) -> list[str]:
    return load_generator(root).validate_source(source, workload)


def validate_migration_status(projection: dict) -> list[str]:
    failures = []
    for target in projection.get("targets", []):
        if target.get("implemented") is not False:
            failures.append(f"D migration target {target.get('id')} is prematurely marked implemented")
        if target.get("legacy_removed") is not False:
            failures.append(f"D migration target {target.get('id')} is prematurely marked legacy_removed")
    return failures


def validate_gate_b_expected(expected: str) -> list[str]:
    if expected == D0_GATE_B_IMPLEMENTATION:
        return []
    return [
        f"Gate B expected implementation drifted to {expected}; required {D0_GATE_B_IMPLEMENTATION}"
    ]


def subprocess_failure(label: str, result: subprocess.CompletedProcess[str]) -> list[str]:
    if result.returncode == 0:
        return []
    output = (result.stderr + result.stdout).strip()
    return [f"{label} failed: {output}"]


def run(root: Path = ROOT) -> list[str]:
    contract_dir = root / CONTRACT_DIR.relative_to(ROOT)
    boundary = json.loads(
        git_source(root, D0_FINAL_COMMIT, "tests/architecture/resident-activation/d0-boundary.json")
    )
    base = D0_PR_BASE
    failures = validate_commit_topology(root)
    if failures:
        return failures
    failures.extend(validate_boundary_policy(boundary))
    try:
        failures.extend(
            validate_changed_paths(
                changed_paths(root, base, D0_FINAL_COMMIT), list(D0_ALLOWED_CHANGES)
            )
        )
    except RuntimeError as error:
        failures.append(str(error))
    failures.extend(validate_inventory_blob(root, D0_FINAL_COMMIT))
    failures.extend(validate_inventory_delta(root, base, D0_FINAL_COMMIT))

    production = production_rust_sources_at_commit(root, D0_FINAL_COMMIT)
    failures.extend(validate_artifact_authority(production))
    current_resident = resident_sources(root)
    baseline_resident = {
        path: git_source(root, D0_FINAL_COMMIT, path) for path in current_resident
    }
    failures.extend(validate_new_legacy_dependencies(current_resident, baseline_resident))
    failures.extend(validate_pointer_identity(current_resident))
    failures.extend(validate_hot_turn_boundary(current_resident, baseline_resident))

    generator = command(
        [sys.executable, str(root / GENERATOR_PATH.relative_to(ROOT)), "--check"], root
    )
    failures.extend(subprocess_failure("D0 generated contract", generator))

    try:
        gate_b = json.loads(
            git_source(root, D0_FINAL_COMMIT, "benchmarks/runtime/gate-b/b2-resident-turn.json")
        )
        failures.extend(validate_gate_b_expected(gate_b.get("git_commit", "")))
    except json.JSONDecodeError as error:
        failures.append(f"unable to read frozen D0 Gate B evidence: {error}")
    migration = json.loads(
        git_source(
            root,
            D0_FINAL_COMMIT,
            "tests/architecture/resident-activation/d0-migration-projection.json",
        )
    )
    failures.extend(validate_migration_status(migration))
    return failures


def main() -> int:
    failures = run()
    if failures:
        for failure in failures:
            print(f"D0 contract failure: {failure}", file=sys.stderr)
        return 1
    print("D0 resident activation contract checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
