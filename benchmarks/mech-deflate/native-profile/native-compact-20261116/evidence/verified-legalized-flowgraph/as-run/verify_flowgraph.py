"""Link the same optimization and invalidation checks against both Cranelift libraries."""

from pathlib import Path
from hashlib import file_digest
import json
import shutil
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
deps = q / "host-opt-levels/opt3-cargo/release/deps"
probe = study / "flowgraph_probe.rs"


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def library(log):
    artifacts = []
    for line in log.read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message["reason"] == "compiler-artifact" and message["target"]["name"] == "cranelift_codegen":
            artifacts.extend(Path(name) for name in message["filenames"] if name.endswith(".rlib"))
    reference, = artifacts
    return reference


mode, = sys.argv[1:]
if mode == "prepare":
    original = library(q / "native-dominance-search/integrated-cargo.jsonl")
    assert original.parent == deps
    shutil.copyfile(original, study / "control-cranelift-codegen.rlib")
    # rustc locates libraries by their lib-prefixed basename.
    (study / "control-cranelift-codegen.rlib").rename(study / "libcontrol_cranelift_codegen.rlib")
else:
    assert mode == "verify"
    results = {}
    output = []
    for actor, module in (("control", study / "libcontrol_cranelift_codegen.rlib"),
                          ("candidate", library(study / "integrated-cargo.jsonl"))):
        target = study / f"flowgraph-{actor}"
        command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "-D", "warnings",
                   "-L", f"dependency={deps}", "--extern", f"cranelift_codegen={module}", str(probe),
                   "--crate-name", f"flowgraph_{actor}", "-o", str(target)]
        with (study / f"flowgraph-{actor}-build.log").open("wb") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        data = subprocess.check_output([str(target)], stderr=subprocess.STDOUT)
        output.append(data)
        (study / f"flowgraph-{actor}.txt").write_bytes(data)
        results[actor] = {"command": command, "library_sha256": sha(module), "binary_sha256": sha(target)}
    assert output[0] == output[1], output
    (study / "flowgraph-verification.json").write_text(json.dumps(results | {"matching_output": True}, indent=2) + "\n")
    print(output[0].decode(), end="")
