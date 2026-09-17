"""Explicit S8 review roles; branch prefixes and labels cannot opt a PR out."""
import json
import os
import re
import subprocess
from pathlib import Path

REGISTRY = Path(__file__).resolve().parents[1] / ".github/ci/s8-review-slices.json"


def trusted_registry_from_revision(revision, run=None):
    if not re.fullmatch(r"[0-9a-f]{40}", revision or ""):
        return None
    run = subprocess.run if run is None else run
    try:
        result = run(
            ["git", "show", f"{revision}:.github/ci/s8-review-slices.json"],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return None
    return json.loads(result.stdout)


def review_role(number="", branch="", repository="", registry=None, *, base=""):
    registry = registry if registry is not None else json.loads(REGISTRY.read_text())
    if any(not key.isdecimal() or int(key) <= 0 for key in registry["slices"]):
        raise ValueError("registered review slice keys must be positive PR numbers")
    if repository != registry["repository"]:
        return "ordinary", None
    landing = registry["landing"]
    if str(number) == str(landing["number"]) and branch == landing["branch"]:
        return "landing", None
    entry = registry["slices"].get(str(number))
    if entry is not None and branch == entry["branch"] and base == entry["base"]:
        if not entry["checks"] or any(not command for command in entry["checks"]):
            raise ValueError("registered review slices require executable checks")
        return "review", entry
    return "ordinary", None


def environment_role():
    registry = trusted_registry_from_revision(os.environ.get("PR_BASE_SHA", ""))
    if registry is None:
        return "ordinary", None
    return review_role(os.environ.get("PR_NUMBER", ""),
                       os.environ.get("PR_HEAD_REF", ""),
                       os.environ.get("PR_HEAD_REPOSITORY", ""),
                       registry,
                       base=os.environ.get("PR_BASE_REF", ""))
