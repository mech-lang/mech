#!/usr/bin/env python3
"""Static ARM64 Object audit. Never compiles or executes the program.

Example:
  python3 compare-native-objects.py --before /path/to/R59/linked-primary/after \
      --after /path/to/R60/linked-primary/after --out /path/to/static-comparison

Each input directory contains object.o, metadata.json and descriptor.json. Omit
--after for a baseline-only receipt. Output counts are encoded instructions,
NOT retired instructions or dynamic visits. Source-site excerpts are explicit
helper-argument anchors, not an automatic source-level debug map.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import re
import subprocess

INSN = re.compile(r'^\s*([0-9a-f]+):\s+([^\s]+)\s*(.*?)\s*$')
SITE = re.compile(r'^x1,\s*#(?:0x31d|797)(?:\s|;|$)')
R56_SHA = '1a8bb46e9f78fec2147b3242fab5651e470f86ba0347f4f11c7f1aedd5008777'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def audit(base, out):
    out.mkdir(parents=True, exist_ok=True)
    metadata = json.loads((base / 'metadata.json').read_text())
    descriptor = json.loads((base / 'descriptor.json').read_text())['descriptor']
    hashes = {name: {'path': str((base / name).resolve()), 'sha256': digest(base / name)}
              for name in ['object.o', 'metadata.json', 'descriptor.json']}
    command = ['xcrun', 'llvm-objdump', '-d', '--disassemble-zeroes',
               '--no-show-raw-insn', str(base / 'object.o')]
    (out / 'disassembly-command.json').write_text(json.dumps(command, indent=2) + '\n')
    opcodes = collections.Counter()
    # Keep only bounded context around explicit callback site arguments. The
    # complete >3-million-instruction assembly need not be retained in memory.
    previous = collections.deque(maxlen=192)
    windows = []
    pending = []
    anchors = []
    sections = collections.Counter()
    section = None
    function = None
    functions = collections.Counter()
    unparsed = []
    with (out / 'objdump.stderr').open('w') as errors:
        proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=errors, text=True)
        for line in proc.stdout:
            if line.startswith('Disassembly of section '):
                section = line.strip().removeprefix('Disassembly of section ').rstrip(':')
            if re.match(r'^[0-9a-f]+ <.*>:$', line.strip()):
                function = line.strip().split(' <', 1)[1][:-2]
            match = INSN.match(line)
            if not match:
                if re.match(r'^\s*[0-9a-f]+:', line):
                    unparsed.append(line.rstrip())
                continue
            address, opcode, operands = match.groups()
            opcodes[opcode] += 1
            sections[section] += 1
            functions[function] += 1
            for window in pending:
                window['lines'].append(line)
                window['remaining'] -= 1
            pending = [window for window in pending if window['remaining'] > 0]
            # In the maintained helper ABI x1 carries the exact source call
            # site. Site797 is still independently checked against descriptor.
            if opcode == 'mov' and SITE.match(operands):
                address = int(address, 16)
                anchors.append(address)
                window = {'anchor': address, 'lines': list(previous) + [line], 'remaining': 192}
                windows.append(window)
                pending.append(window)
            previous.append(line)
        if proc.wait() != 0:
            raise RuntimeError(f'objdump failed: {out / "objdump.stderr"}')
    (out / 'site797-helper-anchor-context.asm').write_text(
        '; These windows are anchored by mov x1,#797, the source-call argument.\n'
        '; Inspect actual successful bounds edge and byte load before treating\n'
        '; them as the precise native source797 load. No addresses are reused\n'
        '; blindly across Objects. Overlapping windows are intentional.\n\n' +
        ''.join(f'; Anchor 0x{w["anchor"]:x}\n' + ''.join(w['lines']) + '\n' for w in windows))
    if hashes['object.o']['sha256'] == R56_SHA:
        focus = ['xcrun', 'llvm-objdump', '-d', '--start-address=0x201a0',
                 '--stop-address=0x203f0', str(base / 'object.o')]
        (out / 'verified-R56-site797.asm').write_text(subprocess.check_output(focus, text=True))
        (out / 'verified-R56-site797-command.json').write_text(json.dumps(focus, indent=2) + '\n')
    plan = descriptor['plan']
    site = plan['blocks'][797]
    regions = [r for r in metadata['compiled_readonly_numeric_loops'] if r['header'] == 775]
    report = {
        'scope': 'Static encoded instruction counts, not dynamic visits, retired instructions, timing or a performance prediction.',
        'inputs': hashes,
        'metadata': {key: metadata[key] for key in [
            'code_bytes', 'stack_bytes', 'object_bytes', 'entry_symbol',
            'typed_ir_sha256', 'typed_ir_version', 'work_policy_version',
            'abi_version', 'program_revision', 'object_optimization']},
        'descriptor': {'code': descriptor['code'], 'stack': descriptor['stack'],
                       'format_version': json.loads((base / 'descriptor.json').read_text())['version']},
        'metadata_descriptor_agree': metadata['code_bytes'] == descriptor['code'] and metadata['stack_bytes'] == descriptor['stack'],
        'static_instruction_count': sum(opcodes.values()),
        'static_instruction_bytes_at_four_bytes_each': sum(opcodes.values()) * 4,
        'opcodes': dict(sorted(opcodes.items())),
        'UDIV': opcodes['udiv'], 'MSUB': opcodes['msub'],
        'sections': dict(sections), 'functions': dict(functions),
        'unparsed_instruction_lines': unparsed,
        'site797_helper_argument_anchors': [hex(address) for address in anchors],
        'source797_action': site,
        'compiled_header775': regions,
    }
    (out / 'audit.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    before = audit(args.before, args.out / 'before')
    if args.after:
        after = audit(args.after, args.out / 'after')
        keys = sorted(set(before['opcodes']) | set(after['opcodes']))
        report = {
            'scope': 'Static before/after encoded instructions; no dynamic or elapsed claims.',
            'before': before['inputs'], 'after': after['inputs'],
            'static_instructions': {'before': before['static_instruction_count'], 'after': after['static_instruction_count'],
                                    'delta': after['static_instruction_count'] - before['static_instruction_count']},
            'opcode_delta': {key: after['opcodes'].get(key, 0) - before['opcodes'].get(key, 0) for key in keys},
            'code_bytes': {'before': before['metadata']['code_bytes'], 'after': after['metadata']['code_bytes'],
                           'delta': after['metadata']['code_bytes'] - before['metadata']['code_bytes']},
            'stack_bytes': {'before': before['metadata']['stack_bytes'], 'after': after['metadata']['stack_bytes'],
                            'delta': after['metadata']['stack_bytes'] - before['metadata']['stack_bytes']},
            'same_typed_ir': before['metadata']['typed_ir_sha256'] == after['metadata']['typed_ir_sha256'],
            'UDIV': {'before': before['UDIV'], 'after': after['UDIV'], 'delta': after['UDIV'] - before['UDIV']},
            'MSUB': {'before': before['MSUB'], 'after': after['MSUB'], 'delta': after['MSUB'] - before['MSUB']},
        }
        (args.out / 'comparison.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    else:
        report = {key: before[key] for key in ['static_instruction_count', 'UDIV', 'MSUB', 'metadata', 'metadata_descriptor_agree']}
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
