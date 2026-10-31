from pathlib import Path
import json, os, subprocess, sys, time
r = Path(sys.argv[1])
command = json.loads((r / "build-command.json").read_text())
environment = {**os.environ, **json.loads((r / "build-environment.json").read_text())}
binary = command[command.index("-o") + 1]
steps = [("build", command), ("witness", [binary, "parent_child_design_witness::parent_child_huffman_current_contract", "--exact", "--nocapture", "--test-threads=1"])]
if "--reuse-build" in sys.argv[2:]:
    steps = steps[1:]
for label, args in steps:
    start = time.monotonic()
    result = subprocess.run(args, capture_output=True, text=True, env=environment)
    (r / (label + ".stdout")).write_text(result.stdout)
    (r / (label + ".stderr")).write_text(result.stderr)
    (r / (label + ".json")).write_text(json.dumps({"command":args,"exit":result.returncode,"seconds":time.monotonic()-start,"performance_claim":False},indent=2)+"\n")
    print(label, result.returncode, result.stdout[-3000:] + result.stderr[-3000:], flush=True)
    result.check_returncode()
