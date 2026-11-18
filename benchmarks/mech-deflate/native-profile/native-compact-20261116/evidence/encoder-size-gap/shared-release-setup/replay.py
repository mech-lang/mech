"""Reproduce the measured diagnostic encoder from the patched compiler."""

from argparse import ArgumentParser
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('rebuild', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--compiler', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    recipe = json.loads((here / 'candidate.json').read_text())
    rebuild = json.loads((args.rebuild / 'rebuild.json').read_text())
    assert rebuild['full_integrated_source_identity']
    assert rebuild['object_text_bytes'] == 71192
    assert sha(args.rebuild / 'native/program.o') == recipe['maintained_encoder_sha256']
    command = rebuild['phases']['source_to_native']['command'].copy()
    if args.compiler is not None:
        assert sha(args.compiler) == recipe['compiler_sha256']
        command[0] = str(args.compiler)
    assert sha(command[1]) == recipe['source_sha256']
    command[2] = str(args.output)
    flags = rebuild['native_flags'].copy()
    for name in recipe['remove_flags']:
        flags.pop(name)
    flags.update(recipe['override_flags'])
    profile_key = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
    profiles = recipe['profiles']
    for row in profiles:
        assert sha(here / row['path']) == row['sha256']
    flags[profile_key] = ';'.join(row['symbol'] + '@' + str(here / row['path']) for row in profiles)
    assert all('LLVM' not in name for name in flags)
    environment = {name:value for name,value in os.environ.items() if not name.startswith('MECH_')}
    subprocess.run(command, env=environment | flags, check=True)
    for name, pin in recipe['artifact_sha256'].items():
        assert sha(args.output / name) == pin
    print(f"Reproduced the {recipe['encoder_text_bytes']:,}-byte encoder.")


if __name__ == '__main__':
    main()
