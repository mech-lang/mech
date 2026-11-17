"""Reproduce the published static size screens; this does not qualify runtime wins."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def text_bytes(path):
    data = path.read_bytes()
    if struct.unpack_from('<I', data)[0] != 0xFEEDFACF:
        raise ValueError('expected the recorded Apple ARM64 Mach-O object')
    offset = 32
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            for section in range(struct.unpack_from('<I', data, offset + 64)[0]):
                at = offset + 72 + section * 80
                if data[at:at + 16].rstrip(b'\0') == b'__text':
                    return struct.unpack_from('<Q', data, at + 40)[0]
        offset += size
    raise ValueError('object has no __text section')


def main():
    here = Path(__file__).resolve().parent
    index = json.loads((here / 'evidence/size-screens/index.json').read_text())
    default_cases = {row['name']: row for row in index['cases']}
    continuation = json.loads((here / 'evidence/continuation-widths/index.json').read_text())
    cases = default_cases | {row['name']: row for row in continuation['cases']}
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', nargs='?', type=Path)
    parser.add_argument('--emitter', type=Path, help='emitter built from this PR by rebuild.py')
    parser.add_argument('--case', action='append', choices=cases, help='repeat to select cases; default: original 24')
    parser.add_argument('--list', action='store_true')
    args = parser.parse_args()
    if args.list:
        for name, case in cases.items():
            print(f"{name}: {case['expected']['object_text_bytes']} bytes")
        return
    if args.output is None or args.emitter is None:
        parser.error('output and --emitter are required unless --list is used')

    emitter = args.emitter.resolve(strict=True)
    recipe = json.loads((here / 'recipe.json').read_text())
    image = here / 'program.mecb'
    if sha(image) != recipe['bytecode_sha256']:
        raise ValueError('pinned bytecode identity changed')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    flags = dict(recipe['flags'])
    flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'] = ';'.join(
        profile['symbol'] + '@' + str(here / profile['path'])
        for profile in recipe['profiles']
    )
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    environment['PATH'] = '/usr/bin:/bin'
    results = []
    emitter_sha = sha(emitter)
    for name in dict.fromkeys(args.case or default_cases):
        case = cases[name]
        selected = dict(flags)
        for key, value in case['flag_overrides'].items():
            if value is None:
                selected.pop(key, None)
            else:
                selected[key] = value
        if any('LLVM' in key for key in selected):
            raise ValueError('this screen must not invoke LLVM tools')
        destination = output / name
        destination.mkdir()
        command = [str(emitter), str(image), str(destination)]
        start = time.perf_counter()
        with (destination / 'emit.log').open('w') as log:
            subprocess.run(command, env=environment | selected, stdout=log, stderr=subprocess.STDOUT, check=True)
        seconds = time.perf_counter() - start
        metadata = json.loads((destination / 'metadata.json').read_text())
        actual = {
            'object_sha256': sha(destination / 'program.o'),
            'metadata_sha256': sha(destination / 'metadata.json'),
            'object_text_bytes': text_bytes(destination / 'program.o'),
            'root_code_bytes': metadata['code_bytes'],
            'root_frame_bytes': metadata['stack_bytes'],
            'root_direct_calls': metadata['calls'],
        }
        mismatches = {key: {'expected': value, 'actual': actual[key]}
                      for key, value in case['expected'].items() if actual[key] != value}
        record = {
            'case': name, 'command': command, 'flags': selected,
            'emitter_sha256': emitter_sha, 'image_sha256': sha(image),
            'actual': actual, 'mismatches': mismatches, 'emit_seconds': seconds,
            'runtime_qualification': 'not measured',
            'timing_scope': 'single static-emission check; not a matched build benchmark',
        }
        (destination / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
        results.append(record)
        (output / 'verification.json').write_text(json.dumps(results, indent=2) + '\n')
        if mismatches:
            raise RuntimeError(f'{name}: output differs; inspect {destination / "receipt.json"}')
        print(f"{name}: exact object and metadata; {actual['object_text_bytes']} bytes", flush=True)


if __name__ == '__main__':
    main()
