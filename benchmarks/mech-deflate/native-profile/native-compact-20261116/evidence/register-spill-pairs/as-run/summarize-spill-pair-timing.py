"""Replay a focused screen or complete runs with the published runtime policy."""
from hashlib import file_digest
import gzip
import json
from pathlib import Path
import sys

q = Path(__file__).resolve().parent
study = q / "runtime-matchfinder-20261117"
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
from qualification_records import Findings
import qualification_timings as timings

policy = json.loads((packet / "qualification-policy-v1.json").read_text())
fixture_list = json.loads((packet / "fixtures.json").read_text())
fixtures = {row["name"]: row for row in fixture_list}
assert len(fixtures) == 81
plan = json.loads((study / "spill-pair-screen-plan.json").read_text())
control = q / "cranelift-width-16-owners-cold-native/final-candidate"
candidate = q / "spill-pairs-20261117-native/final-candidate"
peers = json.loads((q / "pinned-peers/pins.json").read_text())["actors"]


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


assert sha(control / "mech-worker") == plan["baseline_worker_sha256"]


def verify(folder, kind, order):
    root = q / folder
    summary = json.loads((root / "summary.json").read_text())
    pins = json.loads((root / "pins.json").read_text())
    for actor, directory in (("mech-control", control), ("mech-experiment", candidate)):
        assert pins["actors"][actor]["binary_sha256"] == sha(directory / "mech-worker")
        assert pins["actors"][actor]["metadata_sha256"] == sha(directory / "metadata.json")
        assert (root / f"{actor}.stderr").read_text().endswith(
            "CLEANUP private_payload_bytes=0 live_owner_records=0\n"
        )
    for actor in ("roc", "libdeflate"):
        pinned = peers[actor]
        assert pins["actors"][actor] == pinned
        assert sha(Path(pinned["command"][0])) == pinned["binary_sha256"]
    complete = json.loads((root / "complete.json").read_text())
    full = kind == "full"
    expected = list(fixtures) if full else [name for name in fixtures if name in plan["cases"]]
    assert [row["case"] for row in summary] == (expected[::-1] if order == "reverse" else expected)
    assert len(summary) == complete["cases"]
    assert complete["balanced_blocks_per_case"] == plan["balanced_blocks_per_case"]
    result = {"id": folder, "kind": kind, "order": order, "cases": {}, "ratios": {}, "peer_output": {},
              "peak": dict.fromkeys(timings.MECH, 0)}
    for row in summary:
        case = row["case"]
        with gzip.open(root / case / "observations.json.gz", "rt") as stream:
            raw = json.load(stream)
        blocks, samples, peak, peer = timings.case_blocks(raw, fixtures[case],
                                                          complete["balanced_blocks_per_case"])
        result["ratios"][case] = timings.verify_summary(row, fixtures[case], samples, blocks, peer)
        result["cases"][case], result["peer_output"][case] = blocks, peer
        for actor, amount in peak.items():
            result["peak"][actor] = max(result["peak"][actor], amount)
    return result


focus = "spill-pairs-20261117-focus-spill-pair-cases"
runs = [verify(focus, "focus", "reverse"), verify(focus + "-reverse", "focus", "forward")]
full = sys.argv[1:] == ["--full"]
assert not sys.argv[1:] or full
if full:
    runs += [verify("spill-pairs-20261117-full", "full", "forward"),
             verify("spill-pairs-20261117-full-reverse", "full", "reverse")]
findings = Findings()
report = timings.decide_runtime(runs, [], policy, fixtures, findings)
report["timing_only_findings"] = {"rejected": findings.rejected, "unresolved": findings.unresolved}
report["qualification_scope"] = "Timing and per-case review. Promotion also requires a complete provenance manifest and linked contract checks."
report["paired_control_worker_sha256"] = sha(control / "mech-worker")
report["candidate_worker_sha256"] = sha(candidate / "mech-worker")
report["plan_sha256"] = sha(study / "spill-pair-screen-plan.json")
output = study / ("full-runtime-policy.json" if full else "focused-runtime-policy.json")
output.write_text(json.dumps(report, indent=2) + "\n")
for run in report["runs"]:
    print(run["id"], {
        peer: (details["point_estimate_wins"], details["lost_cases"], details["gained_cases"])
        for peer, details in run["peers"].items()
    })
print(report["timing_only_findings"])
