"""Read pinned qualification evidence and the frozen case contract."""

from dataclasses import dataclass, field
import gzip
from hashlib import file_digest, sha256
import json
import math
from pathlib import Path
import re

CONTROL = "mech-control"
CANDIDATE = "mech-experiment"
PEERS = ("roc", "libdeflate")
ACTORS = (CONTROL, CANDIDATE, *PEERS)
MECH = (CONTROL, CANDIDATE)
POLICY_PATH = Path(__file__).with_name("qualification-policy-v1.json")
POLICIES = {
    1: POLICY_PATH,
    2: POLICY_PATH.with_name("qualification-policy-v2.json"),
    3: POLICY_PATH.with_name("qualification-policy-v3.json"),
}


class InvalidEvidence(ValueError):
    """Evidence conflicts with the contract or another pinned record."""


def require(condition, message):
    if not condition:
        raise InvalidEvidence(message)


def distinct_object(pairs):
    obj = {}
    for key, value in pairs:
        require(key not in obj, f"duplicate JSON key {key!r}")
        obj[key] = value
    return obj


def read_json(path):
    try:
        opener = gzip.open if path.suffix == ".gz" else open
        with opener(path, "rt", encoding="utf-8") as stream:
            return json.load(stream, object_pairs_hook=distinct_object)
    except (EOFError, OSError, UnicodeError, json.JSONDecodeError) as error:
        raise InvalidEvidence(f"cannot read {path}: {error}") from error


def load_policy(version):
    require(type(version) is int and version in POLICIES, "unsupported qualification policy version")
    policy = read_json(POLICIES[version])
    require(policy["version"] == version, "qualification policy file has the wrong version")
    return policy


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def is_digest(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def is_number(value, *, positive=False):
    return type(value) in (int, float) and math.isfinite(value) and (value > 0 if positive else value >= 0)


def is_count(value, *, positive=False):
    return type(value) is int and (value > 0 if positive else value >= 0)


def close(recorded, actual):
    return type(recorded) in (int, float) and math.isclose(recorded, actual, abs_tol=1e-14, rel_tol=1e-12)


@dataclass
class Findings:
    rejected: list = field(default_factory=list)
    unresolved: list = field(default_factory=list)

    @property
    def decision(self):
        if self.rejected:
            return "Reject"
        if self.unresolved:
            return "Unresolved"
        return "Retain"

    def check(self, label, action, *args):
        try:
            return action(*args)
        except (ValueError, KeyError, IndexError, TypeError, OverflowError, AttributeError, OSError) as error:
            self.rejected.append(f"{label}: {error}")
            return None


class Evidence:
    def __init__(self, root, findings):
        self.root = Path(root).resolve()
        self.findings = findings
        self._digests = {}

    def path(self, reference, label):
        if reference is None:
            self.findings.unresolved.append(f"{label}: missing reference")
            return None
        require(isinstance(reference, dict) and is_digest(reference.get("sha256")),
                f"{label}: invalid SHA-256 reference")
        filename = reference.get("path")
        if filename is None:
            self.findings.unresolved.append(f"{label}: digest recorded, file unavailable")
            return None
        require(isinstance(filename, str) and bool(filename) and not Path(filename).is_absolute(),
                f"{label}: path must be relative to the evidence root")
        path = (self.root / filename).resolve()
        require(path.is_relative_to(self.root), f"{label}: path escapes the evidence root")
        if not path.is_file():
            self.findings.unresolved.append(f"{label}: evidence file missing: {filename}")
            return None
        if path not in self._digests:
            self._digests[path] = digest(path)
        require(self._digests[path] == reference["sha256"], f"{label}: SHA-256 mismatch: {filename}")
        return path

    def json(self, reference, label):
        path = self.path(reference, label)
        return read_json(path) if path is not None else None


def reference_sha(reference):
    if isinstance(reference, dict) and is_digest(reference.get("sha256")):
        return reference["sha256"]
    return None


def fixture_contract(row):
    return (
        row["input_sha256"],
        Path(row["prefix"]).name,
        row["expected_sha256"],
        row["input_bytes"],
        row["prefix_bytes"],
        row["raw_bytes"],
        row["capacity"],
    )


def load_fixtures(evidence, policy):
    path = evidence.path(
        {"path": "fixtures.json", "sha256": policy["fixture_inventory_sha256"]},
        "frozen fixture inventory",
    )
    if path is None:
        return {}
    fixtures = read_json(path)
    require(isinstance(fixtures, list) and len(fixtures) == policy["named_cases"],
            "invalid frozen named fixture count")
    by_name = {}
    for row in fixtures:
        name = row["name"]
        require(name not in by_name, f"duplicate frozen fixture {name}")
        input_path = evidence.path({"path": row["input"], "sha256": row["input_sha256"]}, f"{name} input")
        prefix_path = evidence.path({"path": row["prefix"], "sha256": Path(row["prefix"]).name}, f"{name} prefix")
        output_path = evidence.path({"path": row["expected"], "sha256": row["expected_sha256"]},
                                    f"{name} expected output")
        if input_path is not None:
            require(input_path.stat().st_size == row["input_bytes"], f"{name}: input size differs")
        if output_path is not None and prefix_path is not None:
            output = output_path.read_bytes()
            prefix = prefix_path.read_bytes()
            require(len(prefix) == row["prefix_bytes"] and output.startswith(prefix)
                    and len(output) - len(prefix) == row["raw_bytes"],
                    f"{name}: output bytes do not match the frozen frame shape")
            row["expected_raw_sha256"] = sha256(output[len(prefix):]).hexdigest()
        by_name[name] = row
    require(len({fixture_contract(row) for row in fixtures}) == policy["distinct_contracts"],
            "invalid count of distinct input, prefix, and output contracts")
    return by_name
