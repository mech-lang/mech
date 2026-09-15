"""Explicit S8 review roles; branch prefixes and labels cannot opt a PR out."""
import json
import os
from pathlib import Path

REGISTRY = Path(__file__).resolve().parents[1] / ".github/ci/s8-review-slices.json"


def review_role(number="", branch="", repository="", registry=None):
    registry = registry if registry is not None else json.loads(REGISTRY.read_text())
    if repository != registry["repository"]:
        return "ordinary", None
    landing = registry["landing"]
    if str(number) == str(landing["number"]) and branch == landing["branch"]:
        return "landing", None
    entry = registry["slices"].get(str(number))
    if entry is not None and branch == entry["branch"]:
        if not entry["checks"] or any(not command for command in entry["checks"]):
            raise ValueError("registered review slices require executable checks")
        return "review", entry
    return "ordinary", None


def environment_role():
    return review_role(os.environ.get("PR_NUMBER", ""),
                       os.environ.get("PR_HEAD_REF", ""),
                       os.environ.get("PR_HEAD_REPOSITORY", ""))
