"""Verify and compare the one-shot journal against the existing optimized parser."""

from collections import Counter
import filecmp
from hashlib import file_digest
import json
import os
from pathlib import Path
import platform
import shutil
from statistics import median
import subprocess
import sys
import time

study = Path(__file__).resolve().parent
qualification = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
previous = qualification / "contiguous-syntax-study"
checkout = qualification / "native-review-pr/checkout"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
source = packet / "program.mec"
control = previous / "candidate"
candidate = study / "candidate"
crate = checkout / "src/syntax/src"
recorded = json.loads((control / "build-receipt.json").read_text())
expected = recorded["exact_tree_and_work"]
env = os.environ | {
    "CARGO_MANIFEST_DIR": str(crate.parent),
    "CARGO_PKG_VERSION": "0.3.5",
}
actors = {"before": control, "candidate": candidate}
identities = ("tree_sha256", "source_sha256", "source_bytes", "parser_steps", "events")


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def run(args, log):
    command = list(map(str, args))
    with log.open("wb") as stream:
        subprocess.run(
            command, cwd=checkout, env=env, stdout=stream,
            stderr=subprocess.STDOUT, check=True,
        )
    return command


def check_control():
    observed = {path.relative_to(crate) for path in crate.rglob("*") if path.is_file()}
    old_crate = control / "source"
    original = {path.relative_to(old_crate) for path in old_crate.rglob("*") if path.is_file()}
    assert observed == original
    for path in observed:
        assert filecmp.cmp(crate / path, old_crate / path, shallow=False), path
    for artifact, key in (("libmech_syntax.rlib", "library_sha256"), ("parser", "probe_sha256")):
        assert sha(control / artifact) == recorded[key]


def check(row):
    for name in identities:
        assert row[name] == expected[name], (name, row[name], expected[name])


def build():
    check_control()
    code = candidate / "source"
    shutil.copytree(crate, code, dirs_exist_ok=True)
    for path in (candidate / "src/syntax/src").rglob("*.rs"):
        active = code / path.relative_to(candidate / "src/syntax/src")
        active.parent.mkdir(exist_ok=True)
        shutil.copyfile(path, active)
    library = candidate / "libmech_syntax.rlib"
    rustc = [
        str(code / "lib.rs") if opt == str(control / "source/lib.rs")
        else str(library) if opt == str(control / "libmech_syntax.rlib")
        else "metadata=batch-journal-candidate" if opt == "metadata=queue-study-candidate"
        else opt
        for opt in recorded["build"]
    ]
    compiler = run(rustc, candidate / "build.log")
    link = [
        f"mech_syntax={library}" if opt.startswith("mech_syntax=")
        else str(candidate / "parser") if opt == str(control / "parser")
        else opt
        for opt in recorded["link"]
    ]
    linker = run(link, candidate / "link.log")
    correctness = candidate / "correctness.json"
    probe = run([candidate / "parser", source, correctness], candidate / "correctness.log")
    result = json.loads(correctness.read_text())
    check(result)
    write(study / "build.json", {
        "source_patch_sha256": sha(study / "batch-journal.patch"),
        "source_revision": "b72b7419b39b22765fe1eed6b5c636e63964b905",
        "build": compiler,
        "link": linker,
        "probe": probe,
        "library_sha256": sha(library),
        "probe_sha256": sha(candidate / "parser"),
        "exact_tree_and_work": {name: result[name] for name in identities},
    })
    print("Standalone parser matched the recorded tree, events, and steps", flush=True)


