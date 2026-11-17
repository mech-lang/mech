"""Verify the two archived full orders using the packet's qualification v1."""

import argparse
import json
from pathlib import Path
import sys


ARCHIVE = Path(__file__).resolve().parent


def verify_provenance(packet, plan, policy, fixtures):
    identities = plan["identities"]
    for field in ("environment", "contract"):
        require(digest(ARCHIVE / f"{field}.json") == identities[f"{field}_sha256"],
                f"{field} pin differs")
    environment = read_json(ARCHIVE / "environment.json")
    require(digest(ARCHIVE / "as-run" / "cpu-source.json") == environment["cpu_source"]["sha256"],
            "CPU-source digest differs")

    for field, name in {"driver": "full-timing.py", "timer": "compare-library.py",
                        "as_run_inventory": "fixtures-as-run.json",
                        "pinned_peers": "peer-pins.json"}.items():
        require(digest(ARCHIVE / "as-run" / name) == plan[field]["sha256"], f"{field} digest differs")

    frozen = ARCHIVE / "fixture-content.json"
    require(digest(frozen) == plan["fixture_content_sha256"], "fixture-content digest differs")
    contents = read_json(frozen)
    as_run = read_json(ARCHIVE / "as-run" / "fixtures-as-run.json")
    require([row["name"] for row in as_run] == list(fixtures) == list(contents),
            "fixture inventory order or cases differ")
    for row in as_run:
        name = row["name"]
        frozen_case = fixtures[name]
        portable_row = row | {"prefix": frozen_case["prefix"]}
        require(fixture_contract(portable_row) == fixture_contract(frozen_case), f"{name} contract differs")
        require(set(contents[name]) == {"input", "prefix", "expected"}, f"{name} content fields differ")
        for field, sha256 in contents[name].items():
            require(digest(packet / frozen_case[field]) == sha256, f"{name} {field} digest differs")
    contract = read_json(ARCHIVE / "contract.json")
    require(contract["canonical_inventory_sha256"] == policy["fixture_inventory_sha256"],
            "call contract refers to a different fixture inventory")

    observed = read_json(ARCHIVE / "as-run" / "artifact-verification.json")
    proof = packet / "evidence" / "shared-copy-tails"
    for field, path in {"source_reproduction": "source-proof/exact-source-identity.json",
                        "metadata_reconciliation": "source-proof/metadata-format.json",
                        "original_focus": "focus-replay.json",
                        "original_repeatability": "repeatability/combined-replay.json"}.items():
        require(digest(proof / path) == plan[field]["sha256"], f"{field} digest differs")
    source = read_json(proof / "source-proof" / "exact-source-identity.json")
    focus_plan = proof / "plan.json"
    focus = read_json(proof / "focus-replay.json")
    require(digest(focus_plan) == focus["plan"]["sha256"] == (proof / "plan.sha256").read_text().strip(),
            "earlier focus plan digest differs")
    static = proof / "static-report.json"
    old = read_json(focus_plan)
    require(digest(static) == old["static_report"]["sha256"], "frame-copy size report digest differs")
    sizes = read_json(static)
    for actor, source_role, size_role in (("mech-control", "before", "reference"),
                                          ("mech-experiment", "shared-copies", "candidate")):
        bindings = {name: ref["sha256"] for name, ref in plan["bindings"][actor].items()}
        require(observed[actor]["sha256"] == bindings, f"{actor} local artifact verification differs")
        source_hashes = source[source_role]["artifact_sha256"]
        require(all(bindings[name] == sha256 for name, sha256 in source_hashes.items()),
                f"{actor} emitted or linked artifact differs from source proof")
        require(observed[actor]["encoder_text_bytes"] == sizes[f"{size_role}_text_bytes"]
                and sizes[f"{size_role}_sha256"] == bindings["program.o"],
                f"{actor} encoder-size measurement differs from source proof")
        require(identities[actor] == {"worker": bindings["mech-worker"], "metadata": bindings["metadata.json"]},
                f"{actor} worker or metadata digest differs")
        for name in ("metadata.json", "runtime.json"):
            path = ARCHIVE / "identities" / actor / name
            require(digest(path) == bindings[name], f"{actor} {name} digest differs")

    reconciliation = read_json(proof / "source-proof" / "metadata-format.json")
    previous = old["identities"]["mech-experiment"]
    candidate = identities["mech-experiment"]
    require(candidate["worker"] == previous["worker"] == reconciliation["worker_sha256"]
            and candidate["metadata"] == reconciliation["source_sha256"]
            and previous["metadata"] == reconciliation["prototype_sha256"],
            "focus and full timing metadata reconciliation differs")
    prototype_metadata = proof / "identities" / "mech-experiment" / "metadata.json"
    current_metadata = ARCHIVE / "identities" / "mech-experiment" / "metadata.json"
    require(digest(prototype_metadata) == previous["metadata"], "formatted prototype metadata differs")
    compact = json.dumps(read_json(prototype_metadata), separators=(",", ":"), ensure_ascii=False).encode()
    require(compact == current_metadata.read_bytes(), "formatted prototype metadata serializes differently")

    peer_pins = read_json(ARCHIVE / "as-run" / "peer-pins.json")["actors"]
    for actor in ("roc", "libdeflate"):
        pin = peer_pins[actor]
        values = {"worker": pin["binary_sha256"], "metadata": pin.get("metadata_sha256")}
        require(identities[actor] == values
                and all(observed[actor][key] == value for key, value in values.items()),
                f"{actor} peer records differ")
        if observed[actor].get("metadata_bytes_rehashed"):
            require(digest(ARCHIVE / "identities" / actor / "metadata.json") == values["metadata"],
                    f"{actor} independently checked metadata differs")


