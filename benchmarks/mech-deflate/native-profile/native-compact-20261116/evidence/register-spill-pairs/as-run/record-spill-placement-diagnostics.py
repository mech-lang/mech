"""Replay the paired-spill placement diagnostics and freeze their raw evidence."""
from hashlib import file_digest
import gzip
import json
from pathlib import Path
import shutil
import sys

q = Path(__file__).resolve().parent
study = q / "runtime-matchfinder-20261117"
out = study / "evidence-draft"
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
import qualification_timings as timings
from qualification_records import Findings


def read(path):
    return json.loads(path.read_text())


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


policy = read(packet / "qualification-policy-v1.json")
fixtures = {row["name"]: row for row in read(packet / "fixtures.json")}
control = q / "cranelift-width-16-owners-cold-native/final-candidate"
summary = read(out / "report.json")
experiments = {}
for key, label, suffix, plan_filename, script in (
    ("original_addresses", "spill-pairs-restored-placement", "placement-cases",
     "placement-plan.json", "restore-spill-pair-placement.py"),
    ("alignment_modulo_64", "spill-pairs-residue-64", "residue-64-cases",
     "residue-64-plan.json", "residue-64-spill-pair-placement.py"),
):
    plan = read(study / plan_filename)
    candidate = q / f"{label}-native/final-candidate"
    receipt = read(candidate / "receipt.json")
    assert receipt["worker_sha256"] == plan["restored_worker_sha256"] == sha(candidate / "mech-worker")
    assert plan["cases"] and set(plan["cases"]) <= set(fixtures)
    result_rows = []
    for postfix, direction in (("", "reverse"), ("-reverse", "forward")):
        run = f"{label}-focus-{suffix}{postfix}"
        folder = q / run
        completion = read(folder / "complete.json")
        rows = read(folder / "summary.json")
        expected = [name for name in fixtures if name in plan["cases"]]
        assert [row["case"] for row in rows] == (expected[::-1] if direction=="reverse" else expected)
        assert completion["balanced_blocks_per_case"] == 96 and completion["cases"] == len(expected)
        actors = read(folder / "pins.json")["actors"]
        for actor, source in (("mech-control", control), ("mech-experiment", candidate)):
            assert actors[actor]["binary_sha256"] == sha(source / "mech-worker")
            assert actors[actor]["metadata_sha256"] == sha(source / "metadata.json")
        result = {"id":run,"kind":"focus","order":direction,"cases":{},"ratios":{},"peer_output":{},
                  "peak":dict.fromkeys(timings.MECH,0)}
        for row in rows:
            case = row["case"]
            with gzip.open(folder / case / "observations.json.gz", "rt") as stream:
                raw = json.load(stream)
            blocks, samples, peaks, peer = timings.case_blocks(raw, fixtures[case], 96)
            result["ratios"][case] = timings.verify_summary(row, fixtures[case], samples, blocks, peer)
            result["cases"][case] = blocks
            result["peer_output"][case] = peer
            for actor, peak in peaks.items():
                result["peak"][actor] = max(result["peak"][actor], peak)
        result_rows.append(result)
        shutil.copytree(folder,out/"runs"/run)
    findings = Findings()
    analysis = timings.decide_runtime(result_rows, [], policy, fixtures, findings)
    analysis["timing_only_findings"] = {"rejected": findings.rejected, "unresolved": findings.unresolved}
    archive = out / key
    archive.mkdir()
    shutil.copyfile(study / plan_filename, archive / "plan.json")
    for filename in ("receipt.json","runtime.json"):
        shutil.copyfile(candidate/filename,archive/filename)
    proof = Path(receipt["proof"])
    (archive/"exact-calls.json.gz").write_bytes(gzip.compress(proof.read_bytes(),mtime=0))
    write(archive / "focused-policy.json", analysis)
    shutil.copyfile(q/script,out/"as-run"/script)
    compared = ("prefix-natural-65535","prefix-natural-262144")
    important = {name:analysis["cases"][name]["combined"] for name in compared}
    experiments[key] = {
        "text_bytes": receipt["object_text_bytes"],
        "text_x_roc":receipt["object_text_x_roc"],
        "text_x_libdeflate":receipt["object_text_x_libdeflate"],
        "worker_sha256":receipt["worker_sha256"],
        "native_symbol_count":receipt["linked_native_symbol_addresses_checked"],
        "diagnostic_only":True,
        "archive_directory":key,
        "natural_text_intervals":important,
        "timing_focused_findings":analysis["timing_only_findings"],
    }
    print(key, receipt["object_text_bytes"], important, findings.rejected, findings.unresolved)
summary["placement_diagnostics"] = experiments
write(out/"report.json", summary)
