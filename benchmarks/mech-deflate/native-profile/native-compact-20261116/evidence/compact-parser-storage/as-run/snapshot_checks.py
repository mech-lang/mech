"""Compare syntax and native identities with the previous compiler."""

import json
from pathlib import Path
import subprocess
import sys

from study_tools import checkout, link, packet, probe, q, run, sha, source, study, toolchain, write

out = study / "identity"
out.mkdir()
target = q / "host-opt-levels/opt3-cargo/release"
parent = json.loads((study / "source-receipt.json").read_text())["parent"]
recipe = json.loads((packet / "recipe.json").read_text())

rows = [json.loads(line) for line in (study / "integrated-cargo.jsonl").read_text().splitlines()
        if line.startswith('{"reason":"compiler-artifact"')]
syntax, = (row for row in rows if row["target"]["name"] == "mech_syntax")
combined, = (Path(path) for path in syntax["filenames"] if path.endswith(".rlib"))
exes = {role: path for role, path in (
    ("control", study / "fixed-tools/native_compact_frontend"),
    ("combined", target / "native_compact_frontend"),
)}
libs = {"control": study / "current/libmech_syntax.rlib", "combined": combined}
fixture_root = checkout / "tests/fixtures/syntax-source-boundary"
fixtures = sorted(fixture_root.glob("*.mec"))
extra = out / "unicode-and-numeric.mec"
extra.write_text("A 🇺🇳 e\u0301 example.\n\n```mech\nα := 0xF + 1.2e3 + -42\nβ := :👩\u200d💻\n```\n")
fixtures.append(extra)
assert len(fixtures) == 10
syntax_results = {}
for name, library in libs.items():
    executable = out / f"snapshot-{name}"
    original = toolchain["link_commands"]["instrumented"]
    command = [f"mech_syntax={library}" if arg.startswith("mech_syntax=")
               else str(executable) if arg.endswith("/parser-stages/instrumented")
               else str(study / "snapshot-probe.rs") if arg.endswith("/parser-journal-batch-study/probe.rs")
               else arg for arg in original]
    assert str(executable) in command and str(study / "snapshot-probe.rs") in command
    linker = run([*command, "--crate-name", "snapshot_probe"], out / f"snapshot-link-{name}.log")
    observation = out / f"snapshot-{name}.txt"
    parse = run([executable, *fixtures], observation)
    assert len(observation.read_text().splitlines()) == len(fixtures) * 6
    syntax_results[name] = {"syntax_rlib_sha256":sha(library),"linker":linker,"command":parse,"digest":sha(observation)}
assert (out / "snapshot-control.txt").read_bytes() == (out / "snapshot-combined.txt").read_bytes()

whole_program = out / "large-tree"
whole_program.mkdir()
large_probe, linker = link(whole_program, combined)
full_tree = probe(large_probe, whole_program / "tree.json")
frontends = {}
for role, frontend in exes.items():
    folder = out / f"frontend-{role}"
    cmd = run([frontend, source, folder, "frontend"], out / f"frontend-{role}.log")
    image = sha(folder / "program.mecb")
    assert image == recipe["bytecode_sha256"]
    frontends[role] = {"sha256":sha(frontend),"command":cmd,"bytecode_sha256":image}
assert frontends["control"]["sha256"] != frontends["combined"]["sha256"]
image = out / "frontend-combined/program.mecb"
checks = {}
for name, pinned in {
    "retained": q / "stale-cache-accounting/pinned-retained",
    "smaller": q / "stale-cache-accounting/pinned-smaller",
    "aligned": q / "aligned-register-spills-native/final-candidate",
}.items():
    dest = out / name
    command = [sys.executable, "-B", q / "probe-literal-reuse.py", "--native-reference", pinned,
               "--emitter", target / "emit_compact_size", dest, image,
               "MECH_COMPACT_INLINE_SINGLE_CALLS=preselected"]
    actual = run(command, out / f"{name}.log")
    artifacts = {}
    for file in ("program.o", "metadata.json", "mech-worker"):
        artifacts[file] = sha(dest / file)
        assert artifacts[file] == sha(pinned / file), (name, file)
    checks[name] = {"command":actual,"artifacts":artifacts,"pinned_reference":str(pinned),
                    "linked_smokes":json.loads((dest / "receipt.json").read_text())["verified_calls"]}
record = {"parent":parent,"input_snapshots":len(fixtures)*6,
          "source_hashes":{file.name:sha(file) for file in fixtures},
          "snapshot_comparison":syntax_results,"large_tree":full_tree,"large_tree_link":linker,
          "frontends":frontends,"native":checks,"emitter_sha256":sha(target / "emit_compact_size")}
write(out / "receipt.json",record)
print(len(fixtures)*6, "syntax snapshots matched including diagnostic, Unicode, piece and finite limits")
print(json.dumps(checks,indent=2))
