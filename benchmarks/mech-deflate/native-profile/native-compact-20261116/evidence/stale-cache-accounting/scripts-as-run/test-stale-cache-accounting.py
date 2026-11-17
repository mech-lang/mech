"""Verify syntax tests using the same parser source and dependency build."""

import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile

q = Path(__file__).resolve().parent
target = q / "stale-cache-accounting"
checkout = q / "native-review-pr/checkout"
revision = "bef604b4ea4facb27263c034d68f4341277600ca"
inputs = ("docs/design/grammar-audit", "src/syntax/tests/fixtures", "tests/fixtures/syntax-source-boundary")
archive = subprocess.check_output(["git", "archive", revision, *inputs], cwd=q.parent)
with tarfile.open(fileobj=io.BytesIO(archive)) as files:
    for asset in files:
        if not asset.isfile():
            continue
        content = files.extractfile(asset).read()
        destination = checkout / asset.name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            assert destination.read_bytes() == content, asset.name
        else:
            destination.write_bytes(content)

build = json.loads((target / "parser-timing.json").read_text())["build"]["command"]
at = build.index("--crate-type")
build[at:at + 2] = ["--test"]
build[-1] = str(target / "syntax-tests")
deps = q / "parser-test-dependencies/target/release/deps"
proptest, = deps.glob("libproptest-*.rlib")
build += ["-L", f"dependency={deps}", "--extern", f"proptest={proptest}"]
env = os.environ | {"CARGO_MANIFEST_DIR": str(checkout / "src/syntax"), "CARGO_PKG_VERSION": "0.3.5"}
run = [str(target / "syntax-tests")]
commands = [(run, "tests-complete.log")]
if sys.argv[1:] != ["--run-only"]:
    commands.insert(0, (build, "test-build-complete.log"))
for command, log in commands:
    with (target / log).open("wb") as out:
        subprocess.run(command, env=env, stdout=out, stderr=subprocess.STDOUT, check=True)
summary = next(line for line in (target / "tests-complete.log").read_text().splitlines() if line.startswith("test result:"))
assert "533 passed; 0 failed" in summary, summary
(target / "tests.json").write_text(json.dumps({"build": build, "run": run, "result": summary}, indent=2) + "\n")
print(summary)
