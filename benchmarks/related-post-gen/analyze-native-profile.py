#!/usr/bin/env python3
"""Archive an own-process native profile with exact linked-code verification."""
import argparse
import collections
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--measured', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True, help='Output prefix')
    parser.add_argument('--unqualified', action='store_true')
    args = parser.parse_args()
    p, prefix = args.profile.resolve(), args.out.resolve()
    prefix.parent.mkdir(parents=True, exist_ok=True)
    spec = importlib.util.spec_from_file_location('native_link_audit', Path(__file__).with_name('audit-native-link.py'))
    audit = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(audit)
    sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
    pin = lambda path: {'path': str(path), 'sha256': sha(path)}
    write = lambda path, obj: path.write_text(json.dumps(obj, indent=2) + '\n')
    receipt = json.loads((p / 'receipt.json').read_text())
    assert receipt['status'] == 'passed' and receipt['all_iterations_indices_equal_checked_inside_host']
    assert receipt['full_final_parsed_json_equals_rust']
    stats = json.loads((p / 'actual-5000.json.sampler.json').read_text())
    assert stats['suspended'] == stats['resumed'] == stats['got_state'] + stats['dropped_disabled']
    assert stats['got_state'] == stats['records'] + 1
    assert stats['suspend_failures'] == stats['state_failures'] == stats['resume_failures'] == 0
    assert stats['record_bytes'] == struct.calcsize('<5Q2I64Q') == 560
    data = list(struct.iter_unpack('<5Q2I64Q', (p / 'actual-5000.json.samples.bin').read_bytes()))
    assert len(data) == stats['records']
    symbols = {int(s[0]): s for l in (p / 'actual-5000.json.symbols.tsv').read_text().splitlines() if (s := l.split('\t'))}
    names = sorted(set(s[3] for s in symbols.values()))
    result = subprocess.run(['/opt/homebrew/opt/llvm/bin/llvm-cxxfilt', '-n'], input='\n'.join(names) + '\n', capture_output=True, text=True, check=True)
    demangled = dict(zip(names, result.stdout.splitlines()))
    write(p / 'demangled-names.json', demangled)
    proofs = []
    for label, executable in [('measured', args.measured.resolve()), ('diagnostic', p / 'runner-sampler')]:
        proof = audit.audit(p / 'build/object.o', executable)
        path = p / (label + '-byte-proof.json')
        write(path, proof)
        proofs.append(proof)
    assert proofs[0]['normalized_text_sha256'] == proofs[1]['normalized_text_sha256']
    _, sections, object_symbols = audit.macho(p / 'build/object.o')
    native_symbols = {s['name'].removeprefix('_'): s['value'] for s in object_symbols if s['name'].startswith('_mech_canonical_program_entry_v1')}
    runtime_symbols = {s[3]: int(s[2]) for s in symbols.values() if s[3] in native_symbols}
    slides = {base - native_symbols[name] for name, base in runtime_symbols.items()}
    assert len(slides) == 1
    slide = slides.pop()
    text = next(s for s in sections if s['name'] == '__text')
    native = lambda address: text['address'] + slide <= address < text['address'] + text['size'] + slide
    name = lambda address: demangled.get(symbols.get(address, ['', '', '', '(unknown)'])[3], '(unknown)')
    leaf, inclusive, native_pcs, stops = (collections.Counter() for _ in range(4))
    for row in data:
        timestamp, pc, lr, sp, fp, n, reason = row[:7]
        assert stats['stack_low'] <= sp < stats['stack_high'] and n <= 64
        frames = list(row[7:7+n])
        leaf[name(pc)] += 1
        stops[reason] += 1
        if native(pc):
            native_pcs[pc - slide] += 1
        active = [pc] + frames
        if not native(pc) and native(lr) and not any(native(a) for a in frames):
            active.append(lr)
        inclusive.update(set(name(a) for a in active))
    result = subprocess.run(['/opt/homebrew/opt/llvm/bin/llvm-objdump', '-d', '--no-show-raw-insn', str(p / 'build/object.o')], capture_output=True, text=True, check=True)
    (p / 'object-disassembly.txt').write_text(result.stdout)
    instructions = {}
    for line in result.stdout.splitlines():
        if m := re.match(r'^\s*([0-9a-f]+):\s+(\w+)\s*(.*)', line):
            instructions[int(m[1], 16)] = (m[2], m[3])
    mnemonics = collections.Counter()
    for pc, count in native_pcs.items():
        assert pc in instructions, hex(pc)
        mnemonics[instructions[pc][0]] += count
    total = len(data)
    rows = lambda counter: [{'name': name, 'samples': count, 'percent': 100 * count / total} for name, count in counter.most_common()]
    archive = []
    for file in ['receipt.json', 'build.json', 'runner.rs', 'capture.py', 'sampler.c', 'sdk-definitions.json', 'pause-safety-review.json', 'measured-byte-proof.json', 'diagnostic-byte-proof.json', 'runner.stdout.log', 'runner.stderr.log', 'actual-5000.json.sampler.json']:
        target = Path(str(prefix) + '-' + file)
        shutil.copy2(p / file, target)
        archive.append(pin(target))
    for file in ['actual-5000.json.samples.bin', 'actual-5000.json.symbols.tsv', 'demangled-names.json', 'object-disassembly.txt']:
        target = Path(str(prefix) + '-' + file + '.gz')
        target.write_bytes(gzip.compress((p / file).read_bytes(), mtime=0))
        archive.append({**pin(target), 'uncompressed_sha256': sha(p / file)})
    summary = {
        'schema': 'mech.related-post-gen.native-profile.v2', 'status': 'passed',
        'qualified_backend': not args.unqualified, 'profiled_timings_excluded': True,
        'samples': total, 'completed_5000_post_evaluations': receipt['completed_iterations'],
        'all_25000_indices_equal_every_iteration': True, 'full_final_json_equals_rust': True,
        'fresh_inputs_aliases_and_zero_memory_node_cleanup_every_iteration': True,
        'native_text_bytes': text['size'], 'exact_text_and_referenced_literal_bytes_verified': True,
        'native_leaf_samples': sum(native_pcs.values()), 'native_leaf_percent': 100 * sum(native_pcs.values()) / total,
        'leaf_counts': rows(leaf), 'inclusive_frame_counts': rows(inclusive),
        'native_instruction_counts': rows(mnemonics), 'frame_stop_reasons': dict(stops),
        'hot_native_pcs': [{'object_offset': pc, 'hex': hex(pc), 'samples': count,
                            'instruction': ' '.join(instructions[pc])} for pc, count in native_pcs.most_common(40)],
        'measured': pin(args.measured.resolve()), 'source': receipt['source'],
        'caveats': ['Statistical samples conditioned on enabled evaluate_into, not exact CPU shares or hardware counters.',
                    'Suspension disturbs execution. Profiled elapsed values are excluded from performance cohorts.',
                    'Inclusive physical frames overlap; inline, tail-called, or frame-pointer-elided functions may be absent.',
                    'LR is added only for an external leaf lacking a physical native ancestor; stale completed native LR is excluded.'],
        'artifacts': archive,
    }
    if args.unqualified:
        summary['caveats'].append('LLVM backend remains unqualified: original descriptor does not admit LLVM code/stack/backend identity.')
    write(Path(str(prefix) + '-summary.json'), summary)
    print(json.dumps({k: summary[k] for k in ['samples', 'native_leaf_percent', 'qualified_backend', 'hot_native_pcs']}, indent=2))


if __name__ == '__main__':
    main()
