"""Replace one packed-sort body in a separately pinned copy of the codec."""
from hashlib import sha256
from pathlib import Path
import argparse
import re

SOURCE = '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
OLD = '''huff-packed-sort-symbols(n<u64>, freqs<[u32]>, freq-start<u64>, scratch<[u32]>) => (out<([u32],u64)>) :=
  (words, used) := huff-pack-active-symbols(freqs, freq-start, n, 0u64, scratch, 0u64)
  sorted := huff-sort-key-gaps(words, used, 4362938489u64)
  out := (sorted, used).
'''
NEW = '''huff-packed-sort-symbols(n<u64>, freqs<[u32]>, freq-start<u64>, scratch<[u32]>) => (out<([u32],u64)>) :=
  top := n - 1u64
  counters := [0u64 | i <- 0u64..=n, i < n]
  counted := huff-packed-count-symbols(freqs, freq-start, n, counters, 0u64, top)
  (cumulative, used) := huff-cumulative(counted, n, 0u64, 1u64)
  (placed, words) := huff-packed-place-symbols(freqs, freq-start, n, cumulative, scratch, 0u64, top)
  base := huff-get64(placed, n - 2u64)
  size := huff-get64(placed, top) - base
  sorted := huff-heap-sort(words, base, size)
  out := (sorted, used).
'''

HYBRID = '''huff-packed-sort-symbols(n<u64>, freqs<[u32]>, freq-start<u64>, scratch<[u32]>) => (out<([u32],u64)>) :=
  (words, used) := huff-pack-active-symbols(freqs, freq-start, n, 0u64, scratch, 0u64)
  buckets := n >= 64u64 && used >= 32u64
  out := (buckets?
    | true => huff-packed-sort-buckets(n, freqs, freq-start, words)
    | false => (huff-sort-key-gaps(words, used, 4362938489u64), used)).
'''

COUNTER_HELPERS = ['huff-count-active', 'huff-count-step', 'huff-count-symbols',
    'huff-cumulative-step', 'huff-cumulative', 'huff-place-nonzero', 'huff-place-step',
    'huff-place-symbols', 'huff-packed-count-step', 'huff-packed-count-symbols',
    'huff-packed-place-step', 'huff-packed-place-symbols']


def narrow_counters(source):
    for name in COUNTER_HELPERS:
        start = re.search(r'(?m)^'+re.escape(name)+r'\(', source).start()
        header_end = source.index('\n', start)+1
        end = header_end+re.search(r'(?m)^[a-z][a-z0-9-]*\(', source[header_end:]).start()
        old = source[start:end]
        assert '[u64]' in old
        new = old.replace('[u64]', '[u32]')
        if name == 'huff-count-active':
            new = new.replace('huff-get64(counts, slot) + 1u64', 'huff-add-wrap32(huff-get32(counts, slot), 1u32)')
            new = new.replace('huff-set64(counts, slot, count)', 'huff-set32(counts, slot, count)')
        if name == 'huff-cumulative-step':
            new = new.replace('huff-get64(counts, i)', 'huff-word(huff-get32(counts, i))')
            new = new.replace('huff-set64(counts, i, used)', 'huff-set32(counts, i, huff-packed(used))')
        if name == 'huff-place-nonzero':
            new = new.replace('at := huff-get64(counts, slot)', 'at<u64> := huff-get32(counts, slot)')
            new = new.replace('huff-set64(counts, slot, at + 1u64)', 'huff-set32(counts, slot, huff-packed(at + 1u64))')
        assert source.count(old) == 1
        source = source.replace(old, new)
    replacements = {
        'counters := [0u64 | i <- 0u64..=n, i < n]': 'counters := [0u32 | i <- 0u64..=n, i < n]',
        'base := huff-get64(placed, n - 2u64)': 'base := huff-word(huff-get32(placed, n - 2u64))',
        'size := huff-get64(placed, top) - base': 'size := huff-word(huff-get32(placed, top)) - base',
    }
    for old, new in replacements.items():
        assert source.count(old) == 2, (old, source.count(old))
        source = source.replace(old, new)
    return source


def generate(source, role='buckets'):
    assert sha256(source.encode()).hexdigest() == SOURCE and source.count(OLD) == 1
    assert role in ('buckets', 'hybrid', 'hybrid32')
    replacement = NEW if role == 'buckets' else NEW.replace('huff-packed-sort-symbols(', 'huff-packed-sort-buckets(', 1)+'\n'+HYBRID
    result = source.replace(OLD, replacement)
    return narrow_counters(result) if role == 'hybrid32' else result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--role', choices=['buckets','hybrid','hybrid32'], default='buckets')
    args = parser.parse_args()
    args.output.write_text(generate(args.source.read_text(), args.role))
