"""Evaluate a native compact qualification manifest against its pinned policy."""

import argparse
import json
from pathlib import Path
import sys

from qualification_artifacts import (
    artifacts_for, check_builds, check_correctness, check_failures, check_stack, collect_sizes,
)
from qualification_records import (
    ACTORS, CANDIDATE, CONTROL, MECH, Evidence, Findings, InvalidEvidence,
    is_count, is_digest, load_fixtures, load_policy, read_json, reference_sha, require,
)
from qualification_timings import decide_runtime, duplicate_study, runtime_run

OBJECTIVES = (
    "encoder_text_bytes", "build_seconds", "compiler_peak_rss_bytes",
    "runtime_peak_payload_bytes", "runtime",
)


def evidence_bindings(manifest, evidence, policy, identities):
    environment = evidence.json(manifest.get("environment"), "measurement environment")
    if environment is not None:
        require(isinstance(environment, dict) and all(environment.get(key) for key in ("cpu", "os", "clock")),
                "measurement environment lacks CPU, operating system, or clock identity")
    contract = evidence.json(manifest.get("contract"), "runtime contract")
    if isinstance(contract, list):
        require(identities["contract_sha256"] == policy["fixture_inventory_sha256"],
                "legacy fixture contract differs from the frozen inventory")
        evidence.findings.unresolved.append("legacy fixture contract lacks the runtime scope and timing boundary")
    elif contract is not None:
        require(contract["fixture_inventory_sha256"] == policy["fixture_inventory_sha256"]
                and contract["level"] == 12, "runtime contract changed the fixture inventory or compression level")
        if contract["scope"] != "diagnostic":
            evidence.findings.unresolved.append(f"policy v{policy['version']} applies to the diagnostic native entry")
        require(isinstance(contract["timing_boundary"], str) and bool(contract["timing_boundary"]),
                "runtime timing boundary is missing")
    return contract


def get_plan(manifest, evidence, policy, pinned_sha, identities):
    reference = manifest.get("plan")
    if pinned_sha is None:
        evidence.findings.unresolved.append("independently pinned predeclared plan SHA-256 required")
    else:
        require(is_digest(pinned_sha), "invalid independently pinned plan SHA-256")
        if reference is not None:
            require(reference_sha(reference) == pinned_sha, "campaign plan differs from the independently pinned plan")
    plan = evidence.json(reference, "predeclared campaign plan")
    if plan is None:
        return {}
    require(plan["policy_version"] == policy["version"] and plan["objective"] == manifest["objective"],
            "campaign plan disagrees on the policy or intended improvement")
    for field in ("environment_sha256", "contract_sha256"):
        require(is_digest(plan[field]) and plan[field] == identities[field], f"campaign plan disagrees on {field}")
    planned = plan["runs"]
    require(isinstance(planned, list) and len(planned) == len({r["id"] for r in planned}),
            "duplicate or missing predeclared timing run IDs")
    for run in planned:
        require(isinstance(run["id"], str) and bool(run["id"]) and run["kind"] in ("full", "focus")
                and run["order"] in ("forward", "reverse") and is_count(run["blocks_per_case"], positive=True),
                "invalid predeclared timing run")
    expected = {r["order"] for r in planned if r["kind"] == "full"}
    if expected != {"forward", "reverse"}:
        evidence.findings.unresolved.append("plan does not predeclare both complete timing orders")
    duplicates = plan["duplicate_control_ids"]
    require(isinstance(duplicates, list) and len(duplicates) == len(set(duplicates))
            and all(isinstance(value, str) and value for value in duplicates), "invalid repeatability plan IDs")
    return plan


def planned_records(entries, expected, label, findings):
    require(isinstance(entries, list), f"{label} inventory must be an array")
    seen = set()
    for entry in entries:
        run_id = entry["id"]
        require(isinstance(run_id, str) and bool(run_id), f"{label} ID must be named")
        require(run_id not in seen, f"duplicate {label} ID {run_id}")
        seen.add(run_id)
    if expected is not None:
        for run_id in sorted(set(expected) - seen):
            findings.unresolved.append(f"predeclared {label} missing: {run_id}")
        for run_id in sorted(seen - set(expected)):
            findings.unresolved.append(f"unplanned {label} included: {run_id}")
    return entries


