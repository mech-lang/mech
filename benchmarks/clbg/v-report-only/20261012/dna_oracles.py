"""Additive exact-token FASTA oracle; frozen original oracles stay unchanged."""
from collections import Counter
import re

WIDTHS = (1, 2, 3, 4, 6, 12, 18)
QUERIES = ('GGT', 'GGTA', 'GGTATT', 'GGTATTTTAATT', 'GGTATTTTAATTTATAGT')


def target_sequence(data):
    selected = False
    parts = []
    for line in re.split(rb'\r\n|\r|\n', data):
        if line.startswith(b';'):
            continue
        if line.startswith(b'>'):
            if selected:
                break
            selected = line == b'>THREE' or line.startswith(b'>THREE ') or line.startswith(b'>THREE\t')
        elif selected:
            parts.append(line.upper())
    sequence = b''.join(parts).decode('ascii')
    assert set(sequence) <= set('ACGT')
    return sequence


def nucleotide_maps(data):
    sequence = target_sequence(data)
    return sequence, [Counter(sequence[i:i + width] for i in range(max(0, len(sequence) - width + 1)))
                      for width in WIDTHS]


def knucleotide(data):
    _, maps = nucleotide_maps(data)
    output = []
    for counts in maps[:2]:
        total = sum(counts.values())
        for key, count in sorted(counts.items(), key=lambda entry: (-entry[1], entry[0])):
            output.append(f'{key} {100.0 * count / total:.3f}\n')
        output.append('\n')
    output.extend(f'{counts[query]}\t{query}\n' for counts, query in zip(maps[2:], QUERIES))
    return ''.join(output).encode()
