#!/usr/bin/env python3
"""Run the preregistered F0 B2 -> D2 -> D3 qualification session."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from f0_evidence import (
    PRODUCT_TREE_MANIFEST,
    PROTOCOL_VERSION,
    ROOT,
    TOOLCHAIN_MANIFEST,
    EvidenceError,
    canonical_json_bytes,
    git_identity,
    load_json,
    measurement_conditions_error,
    sha256_bytes,
    sha256_file,
    uncontrolled_build_environment,
)
from f0_contract import d2_qualification, d3_qualification, gate_b_qualification


RECORDED_CHAINS = ("chain-1", "chain-2", "chain-3")
THREAD_VARIABLES = (
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "RAYON_NUM_THREADS",
)


def now() -> str:
    return datetime.now(timezone.utc).isoformat()


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json_bytes(value))


def conditions() -> dict:
    result = {}
    for name, command in {
        "battery": ["pmset", "-g", "batt"],
        "power_configuration": ["pmset", "-g", "custom"],
        "thermal": [
            "/usr/bin/osascript",
            "-l",
            "JavaScript",
            "-e",
            'ObjC.import("Foundation"); $.NSProcessInfo.processInfo.thermalState',
        ],
    }.items():
        process = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        result[name] = {
            "returncode": process.returncode,
            "output": process.stdout.strip(),
        }
        if name == "thermal":
            result[name]["source"] = "NSProcessInfo.thermalState"
    return result


def run_logged(
    arguments: list[str], environment: dict[str, str], log: Path
) -> int:
    process = subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    log.write_text(process.stdout, encoding="utf-8")
    return process.returncode


def controlled_session_environment(
    source: dict[str, str], lock: dict
) -> dict[str, str]:
    environment = dict(source)
    environment.update(lock["thread_environment"])
    for name, value in lock["compiler_environment"].items():
        if value:
            environment[name] = value
        else:
            environment.pop(name, None)
    environment["RUSTUP_TOOLCHAIN"] = lock["rust"]["channel"]
    return environment


def run_preconditioning(
    session_root: Path, environment: dict[str, str], rust_channel: str
) -> dict:
    """Populate build caches without producing a fourth evidence chain."""
    commands = (
        (
            "gate-b",
            [
                "cargo",
                f"+{rust_channel}",
                "build",
                "--release",
                "--locked",
                "-p",
                "mech-runtime",
                "--bench",
                "resident_ekf",
                "--features",
                "source_default,runtime_bench_gate_b,runtime_bench_probes",
            ],
        ),
        (
            "gate-d2",
            [
                "cargo",
                f"+{rust_channel}",
                "build",
                "--release",
                "--offline",
                "--manifest-path",
                "tests/fixtures/d2-contract-generator/Cargo.toml",
                "--target-dir",
                "target",
            ],
        ),
        (
            "gate-d3",
            [
                "cargo",
                f"+{rust_channel}",
                "test",
                "--locked",
                "--release",
                "-p",
                "mech-runtime",
                "--no-default-features",
                "--features",
                "source_default,resident-routing-source,runtime_bench_gate_d3",
                "--test",
                "resident_external_gate_d3",
                "--no-run",
            ],
        ),
    )
    record = {"status": "running", "started_at_utc": now(), "commands": []}
    for name, arguments in commands:
        log = session_root / f"precondition-{name}.log"
        returncode = run_logged(arguments, environment, log)
        record["commands"].append(
            {
                "name": name,
                "arguments": arguments,
                "returncode": returncode,
                "log_path": log.name,
                "log_sha256": sha256_file(log),
            }
        )
        if returncode:
            record.update(
                {
                    "status": "Fail",
                    "error": f"untimed {name} preparation failed",
                    "finished_at_utc": now(),
                }
            )
            return record
    record.update({"status": "Pass", "finished_at_utc": now()})
    return record


def wait_for_measurement_conditions(
    policy: dict, *, timeout_seconds: int = 600, poll_seconds: int = 10
) -> dict:
    """Retain bounded pre-chain cooling checks without creating evidence chains."""
    started = time.monotonic()
    record = {
        "status": "running",
        "started_at_utc": now(),
        "timeout_seconds": timeout_seconds,
        "poll_seconds": poll_seconds,
        "attempts": [],
    }
    while True:
        snapshot = conditions()
        error = measurement_conditions_error(snapshot, policy)
        record["attempts"].append(
            {"observed_at_utc": now(), "conditions": snapshot, "error": error}
        )
        if error is None:
            record.update({"status": "Pass", "finished_at_utc": now()})
            return record
        elapsed = time.monotonic() - started
        if elapsed >= timeout_seconds:
            record.update(
                {
                    "status": "Fail",
                    "error": f"measurement conditions did not become nominal: {error}",
                    "finished_at_utc": now(),
                }
            )
            return record
        time.sleep(min(poll_seconds, max(0.0, timeout_seconds - elapsed)))


def report_record(path: Path, phase: str, logical_path: str) -> dict:
    report = load_json(path)
    measurement_decision = (
        report.get("b2_decision", {}).get("decision")
        if phase == "B2"
        else report.get("decision")
    )
    if phase == "B2":
        qualification_decision = gate_b_qualification(report)[0]
    elif phase == "D2":
        qualification_decision = d2_qualification(report)[0]
    elif phase == "D3":
        qualification_decision = d3_qualification(report)[0]
    else:
        qualification_decision = measurement_decision
    return {
        "path": logical_path,
        "sha256": sha256_file(path),
        "phase": phase,
        "decision": qualification_decision,
        "measurement_decision": measurement_decision,
    }


def qualification_environment(
    toolchain_root: Path, environment: dict[str, str], output: Path
) -> dict:
    result = subprocess.run(
        [
            sys.executable,
            str(ROOT / "scripts/install-f0-measurement-toolchain.py"),
            "--verify-only",
            "--manifest",
            str(TOOLCHAIN_MANIFEST),
            "--install-root",
            str(toolchain_root),
            "--write-environment",
            str(output),
        ],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if result.returncode:
        raise EvidenceError(result.stdout.strip())
    return load_json(output)


def validate_fresh_value_system_contract(
    session_root: Path,
    identity: dict,
    environment: dict[str, str],
    runner_python: Path,
) -> dict:
    """Run the strict value contract against the session's fresh canonical B2."""
    b2_path = session_root / "chain-1/b2-resident-turn.json"
    contract = load_json(
        ROOT / "tests/architecture/value-system/gate-b-regression.json"
    )
    try:
        logical_b2_path = b2_path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError as error:
        raise EvidenceError("fresh Gate B evidence must remain inside the repository") from error
    contract.update(
        {
            "evidence_path": logical_b2_path,
            "evidence_commit": identity["commit"],
            "evidence_sha256": sha256_file(b2_path),
        }
    )
    contract_path = session_root / "fresh-gate-b-regression.json"
    log_path = session_root / "fresh-value-system-contract.log"
    write_json(contract_path, contract)
    returncode = run_logged(
        [
            str(runner_python),
            str(ROOT / "scripts/check-value-system-contract.py"),
            "--gate-b",
            str(contract_path),
        ],
        environment,
        log_path,
    )
    return {
        "status": "Pass" if returncode == 0 else "Fail",
        "returncode": returncode,
        "fresh_gate_b_sha256": sha256_file(b2_path),
        "log_path": log_path.name,
        "log_sha256": sha256_file(log_path),
    }


