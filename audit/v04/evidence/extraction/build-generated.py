#!/usr/bin/env python3
"""Apply explicit local SDK overrides to a copy of a registry-generated product."""
from pathlib import Path
import difflib, json, shutil, sys

base = Path(sys.argv[1]).resolve()
original = base / 'generated-registry-original'
project = base / 'generated-registry-development'
shutil.copytree(original, project)
manifest = project / 'Cargo.toml'
before = manifest.read_text()
assert '[patch.crates-io]' not in before
patch = '\n[patch.crates-io]\n'
for name in ['core', 'engine', 'runtime', 'syntax', 'bytecode']:
    patch += f'mech-{name} = {{ path = "../mech/src/{name}" }}\n'
manifest.write_text(before + patch)
out = Path(__file__).resolve().parent
(out / 'generated-development-overrides.patch').write_text(''.join(difflib.unified_diff(before.splitlines(True), manifest.read_text().splitlines(True), fromfile='registry/Cargo.toml', tofile='development/Cargo.toml')))
print(json.dumps({'registry_manifest_unchanged': (original/'Cargo.toml').read_text() == before, 'development_project': str(project)}))
