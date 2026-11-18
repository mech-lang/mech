"""Attribute live code and explain diagnostic link identity differences."""
from argparse import ArgumentParser
from collections import defaultdict
from hashlib import file_digest, sha256
from pathlib import Path
from struct import unpack_from
import importlib.util
import json
import re

from measure import ACTORS


def digest(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def group(object_file):
    if object_file.endswith('/program.o'):
        return 'encoder'
    if object_file.endswith('/mimalloc.o'):
        return 'mimalloc'
    if object_file.endswith('/main.o'):
        return 'c_main'
    member = object_file.split('.a(', 1)[-1]
    if member.startswith(('mech_gpu-', 'libmech_gpu_runtime.')):
        return 'compact_runtime'
    if member.startswith(('mech_deploy-', 'libmech_deploy.')):
        return 'rust_bridge'
    if member.startswith(('object-', 'addr2line-', 'gimli-', 'rustc_demangle-')):
        return 'rust_symbolization_objects'
    if member.startswith(('std-', 'core-', 'alloc-', 'panic_abort-', 'compiler_builtins-')):
        return 'rust_library_objects'
    return 'other_attributed'


def live_text(map_path):
    objects = {}
    sizes = defaultdict(int)
    per_object = defaultdict(int)
    in_symbols = False
    text_start = text_end = None
    for line in map_path.read_text(errors='replace').splitlines():
        if line.startswith('# Dead Stripped Symbols:'):
            break
        if line == '# Symbols:':
            in_symbols = True
        if not in_symbols:
            object_line = re.fullmatch(r'\[\s*(\d+)\]\s+(.*)', line)
            if object_line:
                index, object_file = object_line.groups()
                objects[int(index)] = object_file
            fields = line.split()
            if len(fields) == 4 and fields[2:] == ['__TEXT', '__text']:
                text_start = int(fields[0], 16)
                text_end = text_start + int(fields[1], 16)
            continue
        record = re.fullmatch(r'(0x\w+)\s+(0x\w+)\s+\[\s*(\d+)\]\s*.*', line)
        if record:
            pc, size, obj = record.groups()
            pc, size = int(pc,16), int(size,16)
            if size and text_start <= pc < text_end:
                assert pc + size <= text_end
                object_file = objects[int(obj)]
                sizes[group(object_file)] += size
                per_object[object_file] += size
    assert text_start and text_end
    sizes['unattributed_padding'] = text_end - text_start - sum(sizes.values())
    assert sizes['unattributed_padding'] >= 0, sizes
    return {'text_bytes': text_end - text_start, 'groups': dict(sizes), 'object_attribution': dict(per_object)}


def signature_and_uuid(data):
    cursor = 32
    uuid = signature = None
    for _ in range(unpack_from('<I', data, 16)[0]):
        command, size = unpack_from('<II', data, cursor)
        if command == 0x1B:  # LC_UUID
            assert size == 24
            uuid = cursor + 8
        if command == 0x1D:  # LC_CODE_SIGNATURE
            offset, length = unpack_from('<II', data, cursor+8)
            assert offset + length == len(data)
            signature = (offset, length)
        cursor += size
    assert signature and uuid is not None
    return signature, uuid


def identity(binaries, link_report, timing):
    results = {}
    for actor in ACTORS:
        reference = binaries[actor]
        derived = timing / 'final-links' / actor
        before, after = reference.read_bytes(), derived.read_bytes()
        signature, uuid = signature_and_uuid(before)
        assert signature_and_uuid(after) == (signature, uuid), actor
        signed, length = signature
        assert before[:uuid] == after[:uuid], actor
        assert before[uuid+16:signed] == after[uuid+16:signed], actor
        all_blocks = {
            row['samples'][actor]['sha256'] for row in [link_report['warmup'], *link_report['blocks']]
        }
        assert all_blocks == {sha256(after).hexdigest()}, actor
        results[actor] = {
            'original': {'path': str(reference), 'sha256': sha256(before).hexdigest(),
                         'uuid': before[uuid:uuid+16].hex()},
            'timed_basename': {'path': str(derived), 'sha256': sha256(after).hexdigest(),
                               'uuid': after[uuid:uuid+16].hex()},
            'deterministic_across_all_links': True,
            'whole_file_match': before == after,
            'all_bytes_equal_except_uuid_and_code_signature': True,
            'uuid_offset': uuid, 'code_signature_offset': signed, 'code_signature_bytes': length,
        }
    return results


def inspect_binary(loader, path):
    _image, sections, symbols = loader.load(path)
    undefined = sorted(name for name, kind, _section, _address in symbols
                       if kind & 0x0e == 0 and kind & 1)
    return {
        'sha256': digest(path), 'file_bytes': path.stat().st_size,
        'sections': {f'{seg.decode()},{sec.decode()}': size for (seg, sec), (
            _index, _address, size, _offset, _reloc, _count) in sections.items()},
        'undefined': undefined,
    }


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument('trial', type=Path)
    cli.add_argument('timing', type=Path)
    cli.add_argument('--baseline', type=Path, required=True)
    here = Path(__file__).resolve().parent
    cli.add_argument('--loader', type=Path, default=(
        here.parent / 'host-opt-levels/scripts-as-run/reachability-census.py'))
    args = cli.parse_args()
    common, trial, timing = args.baseline.resolve(), args.trial.resolve(), args.timing.resolve()
    binaries = {'mech-std': common/'mech', 'mech-alloc': trial/'mech',
                'roc': common/'roc', 'libdeflate': common/'libdeflate'}
    spec = importlib.util.spec_from_file_location('mach', args.loader.resolve())
    loader = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loader)
    audited = {
        actor: {
            'unstripped': inspect_binary(loader, path),
            'stripped': inspect_binary(loader, path.with_name(f'{path.name}-stripped')),
            'text_attribution': live_text(path.with_suffix('.map')),
        }
        for actor, path in binaries.items()
    }
    for results in audited.values():
        assert (results['unstripped']['sections']['__TEXT,__text']
                == results['stripped']['sections']['__TEXT,__text']
                == results['text_attribution']['text_bytes'])
    records = {
        'binaries': audited,
        'timing_link_identity': identity(binaries, read_json(timing/'link.json'), timing),
    }
    (timing/'binary-audit.json').write_text(json.dumps(records, indent=2) + '\n')
    print(json.dumps({actor: record['text_attribution']['groups']
                      for actor, record in audited.items()}, indent=2))


if __name__ == '__main__':
    main()
