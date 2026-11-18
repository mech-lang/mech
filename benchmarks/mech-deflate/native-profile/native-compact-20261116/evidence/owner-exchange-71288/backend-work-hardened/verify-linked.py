"""Measure native stack and compare checked allocation failure on the exact workers."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
workers = study / "worker-checks"
roles = ("retained", "size-control", "owner-exchange")
actors = [workers / role for role in roles]
out = study / "linked-proofs"
out.mkdir()
env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}


def write(path, item):
    path.write_text(json.dumps(item, indent=2) + "\n")


def copy_driver(name, substitutions):
    original = (q / name).read_text()
    for old, new in substitutions:
        assert original.count(old) == 1, (name, old)
        original = original.replace(old, new, 1)
    path = out / name
    path.write_text(original)
    return path


def run(path, args, overrides):
    command = [sys.executable, "-B", str(path), *map(str, args)]
    with path.with_suffix(".log").open("wb") as log:
        subprocess.run(command, cwd=q, env=env | overrides | {"PYTHONPATH": str(q)}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    return command


record = [json.loads((q / f"hardened-backend-{role}-smokes.json").read_text()) for role in roles]
assert len(record[0]) == 162 and record[0] == record[1] == record[2]
assert all("CLEANUP private_payload_bytes=0 live_owner_records=0" in
           (q / f"hardened-backend-{role}-smokes.stderr").read_text() for role in roles)
origins = [json.loads((path / "metadata.json").read_text())["origins"] for path in actors]
assert origins[0] == origins[1] == origins[2]

fail_support = out / "failure-support"
fail_support.mkdir()
for name in ("pinned_allocator.rs", "mech-worker.rs"):
    shutil.copyfile(q / "pinned-host-link" / name, fail_support / name)
fail = copy_driver("check-linked-error-exits.py", [
    ("q = Path(__file__).resolve().parent", f"q = Path({str(q)!r})"),
    ("link_root = q / 'pinned-host-link'", f"link_root = Path({str(fail_support)!r})"),
    ("batch = 64", "batch = 2"),
])
commands = {"failures": run(fail, actors, {"MECH_ERROR_REPORT": str(out / "checked-failures.json")})}
failed = json.loads((out / "checked-failures.json").read_text())["cases"]
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
policy = json.loads((packet / "qualification-policy-v1.json").read_text())
assert set(policy["checked_failures"]) == set(failed)
for name, row in failed.items():
    want = policy["checked_failures"][name]
    assert (row["outcome"]["status"], row["outcome"]["origin"]) == (want["status"], want["origin"])

stack_support = out / "stack-support"
stack_support.mkdir()
shutil.copyfile(q / "pinned-host-link/pinned_allocator.rs", stack_support / "pinned_allocator.rs")
verifier = copy_driver("verify-worker.py", [
    ("fixture_path = root/'fixtures.json'", f"fixture_path = Path({str(q / 'fixtures.json')!r})"),
])
stack = copy_driver("check-layout-stack.py", [
    ("q=Path(__file__).resolve().parent", f"q=Path({str(q)!r})"),
    ("c_object=q/'controller-stack-probe.o'", f"c_object=Path({str(stack_support / 'stack.o')!r})"),
    ("instrumented=original.with_name('controller-stack-worker.rs')",
     f"instrumented=Path({str(stack_support / 'controller-stack-worker.rs')!r})"),
    ("prefix='layout-stack-'+label.replace('/', '-')+suffix", "prefix='layout-stack-changed-'+p.name+suffix"),
    ("q/'verify-worker.py'", f"Path({str(verifier)!r})"),
    ("q/(prefix+'-smokes.stderr')", f"Path({str(out)!r})/(prefix+'-smokes.stderr')"),
])
stack_report = out / "stack.json"
commands["stack"] = run(stack, actors, {"MECH_STACK_REPORT": str(stack_report)})
depths = json.loads(stack_report.read_text())
peaks = {path.name: max(v for sample in depths[str(path)].values() for v in sample.values()) for path in actors}
receipt = {"commands": commands, "peak_stack_bytes": peaks, "matches_success_observations": len(record[0]),
           "checked_failure_cases": list(failed)}
write(out / "result.json", receipt)
print(json.dumps(receipt, indent=2), flush=True)
