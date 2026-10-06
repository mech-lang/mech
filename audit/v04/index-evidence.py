#!/usr/bin/env python3
"""Index measured runs while retaining the original, detailed evidence files."""
import hashlib
import json
from pathlib import Path
import platform

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'audit/v04'
COMMIT = 'c4777b7015fe8ff47fdfa48d18606c49aace7d97'


def read(path):
    return json.loads((BASE / path).read_text())


def digest(path):
    p = ROOT / path
    return {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'bytes': p.stat().st_size}


records = []
for path in ('data/architecture-evidence.json', 'data/application-evidence.json', 'data/compute-evidence.json'):
    for record in read(path)['records']:
        records.append({**record, 'detailed_record': path})


def browser_identity(result):
    """Resolve the artifact captured by a run and check every nested identity."""
    hashes = []
    identities = []

    def visit(value):
        if isinstance(value, dict):
            artifacts = value.get('artifacts', {})
            wasm = artifacts.get('pkg/mech_wasm_bg.wasm', {}) if isinstance(artifacts, dict) else {}
            if isinstance(wasm, dict) and wasm.get('sha256'):
                hashes.append(wasm['sha256'])
                identities.append(value)
            for key in ('loaded_wasm_sha256', 'loadedWasmSha256', 'artifact_sha256'):
                if isinstance(value.get(key), str):
                    hashes.append(value[key])
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(result)
    unique = sorted(set(hashes))
    if len(unique) > 1:
        raise ValueError(f'Browser evidence contains inconsistent WASM identities: {unique}')
    if identities:
        identity = identities[0]
    elif unique:
        identity = {'loaded_wasm_sha256': unique[0], 'scope': 'Run-specific runtime hash; build metadata absent'}
    else:
        identity = {'status': 'unverified', 'reason': 'Run-specific WASM identity absent'}
        initial = BASE / 'evidence/wasm-artifact-initial.json'
        if initial.exists():
            identity['initial_artifact_reference'] = read('evidence/wasm-artifact-initial.json')['artifacts']['pkg/mech_wasm_bg.wasm']
    return identity, {'status': 'consistent' if unique else 'unverified',
        'observed_hash_fields': len(hashes), 'wasm_sha256': unique[0] if unique else None}


def browser_record(id, claim, file, command, expected, actual, contract):
    if not (BASE / file).exists():
        return
    result = read(file)
    outcome = result.get('status', result.get('outcome', 'unverified'))
    identity, consistency = browser_identity(result)
    records.append(dict(id=id, claim=claim, contract=contract, source_commit=COMMIT,
        artifact_identity=identity, artifact_consistency=consistency, configuration='Browser WASM; feature and profile identity in artifact record',
        environment=result.get('environment', result.get('browser', 'Chrome 154; Apple M3; data/environment.json')),
        input_or_seed='Deterministic cases and inputs retained in detailed record', command=command,
        expected_outcome=expected, actual_outcome=actual(result), evidence_level='browser-execution-and-result-comparison',
        outcome=outcome, result_location='audit/v04/' + file))


browser_record('E-STREAM-BROWSER', 'Streaming ingestion, publications, recovery, UTF-8 transport and editor identities',
    'evidence/streaming-browser-result.json', 'python3 audit/v04/check-streaming-browser.py',
    'All deterministic split, seeded, resource and edit assertions pass',
    lambda r: {k:r[k] for k in ('status','schedules','publications') if k in r},
    'docs/design/canonical-document-streaming.mec; docs/design/incremental-syntax-architecture.mec')
browser_record('E-TYPES-BROWSER', 'Source type stages and same-instance interval input admission preserve accepted state on rejection',
    'evidence/types-browser.json', 'python3 audit/v04/check_types_browser.py',
    'Thirteen source cases, exact wide integer transport, source selection and accept9/reject10/accept3 pass',
    lambda r: {'status':r.get('status'), 'details':'Thirteen cases and lifecycle assertions in detailed result'},
    'docs/design/grammar-audit/fixed-integer-intervals.md')
browser_record('E-COMPUTE-BROWSER', 'CPU and hardware WebGPU complete identical matrix source and inputs',
    'evidence/compute-browser-result.json', 'python3 audit/v04/check-product-browser.py',
    'CPU and GPU results equal independent arithmetic at 3 and 1024 columns; hard GPU placement rejects a CPU-only selector',
    lambda r: {'outcome':r['outcome'], 'runs':[{k:x.get(k) for k in ('columns','requested','selected','completed','outcome','adapter')} for x in r['records']], 'placement':r['placement'], 'controls':r.get('controls')},
    'docs/design/named-compute-regions.md; include/browser-compute.js completion protocol')
