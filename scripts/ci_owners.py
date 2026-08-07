#!/usr/bin/env python3
"""Shared parsing and path matching for owner-scoped CI."""

from __future__ import annotations

import fnmatch
import json
from pathlib import Path
from typing import Any, Dict


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OWNER_CONFIG = REPOSITORY_ROOT / ".github" / "ci" / "owners.toml"


def load_owners(path: Path = DEFAULT_OWNER_CONFIG) -> Dict[str, Dict[str, Any]]:
    """Read the deliberately small, one-value-per-line owner configuration.

    Values in owners.toml use JSON-compatible TOML syntax. Keeping the parser
    here avoids adding a Python package just to run the impact job on Python
    versions that predate tomllib.
    """

    owners: Dict[str, Dict[str, Any]] = {}
    current: Dict[str, Any] | None = None

    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#") or line == "version = 1":
            continue
        if line.startswith("[owners.") and line.endswith("]"):
            name = line[len("[owners.") : -1]
            if not name or name in owners:
                raise ValueError(f"{path}:{line_number}: invalid or duplicate owner {name!r}")
            current = {"name": name}
            owners[name] = current
            continue
        if current is None or "=" not in line:
            raise ValueError(f"{path}:{line_number}: expected an owner field")
        key, encoded_value = (part.strip() for part in line.split("=", 1))
        try:
            current[key] = json.loads(encoded_value)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_number}: {error.msg}") from error

    required_fields = {"paths", "command", "standard", "cross_cutting"}
    for name, owner in owners.items():
        missing = required_fields - owner.keys()
        if missing:
            raise ValueError(f"{path}: owner {name!r} is missing {sorted(missing)}")
        if not owner["paths"]:
            raise ValueError(f"{path}: owner {name!r} has no paths")
    return owners


def path_matches(pattern: str, changed_path: str) -> bool:
    normalized = changed_path.replace("\\", "/")
    if normalized.startswith("./"):
        normalized = normalized[2:]
    return fnmatch.fnmatchcase(normalized, pattern)


def matching_owners(
    changed_path: str, owners: Dict[str, Dict[str, Any]]
) -> list[Dict[str, Any]]:
    return [
        owner
        for owner in owners.values()
        if any(path_matches(pattern, changed_path) for pattern in owner["paths"])
    ]
