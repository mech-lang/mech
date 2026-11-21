"""Replay original source with a compiler from the archived source commit."""
from pathlib import Path
from hashlib import sha256
import argparse
import gzip
import json
import os
import subprocess

HERE = Path(__file__).resolve().parent


def read(path):
    if not path.exists():
        return json.loads(gzip.decompress(path.with_name(path.name + '.gz').read_bytes()))
    return json.loads(path.read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('compiler', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--unit', action='store_true')
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    source = HERE.parent.parent / 'program.mec'
    assert sha256(source.read_bytes()).hexdigest() == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    role = 'early-owner-output-root-preserved' if args.unit else 'selected-map'
    recipe = read(HERE / 'scoped-registers' / role / 'result.json')
    flags = recipe['flags'].copy()
    flags.pop('MECH_COMPACT_SOURCE_BLOCK_REPORT', None)
    key = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
    flags[key] = ';'.join(
        symbol + '@' + str(HERE / 'profiles' / Path(path).name)
        for symbol, path in (item.split('@', 1) for item in flags[key].split(';'))
    )
    for mode in ['1', 'checked']:
        folder = args.output.resolve() / mode
        folder.mkdir(parents=True, exist_ok=False)
        environment = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
        command = [str(compiler), str(source), str(folder / 'frontend'), 'compact']
        with (folder / 'compile.log').open('w') as log:
            subprocess.run(command, env=environment | flags | {'MECH_COMPACT_PAIR_REGISTER_SPILLS': mode},
                           stdout=log, stderr=log, check=True)
        actual = {name: sha256((folder / 'frontend' / name).read_bytes()).hexdigest()
                  for name in recipe['artifacts']}
        assert actual == recipe['artifacts'], (mode, actual, recipe['artifacts'])
        print(role, mode, 'all three artifacts match')


if __name__ == '__main__':
    main()
