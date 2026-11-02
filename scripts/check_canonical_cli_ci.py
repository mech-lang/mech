#!/usr/bin/env python3
"""Fail-closed evidence checks for the required public canonical native CLI job."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

UNIT_PREFIX = "cli::commands::build::cranelift::tests::"
UNIT_NAMES = {
    UNIT_PREFIX + name for name in (
        "canonical_options_reject_unsupported_modes_before_reading_inputs",
        "retained_source_bundle_survives_deletion_and_bytecode_is_explicitly_unpaired",
        "discovered_runtime_configuration_is_not_native_launch_authority",
        "exact_linked_name_grammar_rejects_before_compilation_without_sanitizing",
        "ordinary_turn_inputs_are_rejected_without_another_execution_route",
    )
}
PUBLIC_NAMES = {
    "public_source_and_bytecode_builds_run_unchanged_knucleotide_without_fallback",
    "public_native_build_retains_unused_observations_without_printing_root_values",
    "public_closed_native_program_uses_same_shell_with_zero_grants_and_zero_ports",
}
CASE_CHILDREN = {
    "source-bytecode-": {
        "bytecode": True, "build-0": True, "run-0": True, "denied-0": False,
        "build-1": True, "run-1": True, "denied-1": False,
    },
    "unused-observation-": {
        "unused-build": True, "unused-denied": False, "unused-accepted": True,
    },
    "zero-resources-": {"closed-build": True, "closed-run": True},
}
STAGES = (
    "canonical-cli-lib-compile", "canonical-cli-public-compile",
    "canonical-cli-unit-selection", "canonical-cli-unit-contracts",
    "canonical-cli-public-selection", "canonical-cli-public-qualification",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def selection(log, expected):
    names = [
        line.removesuffix(": test")
        for line in log.read_text(encoding="utf-8").splitlines()
        if line.endswith(": test")
    ]
    require(len(names) == len(expected) and set(names) == expected,
            f"wrong or duplicate selection in {log}: {names}")


def executable_paths(log_dir):
    paths = {
        role: Path((log_dir / receipt).read_text(encoding="utf-8").strip()).resolve(strict=True)
        for role, receipt in (
            ("unit-harness", "canonical-cli-lib-executable"),
            ("public-harness", "canonical-cli-public-executable"),
        )
    }
    bins = set()
    for line in (log_dir / "canonical-cli-public-compile.log").read_text(
            encoding="utf-8").splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = message.get("target", {})
        if (message.get("reason") == "compiler-artifact"
                and target.get("name") == "mech" and target.get("kind") == ["bin"]
                and not message.get("profile", {}).get("test")
                and message.get("executable")):
            bins.add(Path(message["executable"]).resolve(strict=True))
    require(len(bins) == 1, f"expected one actual public mech binary: {bins}")
    paths["public-cli"] = bins.pop()
    require(len(set(paths.values())) == 3, "harnesses and public CLI must be distinct")
    require(all(path.is_file() for path in paths.values()), "missing executable")
    return paths


def pin_executables(log_dir):
    destination = log_dir / "canonical-cli-executable-pins.json"
    require(not destination.exists(), "refusing to overwrite executable pins")
    entries = [
        {"role": role, "path": str(path), "sha256": digest(path)}
        for role, path in executable_paths(log_dir).items()
    ]
    destination.write_text(json.dumps({
        "schema": "mech.public-native-cli-executable-pins.v1", "entries": entries,
    }, indent=2) + "\n", encoding="utf-8")


def verify_pins(log_dir):
    manifest = read_json(log_dir / "canonical-cli-executable-pins.json")
    require(manifest.get("schema") == "mech.public-native-cli-executable-pins.v1",
            "wrong executable pin schema")
    paths = executable_paths(log_dir)
    entries = manifest["entries"]
    require(len(entries) == 3 and {entry["role"] for entry in entries} == set(paths),
            "missing or duplicate executable pin")
    for entry in entries:
        path = Path(entry["path"])
        require(path == paths[entry["role"]] and digest(path) == entry["sha256"],
                f"executable drift: {entry['role']}")


def completed_summary(log_dir, stage):
    summary = read_json(log_dir / f"{stage}.summary.json")
    require(summary.get("stage") == stage and summary.get("reason") == "exit"
            and summary.get("command_exit_status") == 0
            and summary.get("owned_group_cleanup") == "complete",
            f"not a completed successful stage: {stage}")


def passed_count(log, expected):
    rows = re.findall(
        r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",
        log.read_text(encoding="utf-8"), re.MULTILINE,
    )
    require(rows == [(str(expected), "0", "0")], f"wrong execution counts: {log}: {rows}")


def case_evidence(root):
    require(root.is_absolute() and root.is_dir(), "missing absolute evidence root")
    directories = list(root.iterdir())
    require(len(directories) == 3 and all(path.is_dir() for path in directories),
            "expected exactly three retained public case directories")
    for prefix, expected in CASE_CHILDREN.items():
        selected = [path for path in directories if path.name.startswith(prefix)]
        require(len(selected) == 1, f"missing or duplicate case prefix: {prefix}")
        directory = selected[0]
        result = read_json(directory / "qualification-result.json")
        require(result == {
            "schema": "mech.public-native-cli-qualification.v1", "passed": True,
        }, f"case did not pass: {directory}")
        receipts = list(directory.glob("*.result.json"))
        require(len(receipts) == len(expected)
                and {path.name.removesuffix(".result.json") for path in receipts} == set(expected),
                f"wrong child receipt set: {directory}")
        for label, success in expected.items():
            receipt = read_json(directory / f"{label}.result.json")
            require(receipt.get("state") == "completed"
                    and receipt.get("qualification_only") is True
                    and receipt.get("success") is success
                    and ((receipt.get("exit_code") == 0) if success
                         else receipt.get("exit_code") not in (None, 0)),
                    f"wrong or incomplete child outcome: {label}")
            for stream in ("stdout", "stderr"):
                require(digest(directory / f"{label}.{stream}") == receipt[f"{stream}_sha256"],
                        f"retained child output drift: {label}.{stream}")
    unused = next(path for path in directories if path.name.startswith("unused-observation-"))
    require((unused / "denied-launch.json").is_file()
            and (unused / "accepted-launch.json").is_file(),
            "both exact unused-observation launch policies must be retained")


def complete(log_dir, evidence):
    for stage in STAGES:
        completed_summary(log_dir, stage)
    selection(log_dir / "canonical-cli-unit-selection.log", UNIT_NAMES)
    selection(log_dir / "canonical-cli-public-selection.log", PUBLIC_NAMES)
    passed_count(log_dir / "canonical-cli-unit-contracts.log", 5)
    passed_count(log_dir / "canonical-cli-public-qualification.log", 3)
    verify_pins(log_dir)
    case_evidence(evidence)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("unit-selection", "public-selection", "pins", "complete"))
    parser.add_argument("--log-dir", type=Path, default=Path("target/native-plan-logs"))
    parser.add_argument("--evidence-dir", type=Path)
    args = parser.parse_args()
    if args.mode == "unit-selection":
        selection(args.log_dir / "canonical-cli-unit-selection.log", UNIT_NAMES)
    elif args.mode == "public-selection":
        selection(args.log_dir / "canonical-cli-public-selection.log", PUBLIC_NAMES)
    elif args.mode == "pins":
        pin_executables(args.log_dir)
    else:
        require(args.evidence_dir is not None, "complete requires --evidence-dir")
        complete(args.log_dir, args.evidence_dir)


if __name__ == "__main__":
    main()
