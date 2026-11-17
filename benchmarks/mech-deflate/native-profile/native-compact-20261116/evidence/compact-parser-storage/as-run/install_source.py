"""Materialize the measured parser policies in the isolated PR source checkout."""

from pathlib import Path
import shutil
import subprocess

from study_tools import checkout, q, study, sha, write

branch = "refs/heads/codex/native-compact-source-handoff"
parent = "e7e54b488641a4665bdb5221f124ff028ddca5ca"
assert subprocess.check_output(["git", "rev-parse", branch], text=True, cwd=q.parent).strip() == parent
tools_dir = q / "host-opt-levels/opt3-cargo/release"
pins = study / "fixed-tools"
pins.mkdir()
exe_paths = {"emit_compact_size": tools_dir / "emit_compact_size",
             "native_compact_frontend": q / "dead-spill-stores/native_compact_frontend"}
for destination, source in exe_paths.items():
    shutil.copy2(source, pins / destination)
    assert sha(pins / destination) == sha(source)
assert sha(pins / "native_compact_frontend") == sha(tools_dir / "native_compact_frontend")
editor_start = Path("src/syntax/src")
files = ("document/parser/mod.rs", "document/parser/tree_cache.rs",
         "document/parser/canonical/literals/mod.rs", "document/parser/canonical/literals/continuation.rs")
startup, current = {}, {}
for file in files:
    local = checkout / editor_start / file
    original = subprocess.check_output(["git", "show", f"{parent}:{editor_start / file}"], cwd=q.parent)
    assert local.read_bytes() == original, file
    startup[file] = sha(local)
    shutil.copyfile(study / "combined/source" / file, local)
    current[file] = sha(local)
    assert startup[file] != current[file]
write(study / "source-receipt.json", {"parent":parent,"before":startup,"combined":current,
                                      "fixed_compilers":{name:sha(pins/name) for name in exe_paths}})
