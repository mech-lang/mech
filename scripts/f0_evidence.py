#!/usr/bin/env python3
"""Shared fail-closed helpers for final v0.4 qualification evidence."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
QUALIFICATION_ROOT = ROOT / "tests/architecture/qualification"
EVIDENCE_MANIFEST = QUALIFICATION_ROOT / "f0-qualification-manifest.json"
PRODUCT_TREE_MANIFEST = QUALIFICATION_ROOT / "f0-product-tree.json"
TOOLCHAIN_MANIFEST = QUALIFICATION_ROOT / "f0-measurement-toolchain.json"
PROTOCOL_VERSION = "mech-v0.4-f0-qualification-v1"
SHA256_LENGTH = 64
BUILD_ENVIRONMENT_VARIABLES = tuple(
    """AR ARFLAGS RANLIB RANLIBFLAGS CC CXX CFLAGS CXXFLAGS CPPFLAGS LDFLAGS
    MACOSX_DEPLOYMENT_TARGET SDKROOT DEVELOPER_DIR RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
    RUSTDOCFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_RUSTC
    CARGO_BUILD_RUSTC_WRAPPER CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_TARGET
    CARGO_INCREMENTAL CARGO_HOME CRATE_CC_NO_DEFAULTS HOST_AR HOST_ARFLAGS HOST_CC
    HOST_CXX HOST_RANLIB HOST_RANLIBFLAGS TARGET_AR TARGET_ARFLAGS TARGET_CC TARGET_CXX
    TARGET_RANLIB TARGET_RANLIBFLAGS PYTHONHOME PYTHONINSPECT PYTHONNOUSERSITE PYTHONPATH
    PYTHONSTARTUP PYTHONUSERBASE""".split()
)
TARGET_NATIVE_VARIABLE = re.compile(
    r"^(?:(?:CC|CXX|AR|ARFLAGS|RANLIB|RANLIBFLAGS|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS)_.+|"
    r".+_(?:CC|CXX|AR|ARFLAGS|RANLIB|RANLIBFLAGS|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS))$"
)


class EvidenceError(ValueError):
    """A qualification artifact is absent, unauthenticated, or inconsistent."""


def canonical_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def directory_evidence(path: Path, logical_path: str) -> dict[str, Any]:
    files = []
    for entry in sorted(path.rglob("*")):
        if entry.is_symlink():
            raise EvidenceError(f"evidence tree contains symbolic link {entry}")
        if entry.is_file():
            files.append(
                {
                    "path": entry.relative_to(path).as_posix(),
                    "sha256": sha256_file(entry),
                }
            )
    if not files:
        raise EvidenceError(f"evidence tree {path} is empty")
    return {
        "path": logical_path,
        "tree_sha256": sha256_bytes(canonical_json_bytes(files)),
        "files": files,
    }


def copy_criterion_evidence(
    criterion_root: Path, destination: Path, logical_path: str
) -> dict[str, Any]:
    if destination.exists():
        raise EvidenceError(f"Criterion evidence destination already exists: {destination}")
    destination.mkdir(parents=True)
    selected = [
        child
        for child in sorted(criterion_root.iterdir())
        if child.name == "gate_b" or child.name.startswith("gate_b_")
    ] if criterion_root.exists() else []
    if not selected:
        raise EvidenceError(f"no Gate B Criterion samples found under {criterion_root}")
    for child in selected:
        if child.is_symlink() or not child.is_dir():
            raise EvidenceError(f"invalid Gate B Criterion evidence root {child}")
        shutil.copytree(child, destination / child.name)
    return directory_evidence(destination, logical_path)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read {path}: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError(f"{path} must contain a JSON object")
    return value


def command(*arguments: str, cwd: Path = ROOT) -> str:
    try:
        return subprocess.run(
            list(arguments),
            cwd=cwd,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        output = getattr(error, "stdout", "") or str(error)
        raise EvidenceError(f"command {' '.join(arguments)} failed: {output.strip()}") from error


def git_identity(root: Path = ROOT) -> dict[str, Any]:
    status = command("git", "status", "--porcelain=v1", "--untracked-files=all", cwd=root)
    try:
        branch = command("git", "symbolic-ref", "--short", "HEAD", cwd=root)
    except EvidenceError:
        branch = None
    return {
        "commit": command("git", "rev-parse", "HEAD", cwd=root),
        "tree": command("git", "rev-parse", "HEAD^{tree}", cwd=root),
        "branch": branch,
        "clean": not bool(status),
        "status": status.splitlines(),
    }


def validate_sha256(value: Any, label: str, errors: list[str]) -> None:
    if (
        not isinstance(value, str)
        or len(value) != SHA256_LENGTH
        or any(character not in "0123456789abcdef" for character in value)
    ):
        errors.append(f"{label} must be a lowercase SHA-256")


def validate_git_oid(value: Any, label: str, errors: list[str]) -> None:
    if (
        not isinstance(value, str)
        or len(value) != 40
        or any(character not in "0123456789abcdef" for character in value)
    ):
        errors.append(f"{label} must be a full lowercase Git object ID")


def load_qualification_context(path: Path) -> dict[str, Any]:
    context = load_json(path)
    errors: list[str] = []
    required = {
        "b2_evidence_path",
        "protocol_version",
        "runtime_subject_commit",
        "runtime_subject_tree",
        "qualification_protocol_commit",
        "evidence_generation_commit",
        "qualification_environment_id",
        "d2_evidence_path",
        "d3_evidence_path",
        "raw_evidence_prefix",
        "chain_id",
        "session_id",
        "workflow_run_id",
        "workflow_run_attempt",
    }
    missing = sorted(required - context.keys())
    if missing:
        errors.append(f"qualification context is missing {', '.join(missing)}")
    if context.get("protocol_version") != PROTOCOL_VERSION:
        errors.append("qualification context protocol version changed")
    for field in (
        "runtime_subject_commit",
        "runtime_subject_tree",
        "qualification_protocol_commit",
        "evidence_generation_commit",
    ):
        validate_git_oid(context.get(field), f"qualification context {field}", errors)
    validate_sha256(
        context.get("qualification_environment_id"),
        "qualification context qualification_environment_id",
        errors,
    )
    validate_sha256(context.get("session_id"), "qualification context session_id", errors)
    for field in ("workflow_run_id", "workflow_run_attempt"):
        value = context.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            errors.append(f"qualification context {field} is invalid")
    chain_id = context.get("chain_id")
    if chain_id not in {"chain-1", "chain-2", "chain-3"}:
        errors.append(f"unregistered qualification chain {chain_id!r}")
    for field in (
        "b2_evidence_path",
        "d2_evidence_path",
        "d3_evidence_path",
        "raw_evidence_prefix",
    ):
        value = context.get(field)
        if (
            not isinstance(value, str)
            or not value
            or Path(value).is_absolute()
            or ".." in Path(value).parts
        ):
            errors.append(f"qualification context {field} is not repository-relative")
    expected_prefix = f"benchmarks/runtime/f0-evidence/{chain_id}"
    if context.get("raw_evidence_prefix") != expected_prefix:
        errors.append("qualification context raw evidence prefix changed")
    expected_reports = {
        "b2_evidence_path": (
            "benchmarks/runtime/gate-b/b2-resident-turn.json"
            if chain_id == "chain-1"
            else f"{expected_prefix}/b2-resident-turn.json"
        ),
        "d2_evidence_path": (
            "benchmarks/runtime/gate-d/d2-resident-nbody.json"
            if chain_id == "chain-1"
            else f"{expected_prefix}/d2-resident-nbody.json"
        ),
        "d3_evidence_path": (
            "benchmarks/runtime/gate-d/d3-resident-external.json"
            if chain_id == "chain-1"
            else f"{expected_prefix}/d3-resident-external.json"
        ),
    }
    for field, expected in expected_reports.items():
        if context.get(field) != expected:
            errors.append(f"qualification context {field} changed")
    if errors:
        raise EvidenceError("; ".join(errors))
    return context


def attach_provenance(report: dict[str, Any], context: dict[str, Any]) -> None:
    report["provenance"] = {
        key: context[key]
        for key in (
            "protocol_version",
            "runtime_subject_commit",
            "runtime_subject_tree",
            "qualification_protocol_commit",
            "evidence_generation_commit",
            "qualification_environment_id",
            "chain_id",
            "session_id",
            "workflow_run_id",
            "workflow_run_attempt",
        )
    }
    report["canonical"] = True


def same_provenance(
    left: dict[str, Any], right: dict[str, Any], fields: tuple[str, ...]
) -> list[str]:
    errors = []
    left_provenance = left.get("provenance", {})
    right_provenance = right.get("provenance", {})
    for field in fields:
        if left_provenance.get(field) != right_provenance.get(field):
            errors.append(
                f"provenance mismatch for {field}: "
                f"{left_provenance.get(field)!r} != {right_provenance.get(field)!r}"
            )
    return errors


def physical_machine() -> dict[str, Any]:
    result: dict[str, Any] = {
        "os": platform.platform(),
        "architecture": platform.machine(),
        "processor": platform.processor(),
    }
    if platform.system() == "Darwin":
        hardware = command("system_profiler", "SPHardwareDataType", "-detailLevel", "mini")
        fields: dict[str, str] = {}
        for line in hardware.splitlines():
            key, separator, value = line.strip().partition(":")
            if separator:
                fields[key] = value.strip()
        result.update(
            {
                "model_name": fields.get("Model Name"),
                "model_identifier": fields.get("Model Identifier"),
                "chip": fields.get("Chip"),
                "memory": fields.get("Memory"),
            }
        )
    return result


def environment_identity(fingerprint: dict[str, Any]) -> str:
    return sha256_bytes(canonical_json_bytes(fingerprint))


def controlled_thread_environment(source: dict[str, str] | None = None) -> dict[str, str]:
    source = os.environ if source is None else source
    names = (
        "OMP_NUM_THREADS",
        "OPENBLAS_NUM_THREADS",
        "BLIS_NUM_THREADS",
        "MKL_NUM_THREADS",
        "VECLIB_MAXIMUM_THREADS",
        "NUMEXPR_NUM_THREADS",
        "RAYON_NUM_THREADS",
    )
    return {name: source.get(name, "") for name in names}


def controlled_build_environment(
    source: dict[str, str] | None = None,
) -> dict[str, str]:
    """Return every frozen compiler/build input, including required empty values."""
    source = os.environ if source is None else source
    return {name: source.get(name, "") for name in BUILD_ENVIRONMENT_VARIABLES}


def uncontrolled_build_environment(
    source: dict[str, str] | None = None,
    expected: dict[str, str] | None = None,
) -> dict[str, str]:
    """Find build-affecting variables that canonical F0 refuses to inherit."""
    source = os.environ if source is None else source
    expected = {} if expected is None else expected
    result = {
        name: source[name]
        for name in BUILD_ENVIRONMENT_VARIABLES
        if source.get(name) and source.get(name) != expected.get(name, "")
    }
    for name, value in source.items():
        if not value or name in result or name in {"RUSTUP_HOME", "RUSTUP_TOOLCHAIN"}:
            continue
        if name.startswith("CARGO_PROFILE_") or (
            name.startswith("CARGO_TARGET_")
            and name.endswith(("_RUSTFLAGS", "_LINKER", "_RUNNER"))
        ) or TARGET_NATIVE_VARIABLE.fullmatch(name):
            result[name] = value
    return dict(sorted(result.items()))


def power_settings(output: str, source: str) -> dict[str, str]:
    active = False
    result = {}
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.endswith(":"):
            active = stripped.removesuffix(":") == source
            continue
        if active:
            key, separator, value = stripped.rpartition(" ")
            if separator and key:
                result[key.strip()] = value.strip()
    return result


def measurement_conditions_error(record: dict, policy: dict) -> str | None:
    """Validate a retained pre/post-chain power and thermal snapshot."""
    if not isinstance(record, dict) or set(record) != {
        "battery",
        "power_configuration",
        "thermal",
    }:
        return "canonical F0 measurement condition snapshot is incomplete"
    battery = record.get("battery", {})
    required_source = policy.get("power_source")
    if battery.get("returncode") or required_source not in battery.get("output", ""):
        return f"canonical F0 measurement requires {required_source}"
    configuration = record.get("power_configuration", {})
    if configuration.get("returncode"):
        return "canonical F0 measurement cannot read the power configuration"
    active_settings = power_settings(configuration.get("output", ""), required_source)
    for name, expected in policy.get("ac_power_settings", {}).items():
        if active_settings.get(name) != expected:
            return (
                f"unapproved {required_source} setting {name}="
                f"{active_settings.get(name)!r}; expected {expected!r}"
            )
    thermal = record.get("thermal", {})
    if thermal.get("returncode"):
        return "canonical F0 measurement cannot read thermal conditions"
    if thermal.get("source") != policy.get("thermal_source"):
        return "canonical F0 thermal snapshot source changed"
    output = thermal.get("output", "").strip()
    if not output:
        return "canonical F0 thermal snapshot is empty"
    if output != str(policy.get("nominal_thermal_state")):
        return f"canonical F0 thermal state is not nominal: {output!r}"
    return None
