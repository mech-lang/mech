"""Run the complete syntax unit suite against the combined parser source."""

import json
import subprocess

from study_tools import checkout, env, q, study, sha, write

baseline = json.loads((q / "stale-cache-accounting/tests.json").read_text())
workspace = checkout / "tests/fixtures/syntax-source-boundary"
assert (workspace / "document.mec").is_file() and (workspace / "malformed.mec").is_file()
out = study / "syntax-tests"
out.mkdir()
test = out / "tests"
expected = q / "stale-cache-accounting/syntax-tests"
command = [str(test) if option == str(expected)
           else "metadata=parser-queue-snapshot-tests" if option == "metadata=parser-stale-cache-accounting"
           else option for option in baseline["build"]]
run = [str(test)]
for name, argv in (("build", command), ("run", run)):
    with (out / f"{name}.log").open("wb") as log:
        subprocess.run(argv, cwd=checkout, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
line, = [row for row in (out / "run.log").read_text().splitlines() if row.startswith("test result:")]
assert "533 passed; 0 failed" in line
write(out / "receipt.json", {"compile":command,"run":run,"binary_sha256":sha(test),"result":line})
print(line)
