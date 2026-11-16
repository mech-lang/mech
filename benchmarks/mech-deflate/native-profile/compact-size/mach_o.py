"""Relocatable Apple ARM64 object sections and symbols."""
from struct import unpack_from

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
