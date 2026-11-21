"""Derive register weights using every static head, including unvisited blocks."""
import argparse
import bisect
from hashlib import sha256
import json
from pathlib import Path
import re
import struct

read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: sha256(Path(p).read_bytes()).hexdigest()


def kernel(path, start, extent):
    image = path.read_bytes()
    assert struct.unpack_from('<I', image)[0] == 0xfeedfacf
    at = 32
    for _ in range(struct.unpack_from('<I', image, 16)[0]):
        kind, size = struct.unpack_from('<II', image, at)
        if kind == 0x19:
            for index in range(struct.unpack_from('<I', image, at + 64)[0]):
                section = at + 72 + index * 80
                if image[section:section + 16].rstrip(b'\0') == b'__text':
                    address, length, offset = struct.unpack_from('<QQI', image, section + 32)
                    begin = start - address
                    assert 0 <= begin and begin + extent <= length
                    return image[offset + begin:offset + begin + extent]
        at += size
    raise AssertionError('missing native text')


def signed(word, shift, bits):
    value = (word >> shift) & ((1 << bits) - 1)
    return value - (1 << bits) if value & (1 << (bits - 1)) else value


def static_heads(body):
    """Decode direct AArch64 transfers; reject calls, indirect jumps and literals."""
    end = len(body)
    while struct.unpack_from('<I', body, end - 4)[0] == 0xd503201f:
        end -= 4
    heads = {0}
    for at in range(0, end, 4):
        word = struct.unpack_from('<I', body, at)[0]
        target, transfer = None, False
        assert word & 0xfc000000 != 0x94000000, 'call needs separate handling'
        assert word & 0xfffffc1f not in (0xd61f0000, 0xd63f0000), 'indirect transfer'
        assert word & 0x3b000000 != 0x18000000, 'literal pool needs separate handling'
        if word & 0xfc000000 == 0x14000000:
            target, transfer = at + 4 * signed(word, 0, 26), True
        elif word & 0xff000010 == 0x54000000 or word & 0x7e000000 == 0x34000000:
            target, transfer = at + 4 * signed(word, 5, 19), True
        elif word & 0x7e000000 == 0x36000000:
            target, transfer = at + 4 * signed(word, 5, 14), True
        elif (word & 0xfffffc1f == 0xd65f0000 or word in (0xd65f0bff, 0xd65f0fff)
              or word & 0xffe0001f == 0xd4200000 or word & 0xffff0000 == 0):
            transfer = True
        if target is not None:
            assert 0 <= target < end and target % 4 == 0
            heads.add(target)
        if transfer and at + 4 < end:
            heads.add(at + 4)
    return sorted(heads), end


def derive(folder):
    receipt = read(folder / 'receipt.json')
    symbol = 'mech_compact_numeric_475_vector'
    capture = folder / (symbol + '.vcode')
    vcode = capture.read_text()
    expected = vcode.split(';; REGALLOC_INPUT_BEGIN\n', 1)[1].split(';; REGALLOC_INPUT_END\n', 1)[0]
    labels = [(int(i), int(at)) for i, at in re.findall(r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ', vcode, re.M)]
    assert [i for i, _ in labels] == list(range(len(labels)))
    atom = receipt['layout']['symbols']['_' + symbol]
    assert not atom['relocations'] and sha(folder / 'frontend/program.o') == receipt['artifacts']['program.o']
    body = kernel(folder / 'frontend/program.o', atom['start'], atom['extent'])
    assert sha256(body).hexdigest() == atom['sha256']
    metadata = read(folder / 'frontend/metadata.json')
    phase, = [p['vector_variant'] for p in metadata['numeric_phases']
              if p.get('vector_variant', {}).get('symbol') == symbol]
    assert phase['code_bytes'] <= len(body)
    heads, end = static_heads(body[:phase['code_bytes']])
    counts_path = folder / 'counts-475/counts.json'
    counts = read(counts_path)
    assert counts['object_sha256'] == receipt['artifacts']['program.o']
    assert all(0 <= at <= end for _, at in labels)
    weights = [0.0] * len(labels)
    differences = {}
    for case, sample in counts['cases'].items():
        visits = {int(b['address'], 16) - atom['start']: b['visits'] for b in sample['blocks']}
        assert set(visits) <= set(heads) and visits[0] == sample['calls']
        mapped = [heads[bisect.bisect_right(heads, at) - 1] if at < end else None for _, at in labels]
        observed = [visits.get(at, 0) for at in mapped]
        assert observed[0] == sample['calls']
        historical_heads = sorted(visits)
        historical = [visits[historical_heads[bisect.bisect_right(historical_heads, at) - 1]] for _, at in labels]
        differences[case] = {'labels_with_changed_counts': sum(a != b for a, b in zip(observed, historical)),
                             'zero_labels_previously_nonzero': sum(a == 0 < b for a, b in zip(observed, historical)),
                             'visited_static_heads': len(visits), 'complete_static_heads': len(heads)}
        total = sum(visits.values())
        assert total > 0
        for index, value in enumerate(observed):
            weights[index] += value / total / len(counts['cases'])
    peak = max(weights)
    profiles = {}
    for name, power, maximum in [('linear-4096', 1.0, 4096), ('linear-65536', 1.0, 65536), ('sqrt-65536', 0.5, 65536)]:
        profiles[name] = {'expected_vcode': expected,
                          'block_frequencies': [max(1, min(1 << 20, round((value / peak) ** power * maximum))) for value in weights],
                          'scope': 'Equal share of nine cases, mapped through the complete static CFG; absent block visits remain zero before the frequency floor. ' + name,
                          'power': power, 'maximum': maximum, 'cases': list(counts['cases']),
                          'training_object_sha256': receipt['artifacts']['program.o'],
                          'training_vcode_sha256': sha(capture), 'block_counts_sha256': sha(counts_path)}
    return profiles, {'training_object_sha256': receipt['artifacts']['program.o'],
                      'static_heads': heads, 'code_end': end, 'case_differences': differences,
                      'unrounded_weights': weights}


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('capture', type=Path, help='Extracted probe/raw-trace-mapped')
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    profiles, proof = derive(args.capture)
    for name, value in profiles.items():
        (args.output / (name + '.json')).write_text(json.dumps(value, indent=2) + '\n')
    (args.output / 'derivation.json').write_text(json.dumps(proof, indent=2) + '\n')
    print(json.dumps(proof['case_differences'], indent=2))


if __name__ == '__main__':
    main()
