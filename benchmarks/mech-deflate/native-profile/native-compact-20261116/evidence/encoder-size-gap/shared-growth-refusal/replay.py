"""Emit the optional growth-refusal object from a normal reviewed compiler rebuild."""

import argparse
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rebuild', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    receipts = json.loads((args.rebuild / 'rebuild.json').read_text())
    assert receipts['object_text_bytes'] == 71192
    trial = json.loads(Path(__file__).with_name('candidate.json').read_text())
    command = receipts['phases']['source_to_native']['command'].copy()
    flags = receipts['native_flags'].copy()
    assert sha(command[1]) == trial['compressor_source_sha256']
    assert sha(args.rebuild / 'frontend/program.mecb') == trial['bytecode_sha256']
    assert receipts['full_integrated_source_identity']
    assert not any('LLVM' in name for name in flags)
    del flags['MECH_COMPACT_ROOT_TAIL_PROFILE']
    flags['MECH_COMPACT_REFUSE_PHASE_GROWTH'] = '1'
    command[2] = str(args.output)
    env = {name: value for name, value in os.environ.items() if not name.startswith('MECH_')}
    subprocess.run(command, env=env | flags, check=True)
    assert sha(args.output / 'program.mecb') == trial['bytecode_sha256']
    assert sha(args.output / 'program.o') == trial['object_sha256']
    assert sha(args.output / 'metadata.json') == trial['metadata_sha256']
    print('Matched the LLVM-free 69,528-byte growth-refusal candidate.')


if __name__ == '__main__':
    main()
