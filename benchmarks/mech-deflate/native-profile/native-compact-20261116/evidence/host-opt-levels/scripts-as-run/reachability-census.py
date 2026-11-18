from collections import Counter, defaultdict
from bisect import bisect_right
from pathlib import Path
from struct import unpack_from
import json
import sys

TEXT = (b'__TEXT', b'__text')
ROOT = '_mech_compact_closed_checked'

def load(file):
    data = file.read_bytes()
    assert unpack_from('<II', data) == (0xfeedfacf, 0x0100000c)
    sections = {}
    table = None
    command_at = 32
    section_id = 1
    for _ in range(unpack_from('<I', data, 16)[0]):
        command, length = unpack_from('<II', data, command_at)
        if command == 0x19:
            for i in range(unpack_from('<I', data, command_at + 64)[0]):
                at = command_at + 72 + 80 * i
                sect, seg, pc, length_bytes, file_offset, align, reloc_at, reloc_count = unpack_from(
                    '<16s16sQQIIII', data, at
                )
                sections[seg.rstrip(b'\0'), sect.rstrip(b'\0')] = (
                    section_id, pc, length_bytes, file_offset, reloc_at, reloc_count
                )
                section_id += 1
        elif command == 2:
            table = unpack_from('<IIII', data, command_at + 8)
        command_at += length
    sym_at, n, string_at, string_len = table
    symbols = []
    for i in range(n):
        offset, kind, section, _, pc = unpack_from('<IBBHQ', data, sym_at + i * 16)
        start = string_at + offset
        end = data.index(b'\0', start, string_at + string_len)
        name = data[start:end].decode('utf-8', errors='replace')
        symbols.append((name, kind, section, pc))
    return data, sections, symbols

def text_symbols(sections, symbols):
    index, *_ = sections[TEXT]
    return {name: pc for name, kind, section, pc in symbols if section == index and kind & 0xe == 0xe}

def census(folder):
    image, sections, symbols = load(folder / 'program.o')
    linked_image, linked_sections, linked_symbols = load(folder / 'mech-worker')
    text = text_symbols(sections, symbols)
    linked = text_symbols(linked_sections, linked_symbols)
    root_pc = text[ROOT]
    _, text_pc, text_size, offset, reloc_at, reloc_count = sections[TEXT]
    starts = sorted((pc, n) for n, pc in text.items())
    paddings = {}
    for (start, name), (end, _) in zip(starts, starts[1:] + [(text_pc + text_size, None)]):
        paddings[name] = end - start
    names = [name for name in text if name not in linked]
    assert all(name.startswith('_mech_compact_numeric_') for name in names)

    relocations = defaultdict(list)
    root_calls = Counter()
    start_pc = [pc for pc, _ in starts]
    for i in range(reloc_count):
        at, flags = unpack_from('<II', image, reloc_at + i * 8)
        start, owner = starts[bisect_right(start_pc, at + text_pc) - 1]
        ext = flags >> 27 & 1
        kind = flags >> 28
        target = symbols[flags & 0xffffff][0] if ext else flags & 0xffffff
        relocations[owner].append((at + text_pc - start, flags >> 24, target))
        if kind == 2 and owner == ROOT:
            root_calls[target] += 1

    dedup = defaultdict(list)
    for name, pc in text.items():
        if name in linked and name.startswith('_mech_compact_numeric_'):
            at = offset + pc - text_pc
            signature = image[at:at + paddings[name]], tuple(relocations[name])
            dedup[signature].append(name)
    records = json.loads((folder / 'metadata.json').read_text())
    return {
        'object_bytes': len(image),
        'object_text_bytes': text_size,
        'linked_worker_bytes': len(linked_image),
        'linked_text_bytes': linked_sections[TEXT][2],
        'checked_root_body_bytes': records['code_bytes'],
        'checked_root_symbol_extent': paddings[ROOT],
        'unlinked_numeric_functions': len(names),
        'unlinked_numeric_extents_bytes': sum(paddings[name] for name in names),
        'unlinked_numeric_names': sorted(names),
        'retained_numeric_extents_bytes': sum(
            size for name, size in paddings.items()
            if name.startswith('_mech_compact_numeric_') and name in linked
        ),
        'direct_calls_from_root': sum(root_calls.values()),
        'largest_root_call_groups': root_calls.most_common(22),
        'largest_retained_numeric_extents': sorted(
            ((name, size) for name, size in paddings.items()
             if name.startswith('_mech_compact_numeric_') and name in linked),
            key=lambda item: -item[1],
        )[:18],
        'exact_matching_reachable_numeric_extents': [
            {'bytes': len(signature[0]), 'names': names}
            for signature, names in sorted(dedup.items(), key=lambda item: -len(item[1]) * len(item[0][0]))
            if len(names) > 1
        ],
    }

if __name__ == '__main__':
    print(json.dumps(census(Path(sys.argv[1])), indent=2))
