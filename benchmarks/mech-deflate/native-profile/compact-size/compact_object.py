"""Compose native text for the historical size benchmark recipe."""

from struct import pack, pack_into, unpack_from

import mach_o as macho


code = macho.TEXT
const = (b'__TEXT', b'__const')
pool = (b'__TEXT', b'__literal16')


def tables(data):
    assert unpack_from('<I', data, 12) == (1,)  # MH_OBJECT
    by_type = {}
    pos = 32
    for _ in range(unpack_from('<I', data, 16)[0]):
        kind, size = unpack_from('<II', data, pos)
        assert size >= 8 and kind not in by_type
        by_type[kind] = pos
        pos += size
    assert pos == 32 + unpack_from('<I', data, 20)[0]
    sections = {}
    segment = by_type[0x19]
    for ix in range(unpack_from('<I', data, segment + 64)[0]):
        at = segment + 72 + 80 * ix
        section, name = unpack_from('<16s16s', data, at)
        sections[name.rstrip(b'\0'), section.rstrip(b'\0')] = at
    return by_type, sections


def relocs(data, section):
    _, _, _, _, start, count = section
    return [unpack_from('<II', data, start + 8 * ordinal) for ordinal in range(count)]


def replace_text(prepared, compiled, destination, *, symbol=None, coalesce=()):
    """Replace the checked root or one numeric definition and retain relocations."""
    original, first_sections, first_symbols = macho.load(prepared)
    alternative, second_sections, second_symbols = macho.load(compiled)
    assert set(first_sections) == {code, const}
    assert set(second_sections) <= {code, pool, (b'__LD', b'__compact_unwind'), (b'__TEXT', b'__eh_frame')}
    first_commands, first_headers = tables(original)
    _, second_headers = tables(alternative)
    assert set(first_commands) == {0x19, 2, 11}
    assert not any(unpack_from('<12I', original, first_commands[11] + 32))
    root = f'_{symbol}' if symbol else macho.ROOT
    coalesced = {f'_{name}' for name in coalesce}
    assert not (symbol and coalesced) and root not in coalesced
    first_starts = macho.text_symbols(first_sections, first_symbols)
    second_starts = macho.text_symbols(second_sections, second_symbols)
    first_index = {name: ix for ix, (name, _, _, _) in enumerate(first_symbols)}
    assert len(first_index) == len(first_symbols)
    root_start = first_starts[root]
    root_end = min(pos for pos in first_starts.values() if pos > root_start)
    assert all(name == root or not root_start <= address < root_end for name, address in first_starts.items())
    assert second_starts[root] == 0
    _, text_addr, text_size, text_offset, text_reloc_at, text_reloc_count = first_sections[code]
    data_sid, data_addr, data_size, data_offset, _, data_reloc_count = first_sections[const]
    alt_sid, alt_addr, alt_size, alt_offset, _, _ = second_sections[code]
    assert text_addr == alt_addr == 0 and data_reloc_count == 0
    assert 0 <= root_start < root_end <= text_size
    alignment = 1 << unpack_from('<I', original, first_headers[code] + 52)[0]
    assert root_start % alignment == root_end % alignment == 0
    assert alignment % 4 == 0 and alt_size % 4 == 0
    assert all(
        name in (root, 'ltmp0') or name in coalesced or name.startswith('_OUTLINED_FUNCTION_')
        for name in second_starts
    )
    replacement = bytearray(alternative[alt_offset:alt_offset + alt_size])
    padded_size = (alt_size + alignment - 1) // alignment * alignment
    remove = []
    for name in coalesced:
        # A single-use body may be completely inlined by LLVM.
        assert name in second_starts or not any(symbol[0] == name for symbol in second_symbols), name
        _, flags, section, address = first_symbols[first_index[name]]
        assert flags & 1 == 0 and section == first_sections[code][0] and address == first_starts[name]
        end = min(next_address for next_address in first_starts.values() if next_address > address)
        assert address % alignment == end % alignment == 0
        remove.append((address, end, 0))
    rewrites = sorted(remove + [(root_start, root_end, padded_size)])
    assert all(a[1] <= b[0] for a, b in zip(rewrites, rewrites[1:]))


    def moved(address):
        return address + sum(width - (end - start) for start, end, width in rewrites if end <= address)


    new_root_start = moved(root_start)
    delta = moved(text_size) - text_size
    assert len(replacement) == alt_size and (delta < 0)

    pool_bytes = b''
    pool_sid = None
    pool_at_from_anchor = None
    if pool in second_sections:
        pool_sid, pool_pc, pool_size, file_at, _, pool_relocs = second_sections[pool]
        pool_alignment = 1 << unpack_from('<I', alternative, second_headers[pool] + 52)[0]
        const_alignment = 1 << unpack_from('<I', original, first_headers[const] + 52)[0]
        assert pool_relocs == 0 and pool_alignment <= const_alignment
        assert pool_size % pool_alignment == 0
        const_end = data_addr + delta + data_size
        pool_start = (const_end + pool_alignment - 1) & -pool_alignment
        pool_bytes = bytes(pool_start - const_end) + alternative[file_at:file_at + pool_size]
        # Extend the final constant atom. Other constant symbols keep their
        # original offsets and their actual contents.
        anchor, anchor_ix = max((pc, ix) for ix, (_, kind, section, pc) in enumerate(first_symbols)
                                if section == data_sid and kind & 0xe == 0xe)
        assert data_addr <= anchor < data_addr + data_size
        pool_at_from_anchor = pool_start - (anchor + delta)

    # Preserve original relocation order for paired ADDEND and page records.
    old_relocs = relocs(original, first_sections[code])
    fixed_relocs = []
    for at, bits in old_relocs:
        kind = bits >> 28
        assert kind in (2, 3, 4, 5, 6, 10)
        assert bool(bits & (1 << 27)) == (kind != 10)
        if not any(start <= at < end for start, end, _ in rewrites):
            assert kind == 10 or first_symbols[bits & 0xffffff][0] not in coalesced
            fixed_relocs.append((moved(at), bits))

    alt_relocs = relocs(alternative, second_sections[code])
    local_branches = 0
    for ordinal, (at, bits) in enumerate(alt_relocs):
        kind = bits >> 28
        assert at % 4 == 0 and at + 4 <= alt_size
        if kind == 10:
            next_at, next_bits = alt_relocs[ordinal + 1]
            assert not (bits & (1 << 27)) and at == next_at and next_bits >> 28 in (3, 4)
            assert second_symbols[next_bits & 0xffffff][2] != pool_sid
            fixed_relocs.append((new_root_start + at, bits))
            continue
        assert bits & (1 << 27) and kind in (2, 3, 4, 5, 6)
        name, _, section, address = second_symbols[bits & 0xffffff]
        if section == pool_sid:
            assert name.startswith('lCPI') and kind in (3, 4)
            assert pool_pc <= address < pool_pc + pool_size
            instruction, = unpack_from('<I', replacement, at)
            zero_field = 0x60ffffe0 if kind == 3 else 0x003ffc00
            assert instruction & zero_field == 0
            addend = pool_at_from_anchor + (address - pool_pc)
            assert 0 <= addend < 1 << 23
            if addend:
                fixed_relocs.append((new_root_start + at, (10 << 28) | (2 << 25) | addend))
            fixed_relocs.append((new_root_start + at, (bits & 0xff000000) | anchor_ix))
            continue
        if section:
            # Keep each LLVM outline inside the root atom. Applying its relative
            # branch here leaves its target invariant when the host links the atom.
            assert section == alt_sid and kind == 2
            assert name in coalesced or name.startswith('_OUTLINED_FUNCTION_')
            assert bits & (1 << 24)
            displacement = address - at
            assert 0 <= address < alt_size and displacement % 4 == 0
            assert -(1 << 27) <= displacement < (1 << 27)
            instruction, = unpack_from('<I', replacement, at)
            assert instruction in (0x14000000, 0x94000000)
            pack_into('<I', replacement, at, instruction | ((displacement // 4) & 0x3ffffff))
            local_branches += 1
        else:
            assert name in first_index, name
            assert name not in coalesced, name
            fixed_relocs.append((new_root_start + at, (bits & 0xff000000) | first_index[name]))

    symtab = first_commands[2]
    symbols_at, count, strings_at, strings_len = unpack_from('<IIII', original, symtab + 8)
    const_file_end = data_offset + data_size
    pointer_alignment = 8
    assert text_reloc_at == (const_file_end + pointer_alignment - 1) & -pointer_alignment
    assert original[const_file_end:text_reloc_at] == bytes(text_reloc_at - const_file_end)
    assert text_reloc_at + 8 * text_reloc_count == symbols_at
    assert symbols_at + 16 * count == strings_at
    assert strings_at + strings_len == len(original)
    header = bytearray(original[:text_offset])
    layout_delta = delta + len(pool_bytes)
    new_const_file_end = const_file_end + layout_delta
    new_reloc_at = (new_const_file_end + pointer_alignment - 1) & -pointer_alignment
    pack_into('<Q', header, first_commands[0x19] + 32, data_addr + data_size + layout_delta)
    pack_into('<Q', header, first_commands[0x19] + 48, new_const_file_end - text_offset)
    pack_into('<Q', header, first_headers[code] + 40, text_size + delta)
    pack_into('<II', header, first_headers[code] + 56, new_reloc_at, len(fixed_relocs))
    pack_into('<Q', header, first_headers[const] + 32, data_addr + delta)
    pack_into('<Q', header, first_headers[const] + 40, data_size + len(pool_bytes))
    pack_into('<I', header, first_headers[const] + 48, data_offset + delta)
    symbols_new = new_reloc_at + 8 * len(fixed_relocs)
    pack_into('<I', header, symtab + 8, symbols_new)
    pack_into('<I', header, symtab + 16, symbols_new + 16 * count)
    symbol_records = bytearray(original[symbols_at:strings_at])
    for ix, (name, _, section, address) in enumerate(first_symbols):
        if name in coalesced:
            # Hide the old local atom. All callers to the replacement body now
            # use intra-atom branches, which have no linker relocation.
            pack_into('<BBHQ', symbol_records, 16 * ix + 4, 2, 0, 0, 0)
        elif section == first_sections[code][0]:
            pack_into('<Q', symbol_records, 16 * ix + 8, moved(address))
        elif section == first_sections[const][0]:
            pack_into('<Q', symbol_records, 16 * ix + 8, address + delta)
    pad = pack('<I', 0xd503201f) * ((padded_size - alt_size) // 4)
    text_parts = []
    cursor = 0
    for start, end, width in rewrites:
        text_parts.append(original[text_offset + cursor:text_offset + start])
        if width:
            assert start == root_start
            text_parts.extend((replacement, pad))
        cursor = end
    text_parts.append(original[text_offset + cursor:const_file_end])
    object_path = destination
    object_path.write_bytes(b''.join((
        header, *text_parts, pool_bytes, bytes(new_reloc_at - new_const_file_end),
        b''.join(pack('<II', a, b) for a, b in sorted(fixed_relocs, key=lambda pair: -pair[0])),
        symbol_records, original[strings_at:],
    )))
    actual, actual_sections, actual_symbols = macho.load(object_path)
    assert actual_sections[code][2] == text_size + delta
    assert actual_sections[code][4] == new_reloc_at
    assert actual[new_const_file_end:new_reloc_at] == bytes(new_reloc_at - new_const_file_end)
    assert actual_symbols[first_index[root]] == (*first_symbols[first_index[root]][:3], new_root_start)
    _, _, n, off, _, _ = actual_sections[const]
    assert actual[off:off + n] == original[data_offset:data_offset + data_size] + pool_bytes
    return actual_sections[code][2]
