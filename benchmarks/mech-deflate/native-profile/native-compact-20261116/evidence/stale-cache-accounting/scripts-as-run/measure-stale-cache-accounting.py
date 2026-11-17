"""Build the one-shot cache change against the pinned parser dependencies."""

from hashlib import file_digest
import json
import os
from pathlib import Path
from statistics import median
import subprocess

q = Path(__file__).resolve().parent
checkout = q / "native-review-pr/checkout"
baseline = q / "equivalent-vector-phases/parser-stages"
output = q / "stale-cache-accounting"
output.mkdir()
prior = json.loads((baseline / "report.json").read_text())
source = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec"
tree = checkout / "src/syntax/src"
environment = os.environ | {"CARGO_MANIFEST_DIR": str(tree.parent), "CARGO_PKG_VERSION": "0.3.5"}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def run(argv, name):
    with (output / f"{name}.log").open("wb") as log:
        with subprocess.Popen(argv, env=environment, stdout=log, stderr=subprocess.STDOUT) as proc:
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            if proc.returncode:
                raise subprocess.CalledProcessError(proc.returncode, argv)
    return {"command": list(map(str, argv)), "peak_rss_bytes": usage.ru_maxrss}


library = output / "libmech_syntax_stale_accounting.rlib"
build = [str(tree / "lib.rs") if arg.endswith("/syntax/lib.rs")
         else str(library) if arg.endswith("libmech_syntax_profile.rlib")
         else "metadata=parser-stale-cache-accounting" if arg == "metadata=parser-stages"
         else arg for arg in prior["build_command"]]
compiled = run(build, "library")
drivers = {"control": baseline / "control", "accounting": output / "parser"}
link = [f"mech_syntax={library}" if arg.startswith("mech_syntax=")
        else str(drivers["accounting"]) if arg == str(baseline / "instrumented")
        else arg for arg in prior["link_commands"]["instrumented"]]
linked = run(link, "link")
trials = []


def trial(actor, tag):
    dest = output / (tag + ".json")
    receipt = run([drivers[actor], source, dest], tag)
    value = json.loads(dest.read_text())
    for field in ("tree_sha256", "source_sha256", "parser_steps", "events"):
        assert value[field] == prior["trials"][0][field], (tag, field)
    print(tag, value["parse_seconds"], receipt["peak_rss_bytes"], flush=True)
    return {"actor": actor, **value, **receipt}


warmups = [trial(actor, "warmup-" + actor) for actor in drivers]
orders = [("control", "accounting"), ("accounting", "control")] * 3
for order, pair in enumerate(orders, 1):
    trials.extend(trial(actor, f"order-{order}-{actor}") for actor in pair)
report = {
    "scope": "Same Apple M1 parser-only input and fixed opt-level-3 dependencies and driver.",
    "build": compiled, "link": linked, "driver_sha256": {actor: sha(path) for actor, path in drivers.items()},
    "warmups": warmups, "orders": orders, "trials": trials,
    "medians": {actor: {key: median(row[key] for row in trials if row["actor"] == actor)
                        for key in ("parse_seconds", "peak_rss_bytes")} for actor in drivers},
}
(output / "parser-timing.json").write_text(json.dumps(report, indent=2) + "\n")
print(report["medians"], flush=True)

deps = q / "parser-test-dependencies/target/release/deps"
proptest, = deps.glob("libproptest-*.rlib")
test = list(build)
at = test.index("--crate-type")
test[at:at + 2] = ["--test"]
test[-1] = str(output / "syntax-tests")
test += ["-L", f"dependency={deps}", "--extern", f"proptest={proptest}"]
results = {"build": run(test, "test-build"), "run": run([output / "syntax-tests"], "tests")}
summary = next(line for line in (output / "tests.log").read_text().splitlines() if line.startswith("test result:"))
assert "533 passed; 0 failed" in summary, summary
results["result"] = summary
(output / "tests.json").write_text(json.dumps(results, indent=2) + "\n")
print(summary, flush=True)
