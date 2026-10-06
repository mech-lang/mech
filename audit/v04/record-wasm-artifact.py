#!/usr/bin/env python3
"""Copy the completed shared Mech browser package and identify its source inputs."""
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
ROOT = Path(__file__).resolve().parents[2]
def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
site = ROOT / 'audit/v04/site'
package = ROOT / 'src/wasm/pkg'
for name in ('mech_wasm.js', 'mech_wasm_bg.wasm'):
    if not (package / name).is_file(): raise SystemExit(f'Missing completed package: {name}')
exports = ('WasmSyntaxStream', 'WasmSyntaxEditor', 'WasmDocument', 'WasmMixedComputeProject', 'WasmTypeInspector', 'TypePublicationSession', 'inspectMechTypes', 'I64PublicationSession', 'inventorySource')
glue = package.joinpath('mech_wasm.js').read_text()
for name in exports:
    if name not in glue: raise SystemExit(f'Missing expected export {name}')
shutil.copytree(package, site / 'pkg', dirs_exist_ok=True)
changed = command('git', 'diff', '--name-only', 'HEAD').splitlines()
changed += command('git', 'ls-files', '--others', '--exclude-standard', 'src', 'hosts', 'machines').splitlines()
modifications = {path: digest(ROOT / path) for path in sorted(set(changed)) if (ROOT / path).is_file() and not path.startswith('audit/')}
features = 'browser_project,browser_compute,u8,u64,u128,syntax_inspection,type_inspection,i64_publication'
record = {
    'source_commit': command('git', 'rev-parse', 'HEAD'),
    'source_description': 'Preserved integration/v0.4 baseline plus the explicitly hashed source modifications.',
    'source_modifications': modifications,
    'source_modifications_sha256': hashlib.sha256(json.dumps(modifications, sort_keys=True).encode()).hexdigest(),
    'lockfile_sha256': digest(ROOT / 'Cargo.lock'),
    'command': 'CARGO_TARGET_DIR=/private/tmp/mech-v04-target-streaming wasm-pack build src/wasm --target web --out-dir pkg --no-default-features --features ' + features,
    'features': features, 'target': 'wasm32-unknown-unknown', 'profile': 'release',
    'wasm_opt': False, 'expected_exports': list(exports),
    'artifacts': {f'pkg/{name}': {'sha256': digest(site / 'pkg' / name), 'bytes': (site / 'pkg' / name).stat().st_size} for name in ('mech_wasm.js', 'mech_wasm_bg.wasm')},
    'environment': {'system': platform.platform(), 'rustc': command('rustc', '-vV'), 'wasm_pack': command('wasm-pack', '--version')},
    'limits': ['Artifact source identity comprises the baseline commit and listed modifications.', 'Browser device and execution outcomes are recorded separately.'],
}
site.joinpath('artifact.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record['artifacts'], indent=2))