def check_improvement(objective, sizes, builds, runtime, findings):
    improvement = None
    if objective == "encoder_text_bytes":
        if all("encoder_text_bytes" in sizes.get(actor, {}) for actor in MECH):
            improvement = 1 - sizes[CANDIDATE]["encoder_text_bytes"] / sizes[CONTROL]["encoder_text_bytes"]
    elif objective in ("compiler_peak_rss_bytes", "runtime_peak_payload_bytes"):
        peak = builds.get("compiler_peak_rss_bytes", {}) if objective == "compiler_peak_rss_bytes" else runtime.get(
            "peak_private_payload_bytes", {}
        )
        if all(actor in peak for actor in MECH) and peak[CONTROL] > 0:
            improvement = 1 - peak[CANDIDATE] / peak[CONTROL]
    elif objective == "build_seconds" and "paired_change" in builds:
        interval = builds["paired_change"]
        if interval["upper"] < 0 or interval["lower"] >= 0:
            improvement = -interval["estimate"]
    elif objective == "runtime" and runtime.get("cases"):
        cases = [row["combined"] for row in runtime["cases"].values()]
        if improved := [row for row in cases if row["upper"] < 0]:
            improvement = max(-row["estimate"] for row in improved)
        elif all(row["lower"] >= 0 for row in cases):
            improvement = max(-row["estimate"] for row in cases)
    if improvement is None:
        findings.unresolved.append(f"{objective}: intended improvement is unproven")
    elif improvement <= 0:
        findings.rejected.append(f"{objective}: did not improve")
    return {"objective": objective, "fractional_improvement": improvement}


