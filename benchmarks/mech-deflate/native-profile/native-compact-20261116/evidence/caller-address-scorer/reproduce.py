"""Re-emit a recorded scorer object with a compiler built from the pinned source."""

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess

here = Path(__file__).resolve().parent
packet = here.parent.parent
layout_key = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
tail_key = 'MECH_COMPACT_ROOT_TAIL_PROFILE'


def checksum(path):
    return sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('compiler', type=Path, help='opt-level-3 compile_profile')
    parser.add_argument('output', type=Path)
    parser.add_argument('--variant', default='caller-addresses-refined')
    args = parser.parse_args()
    if re.fullmatch(r'[a-z0-9-]+', args.variant) is None:
        parser.error('invalid variant')
    recorded = load(here / 'variants' / args.variant / 'receipt.json')
    flags = recorded['flags']
    recipe = load(packet / 'recipe.json')
    source = packet / 'program.mec'
    assert checksum(source) == recipe['source_sha256']
    inputs = []
    for entry in flags[layout_key].split(';'):
        symbol, path = entry.split('@', 1)
        name = Path(path).name
        profile = (here / 'profiles' / name if name != 'report.json'
                   else packet / 'profiles' / f'{symbol}.json')
        inputs.append(f'{symbol}@{profile.resolve(strict=True)}')
    flags[layout_key] = ';'.join(inputs)
    if tail_key in flags:
        flags[tail_key] = str(packet / 'profiles' / Path(flags[tail_key]).name)
    assert all('LLVM' not in key for key in flags)
    output = args.output.resolve()
    if output.exists():
        parser.error('the output directory must be new')
    clean = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    subprocess.run([args.compiler.resolve(strict=True), source, output, 'compact'],
                   env=clean | flags, check=True)
    expected = {
        'program.o': recorded['layout']['object_sha256'],
        'program.mecb': recorded['bytecode_sha256'],
        'metadata.json': recorded['metadata_sha256'],
    }
    for name, wanted in expected.items():
        actual = checksum(output / name)
        if actual != wanted:
            raise SystemExit(f'{name} SHA-256 is {actual}, expected {wanted}')
        print(name, actual)


if __name__ == '__main__':
    main()
