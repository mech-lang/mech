"""Recover the probe executable after a failed final strip on the build host."""

import json
import os
from pathlib import Path
import subprocess
import sys

from build import BASE, BINARY, CHECKOUT, HERE, MATCHED, OUTPUT, PACKAGE, REFERENCE, TARGET
from build import enabled, git, read, save, sha

before = OUTPUT / 'disabled'
interrupted = OUTPUT / 'disabled-before-relink'
release = TARGET / 'release'
log_path = before / 'compiler-build.jsonl'
target = release / 'compile_profile'
assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == BASE
assert 'No space left on device' in log_path.read_text()
assert target.stat().st_size < 4_096 and not os.access(target, os.X_OK)
assert not BINARY.exists() and not interrupted.exists()

files = read(HERE / 'source.json')['files']
gpu, executable = None, None
for line in log_path.read_text().splitlines():
    if not line.startswith('{'):
        continue
    row = json.loads(line)
    if row.get('reason') != 'compiler-artifact':
        continue
    name = row['target']['name']
    if name == 'mech_gpu':
        (gpu,) = [Path(f) for f in row['filenames'] if f.endswith('.rlib')]
        assert not row['fresh']
    if name == 'compile_profile' and row['executable']:
        executable = Path(row['executable'])
        assert not row['fresh']
assert gpu and gpu.is_file() and executable == target
suffix = gpu.stem.removeprefix('lib')
stamp = release / '.fingerprint' / (suffix.replace('mech_gpu-', 'mech-gpu-')) / 'invoked.timestamp'
original_gpu = sha(gpu)
gpu_invoked_ns = stamp.stat().st_mtime_ns
overlay = {}
for rel, pins in files.items():
    path = CHECKOUT / rel
    baseline = git('show', f'{BASE}:{rel}')
    candidate = (HERE / 'source' / rel).read_bytes()
    assert path.read_bytes() == baseline and sha(path) == pins['baseline']
    assert sha(HERE / 'source' / rel) == pins['probe']
    overlay[path] = (path.stat(), baseline, candidate)

command = [sys.executable, '-B', CHECKOUT / PACKAGE / 'rebuild.py', OUTPUT / 'disabled',
           '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
           '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
before.rename(interrupted)
# The link failed after Cargo built the changed mech-gpu inputs successfully.
# Restore that exact overlay for its cached library and invalidate only the two
# unreadable copies of the executable.
(target.parent / '.fingerprint/mech-compression-native-profile-b123bdd33aa29edc/bin-compile_profile').unlink()
(target.parent / 'deps/compile_profile-b123bdd33aa29edc').rename(interrupted / 'incomplete-binary')
target.unlink()
try:
    for path, (previous, _, candidate) in overlay.items():
        path.write_bytes(candidate)
        os.utime(path, ns=(previous.st_atime_ns, gpu_invoked_ns - 1_000_000_000))
    with (OUTPUT / 'relink.log').open('w') as log:
        subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    assert os.access(target, os.X_OK) and target.stat().st_size > 10_000_000
    assert sha(gpu) == original_gpu, 'cached library changed during final link'
    subprocess.run(['/bin/cp', '-c', target, BINARY], check=True)
finally:
    for path, (previous, baseline, candidate) in overlay.items():
        assert path.read_bytes() in (baseline, candidate)
        path.write_bytes(baseline)
        os.utime(path, ns=(previous.st_atime_ns, previous.st_mtime_ns))

expected = read(MATCHED)['actors'][REFERENCE]['artifact_sha256']
rebuild = OUTPUT / 'disabled'
actual = {'bytecode': sha(rebuild / 'frontend/program.mecb'),
          'encoder': sha(rebuild / 'native/program.o'), 'metadata': sha(rebuild / 'native/metadata.json')}
assert actual == expected
probe = enabled(expected)
suite = {
    'passed': 2,
    'log_sha256': sha(OUTPUT / 'tests.log'),
    'harness_sha256': sha(OUTPUT / 'ssa-word-harness.rs'),
}
assert '2 passed; 0 failed' in (OUTPUT / 'tests.log').read_text()
report = {'base': BASE, 'compiler_sha256': sha(BINARY), 'files': files,
          'build_command': list(map(str, command)), 'tests': suite, 'disabled': actual,
          'enabled': probe, 'exact': probe['artifacts'] == actual,
          'source_patch_sha256': sha(HERE / 'compiler.patch'),
          'retry': {
               'reason': 'Final Rust debug strip ran out of space',
               'original_library_sha256': original_gpu,
               'original_build_log_sha256': sha(interrupted / 'compiler-build.jsonl'),
               'final_build_log_sha256': sha(rebuild / 'compiler-build.jsonl'),
               'deployed_binary_bytes': BINARY.stat().st_size,
          }}
save(OUTPUT / 'result.json', report)
print(json.dumps({key: report[key] for key in ('exact', 'enabled', 'retry')}, indent=2), flush=True)