def evaluate_manifest(manifest, evidence_root, plan_sha256):
    findings = Findings()
    evidence = Evidence(evidence_root, findings)
    require(isinstance(manifest, dict), "manifest must be a JSON object")
    require(type(manifest.get("schema_version")) is int and manifest["schema_version"] == 1,
            "unsupported qualification manifest version")
    policy = load_policy(manifest.get("policy_version"))
    require(manifest.get("objective") in OBJECTIVES, "unsupported intended improvement")
    identities = {actor: {} for actor in ACTORS}
    for field in ("environment", "contract"):
        identities[f"{field}_sha256"] = reference_sha(manifest.get(field))
    paths = {actor: {} for actor in ACTORS}
    bindings = findings.check("campaign bindings", evidence_bindings, manifest, evidence, policy, identities)
    fixtures = findings.check("frozen fixture contract", load_fixtures, evidence, policy) or {}
    plan = findings.check("campaign plan", get_plan, manifest, evidence, policy, plan_sha256, identities) or {}
    artifacts = manifest.get("artifacts", {})
    require(isinstance(artifacts, dict) and artifacts.keys() <= set(ACTORS), "unknown artifact subject")
    for actor in ACTORS:
        findings.check("artifact provenance", artifacts_for, actor, artifacts.get(actor, {}), evidence, identities, paths)
    if plan:
        pinned_workers = plan.get("worker_sha256")
        if pinned_workers is None:
            findings.unresolved.append("predeclared plan does not bind worker identities")
        else:
            require(pinned_workers == {actor: identities[actor].get("worker") for actor in ACTORS},
                    "worker identities differ from the predeclared plan")
        pinned_artifacts = plan.get("artifact_sha256")
        if pinned_artifacts is None:
            findings.unresolved.append("predeclared plan does not bind build and runtime artifacts")
        else:
            require(pinned_artifacts == {actor: identities[actor] for actor in ACTORS},
                    "build or runtime artifacts differ from the predeclared plan")
    expected = {r["id"]: r for r in plan["runs"]} if plan else None
    records = findings.check("timing inventory", planned_records, manifest.get("runs", []), expected,
                             "timing run", findings) or []
    repeats_expected = plan["duplicate_control_ids"] if plan else None
    repeats_recorded = findings.check("repeatability inventory", planned_records, manifest.get("duplicate_controls", []),
                                      repeats_expected, "duplicate-control study", findings) or []
    runs = []
    repeats = []
    if fixtures:
        for entry in records:
            run = findings.check(entry["id"], runtime_run, entry, fixtures, evidence, identities, policy,
                                 expected.get(entry["id"]) if expected is not None else None)
            if run is not None:
                runs.append(run)
        for entry in repeats_recorded:
            repeat = findings.check(entry["id"], duplicate_study, entry, fixtures, evidence, identities, policy)
            if repeat:
                repeats.append((entry["id"], repeat))
    runtime = findings.check("runtime decision", decide_runtime, runs, repeats, policy, fixtures, findings) or {}
    if plan:
        for name in plan.get("duplicate_control_cases", []):
            require(name in fixtures, "repeatability plan contains an unknown case")
            if not runtime.get("cases", {}).get(name, {}).get("duplicate_controls"):
                findings.unresolved.append(f"{name}: required duplicate-control observations missing")
    checks = manifest.get("checks", {})
    proof = {
        "correctness": findings.check("correctness", check_correctness, checks.get("correctness", {}),
                                      fixtures, evidence, identities),
        "checked_failures": findings.check("checked failures and origins", check_failures,
                                           checks.get("checked_failures"), paths, evidence, identities, policy),
        "peak_stack_bytes": findings.check("stack", check_stack, checks.get("stack", {}), fixtures, evidence,
                                           identities, plan),
        "resource_scope": bindings.get("scope") if isinstance(bindings, dict) else "diagnostic-replay",
    }
    builds = findings.check("matched builds", check_builds, manifest.get("metrics"), evidence, identities,
                            plan, policy) or {}
    sizes = findings.check("artifact size", collect_sizes, paths) or {}
    improvement = check_improvement(manifest["objective"], sizes, builds, runtime, findings)
    return {
        "decision": findings.decision,
        "policy_version": policy["version"],
        "issues": {"rejected": findings.rejected, "unresolved": findings.unresolved},
        "frozen_suite": {"named_cases": len(fixtures), "distinct_contracts": policy["distinct_contracts"]},
        "artifacts": identities,
        "runtime": runtime,
        "proofs": proof,
        "sizes": sizes,
        "builds": builds,
        "improvement": improvement,
        "evidence": {"runs": records, "duplicate_controls": repeats_recorded,
                     **{key: manifest.get(key) for key in ("artifacts", "environment", "contract", "plan", "checks", "metrics")}},
    }


def evaluate(manifest, evidence_root, plan_sha256=None):
    """Return a fail-closed decision and replay details for the supplied evidence."""
    try:
        return evaluate_manifest(manifest, evidence_root, plan_sha256)
    except (InvalidEvidence, TypeError, KeyError, AttributeError) as error:
        return {"decision": "Reject", "issues": {"rejected": [str(error)], "unresolved": []}}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--evidence-root", type=Path, help="base for evidence paths, defaulting to the manifest's directory")
    parser.add_argument("--plan-sha256", help="independently recorded SHA-256 of the predeclared campaign plan")
    parser.add_argument("--promotion", action="store_true", help="exit nonzero unless the evidence supports Retain")
    args = parser.parse_args(argv)
    try:
        if not args.manifest.is_file():
            raise FileNotFoundError(args.manifest)
        manifest = read_json(args.manifest)
        result = evaluate(manifest, args.evidence_root or args.manifest.parent, args.plan_sha256)
    except FileNotFoundError as error:
        result = {"decision": "Unresolved", "issues": {"rejected": [], "unresolved": [f"missing file: {error}"]}}
    except (InvalidEvidence, TypeError, KeyError, AttributeError) as error:
        result = {"decision": "Reject", "issues": {"rejected": [str(error)], "unresolved": []}}
    json.dump(result, sys.stdout, indent=2, allow_nan=False)
    print()
    if args.promotion and result["decision"] != "Retain":
        return 1 if result["decision"] == "Reject" else 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
