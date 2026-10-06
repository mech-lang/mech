#!/usr/bin/env python3
"""Classify a change into the smallest safe pull-request validation set."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Iterable

from ci_owners import DEFAULT_OWNER_CONFIG, load_owners, matching_owners


CONTRACT_FAMILIES = (
    "architecture", "artifact", "language", "memory", "numeric", "runtime",
    "cli", "linkage", "native", "browser", "distribution",
)


def representative_paths(encoded: str, expected_revision: str, actual_revision: str) -> list[str]:
    """Validate an explicitly simulated impact set for a real exact-tree run."""
    if not re.fullmatch(r"[0-9a-f]{40}", expected_revision) or expected_revision != actual_revision:
        raise ValueError("measurement revision must equal the exact workflow revision")
    paths = json.loads(encoded)
    if not isinstance(paths, list) or not paths or any(
        not isinstance(path, str) or not path or path.startswith(("/", "\\"))
        or "\\" in path or any(part in {"", ".", ".."} for part in path.split("/"))
        or any(ord(char) < 32 for char in path)
        for path in paths
    ):
        raise ValueError("representative paths must be a nonempty JSON array of repository-relative files")
    return paths


def changed_paths(base: str, head: str) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACDMRTUXB", f"{base}...{head}"],
        check=True,
        capture_output=True,
        text=True,
    )
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def normalize_labels(values: Iterable[str]) -> set[str]:
    labels: set[str] = set()
    for value in values:
        value = value.strip()
        if not value:
            continue
        if value.startswith("["):
            decoded = json.loads(value)
            for item in decoded:
                labels.add(item["name"] if isinstance(item, dict) else str(item))
        else:
            labels.update(part.strip() for part in value.split(",") if part.strip())
    return labels


def make_shards(owner_names: list[str]) -> list[Dict[str, str]]:
    if not owner_names:
        return []
    return [
        {"id": str(index + 1), "owners": owner_name}
        for index, owner_name in enumerate(owner_names)
    ]


def classify(
    paths: Iterable[str],
    labels: Iterable[str],
    owners: Dict[str, Dict[str, Any]],
) -> Dict[str, Any]:
    paths = sorted({path.replace("\\", "/") for path in paths if path})
    labels = set(labels)
    matched_names: set[str] = set()
    unmatched_paths: list[str] = []
    docs_only = bool(paths)
    cross_cutting = False
    browser = False

    for path in paths:
        matches = matching_owners(path, owners)
        if not matches:
            unmatched_paths.append(path)
            docs_only = False
            cross_cutting = True
            continue
        matched_names.update(owner["name"] for owner in matches)
        if not all(owner.get("docs", False) for owner in matches):
            docs_only = False
        cross_cutting = cross_cutting or any(owner["cross_cutting"] for owner in matches)
        browser = browser or any(owner.get("browser", False) for owner in matches)

    if docs_only:
        runnable: set[str] = set()
    else:
        runnable = {
            name
            for name in matched_names
            if owners[name]["command"] and not owners[name].get("docs", False)
        }
        if cross_cutting:
            runnable.update(
                name for name, owner in owners.items()
                if owner["standard"] and owner["command"]
            )
            # The standard-library command already includes the same complete
            # managed-functions suite used by the machine implementation owner.
            if "mech-stdlib" in runnable:
                runnable.discard("machine-functions")

    runnable_names = sorted(runnable)
    code_changed = bool(paths) and not docs_only
    full = "ci:full" in labels or (
        code_changed and (
            bool(unmatched_paths)
            or any(owners[name].get("full", False) for name in matched_names)
        )
    )
    # Invoking an owner's dependent tests does not make its production code
    # changed. Contract families follow direct responsibility; inherited browser
    # capabilities still require the complete product oracles.
    effective_names = matched_names | runnable
    families = set(CONTRACT_FAMILIES) if full else {
        family for name in matched_names for family in owners[name].get("families", [])
    }
    unknown_families = families - set(CONTRACT_FAMILIES)
    if unknown_families:
        raise ValueError(f"unknown contract families: {sorted(unknown_families)}")
    return {
        "paths": paths,
        "matched_owners": sorted(matched_names),
        "unmatched_paths": unmatched_paths,
        "changed_owners": runnable_names,
        "owner_shards": make_shards(runnable_names),
        "docs_only": docs_only,
        "static_contracts_required": code_changed,
        "standard_canaries_required": code_changed,
        "windows_canary_required": code_changed and (
            full or "cli" in families
        ),
        "browser_canary_required": code_changed and (browser or cross_cutting or full),
        "cross_cutting_standard_suite_required": code_changed and cross_cutting,
        # Complete contracts follow maintained responsibility, never branch identity.
        "full_validation_required": full,
        "contract_families": sorted(families),
        "dependent_contracts_required": bool(families),
        "native_validation_required": "native" in families,
        "browser_applications_required": code_changed and (
            full
            or any(owners[name].get("browser_applications", False) for name in effective_names)
        ),
    }


def output_value(value: Any) -> str:
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (list, dict)):
        return json.dumps(value, separators=(",", ":"), sort_keys=True)
    return str(value)


def write_github_output(path: Path, result: Dict[str, Any]) -> None:
    exported = (
        "changed_owners",
        "owner_shards",
        "docs_only",
        "static_contracts_required",
        "standard_canaries_required",
        "windows_canary_required",
        "browser_canary_required",
        "cross_cutting_standard_suite_required",
        "full_validation_required",
        "browser_applications_required",
        "contract_families",
        "dependent_contracts_required",
        "native_validation_required",
    )
    with path.open("a", encoding="utf-8") as output:
        for key in exported:
            output.write(f"{key}={output_value(result[key])}\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--paths", nargs="*")
    parser.add_argument("--representative-paths")
    parser.add_argument("--expected-revision")
    parser.add_argument("--labels", action="append", default=[])
    parser.add_argument("--owners", type=Path, default=DEFAULT_OWNER_CONFIG)
    parser.add_argument("--github-output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.representative_paths is not None:
        if os.environ.get("GITHUB_EVENT_NAME") != "workflow_dispatch":
            raise SystemExit("representative paths are allowed only in an explicit manual measurement")
        actual = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        if actual != os.environ.get("GITHUB_SHA"):
            raise SystemExit("measurement checkout does not match the workflow revision")
        paths = representative_paths(args.representative_paths, args.expected_revision or "", actual)
    elif args.paths is None:
        if not args.base:
            raise SystemExit("--base is required unless --paths is supplied")
        paths = changed_paths(args.base, args.head)
    else:
        paths = args.paths
    result = classify(paths, normalize_labels(args.labels), load_owners(args.owners))
    if args.representative_paths is not None:
        result["selection_kind"] = "simulated paths; real execution on the exact workflow revision"
        result["validation_revision"] = actual
    github_output = args.github_output or (
        Path(os.environ["GITHUB_OUTPUT"]) if "GITHUB_OUTPUT" in os.environ else None
    )
    if github_output:
        write_github_output(github_output, result)
    json.dump(result, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
