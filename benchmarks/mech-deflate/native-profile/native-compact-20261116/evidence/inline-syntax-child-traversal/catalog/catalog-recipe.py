"""Check that the promoted frame-copy recipe reuses the measured compiler exactly."""

import os
from pathlib import Path
import runpy
import subprocess
import sys

from stage import BASE, CHECKOUT, HERE, PACKET, SOURCE, TARGET, git, read, sha, write

parameters = runpy.run_path(str(HERE / 'catalog-build.py'), run_name='overlapped_catalog_recipe')
OUT, PROFILE, MODE = (parameters[name] for name in ('OUT', 'PROFILE', 'MODE'))
RECIPE = PACKET / 'rebuild.py'


def changed_recipe():
    checkout = CHECKOUT / RECIPE
    original = checkout.read_text()
    assert original.encode() == git('show', f'{BASE}:{RECIPE}')
    point = "    flags['MECH_COMPACT_DIRECT_ENTRY_LITERALS'] = '1'\n"
    assert original.count(point) == 1 and MODE not in original
    changed = original.replace(point, point + f"    if args.direct_source:\n        flags['{MODE}'] = '1'\n")
    path = OUT / 'source' / RECIPE
    write(path, changed)
    return path


def main():
    built = read(OUT / 'result.json')
    experiment = OUT / 'matched/comparison.json'
    data = read(experiment)
    for role in ('mech-incumbent', 'mech-serial-catalog'):
        assert data['comparisons'][role]['through_link']['lower_90'] > 0
    recipe = changed_recipe()
    candidate = {SOURCE: HERE / 'source' / SOURCE, PROFILE: OUT / 'source' / PROFILE, RECIPE: recipe}
    before = {rel: ((CHECKOUT / rel).stat(), (CHECKOUT / rel).read_bytes()) for rel in candidate}
    assert all(bits == git('show', f'{BASE}:{rel}') for rel, (_, bits) in before.items())
    assert sha(candidate[PROFILE]) == built['profile_source_sha256']
    assert sha(candidate[SOURCE]) == built['traversal_source_sha256']
    binary = TARGET / 'release/compile_profile'
    assert sha(binary) == built['compiler_sha256']
    syntax = Path(built['libraries']['mech_syntax']['path']).stem.removeprefix('libmech_syntax-')
    syntax_stamp = TARGET / f'release/.fingerprint/mech-syntax-{syntax}/invoked.timestamp'
    profile_stamp = TARGET / 'release/.fingerprint/mech-compression-native-profile-b123bdd33aa29edc/invoked.timestamp'
    times = {SOURCE: syntax_stamp.stat().st_mtime_ns - 1_000_000_000,
             PROFILE: profile_stamp.stat().st_mtime_ns - 1_000_000_000}
    stage = OUT / 'promoted'
    stage.mkdir()
    verified = stage / 'rebuild'
    command = [sys.executable, '-B', CHECKOUT / RECIPE, verified,
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        for path, source in candidate.items():
            local = CHECKOUT / path
            local.write_bytes(source.read_bytes())
            if path in times:
                os.utime(local, ns=(before[path][0].st_atime_ns, times[path]))
        with (stage / 'build.log').open('w') as log:
            subprocess.run(command, check=True, env=env | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT)
    finally:
        for path, (timestamp, original) in before.items():
            local = CHECKOUT / path
            assert local.read_bytes() in (original, candidate[path].read_bytes())
            local.write_bytes(original)
            os.utime(local, ns=(timestamp.st_atime_ns, timestamp.st_mtime_ns))
    info = read(verified / 'rebuild.json')
    phases = read(verified / 'frontend/compile.json')
    assert info['full_integrated_source_identity'] and info['native_flags'][MODE] == '1'
    assert phases['catalog_overlapped_parse'] and sha(binary) == built['compiler_sha256']
    expected = built['artifacts']
    observed = {'bytecode': sha(verified / 'frontend/program.mecb'),
                'encoder': sha(verified / 'native/program.o'), 'metadata': sha(verified / 'native/metadata.json')}
    assert observed == expected
    proof = {'sources': {str(path): sha(src) for path, src in candidate.items()},
             'compiler_sha256': sha(binary), 'recipe_sha256': sha(recipe), 'artifacts': observed,
             'matched_comparison_sha256': sha(experiment), 'command': list(map(str, command)),
             'catalog_work_seconds': phases['catalog_work_seconds'], 'supported_mode_flag': MODE,
             'object_text_bytes': info['object_text_bytes'], 'rebuild_sha256': sha(verified / 'rebuild.json')}
    write(stage / 'result.json', proof)
    print('Normal frame-copy recipe reproduces the compiled worker inputs and enables catalog overlap.', flush=True)


if __name__ == '__main__':
    main()
