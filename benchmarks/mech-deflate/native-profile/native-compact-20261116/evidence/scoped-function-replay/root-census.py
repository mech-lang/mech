"""Measure attributed encoder text from the exact Apple arm64 object."""
from collections import defaultdict
from hashlib import sha256
import json
from pathlib import Path
import struct
import sys


def census(path):
    data = path.read_bytes()
    magic, cpu, _, filetype, command_count, commands_size = struct.unpack_from('<6I', data)
    assert (magic, cpu, filetype) == (0xFEEDFACF, 0x0100000C, 1)
    sections = []
    cursor = 32
    symbols = None
    for _ in range(command_count):
        command, size = struct.unpack_from('<II', data, cursor)
        if command == 0x19:
            count, = struct.unpack_from('<I', data, cursor + 64)
            for number in range(count):
                name, segment, address, length = struct.unpack_from('<16s16sQQ', data, cursor + 72 + 80 * number)
                sections.append((name.rstrip(b'\0'), segment.rstrip(b'\0'), address, length))
        if command == 2:
            symbols = struct.unpack_from('<4I', data, cursor + 8)
        cursor += size
    assert cursor == 32 + commands_size and symbols is not None
    section_id, start, length = next((i + 1, address, length) for i, (n, s, address, length) in enumerate(sections)
                                     if (s, n) == (b'__TEXT', b'__text'))
    entries, count, strings, string_bytes = symbols
    names = defaultdict(list)
    for i in range(count):
        at, kind, section, _, address = struct.unpack_from('<IBBHQ', data, entries + i * 16)
        if section != section_id or kind & 0xEE != 0x0E:
            continue
        assert at < string_bytes and start <= address < start + length
        name = data[strings + at:data.index(b'\0', strings + at, strings + string_bytes)].decode()
        if name.startswith('_'):
            names[address].append(name)
    boundaries = sorted(names)
    assert boundaries[0] == start
    extents = [{'names': names[address], 'offset': address - start, 'bytes': end - address}
               for address, end in zip(boundaries, boundaries[1:] + [start + length], strict=True)]
    assert sum(row['bytes'] for row in extents) == length
    root = next(row['bytes'] for row in extents if '_mech_compact_closed_checked' in row['names'])
    return {'object_sha256': sha256(data).hexdigest(), 'encoder_text_bytes': length, 'checked_root_extent_bytes': root,
            'libdeflate_text_bytes': 37080, 'root_share_of_encoder_text': root / length,
            'symbol_extents_include_alignment': True,
            'text_symbols': sorted(extents, key=lambda row: row['bytes'], reverse=True)}


if __name__ == '__main__':
    print(json.dumps(census(Path(sys.argv[1])), indent=2))
