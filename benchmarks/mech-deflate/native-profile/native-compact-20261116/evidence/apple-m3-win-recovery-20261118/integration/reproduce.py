"""Verify the named M1 encoder with a compiler built from the integrated source."""
import argparse
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import struct
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--source-root', type=Path, required=True)
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--build-receipt', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--campaign-lock', type=Path, required=True)
a = p.parse_args()
s = a.source_root.resolve()
o = a.output.resolve()
o.mkdir(parents=True, exist_ok=False)
here = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
recipe_dir = here / 'evidence/encoder-size-gap/shared-heap-layout-recovery'

def git(*args):
    return subprocess.check_output(['git', '-C', str(s), *args])

def blob(path):
    return git('show', 'HEAD:' + path.as_posix())

def digest(path):
    return sha256(path.read_bytes()).hexdigest()

receipt = json.loads(a.build_receipt.read_text())
assert digest(a.compiler) == receipt['compiler_sha256']
head = git('rev-parse', 'HEAD').decode().strip()
subprocess.run(['git', '-C', str(s), 'merge-base', '--is-ancestor', receipt['source_commit'], head], check=True)
changes = git('diff', '--name-only', receipt['source_commit'], head).decode().splitlines()
documentation = {str(here / name) for name in ('CURRENT.md', 'README.md', 'source-and-evidence-manifest.json')}
assert all(path.startswith(str(here / 'evidence') + '/') or path in documentation for path in changes), changes
assert not git('diff', '--name-only', '--', 'hosts/gpu', 'modules', 'src', 'Cargo.lock').strip()
recipe = json.loads(blob(here / 'recipe.json'))
pinned = json.loads(blob(recipe_dir / 'candidate.json'))
assert digest(s / here / 'program.mec') == pinned['source_sha256']
flags = dict(recipe['flags'])
flags.update({
    'MECH_COMPACT_INLINE_SINGLE_CALLS': 'preselected',
    'MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA': '1',
    'MECH_COMPACT_SHARE_COLD_TAILS': '1',
    'MECH_COMPACT_FOLD_FRESH_OWNER_COUNTS': '1',
    'MECH_COMPACT_BRANCH_ON_ZERO': '1',
    'MECH_COMPACT_ROOT_TAIL_PROFILE': str(s / here / recipe['frame_copies_tail_profile']),
    'MECH_COMPACT_SHARE_FRAME_COPIES': '1',
    'MECH_COMPACT_DIRECT_ENTRY_LITERALS': '1',
    'MECH_COMPILE_OVERLAP_SOURCE_CATALOG': '1',
})
for key in pinned['remove_flags']:
    del flags[key]
flags.update(pinned['override_flags'])
profiles = []
for index, item in enumerate(pinned['profiles']):
    path = Path(os.path.normpath(recipe_dir / item['path']))
    data = blob(path)
    assert sha256(data).hexdigest() == item['sha256']
    target = o / ('profile-' + str(index) + '.json')
    target.write_bytes(data)
    profiles.append(item['symbol'] + '@' + str(target))
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'] = ';'.join(profiles)
(o / 'flags.json').write_text(json.dumps(flags, indent=2) + '\n')
lock_fd = os.open(a.campaign_lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
with os.fdopen(lock_fd, 'w') as f:
    json.dump({'pid': os.getpid(), 'task': 'Verify integrated compiler M1 object identity; no timing claim'}, f)
try:
    command = [str(a.compiler.resolve()), str(s / here / 'program.mec'), str(o / 'native'), 'compact']
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    env.update(flags)
    env['PATH'] = '/usr/bin:/bin'
    with (o / 'compile.log').open('w') as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    artifacts = {}
    for name, expected in pinned['artifact_sha256'].items():
        actual = digest(o / 'native' / name)
        artifacts[name] = {'sha256': actual, 'expected_sha256': expected, 'exact': actual == expected}
    metadata = json.loads((o / 'native/metadata.json').read_text())
    data = (o / 'native/program.o').read_bytes()
    assert struct.unpack_from('<I', data)[0] == 0xfeedfacf
    text_bytes = None
    offset = 32
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command_kind, size = struct.unpack_from('<II', data, offset)
        if command_kind == 0x19:
            for section in range(struct.unpack_from('<I', data, offset + 64)[0]):
                at = offset + 72 + section * 80
                if data[at:at + 16].rstrip(b'\0') == b'__text':
                    text_bytes = struct.unpack_from('<Q', data, at + 40)[0]
        offset += size
    assert text_bytes is not None
    result = {
        'schema': 'mech.integrated-m1-object-replay.v1',
        'recorded_at': datetime.now(timezone.utc).isoformat(),
        'source_commit': head,
        'compiler_source_commit': receipt['source_commit'],
        'compiler_sha256': digest(a.compiler),
        'compiler_build_receipt_sha256': digest(a.build_receipt),
        'recipe': 'm1-shared-heap-layout',
        'recipe_manifest_sha256': sha256(blob(recipe_dir / 'candidate.json')).hexdigest(),
        'source_sha256': digest(s / here / 'program.mec'),
        'command': command,
        'artifacts': artifacts,
        'encoder_text_bytes': text_bytes,
        'encoder_text_bytes_expected': pinned['encoder_text_bytes'],
        'passed': all(row['exact'] for row in artifacts.values()) and text_bytes == pinned['encoder_text_bytes'],
        'runtime_timings': 'Not measured; the archived M1 runtime results are not new M3 results.',
        'scope': 'Compiler object, complete metadata and bytecode identity. No linked runtime identity or new timing claim.',
    }
    (o / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': result['passed'], 'encoder_text_bytes': result['encoder_text_bytes'], 'artifacts': artifacts}, indent=2))
    assert result['passed'], 'Integrated source changed the archived encoder; retain and inspect this result.'
finally:
    if a.campaign_lock.exists() and json.loads(a.campaign_lock.read_text())['pid'] == os.getpid():
        a.campaign_lock.unlink()
