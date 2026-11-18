"""Check and link the static cold-origin layout prototype."""

import json
import os
from hashlib import file_digest
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
reference = q / "native-size-20261117/source-aligned"
out = q / "cold-origin-packed-native/final-candidate"
out.mkdir(parents=True)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def run(args, env=None):
    subprocess.run(list(map(str, args)), cwd=q, env=env, check=True)


scope = runpy.run_path(str(study / "probe.py"))
static = read(study / "report.json")
assert sha(reference / "program.o") == static["reference_sha256"]
assert sha(study / "program.o") == static["candidate_sha256"]
assert static["candidate_text_bytes"] == 71256
shutil.copyfile(study / "program.o", out / "program.o")
metadata = read(reference / "metadata.json")
assert metadata["code_bytes"] == static["reference_root_bytes"]
metadata["code_bytes"] = static["candidate_root_before_alignment_bytes"]
write(out / "metadata.json", metadata)
runtime = read(reference / "runtime.json")
link = [arg.replace(str(reference / "program.o"), str(out / "program.o"))
        .replace(str(reference / "mech-worker"), str(out / "mech-worker"))
        for arg in runtime["link"]]
assert sum(arg.endswith(str(out / "program.o")) for arg in link) == 1
assert link.count(str(out / "mech-worker")) == 1
write(out / "runtime.json", runtime | {
    "link": link,
    "diagnostic_object_postprocess": {
        "scope": "Static Mach-O probe on the source-emitted control; the original flags describe that input.",
        "probe_sha256": sha(study / "probe.py"), "report_sha256": sha(study / "report.json"),
    },
})
environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
run(link, environment)
run([sys.executable, "-B", q / "verify-worker.py", out / "mech-worker", out / "metadata.json",
     "cold-origin-packed"], environment)
origin_map = study / "source-origins.json"
write(origin_map, {"source_mapping": {i: {"site": i} for i in range(len(metadata["origins"]))}})
run([sys.executable, "-B", q / "check-linked-error-exits.py",
     reference.relative_to(q), out.relative_to(q)],
    environment | {"MECH_ERROR_SOURCE_MAP": str(origin_map), "MECH_ERROR_REPORT": "cold-origin-packed-checked-failures.json"})
run([sys.executable, "-B", q / "check-layout-stack.py", out.relative_to(q)],
    environment | {"MECH_STACK_REPORT": "cold-origin-packed-stack.json"})
decoded = scope["decoded"]
assert len(decoded) == 291
report = {
    "artifacts": {name: sha(out / name) for name in ("program.o", "metadata.json", "mech-worker")},
    "image_sha256": runtime["image_sha256"], "stack_report": "cold-origin-packed-stack.json",
    "cold_origin_map": [{"site": row[0], "stub": row[1], "decoder": row[2], "table": row[3],
                         "origin": row[4], "original_return_tail": row[5]} for row in decoded],
    "checked_failure_report": "cold-origin-packed-checked-failures.json",
    "linked_smoke_report": "cold-origin-packed-qualification.json",
}
write(study / "link-proof.json", report)
print(report["artifacts"])
