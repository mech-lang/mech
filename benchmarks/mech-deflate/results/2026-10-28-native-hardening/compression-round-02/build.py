import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent
workspace = Path('/private/tmp/mech-deflate-linear-stack')
deps = Path('/private/tmp/mech-live-control-target/release/deps')
rustc = '/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc'
allocator = Path('/private/tmp/roc-deflate-repro-20261024/roc-deflate/benchmark/echo-clock/targets/arm64mac/mimalloc.o')
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert digest(allocator) == '38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874'

def resolve(name, wanted):
    for folder in (deps.parent / '.fingerprint').iterdir():
        fingerprint = folder / ('lib-' + name)
        if fingerprint.is_file() and int.from_bytes(bytes.fromhex(fingerprint.read_text().strip()), 'little') == wanted:
            path = deps / ('lib' + name + '-' + folder.name.rsplit('-', 1)[1] + '.rlib')
            if path.is_file():
                return path
    raise ValueError((name, wanted))

libs = {name: deps / ('lib' + name + '-' + suffix + '.rlib') for name, suffix in {
    'mech_gpu': '3c73528bc7ca0ab2', 'mech_engine': '4aee2ad1a00f13dd',
    'mech_stdlib': 'bf12058d26333a71', 'mech_core': '3f09556dc3716cd3',
    'mech_syntax': '9be4337841a5564e',
}.items()}
fingerprint = json.loads((deps.parent / '.fingerprint/mech-gpu-3c73528bc7ca0ab2/lib-mech_gpu.json').read_text())
for _, name, _, wanted in fingerprint['deps']:
    if name in ['serde_json', 'serde', 'miniz_oxide', 'mech_engine', 'mech_core']:
        libs[name] = resolve(name, wanted)

base = [rustc, '--edition=2024', '-C', 'opt-level=3', '-D', 'warnings', '-L', 'dependency=' + str(deps)]
for name, path in libs.items():
    base += ['--extern', name + '=' + str(path)]
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=workspace, text=True).strip()
for label, source, use_allocator in [
    ('current', Path('/private/tmp/mech-timing-round.rs'), True),
    ('word-updates', Path('/private/tmp/mech-deflate-profiling-20261028/word-updates.rs'), False),
]:
    assert not (root / label).exists(), 'never overwrite a retained executable'
    command = base + (['-C', 'link-arg=' + str(allocator)] if use_allocator else []) + [str(source), '-o', str(root / label)]
    result = subprocess.run(command, capture_output=True, text=True)
    (root / (label + '-build.log')).write_text(result.stdout + result.stderr)
    receipt = {'command': command, 'exit': result.returncode, 'head': head, 'source_sha256': digest(source),
               'libraries': {name: {'path': str(path), 'sha256': digest(path)} for name, path in libs.items()}}
    if result.returncode == 0:
        receipt['binary_sha256'] = digest(root / label)
    (root / (label + '-build.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    result.check_returncode()
    print(label, receipt['binary_sha256'], flush=True)
