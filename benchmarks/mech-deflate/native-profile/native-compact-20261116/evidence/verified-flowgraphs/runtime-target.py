"""Read runtime margins from the two exact selected-worker orders."""
from hashlib import sha256
from importlib import import_module
from pathlib import Path
from struct import unpack_from
import sys

from production import HERE, Q, ROADMAP, read, save, sha

FRAMES = ROADMAP / 'frame-copy-v2'
ARCHIVE = (Q / 'native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116'
           / 'evidence/match-cache-partition-cause/archived-costs.json')
SYMBOLS = ('_mech_compact_numeric_475_vector', '_mech_compact_numeric_3009_no_growth_composed')
sys.path.insert(0, str(Q))
macho = import_module('reachability-census')


def atoms(path):
    image, sections, symbols = macho.load(path)
    _, base, size, offset, reloc_at, reloc_count = sections[macho.TEXT]
    names = macho.text_symbols(sections, symbols)
    result = {}
    for name in SYMBOLS:
        start = names[name]
        end = min([at for at in names.values() if at > start] + [base + size])
        relocs = []
        for index in range(reloc_count):
            address, flags = unpack_from('<II', image, reloc_at + 8 * index)
            if start <= base + address < end:
                target = symbols[flags & 0xFFFFFF][0] if flags >> 27 & 1 else flags & 0xFFFFFF
                relocs.append((base + address - start, flags >> 24, target))
        result[name] = {'extent': end - start, 'raw_sha256': sha256(image[offset + start - base:offset + end - base]).hexdigest(),
                        'relocations': sorted(relocs)}
    return result


def main():
    full = read(FRAMES / 'full/full-replay.json')
    worker = FRAMES / 'bound/worker-checks/shared-frame-copies/mech-worker'
    current = worker.with_name('program.o')
    old = FRAMES / 'bound/worker-checks/control/program.o'
    published = read(FRAMES / 'bound/binding.json')['artifacts']['shared-frame-copies']
    assert sha(worker) == published['worker_sha256'] and sha(current) == published['object_sha256']
    assert full['decision'] == 'Retain' and len(full['runs']) == 2
    orders = {}
    receipts = {}
    for run in full['runs']:
        folder = Q / run['id']
        pin_file = folder / 'pins.json'
        summary = folder / 'summary.json'
        pins = read(pin_file)
        assert pins['actors']['mech-experiment']['binary_sha256'] == sha(worker)
        done = read(folder / 'complete.json')
        assert done['cases'] == 81 and done['balanced_blocks_per_case'] == 96
        assert done['all_outputs_independently_inflated'] and done['all_input_and_output_aliases_preserved']
        rows = read(summary)
        assert len(rows) == 81
        orders[run['order']] = {row['case']: row for row in rows}
        receipts[run['order']] = {
            'pins_sha256': sha(pin_file), 'summary_sha256': sha(summary),
            'roc_wins': run['peers']['roc']['point_estimate_wins']['mech-experiment'],
            'libdeflate_wins': run['peers']['libdeflate']['point_estimate_wins']['mech-experiment']}
    assert orders['forward'].keys() == orders['reverse'].keys()
    cases = []
    for name in orders['forward']:
        ratios = {order: rows[name]['paired_x_libdeflate']['mech-experiment'] for order, rows in orders.items()}
        cases.append({'case': name, 'paired_x_libdeflate': ratios,
                      'worst_reduction_required': max(0., 1. - 1. / max(ratios.values()))})
    cases.sort(key=lambda row: row['worst_reduction_required'])
    lost = [row for row in cases if min(row['paired_x_libdeflate'].values()) >= 1]
    assert len(lost) == 20 and lost[0]['case'] == 'prefix-natural-8192'
    archive = read(ARCHIVE)
    assert sha(old) == archive['exact_reference']['object_sha256']
    old_symbols = atoms(old)
    new_symbols = atoms(current)
    for symbol, old_symbol in old_symbols.items():
        # The existing archive records relocation triples as JSON arrays.
        assert [list(reloc) for reloc in old_symbol['relocations']] == archive['exact_reference']['atoms'][symbol]['relocations']
        assert old_symbol['raw_sha256'] == archive['exact_reference']['atoms'][symbol]['raw_sha256']
    report = {
        'worker_sha256': sha(worker), 'encoder_sha256': sha(current), 'orders': receipts,
        'case_margins': cases, 'nearest_losses': lost[:4],
        'archived_counts': archive['cases']['prefix-natural-8192'],
        'archived_counts_source_sha256': sha(ARCHIVE), 'matching_atoms': {name: old_symbols[name] == new_symbols[name] for name in SYMBOLS},
        'archived_atoms': old_symbols, 'current_atoms': new_symbols,
    }
    save(HERE / 'runtime-target.json', report)
    print(report['nearest_losses'], report['matching_atoms'], flush=True)


if __name__ == '__main__':
    main()
