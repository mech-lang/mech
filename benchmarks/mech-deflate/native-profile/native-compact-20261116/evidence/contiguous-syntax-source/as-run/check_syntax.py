"""Verify single-piece source reads across parser unit tests and syntax snapshots."""

import json
import os

from study_tools import checkout, q, run, sha, study, toolchain, write

out = study / "validation"
out.mkdir()
os.chdir(checkout)
source = checkout / "src/syntax/src/lib.rs"
for name in ("document/retained_sequence.rs", "document/source.rs", "document/parser/grapheme_scan.rs"):
    assert (source.parent / name).read_bytes() == (study / "candidate/source" / name).read_bytes()
fixtures = sorted((checkout / "tests/fixtures/syntax-source-boundary").glob("*.mec"))
unicode = out / "unicode-and-numeric.mec"
unicode.write_text("A 🇺🇳 e\u0301 example.\n\n```mech\nα := 0xF + 1.2e3 + -42\nβ := :👩\u200d💻\n```\n")
fixtures.append(unicode)
baseline = json.loads((q / "stale-cache-accounting/tests.json").read_text())
test = out / "syntax-tests"
command = [str(test) if option == str(q / "stale-cache-accounting/syntax-tests")
           else "metadata=parser-cache-frame-tests" if option == "metadata=parser-stale-cache-accounting"
           else option for option in baseline["build"]]
assert str(test) in command and str(source) in command
test_build = run(command, out / "unit-build.log")
test_command = run([test], out / "unit-run.log")
line, = [line for line in (out / "unit-run.log").read_text().splitlines() if line.startswith("test result:")]
assert "534 passed; 0 failed" in line
checks = {}
probe = study / "snapshot-probe.rs"
for role in ("before", "candidate"):
    lib = study / role / "libmech_syntax.rlib"
    binary = out / f"{role}-probe"
    build = [f"mech_syntax={lib}" if arg.startswith("mech_syntax=")
             else str(binary) if arg.endswith("/parser-stages/instrumented")
             else str(probe) if arg.endswith("/parser-journal-batch-study/probe.rs")
             else arg for arg in toolchain["link_commands"]["instrumented"]]
    assert str(probe) in build and str(binary) in build
    link = run([*build, "--crate-name", "snapshot_probe"], out / f"{role}-link.log")
    snapshots = out / f"{role}.txt"
    command = run([binary, *fixtures], snapshots)
    assert len(snapshots.read_text().splitlines()) == 6 * len(fixtures)
    checks[role] = {"link": link, "command": command, "snapshot_sha256": sha(snapshots)}
assert (out / "before.txt").read_bytes() == (out / "candidate.txt").read_bytes()
write(out / "receipt.json", {
    "tests": {"compile": test_build, "run": test_command, "result": line},
    "fixture_sha256": {file.name: sha(file) for file in fixtures},
    "matched_snapshots": len(fixtures) * 6,
    "checks": checks,
})
print(line)
print(len(fixtures) * 6, "matching snapshots, including malformed Unicode and finite limits")
