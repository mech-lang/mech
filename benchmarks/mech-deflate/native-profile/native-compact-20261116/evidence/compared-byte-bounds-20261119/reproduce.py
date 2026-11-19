"""Rebuild a source-pinned compared-byte experiment with portable, hash-checked profiles."""
from argparse import ArgumentParser
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import tarfile

HERE = Path(__file__).resolve().parent
ROLES = {'control': 'byte-bounds-geometry-control', 'wide': 'byte-bounds-geometry-wide-profiled', 'narrow': 'byte-bounds-geometry-narrow-profiled', 'complements': 'byte-bounds-geometry-complements-profiled', 'hash32': 'byte-bounds-geometry-hash32-profiled'}


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('compiler', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--variant', choices=ROLES, default='narrow')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True)
    study = json.loads((HERE / 'study.json').read_text())
    pins = json.loads((HERE / 'archive-files.json').read_text())['source-and-receipts.tar.gz']
    with tarfile.open(HERE / 'source-and-receipts.tar.gz') as archive:
        def read(name):
            raw = archive.extractfile(name).read()
            assert len(raw) == pins[name]['bytes']
            assert sha256(raw).hexdigest() == pins[name]['sha256']
            return raw

        recipe = json.loads(read(ROLES[args.variant] + '/receipt.json'))
        flags = recipe['flags'].copy()
        rebound = []
        for item in flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';'):
            symbol, original = item.split('@', 1)
            digest = study['profile_inputs'][original]
            raw = read('portable-profiles/' + digest + '.json')
            assert sha256(raw).hexdigest() == digest
            profile = output / (digest + '.json')
            profile.write_bytes(raw)
            rebound.append(symbol + '@' + str(profile))
        flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'] = ';'.join(rebound)
    compiler = args.compiler.resolve()
    source = HERE.parents[1] / 'program.mec'
    command = [str(compiler), str(source), str(output / 'frontend'), 'compact']
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    with (output / 'compile.log').open('w') as log:
        subprocess.run(command, env=environment | flags, stdout=log, stderr=log, check=True)
    artifacts = {
        name: sha256((output / 'frontend' / name).read_bytes()).hexdigest()
        for name in ['program.o', 'metadata.json', 'program.mecb']
    }
    result = {
        'required_compiler_source': study['live_source_commit'],
        'compiler_sha256': sha256(compiler.read_bytes()).hexdigest(),
        'command': command,
        'flags': flags,
        'artifacts': artifacts,
        'expected': recipe['artifacts'],
        'matches': artifacts == recipe['artifacts'],
        'runtime_qualification': 'No new hardware/runtime qualification is inferred by this rebuild.',
    }
    (output / 'reproduction.json').write_text(json.dumps(result, indent=2) + '\n')
    assert result['matches'], 'Artifacts differ; actual hashes are recorded in reproduction.json'
    print('Reproduced object, complete metadata and canonical bytecode:', args.variant)


if __name__ == '__main__':
    main()