def context_for(
    chain_id: str,
    identity: dict,
    subject: dict,
    environment_id: str,
    session: dict,
) -> dict:
    logical_d2 = (
        "benchmarks/runtime/gate-d/d2-resident-nbody.json"
        if chain_id == "chain-1"
        else f"benchmarks/runtime/f0-evidence/{chain_id}/d2-resident-nbody.json"
    )
    logical_b2 = (
        "benchmarks/runtime/gate-b/b2-resident-turn.json"
        if chain_id == "chain-1"
        else f"benchmarks/runtime/f0-evidence/{chain_id}/b2-resident-turn.json"
    )
    logical_d3 = (
        "benchmarks/runtime/gate-d/d3-resident-external.json"
        if chain_id == "chain-1"
        else f"benchmarks/runtime/f0-evidence/{chain_id}/d3-resident-external.json"
    )
    return {
        "protocol_version": PROTOCOL_VERSION,
        "runtime_subject_commit": subject["baseline_commit"],
        "runtime_subject_tree": subject["baseline_tree"],
        "qualification_protocol_commit": identity["commit"],
        "evidence_generation_commit": identity["commit"],
        "qualification_environment_id": environment_id,
        "chain_id": chain_id,
        "session_id": session["session_id"],
        "workflow_run_id": session["provider"]["run_id"],
        "workflow_run_attempt": session["provider"]["run_attempt"],
        "b2_evidence_path": logical_b2,
        "d2_evidence_path": logical_d2,
        "d3_evidence_path": logical_d3,
        "raw_evidence_prefix": f"benchmarks/runtime/f0-evidence/{chain_id}",
    }


