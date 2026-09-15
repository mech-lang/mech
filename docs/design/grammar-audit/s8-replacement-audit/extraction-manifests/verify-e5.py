#!/usr/bin/env python3
"""Verify E5 contains only the declared frozen extraction and untouched base code."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd()
manifest = json.loads(Path(__file__).with_name("e5-symbols.json").read_text())


def git_file(revision, path):
    return subprocess.check_output(["git", "show", f"{revision}:{path}"], cwd=root)


copied = {}
for entry in manifest["copied"]:
    path = entry["path"]
    frozen = git_file(manifest["frozen"], path)
    start = entry["frozen_start_byte"]
    block = frozen[start:start + entry["bytes"]]
    assert hashlib.sha256(block).hexdigest() == entry["sha256"], entry["symbol"]
    assert block in (root / path).read_bytes(), entry["symbol"]
    copied.setdefault(path, []).append(block)

directory = "src/runtime/src/runtime/program/"
path = directory + "compiler.rs"
current = (root / path).read_bytes()
prefix, *additions = copied[path]
for block in additions:
    assert current.count(block) == 1
    current = current.replace(block, b"", 1)
anchor = b"#[derive(Clone, Copy)]\nenum RootOutputProjection"
base = git_file(manifest["base"], path)
assert current == prefix + base[base.index(anchor):], "existing compiler code changed"

for name in ("loading.rs", "value.rs"):
    path = directory + name
    assert (root / path).read_bytes() == git_file(manifest["frozen"], path)

path = directory + "query.rs"
base = git_file(manifest["base"], path)
start = base.index(b"    /// Return the resident output that represents the program's implicit")
end = base.index(b"\n    }", start) + len(b"\n    }\n")
assert (root / path).read_bytes() == base[:start] + copied[path][0] + base[end:]

path = directory + "tests.rs"
expected = git_file(manifest["base"], path)
for block in copied[path]:
    expected += b"\n" + block
assert (root / path).read_bytes() == expected, "base tests changed or new tests added"
print(f"Verified {len(manifest['copied'])} exact frozen blocks; existing compiler code and tests unchanged.")
