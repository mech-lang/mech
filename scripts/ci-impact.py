#!/usr/bin/env python3
"""Classify a change into the smallest safe pull-request validation set."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Iterable

from ci_owners import DEFAULT_OWNER_CONFIG, load_owners, matching_owners


MAX_OWNER_SHARDS = 10


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


def make_shards(owner_names: list[str], limit: int = MAX_OWNER_SHARDS) -> list[Dict[str, str]]:
    if not owner_names:
        return []
    shard_count = min(limit, len(owner_names))
    buckets: list[list[str]] = [[] for _ in range(shard_count)]
    for index, owner_name in enumerate(owner_names):
        buckets[index % shard_count].append(owner_name)
    return [
        {"id": str(index + 1), "owners": ",".join(bucket)}
        for index, bucket in enumerate(buckets)
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
        if not any(owner.get("docs", False) for owner in matches):
            docs_only = False
        cross_cutting = cross_cutting or any(owner["cross_cutting"] for owner in matches)
        browser = browser or any(owner.get("browser", False) for owner in matches)

    if cross_cutting:
        runnable = {
            name
            for name, owner in owners.items()
            if owner["standard"] and owner["command"]
        }
    else:
        runnable = {
            name
            for name in matched_names
            if owners[name]["command"] and not owners[name].get("docs", False)
        }

    runnable_names = sorted(runnable)
    code_changed = bool(paths) and not docs_only
    return {
        "paths": paths,
        "matched_owners": sorted(matched_names),
        "unmatched_paths": unmatched_paths,
        "changed_owners": runnable_names,
        "owner_shards": make_shards(runnable_names),
        "docs_only": docs_only,
        "static_contracts_required": code_changed,
        "standard_canaries_required": code_changed,
        "windows_canary_required": code_changed,
        "browser_canary_required": code_changed and browser,
        "cross_cutting_standard_suite_required": code_changed and cross_cutting,
        "full_validation_required": "ci:full" in labels,
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
    )
    with path.open("a", encoding="utf-8") as output:
        for key in exported:
            output.write(f"{key}={output_value(result[key])}\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--paths", nargs="*")
    parser.add_argument("--labels", action="append", default=[])
    parser.add_argument("--owners", type=Path, default=DEFAULT_OWNER_CONFIG)
    parser.add_argument("--github-output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.paths is None:
        if not args.base:
            raise SystemExit("--base is required unless --paths is supplied")
        paths = changed_paths(args.base, args.head)
    else:
        paths = args.paths
    result = classify(paths, normalize_labels(args.labels), load_owners(args.owners))
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
