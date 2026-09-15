#!/usr/bin/env python3
"""Reconstruct E7 solely from E6 and the declared exact frozen byte spans."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd()
manifest = json.loads(Path(__file__).with_name("e7-symbols.json").read_text())
state = {}


def git_file(revision, path):
    return subprocess.check_output(["git", "show", f"{revision}:{path}"], cwd=root)


for entry in manifest["copied"]:
    path = entry["path"]
    if path not in state:
        state[path] = git_file(manifest["base"], path)
    frozen = git_file(manifest["frozen"], path)
    start = entry["frozen_start_byte"]
    copied = frozen[start:start + entry["bytes"]]
    assert hashlib.sha256(copied).hexdigest() == entry["sha256"], entry["symbol"]
    at = entry["at_byte"]
    end = at + entry["remove_bytes"]
    removed = state[path][at:end]
    assert hashlib.sha256(removed).hexdigest() == entry["remove_sha256"], entry["symbol"]
    state[path] = state[path][:at] + copied + state[path][end:]

for path, expected in state.items():
    assert (root / path).read_bytes() == expected, path
changed = subprocess.check_output(
    ["git", "diff", "--name-only", manifest["base"]], cwd=root, text=True
).splitlines()
assert set(changed) == set(state), "unexpected tracked file delta"
print(f"Verified {len(manifest['copied'])} frozen spans across {len(state)} files; no other tracked deltas.")
