"""Check match-cache offset encodings against one compiler and one link."""

from hashlib import file_digest
from importlib import import_module
import json
import os
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
macho = import_module("reachability-census")
plan = json.loads((study / "inputs.json").read_text())
compiler = study / "compile_profile"
reference = q / "mech-unmatched-runs/native/generic-control"
runtime = json.loads((reference / "runtime.json").read_text())
assert runtime["flags"] == plan["common_flags"]
env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def run(args, log, environment):
    command = list(map(str, args))
    with log.open("wb") as stream:
        process = subprocess.run(command, cwd=q, env=environment, stdout=stream, stderr=subprocess.STDOUT)
    return {"command": command, "exit": process.returncode}


assert digest(compiler) == plan["compiler_sha256"]
roles = {"control": "control.mec", "stored-table-slot": "program.mec", "stored-arithmetic-slot": "program-arithmetic.mec"}
result = {}
for role, name in roles.items():
    out = study / role
    out.mkdir()
    source = study / name
    commands = {"compile": run([compiler, source, out, "compact"], out / "compile.log", env | runtime["flags"])}
    row = {"source_sha256": digest(source), "commands": commands}
    result[role] = row
    if commands["compile"]["exit"] != 0:
        print(role, "compile did not finish", commands["compile"]["exit"], flush=True)
        continue
    link = runtime["link"]
    for filename in ("program.o", "mech-worker"):
        assert sum(arg.count(str(reference / filename)) for arg in link) == 1
        link = [arg.replace(str(reference / filename), str(out / filename)) for arg in link]
    commands["link"] = run(link, out / "link.log", env)
    if commands["link"]["exit"] != 0:
        print(role, "link did not finish", commands["link"]["exit"], flush=True)
        continue
    artifacts = {file: digest(out / file) for file in ("program.mecb", "program.o", "metadata.json", "mech-worker")}
    write(out / "runtime.json", runtime | {"link": link, "image_sha256": artifacts["program.mecb"]})
    label = f"offset-slot-source-{role}"
    commands["checks"] = run([sys.executable, "-B", q / "verify-worker.py", out / "mech-worker",
                              out / "metadata.json", label], out / "smokes.log", env)
    smoke = q / f"{label}-qualification.json"
    receipt = json.loads(smoke.read_text()) if smoke.exists() else None
    if receipt:
        assert receipt["complete_smokes"] == 162
    if role == "control":
        for file in ("program.o", "metadata.json", "mech-worker"):
            assert artifacts[file] == digest(reference / file)
    metadata = json.loads((out / "metadata.json").read_text())
    row.update(artifacts=artifacts, smokes=receipt,
               encoder_text_bytes=macho.load(out / "program.o")[1][macho.TEXT][2],
               root_code_bytes=metadata["code_bytes"], root_frame_bytes=metadata["stack_bytes"])
    print(role, row["encoder_text_bytes"], "exact checks", receipt["complete_smokes"] if receipt else 0, flush=True)
write(study / "screen.json", result)
