"""Bind a fresh aligned spill-pair runtime campaign before any timings."""

from hashlib import file_digest
import json
from pathlib import Path
import platform

study = Path(__file__).resolve().parent
q = study.parent
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
baseline = q / "cranelift-width-16-owners-cold-native/final-candidate"
candidate = q / "aligned-register-spills-native/final-candidate"
recorded = q / "runtime-matchfinder-20261117"
label = "aligned-register-spills"


def sha(path):
    with path.open("rb") as f:
        return file_digest(f, "sha256").hexdigest()


def write(name, value):
    path = study / name
    assert not path.exists()
    path.write_text(json.dumps(value, indent=2) + "\n")
    return sha(path)


identities = json.loads((study / "preflight-identities.json").read_text())
assert identities["candidate"] == identities["placement_diagnostic"] == identities["checked"]
prior = json.loads((recorded / "spill-pair-full-plan.json").read_text())
contract_hash = write("contract.json", {
    "scope": "diagnostic",
    "level": 12,
    "fixture_inventory_sha256": sha(packet / "fixtures.json"),
    "timing_boundary": "Complete compress-into call including heap cleanup on Apple arm64.",
})
environment_hash = write("environment.json", {
    "cpu": "Apple M1",
    "os": platform.platform(),
    "clock": "Rust Instant on Darwin, with pinned peers",
    "conditions": "Serial four-actor runs with a fixed runtime, adapters and comparators",
})
cases = json.loads((recorded / "residue-64-cases.json").read_text())
write("residue-64-cases.json", cases)
actors = ("mech-control", "mech-experiment", "roc", "libdeflate")
plan = {
    "policy_version": 1,
    "policy_sha256": sha(packet / "qualification-policy-v1.json"),
    "compiler_parent": "0da34baad584c3365642cbd18441e200ab7cf2ca",
    "objective": "encoder_text_bytes",
    "environment_sha256": environment_hash,
    "contract_sha256": contract_hash,
    "cases": cases,
    "runs": [
        {"id": f"{label}-{suffix}", "kind": kind, "order": order, "blocks_per_case": 96}
        for suffix, kind, order in (
            ("focus-residue-64-cases", "focus", "reverse"),
            ("focus-residue-64-cases-reverse", "focus", "forward"),
            ("full", "full", "forward"),
            ("full-reverse", "full", "reverse"),
        )
    ],
    "artifacts": {
        actor: {**{f: sha(path / f) for f in ("program.o", "metadata.json", "mech-worker", "runtime.json")},
                "bytecode_sha256": json.loads((path / "runtime.json").read_text())["image_sha256"]}
        for actor, path in zip(actors[:2], (baseline, candidate), strict=True)
    } | {actor: prior["artifacts"][actor] for actor in actors[2:]},
    "compiler_emitter_sha256": sha(q / "host-opt-levels/opt3-cargo/release/emit_compact_size"),
    "diagnostic_worker_sha256": identities["placement_diagnostic"]["mech-worker"],
    "build_orders": [[actors[(i + shift) % len(actors)] for i in (0, 1, 3, 2)]
                     for shift in range(len(actors))] * 2,
    "conditional_repeatability": {
        "trigger": "Unresolved per-case bound with all full individual peer wins preserved and no established regression",
        "scope": "Every unresolved per-case bound, one distinct-file duplicate-control study",
        "seed": 88161,
        "repetitions_per_order": 64,
        "batch_floor": 24,
    },
    "max_peak_stack_bytes": 5296,
}
plan["worker_sha256"] = {
    actor: plan["artifacts"][actor].get("mech-worker", plan["artifacts"][actor].get("worker_sha256"))
    for actor in actors
}
assert plan["artifacts"]["mech-experiment"]["mech-worker"] == plan["diagnostic_worker_sha256"]
for actor in actors[2:]:
    pinned = json.loads((q / "pinned-peers/pins.json").read_text())["actors"][actor]
    assert pinned["binary_sha256"] == plan["worker_sha256"][actor] == sha(Path(pinned["command"][0]))
print(write("plan.json", plan), flush=True)
