"""Sample native emission of the pinned Cranelift encoder."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
compiler = q / "host-opt-levels/opt3-cargo/release/emit_compact_size"
reference = q / "stale-cache-accounting/pinned-smaller"
fixtures = q / "parser-queue-snapshot-20261117/identity/frontend-combined/program.mecb"
expected = json.loads((reference / "receipt.json").read_text())
runtime = json.loads((reference / "runtime.json").read_text())
flags = runtime["flags"] | {"MECH_COMPACT_INLINE_SINGLE_CALLS": "preselected"}


def sha(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")} | flags | {"PATH":"/usr/bin:/bin"}
assert not any("LLVM" in key for key in flags)
assert sha(fixtures) == runtime["image_sha256"]
out = study / "sample"
out.mkdir()
for role in ("warmup", "profile"):
    stage = out / role
    stage.mkdir()
    command = [compiler, fixtures, stage]
    with (stage / "native.log").open("wb") as log:
        with subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT) as compiler_process:
            if role == "profile":
                sampler = ["/usr/bin/sample", str(compiler_process.pid), "3", "1", "-file", str(out / "sample.txt")]
                with (out / "sample.log").open("wb") as output:
                    subprocess.run(sampler, stdout=output, stderr=output, check=True)
            assert compiler_process.wait(timeout=20) == 0
    assert sha(stage / "program.o") == expected["object_sha256"]
    assert sha(stage / "metadata.json") == expected["metadata_sha256"]
(out / "receipt.json").write_text(json.dumps({
    "emitter_sha256":sha(compiler),"program_sha256":sha(fixtures),"flags":flags,
    "command":list(map(str, command)),"sample":sampler,"object_sha256":expected["object_sha256"],
    "metadata_sha256":expected["metadata_sha256"],
},indent=2)+"\n")