def validate():
    out = study / "validation"
    out.mkdir(exist_ok=True)
    earlier = json.loads((previous / "validation/receipt.json").read_text())
    probe = previous / "snapshot-probe.rs"
    parser_probe = qualification / "parser-journal-batch-study/probe.rs"
    bins = {}
    for role, artifact in actors.items():
        library = artifact / "libmech_syntax.rlib"
        binary = out / f"{role}-probe"
        link = [
            f"mech_syntax={library}" if opt.startswith("mech_syntax=")
            else str(binary) if opt == str(control / "parser")
            else str(probe) if opt == str(parser_probe)
            else opt
            for opt in recorded["link"]
        ]
        bins[role] = (binary, run([*link, "--crate-name", "snapshot_probe"], out / f"{role}-link.log"))
    fixture_dir = checkout / "tests/fixtures/syntax-source-boundary"
    fixtures = sorted(fixture_dir.glob("*.mec"))
    fixtures.append(previous / "validation/unicode-and-numeric.mec")
    assert {item.name: sha(item) for item in fixtures} == earlier["fixture_sha256"]
    checks = {}
    for role, (binary, link) in bins.items():
        snapshot = out / f"{role}.txt"
        command = run([binary, *fixtures], snapshot)
        assert 6 * len(fixtures) == len(snapshot.read_text().splitlines())
        checks[role] = {"link": link, "command": command, "snapshot_sha256": sha(snapshot)}
    assert checks["before"]["snapshot_sha256"] == checks["candidate"]["snapshot_sha256"]
    print(len(fixtures) * 6, "matching finite-limit and segmented-source snapshots", flush=True)

    test = out / "syntax-tests"
    support = study / "core/tests/support"
    support.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(checkout / "src/core/tests/support/allocation_probe.rs",
                    support / "allocation_probe.rs")
    shutil.copytree(crate.parent / "tests/canonical_rules",
                    candidate / "tests/canonical_rules", dirs_exist_ok=True)
    command = [
        str(test) if opt == str(previous / "validation/syntax-tests")
        else str(candidate / "source/lib.rs") if opt == str(crate / "lib.rs")
        else "metadata=batch-journal-tests" if opt == "metadata=parser-cache-frame-tests"
        else opt
        for opt in earlier["tests"]["compile"]
    ]
    assert str(test) in command and str(candidate / "source/lib.rs") in command
    build = run(command, out / "unit-build.log")
    execute = run([test], out / "unit-run.log")
    result, = [
        line for line in (out / "unit-run.log").read_text().splitlines()
        if line.startswith("test result:")
    ]
    assert "535 passed; 0 failed" in result
    write(out / "receipt.json", {
        "tests": {"build": build, "run": execute, "result": result},
        "fixture_sha256": {item.name: sha(item) for item in fixtures},
        "snapshots": checks,
        "matched_snapshots": 6 * len(fixtures),
    })
    print(result, flush=True)


def measure():
    out = study / "timings"
    out.mkdir()
    names = tuple(actors)
    orders = [names, tuple(reversed(names))] * 4
    assert all(Counter(column) == Counter(dict.fromkeys(names, 4)) for column in zip(*orders))
    write(out / "plan.json", {
        "scope": "Standalone canonical parser with fixed compiler, allocator and input",
        "host": {"platform": platform.platform(), "architecture": platform.machine()},
        "baseline_syntax_revision": "b72b7419b39b22765fe1eed6b5c636e63964b905",
        "source_file": str(source), "source_sha256": sha(source),
        "probe_sha256": {role: sha(path / "parser") for role, path in actors.items()},
        "paired_orders": orders,
        "warmups_excluded": 1,
    })

    def trial(actor, label):
        result = out / f"{label}-{actor}.json"
        command = list(map(str, [actors[actor] / "parser", source, result]))
        with result.with_suffix(".log").open("wb") as log:
            start = time.perf_counter()
            with subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT) as worker:
                _, status, usage = os.wait4(worker.pid, 0)
                worker.returncode = os.waitstatus_to_exitcode(status)
                assert worker.returncode == 0, (actor, label)
            elapsed = time.perf_counter() - start
        row = json.loads(result.read_text())
        check(row)
        print(actor, label, row["parse_seconds"], usage.ru_maxrss, flush=True)
        return {
            "actor": actor, "command": command, "wall_time_seconds": elapsed,
            "peak_rss_bytes": usage.ru_maxrss, **row,
        }

    warmups = [trial(actor, "warmup") for actor in names]
    trials = [{actor: trial(actor, f"order-{n}") for actor in order}
              for n, order in enumerate(orders, 1)]
    report = {
        "warmups": warmups, "orders": orders, "trials": trials,
        "median_parse_seconds": {
            actor: median(row[actor]["parse_seconds"] for row in trials) for actor in names
        },
        "median_peak_rss_bytes": {
            actor: median(row[actor]["peak_rss_bytes"] for row in trials) for actor in names
        },
        "paired_parse_ratio": [
            row["candidate"]["parse_seconds"] / row["before"]["parse_seconds"] for row in trials
        ],
    }
    report["paired_parse_ratio_median"] = median(report["paired_parse_ratio"])
    report["candidate_faster_pairs"] = sum(ratio < 1 for ratio in report["paired_parse_ratio"])
    write(out / "report.json", report)
    print(json.dumps({key: report[key] for key in (
        "median_parse_seconds", "median_peak_rss_bytes", "paired_parse_ratio_median",
        "candidate_faster_pairs"
    )}, indent=2), flush=True)


if __name__ == "__main__":
    {"build": build, "validate": validate, "measure": measure}[sys.argv[1]]()
