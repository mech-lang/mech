"""Sample the verified, prebuilt Mech frontend outside the timing campaign."""

from hashlib import file_digest
import json
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
frontend = q / "dead-spill-stores/native_compact_frontend"
source = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec"


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


assert sha(frontend) == "7ad7aa9b422266367568f06cbe4ac1f52346e78b146bba0f23139f8a978619eb"
assert sha(source) == "653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299"
run = study / "sample"
run.mkdir()
with (run / "warmup.log").open("wb") as log:
    subprocess.run([frontend, source, run / "warmup", "frontend"], stdout=log, stderr=subprocess.STDOUT, check=True)
with (run / "frontend.log").open("wb") as log, (run / "sample.log").open("wb") as sample_log:
    worker = subprocess.Popen([frontend, source, run / "frontend", "frontend"], stdout=log, stderr=subprocess.STDOUT)
    sample = ["/usr/bin/sample", str(worker.pid), "4", "1", "-file", str(run / "sample.txt")]
    subprocess.run(sample, check=True, stdout=sample_log, stderr=sample_log)
    assert worker.wait(timeout=30) == 0
bytecode = sha(run / "frontend/program.mecb")
assert bytecode == "c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3"
(run / "receipt.json").write_text(json.dumps({"frontend_sha256": sha(frontend), "source_sha256": sha(source),
    "bytecode_sha256": bytecode, "sample_command": sample}, indent=2) + "\n")
