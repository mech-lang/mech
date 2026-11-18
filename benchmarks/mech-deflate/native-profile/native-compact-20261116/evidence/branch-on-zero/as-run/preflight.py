"""Rebuild the checked-function experiment with exact pinned compiler inputs."""

from collections import Counter
from hashlib import file_digest
import json
from pathlib import Path
import platform
import re
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
profile = q / "host-opt-levels/opt3-cargo/release"
emitter = profile / "emit_compact_size"
bytecode = q / "direct-source-20261117/rebuild-direct/frontend/program.mecb"
references = {name: q / f"stale-cache-accounting/pinned-{name}" for name in ("retained", "smaller")}
candidate = q / "branch-on-zero-native/final-candidate"
objects = {name: study / f"{name}-disabled" for name in references}
objects["candidate"] = candidate


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def save(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def probe(name, reference, destination, *changes):
    command = [sys.executable, "-B", q / "probe-literal-reuse.py", "--native-reference", reference,
               "--emitter", emitter, destination, bytecode, "MECH_COMPACT_INLINE_SINGLE_CALLS=preselected",
               *changes]
    subprocess.run(command, check=True)
    receipt = json.loads((destination / "receipt.json").read_text())
    receipt["object_text_x_libdeflate"] = receipt["object_text_bytes"] / 37080
    receipt["libdeflate_object_text_bytes"] = 37080
    receipt["command"] = list(map(str, command))
    receipts[name] = receipt


def disassemble(folder, name):
    with (study / f"{name}.s").open("w") as stream:
        subprocess.run(["xcrun", "llvm-objdump", "--macho", "-d", folder / "program.o"], check=True, stdout=stream)
    listing = (study / f"{name}.s").read_text()
    begin = listing.index("_mech_compact_closed_checked:\n")
    root, _ = listing[begin:].split("\n_mech_compact_unsigned_dot_product:", 1)
    instructions = []
    for line in root.splitlines()[1:]:
        parts = line.split("\t")
        if len(parts) >= 3:
            instructions.append((parts[2].strip(), "\t".join(parts[3:]).strip()))
    ops = Counter(op for op, _ in instructions)
    at_zero = sum(
        first[0] == "cmp" and re.fullmatch(r"[wx]\d+, #0x0", first[1]) is not None
        and second[0] in ("b.eq", "b.ne")
        for first, second in zip(instructions, instructions[1:])
    )
    return {
        "root_instructions_including_padding": len(instructions),
        "compare_zero_followed_by_equality_branch": at_zero,
        "cbz_cbnz": ops["cbz"] + ops["cbnz"],
        "cmp": ops["cmp"], "b_eq_ne": ops["b.eq"] + ops["b.ne"],
    }


assert sha(bytecode) == "c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3"
receipts = {}
for name, reference in references.items():
    probe(name, reference, objects[name])
    for file in ("metadata.json", "program.o", "mech-worker"):
        assert (objects[name] / file).read_bytes() == (reference / file).read_bytes(), (name, file)
probe("candidate", references["smaller"], candidate, "MECH_COMPACT_BRANCH_ON_ZERO=1")
control_meta = json.loads((objects["smaller"] / "metadata.json").read_text())
candidate_meta = json.loads((candidate / "metadata.json").read_text())
assert candidate_meta["origins"] == control_meta["origins"]
receipts["metadata_fields_changed"] = sorted(key for key in control_meta if control_meta[key] != candidate_meta[key])
receipts["root_instruction_census"] = {
    name: disassemble(objects[name], name)
    for name in ("smaller", "candidate")
}
save(study / "preflight.json", receipts)

focus = (
    "empty", "one-stored", "seven-stored-prefix", "eight-overlap", "random-64",
    "random-1024", "prefix-random-1024", "320-overlap", "prefix-periodic-1024",
    "prefix-natural-4096", "prefix-natural-8192", "prefix-natural-16384",
    "reymont-prefix-32769", "heldout-mixed-131073", "prefix-periodic-262144", "prefix-random-1048576",
)
fixture_file = q / "fixtures.json"
fixtures = json.loads(fixture_file.read_text())
assert len(fixtures) == 81 and set(focus) <= {row["name"] for row in fixtures}
save(study / "zero-branch-cases.json", focus)
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
peers = json.loads((q / "pinned-peers/pins.json").read_text())
assert all(sha(Path(peer["command"][0])) == peer["binary_sha256"] for peer in peers["actors"].values())
plan = {
    "objective": "encoder_text_bytes",
    "policy_version": 1, "policy_sha256": sha(packet / "qualification-policy-v1.json"),
    "measurement_host": "Apple M1",
    "platform": platform.platform(),
    "contract": "Level 12 compress-into invocation through private heap cleanup.",
    "fixture_inventory_sha256": sha(fixture_file),
    "driver_sha256": sha(q / "benchmark-matrix-size-variants.py"),
    "timer_sha256": sha(q / "compare-library.py"),
    "compilers": {name: sha(profile / name) for name in ("emit_compact_size", "compile_profile")},
    "actors": {
        actor: {name: sha(folder / name) for name in ("metadata.json", "program.o", "mech-worker", "runtime.json")}
        for actor, folder in (("mech-control", references["smaller"]), ("mech-experiment", candidate))
    } | peers["actors"],
    "optional_full_suite_trigger": "Continue from both focused orders after screening for established per-case regressions.",
    "runs": [
        {"id": f"branch-on-zero-{suffix}", "kind": kind, "order": order, "blocks_per_case": 96}
        for suffix, kind, order in (
            ("focus-zero-branch-cases", "focus", "reverse"),
            ("focus-zero-branch-cases-reverse", "focus", "forward"),
            ("full", "full", "forward"),
            ("full-reverse", "full", "reverse"),
        )
    ],
    "focus_cases": list(focus),
}
save(study / "timing-plan.json", plan)
(study / "timing-plan.sha256").write_text(sha(study / "timing-plan.json") + "\n")
print(json.dumps({
    "size": {name: receipts[name]["object_text_bytes"] for name in objects},
    "metadata_changed": receipts["metadata_fields_changed"],
    "root_instructions": receipts["root_instruction_census"],
}, indent=2), flush=True)
