#!/usr/bin/env python3
"""Copy the completed shared Mech browser package and identify its source inputs."""
import hashlib
import gzip
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
exports = ('WasmSyntaxStream', 'WasmSyntaxEditor', 'WasmDocument', 'WasmMixedComputeProject', 'WasmTypeInspector', 'TypePublicationSession', 'inspectMechTypes', 'inspectMechDocument', 'I64PublicationSession', 'inventorySource')
glue = package.joinpath('mech_wasm.js').read_text()
for name in exports:
    if name not in glue: raise SystemExit(f'Missing expected export {name}')
shutil.copytree(package, site / 'pkg', dirs_exist_ok=True)
# The browser reconstructs and verifies the package from bounded gzip chunks.
wasm = package.joinpath('mech_wasm_bg.wasm').read_bytes()
wasm_digest = hashlib.sha256(wasm).hexdigest()
compressed = gzip.compress(wasm, compresslevel=9, mtime=0)
chunks = []
for index, offset in enumerate(range(0, len(compressed), 8 * 1024 * 1024)):
    data = compressed[offset:offset + 8 * 1024 * 1024]
    name = f'mech-wasm-{wasm_digest[:12]}-{index:02d}.gz.part'
    (site / 'pkg' / name).write_bytes(data)
    chunks.append({'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
for old in (site / 'pkg').glob('mech-wasm-*.gz.part'):
    if old.name not in {chunk['path'] for chunk in chunks}: old.unlink()
(site / 'pkg/transport.json').write_text(json.dumps({
    'encoding': 'gzip', 'uncompressed_bytes': len(wasm),
    'uncompressed_sha256': wasm_digest, 'compressed_bytes': len(compressed),
    'chunks': chunks,
}, indent=2) + '\n')
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