def trusted_dispatch_workflow_ref(value: object) -> bool:
    return value in {
        "mech-lang/mech/.github/workflows/ci-full.yml@"
        "refs/heads/qualification/f0-final-evidence",
        "mech-lang/mech/.github/workflows/f0-controlled.yml@"
        "refs/heads/qualification/f0-final-evidence",
    }


def trusted_pull_request_provider(
    provider: dict[str, object], validation_commit: str, event: object
) -> bool:
    if not isinstance(event, dict):
        return False
    pull_request = event.get("pull_request")
    repository = event.get("repository")
    label = event.get("label")
    if not all(isinstance(value, dict) for value in (pull_request, repository, label)):
        return False
    head = pull_request.get("head")
    if not isinstance(head, dict):
        return False
    head_repository = head.get("repo")
    number = pull_request.get("number")
    if not isinstance(head_repository, dict) or not isinstance(number, int):
        return False
    expected_workflow_ref = (
        "mech-lang/mech/.github/workflows/f0-controlled.yml@"
        f"refs/pull/{number}/merge"
    )
    return (
        event.get("action") == "labeled"
        and label.get("name") == "f0-controlled"
        and repository.get("full_name") == "mech-lang/mech"
        and head_repository.get("full_name") == "mech-lang/mech"
        and head.get("ref") == "qualification/f0-final-evidence"
        and head.get("sha") == validation_commit
        and pull_request.get("merge_commit_sha") == provider.get("workflow_sha")
        and provider.get("workflow_ref") == expected_workflow_ref
    )


def trusted_provider_identity(
    provider: dict[str, object], validation_commit: str, event: object
) -> bool:
    if provider.get("event_name") == "workflow_dispatch":
        return (
            trusted_dispatch_workflow_ref(provider.get("workflow_ref"))
            and provider.get("workflow_sha") == validation_commit
        )
    if provider.get("event_name") == "pull_request":
        return trusted_pull_request_provider(provider, validation_commit, event)
    return False