def run_replay(plan, original, fixtures, policy, findings):
    evidence = Evidence(ARCHIVE, findings)
    runs = []
    for entry in plan["runs"]:
        run_id = entry["id"]
        require(entry["kind"] == "full", f"{run_id} is not a full timing order")
        receipt = original["receipts"][run_id]
        canonical_pins = receipt["pins"] | {"path": f"canonical-pins/{run_id}.json"}
        actual = evidence.json(receipt["original_pins"], f"{run_id} original pins")
        canonical = evidence.json(canonical_pins, f"{run_id} canonical pins")
        require(actual is not None and actual["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"],
                f"{run_id} original pins differ from acquisition plan")
        require(canonical == actual | {"input_inventory_sha256": policy["fixture_inventory_sha256"]},
                f"{run_id} pin changes extend beyond the portable inventory mapping")
        run = runtime_run(receipt | {"pins": canonical_pins}, fixtures, evidence, plan["identities"], policy, entry)
        require(run is not None and len(run["cases"]) == policy["named_cases"], f"{run_id} incomplete")
        runs.append(run)
    require(len(runs) == 2 and {run["order"] for run in runs} == {"forward", "reverse"},
            "exactly two complete forward and reverse runs are required")
    actual = decide_runtime(runs, [], policy, fixtures, findings)
    for key, value in actual.items():
        require(value == original[key], f"recomputed runtime report differs in {key}")
    for key in ("decision", "rejected", "unresolved"):
        require(getattr(findings, key) == original[key], f"recomputed findings differ in {key}")
    return runs


def main(args, packet):
    policy_path = packet / "qualification-policy-v1.json"
    plan_file = ARCHIVE / "plan.json"
    plan = read_json(plan_file)
    stored_replay = ARCHIVE / "full-replay.json"
    original = read_json(stored_replay)
    require(digest(plan_file) == (ARCHIVE / "plan.sha256").read_text().strip() == original["plan"]["sha256"],
            "full replay acquisition plan digest differs")
    require(digest(stored_replay) == (ARCHIVE / "full-replay.sha256").read_text().strip(),
            "full replay receipt digest differs")
    require(digest(policy_path) == plan["policy"]["sha256"], "runtime policy differs")
    policy = read_json(policy_path)
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    verify_provenance(packet, plan, policy, fixtures)
    runs = run_replay(plan, original, fixtures, policy, findings)
    print(f"Recorded v1 runtime decision: {findings.decision}", flush=True)
    for reason in findings.rejected + findings.unresolved:
        print(reason, flush=True)

    if args.write_diagnostic or (ARCHIVE / "baseline-drift.json").exists():
        from baseline_drift import compare_campaigns
        diagnostic = compare_campaigns(ARCHIVE, packet, plan, original, runs, fixtures, policy)
        if args.write_diagnostic:
            with args.write_diagnostic.open("x") as stream:
                json.dump(diagnostic, stream, indent=2)
                stream.write("\n")
        else:
            require(diagnostic == read_json(ARCHIVE / "baseline-drift.json"), "baseline diagnostic differs")
        print("Baseline drift diagnostic matches exact archived inputs", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", type=Path, default=ARCHIVE.parent.parent)
    parser.add_argument("--write-diagnostic", type=Path)
    options = parser.parse_args()
    review_packet = options.packet.resolve()
    sys.path.insert(0, str(review_packet))

    from qualification_records import Evidence, Findings, digest, fixture_contract, load_fixtures, read_json, require
    from qualification_timings import decide_runtime, runtime_run

    main(options, review_packet)