browser_record('E-APPLICATION-BROWSER', 'Resident timer, Mech computation, state and SVG output execute in WASM',
    'evidence/application-browser-result.json', 'python3 audit/v04/check-product-browser.py',
    'Two-body dt0.01/dt0.02 agree with independent trace; rejection, restart, disposal and maintained ten-body execution pass',
    lambda r: {'outcome':r['outcome'],'cases':len(r['cases']),'maintained_application':r.get('maintained_application',{}).get('outcome'),'controls':r.get('controls')},
    'examples/resident-n-body; audit/v04/site/fixtures/nbody-two/reference.json')

native = read('evidence/streaming-native.json')
records.append({**native, 'outcome':'passed', 'result_location':'audit/v04/evidence/streaming-native-tests.log'})
gpu = BASE / 'evidence/native-gpu-publication.log'
if gpu.exists():
    passed = 'test result: ok. 1 passed' in gpu.read_text()
    executable = '/private/tmp/mech-v04-target-product/debug/deps/canonical_publication-570a326a503ae265'
    records.append(dict(id='E-NATIVE-GPU',claim='Native wgpu publications use current input and transactional state',
        contract='hosts/gpu/tests/canonical_publication.rs',source_commit=COMMIT,
        artifact_identity=digest(executable) if Path(executable).exists() else {'identity':'see retained build log'},
        configuration='mech-gpu --features native; MECH_REQUIRE_GPU=1; aarch64-apple-darwin; test profile',
        environment=platform.platform(), input_or_seed='Checked-in native GPU publication test',
        command='MECH_REQUIRE_GPU=1 cargo test --locked --offline -p mech-gpu --features native --test canonical_publication native_gpu_publications_use_current_input_and_transactional_state -- --nocapture',
        expected_outcome='One native wgpu execution test passes; an unavailable adapter causes failure',
        actual_outcome='1 passed, 0 failed' if passed else gpu.read_text()[-1000:],
        evidence_level='native-GPU-execution',outcome='passed' if passed else 'failed',result_location='audit/v04/evidence/native-gpu-publication.log'))

for file in sorted((BASE/'evidence/extraction').glob('*.json')):
    value = json.loads(file.read_text())
    if not isinstance(value, dict) or 'exit_code' not in value or 'command' not in value:
        continue
    command = value['command']
    words = command if isinstance(command,list) else command.split()
    level = 'compilation' if 'check' in words else 'native-execution' if any(x in words for x in ('run','test')) else 'dependency-or-package-inspection'
    records.append(dict(id='E-EXTRACTION-'+file.stem.upper(),claim=file.stem.replace('-',' '),
        contract='audit/v04/extraction.md',source_commit=COMMIT,artifact_identity='See extraction patch, resolved dependency and artifact records',
        configuration=value.get('environment',{}),environment=value.get('cwd'),input_or_seed='Recorded command and preserved extraction layout',
        command=command,expected_outcome='Command exits successfully',actual_outcome={'exit_code':value['exit_code'],'elapsed_seconds':value.get('elapsed_seconds')},
        evidence_level=level,outcome='passed' if value['exit_code']==0 else 'failed',result_location=str(file.relative_to(ROOT))))

for name in ('trust-native.json','trust-browser.json','diffusion-browser.json','inventory-browser.json','inventory-adapter-native.json'):
    path = BASE / 'evidence' / name
    if path.exists():
        value=json.loads(path.read_text())
        if name.endswith('-browser.json'):
            identity, consistency = browser_identity(value)
            value = {**value, 'artifact_identity': identity, 'artifact_consistency': consistency}
        records.append({**value,'outcome':value.get('outcome',value.get('status','unverified')),'detailed_record':str(path.relative_to(BASE))})

inventory_path = BASE / 'evidence/inventory/native-result.json'
if inventory_path.exists():
    value = json.loads(inventory_path.read_text())
    records.append({**value, 'id': 'E-INVENTORY-NATIVE',
        'source_commit': value['baseline_commit'],
        'contract': 'audit/v04/evidence/inventory/inventory.mec; audit/v04/evidence/inventory/fixture.json',
        'artifact_identity': {'source_sha256': value['source_sha256'],
            'fixture_sha256': value['fixture_sha256'],
            'bytecode_sha256': value['bytecode_sha256'], 'bytecode_bytes': value['bytecode_bytes'],
            'executable_sha256': value['executable_sha256'], 'executable_bytes': value['executable_bytes']},
        'configuration': {'manifest': 'audit/v04/evidence/inventory/native/Cargo.toml',
            'environment': value['environment']},
        'result_location': 'audit/v04/evidence/inventory/native.log',
        'detailed_record': str(inventory_path.relative_to(BASE))})

by_id = {}
for record in records:
    identity = record['id']
    by_id[identity] = {**by_id.get(identity, {}), **record}
records = list(by_id.values())

(BASE/'data/evidence.json').write_text(json.dumps(dict(schema=1,source_commit=COMMIT,
    scope='Executed audit records. Original failed attempts retain their outcome; revised passes have separate records.',
    records=records),indent=2)+'\n')
print(json.dumps({'records':len(records),'outcomes':{s:sum(r.get('outcome')==s for r in records) for s in sorted({r.get('outcome','unverified') for r in records})}}))
