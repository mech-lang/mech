#!/usr/bin/env python3
"""Re-home Mech-owned stdlib contract tests after preparing the isolated extraction."""
from pathlib import Path
import json, re, shutil, sys

base = Path(sys.argv[1]).resolve()
core = base / 'mech'
library = base / 'external/stdlib'
harness = core / 'tests/stdlib-integration'
assert not harness.exists()
missing = []
for path in (library / 'tests').rglob('*.rs'):
    for suffix in re.findall(r'"(/../../tests/[^"]+)"', path.read_text()):
        target = (library / suffix.lstrip('/')).resolve()
        missing.append({'source': str(path), 'include_target': str(target), 'exists': target.exists()})
(root := Path(__file__).resolve().parents[4]).joinpath('audit/v04/evidence/extraction/fixture-includes-before-rehome.json').write_text(json.dumps(missing, indent=2))
harness.mkdir(parents=True)
shutil.move(library / 'tests', harness / 'tests')
# The macOS test linker adjustment belongs to the integration test owner.
shutil.move(library / 'build.rs', harness / 'build.rs')
used_features = sorted({feature for path in (harness / 'tests').rglob('*.rs') for feature in re.findall(r'feature = "([^"]+)"', path.read_text())})
for feature in ['runtime', 'standard_runtime', 'standard_source', 'f64', 'math_add']:
    if feature not in used_features:
        used_features.append(feature)
manifest = '''[package]
name = "mech-stdlib-integration"
version = "0.0.0"
edition = "2024"
publish = false
[features]
default = []
'''
for feature in sorted(used_features):
    forwards = [f'mech-stdlib/{feature}']
    if feature in ['full_source', 'standard_source', 'compiler']:
        forwards.append('source')
    if feature in ['full_compiler', 'standard_compiler']:
        forwards.extend(['compiler', 'source'])
    if feature == 'full_compiler':
        forwards.extend(['full_source', 'full_runtime'])
    if feature == 'full_source':
        forwards.append('full_runtime')
    manifest += feature + ' = ' + json.dumps(forwards) + '\n'
manifest += '''[dependencies]
mech-stdlib = { path = "../../../external/stdlib", default-features = false }
mech-core = { path = "../../src/core", default-features = false }
mech-engine = { path = "../../src/engine", default-features = false }
mech-syntax = { path = "../../src/syntax", default-features = false }
nalgebra = "0.34.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
[workspace]
[patch.crates-io]
'''
for path in sorted((base / 'external/machines').iterdir()):
    if (path / 'Cargo.toml').exists():
        manifest += f'mech-{path.name} = {{ path = "../../../external/machines/{path.name}" }}\n'
(harness / 'Cargo.toml').write_text(manifest)
# Avoid keeping unused dev dependency declarations in the external library.
text = (library / 'Cargo.toml').read_text().replace('[package]\n', '[package]\nbuild = false\n', 1)
text = re.sub(r'\[dev-dependencies\]\n.*?(?=\n\[)', '', text, flags=re.S)
(library / 'Cargo.toml').write_text(text)
print(json.dumps({'harness': str(harness), 'manifest_lines': len(manifest.splitlines()), 'rust_test_lines_changed': 0, 'missing_fixture_includes_before': len(missing)}))