def run_chain(
    chain_id: str,
    session_root: Path,
    context: dict,
    environment: dict[str, str],
    runner_python: Path,
    numpy_python: Path,
    measurement_conditions: dict,
) -> dict:
    chain_root = session_root / chain_id
    chain_root.mkdir(parents=True, exist_ok=False)
    context_path = chain_root / "context.json"
    write_json(context_path, context)
    record = {
        "chain_id": chain_id,
        "status": "running",
        "started_at_utc": now(),
        "qualification_environment_id": context["qualification_environment_id"],
        "conditions_before": conditions(),
        "steps": [],
    }
    condition_error = measurement_conditions_error(
        record["conditions_before"], measurement_conditions
    )
    if condition_error:
        record.update({"status": "Fail", "error": condition_error, "finished_at_utc": now()})
        return record

    b2 = chain_root / "b2-resident-turn.json"
    b2_command = [
        str(runner_python),
        str(ROOT / "scripts/run-gate-b-benchmarks.py"),
        "--phase",
        "B2-resident-turn",
        "--sample-size",
        "10",
        "--warm-up-time",
        "1",
        "--measurement-time",
        "3",
        "--python",
        str(numpy_python),
        "--machine-label",
        "MacBook Air, Mac15,13, Apple M3",
        "--qualification-context",
        str(context_path),
        "--output",
        str(b2),
        "--raw-output",
        str(chain_root / "b2-criterion.log"),
        "--raw-structural-output",
        str(chain_root / "b2-structural.log"),
        "--raw-numpy-output",
        str(chain_root / "b2-numpy.json"),
        "--criterion-evidence-directory",
        str(chain_root / "b2-criterion-samples"),
    ]
    code = run_logged(b2_command, environment, chain_root / "b2-runner.log")
    record["steps"].append({"phase": "B2", "returncode": code})
    if code:
        record.update({"status": "Fail", "error": "B2 failed", "finished_at_utc": now()})
        return record
    record["steps"][-1]["report"] = report_record(
        b2, "B2", context["b2_evidence_path"]
    )

    d2 = chain_root / "d2-resident-nbody.json"
    d2_command = [
        str(runner_python),
        str(ROOT / "scripts/run-gate-d-benchmarks.py"),
        "--phase",
        "D2-resident-nbody",
        "--qualification-context",
        str(context_path),
        "--gate-b-report",
        str(b2),
        "--expected-gate-b-sha256",
        sha256_file(b2),
        "--gate-b-evidence-root",
        str(chain_root),
        "--historical-output-directory",
        str(chain_root / "historical"),
        "--python",
        str(numpy_python),
        "--raw-output",
        str(chain_root / "d2-raw.log"),
        "--output",
        str(d2),
    ]
    code = run_logged(d2_command, environment, chain_root / "d2-runner.log")
    record["steps"].append({"phase": "D2", "returncode": code})
    if d2.exists():
        record["steps"][-1]["report"] = report_record(
            d2, "D2", context["d2_evidence_path"]
        )
    if code:
        record.update({"status": "Fail", "error": "D2 failed", "finished_at_utc": now()})
        return record
    d2_sha256 = sha256_file(d2)

    d3 = chain_root / "d3-resident-external.json"
    d3_command = [
        str(runner_python),
        str(ROOT / "scripts/run-gate-d-benchmarks.py"),
        "--phase",
        "D3-resident-external",
        "--qualification-context",
        str(context_path),
        "--d2-report",
        str(d2),
        "--gate-b-report",
        str(b2),
        "--gate-b-evidence-root",
        str(chain_root),
        "--expected-d2-sha256",
        d2_sha256,
        "--raw-output",
        str(chain_root / "d3-raw.log"),
        "--output",
        str(d3),
    ]
    code = run_logged(d3_command, environment, chain_root / "d3-runner.log")
    record["steps"].append({"phase": "D3", "returncode": code})
    if d3.exists():
        record["steps"][-1]["report"] = report_record(
            d3,
            "D3",
            context["d3_evidence_path"],
        )
    record["conditions_after"] = conditions()
    condition_error = measurement_conditions_error(
        record["conditions_after"], measurement_conditions
    )
    if code or condition_error:
        record.update(
            {
                "status": "Fail",
                "error": condition_error or "D3 failed",
                "finished_at_utc": now(),
            }
        )
        return record
    if any(step["report"]["decision"] != "Pass" for step in record["steps"]):
        record.update(
            {"status": "Fail", "error": "a hard gate failed", "finished_at_utc": now()}
        )
        return record
    record.update({"status": "Pass", "finished_at_utc": now()})
    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument(
        "--toolchain-root", type=Path, default=ROOT / "target/f0-toolchain"
    )
    parser.add_argument("--plan-only", action="store_true")
    args = parser.parse_args(argv)
    session_root = args.output_directory.resolve()
    toolchain_root = args.toolchain_root.resolve()
    identity = git_identity()
    subject = load_json(PRODUCT_TREE_MANIFEST)
    errors = []
    if not identity["clean"]:
        errors.append("F0 qualification refuses a dirty worktree")
    if identity["branch"] is None:
        errors.append("F0 qualification refuses a detached HEAD")
    guard = subprocess.run(
        [sys.executable, str(ROOT / "scripts/check-f0-product-tree.py")],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if guard.returncode:
        errors.append(guard.stdout.strip())
    if session_root.exists():
        errors.append("F0 session output already exists; recorded chains cannot be replaced")
    if errors:
        print(*errors, sep="\n", file=sys.stderr)
        return 2
    if args.plan_only:
        print(
            json.dumps(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "runtime_subject_commit": subject["baseline_commit"],
                    "runtime_subject_tree": subject["baseline_tree"],
                    "evidence_generation_commit": identity["commit"],
                    "preconditioning": "untimed-build-only",
                    "chains": list(RECORDED_CHAINS),
                    "canonical_chain": "chain-1",
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    lock = load_json(TOOLCHAIN_MANIFEST)
    uncontrolled = uncontrolled_build_environment(
        os.environ, lock.get("compiler_environment", {})
    )
    if uncontrolled:
        print(
            "F0 refuses uncontrolled compiler environment variables: "
            + ", ".join(f"{key}={value!r}" for key, value in uncontrolled.items()),
            file=sys.stderr,
        )
        return 2
    required_provider = {
        "event_name": os.environ.get("GITHUB_EVENT_NAME"),
        "repository": os.environ.get("GITHUB_REPOSITORY"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "validation_ref": os.environ.get("F0_VALIDATION_REF"),
        "workflow_ref": os.environ.get("GITHUB_WORKFLOW_REF"),
        "workflow_sha": os.environ.get("GITHUB_WORKFLOW_SHA"),
    }
    try:
        event_payload = load_json(Path(os.environ["GITHUB_EVENT_PATH"]))
    except (KeyError, OSError, EvidenceError):
        event_payload = None
    if (
        required_provider["repository"] != "mech-lang/mech"
        or required_provider["validation_ref"] != identity["commit"]
        or not str(required_provider["run_id"] or "").isdigit()
        or not str(required_provider["run_attempt"] or "").isdigit()
        or not trusted_provider_identity(
            required_provider, identity["commit"], event_payload
        )
    ):
        print("F0 qualification requires a complete exact-head GitHub run identity", file=sys.stderr)
        return 2
    provider = {
        **required_provider,
        "run_id": int(required_provider["run_id"]),
        "run_attempt": int(required_provider["run_attempt"]),
    }
    session_identity = {
        "protocol_version": PROTOCOL_VERSION,
        "runtime_subject_commit": subject["baseline_commit"],
        "runtime_subject_tree": subject["baseline_tree"],
        "qualification_protocol_commit": identity["commit"],
        "evidence_generation_commit": identity["commit"],
        "provider": provider,
    }
    session_identity["session_id"] = sha256_bytes(canonical_json_bytes(session_identity))
    session_root.mkdir(parents=True, exist_ok=False)
    ledger = {
        "schema_version": 1,
        "protocol_version": PROTOCOL_VERSION,
        "status": "environment-preflight",
        "registered_at_utc": now(),
        "runtime_subject_commit": subject["baseline_commit"],
        "runtime_subject_tree": subject["baseline_tree"],
        "qualification_protocol_commit": identity["commit"],
        "evidence_generation_commit": identity["commit"],
        "qualification_environment_id": None,
        "session_id": session_identity["session_id"],
        "provider": provider,
        "selection_rule": {
            "preconditioning": "untimed-build-only",
            "recorded_chains": 3,
            "canonical_chain": "chain-1",
            "retain_failed_chains": True,
        },
        "chains": [],
    }
    ledger_path = session_root / "session.json"
    write_json(ledger_path, ledger)
    environment = controlled_session_environment(os.environ, lock)
    environment_path = session_root / "qualification-environment.json"
    try:
        qualified = qualification_environment(toolchain_root, environment, environment_path)
    except EvidenceError as error:
        ledger.update(
            {"status": "Fail", "error": str(error), "finished_at_utc": now()}
        )
        write_json(ledger_path, ledger)
        print(f"F0 environment failed: {error}", file=sys.stderr)
        return 2
    runner_python = Path(lock["python"]["executable"])
    numpy_python = toolchain_root / "python/bin/python"
    ledger.update(
        {
            "status": "running",
            "qualification_environment_id": qualified[
                "qualification_environment_id"
            ],
        }
    )
    write_json(ledger_path, ledger)
    preconditioning = run_preconditioning(
        session_root, environment, lock["rust"]["channel"]
    )
    ledger["preconditioning"] = preconditioning
    write_json(ledger_path, ledger)
    if preconditioning["status"] != "Pass":
        ledger.update({"status": "Fail", "finished_at_utc": now()})
        write_json(ledger_path, ledger)
        print(
            f"F0 preconditioning failed: {preconditioning.get('error')}",
            file=sys.stderr,
        )
        return 3
    cooldown = wait_for_measurement_conditions(lock["measurement_conditions"])
    ledger["cooldown"] = cooldown
    write_json(ledger_path, ledger)
    if cooldown["status"] != "Pass":
        ledger.update({"status": "Fail", "finished_at_utc": now()})
        write_json(ledger_path, ledger)
        print(f"F0 cooldown failed: {cooldown.get('error')}", file=sys.stderr)
        return 3
    for chain_id in RECORDED_CHAINS:
        context = context_for(
            chain_id,
            identity,
            subject,
            qualified["qualification_environment_id"],
            session_identity,
        )
        result = run_chain(
            chain_id,
            session_root,
            context,
            environment,
            runner_python,
            numpy_python,
            lock["measurement_conditions"],
        )
        ledger["chains"].append(result)
        write_json(ledger_path, ledger)
        if result["status"] != "Pass":
            ledger.update({"status": "Fail", "finished_at_utc": now()})
            write_json(ledger_path, ledger)
            print(f"F0 stopped at {chain_id}: {result.get('error')}", file=sys.stderr)
            return 3
    try:
        strict_value_system = validate_fresh_value_system_contract(
            session_root, identity, environment, runner_python
        )
    except (EvidenceError, OSError, KeyError) as error:
        strict_value_system = {
            "status": "Fail",
            "returncode": 2,
            "error": str(error),
        }
    ledger["fresh_value_system_contract"] = strict_value_system
    if strict_value_system["status"] != "Pass":
        ledger.update({"status": "Fail", "finished_at_utc": now()})
        write_json(ledger_path, ledger)
        print("F0 fresh value-system qualification failed", file=sys.stderr)
        return 3
    recorded = [row for row in ledger["chains"] if row["chain_id"] in RECORDED_CHAINS]
    ledger.update(
        {
            "status": "Pass" if all(row["status"] == "Pass" for row in recorded) else "Fail",
            "finished_at_utc": now(),
        }
    )
    write_json(ledger_path, ledger)
    print(f"F0 qualification session {ledger['status']}: {ledger_path}")
    return 0 if ledger["status"] == "Pass" else 3


if __name__ == "__main__":
    raise SystemExit(main())
