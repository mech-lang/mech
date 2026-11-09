#!/usr/bin/env python3
"""Verify native text, direct calls, and exact referenced AArch64 literals."""
import argparse
import hashlib
import json
from pathlib import Path
import struct


def macho(path):
    data = path.read_bytes()
    magic, cpu, _, _, commands, _, _, _ = struct.unpack_from('<IiiIIIII', data)
    assert magic == 0xfeedfacf and cpu == 0x100000c, 'Expected AArch64 Mach-O'
    sections, symbols = [], []
    position = 32
    for _ in range(commands):
        command, size = struct.unpack_from('<II', data, position)
        if command == 0x19:
            for ordinal in range(struct.unpack_from('<I', data, position + 64)[0]):
                values = struct.unpack_from('<16s16sQQIIIIIIII', data, position + 72 + ordinal * 80)
                sections.append(dict(name=values[0].split(b'\0')[0].decode(),
                                     segment=values[1].split(b'\0')[0].decode(),
                                     address=values[2], size=values[3], offset=values[4],
                                     relocations=values[6], relocation_count=values[7]))
        elif command == 2:
            _, _, start, count, strings, string_bytes = struct.unpack_from('<IIIIII', data, position)
            for ordinal in range(count):
                index, kind, section, _, value = struct.unpack_from('<IBBHQ', data, start + ordinal * 16)
                assert index < string_bytes
                end = data.index(b'\0', strings + index)
                symbols.append(dict(name=data[strings + index:end].decode(),
                                    kind=kind, section=section, value=value))
        position += size
    return data, sections, symbols


def audit(object_path, executable_path):
    ob, os, symbols = macho(object_path)
    eb, es, linked_symbols = macho(executable_path)
    source = {s['name']: s for s in symbols}
    linked = {s['name']: s for s in linked_symbols}
    entry = '_mech_canonical_program_entry_v1'
    text = os[source[entry]['section'] - 1]
    assert text['name'] == '__text'
    displacement = linked[entry]['value'] - source[entry]['value']
    address = text['address'] + displacement
    destination = next(s for s in es if s['address'] <= address and
                       address + text['size'] <= s['address'] + s['size'])
    offset = destination['offset'] + address - destination['address']
    original = bytearray(ob[text['offset']:text['offset'] + text['size']])
    actual = bytearray(eb[offset:offset + text['size']])
    functions = [s for s in symbols if s['section'] == source[entry]['section'] and
                 s['name'].startswith(entry)]
    for symbol in functions:
        assert linked[symbol['name']]['value'] == symbol['value'] + displacement
    relocations, pending, consumed = [], {}, set()
    for ordinal in range(text['relocation_count']):
        at, info = struct.unpack_from('<iI', ob, text['relocations'] + ordinal * 8)
        assert at not in pending
        pending[at] = info
    # PAGE21/PAGEOFF12 are independently defined relocations. LLVM can hoist
    # one ADRP across blocks and share it among loads, so adjacency is not a
    # proof requirement. Constrain every relocation of a symbol to one common
    # exact linked address; compare complete literal/constant bytes when local
    # assembler symbols disappear. All opcode/register bits remain identical.
    pristine = bytes(actual)
    page_candidates, page_records = {}, []
    def exact_targets(index):
        if index in page_candidates:
            return page_candidates[index]
        target = symbols[index]
        assert target['section'], 'PAGE relocation has no defined source target'
        section = os[target['section'] - 1]
        if section['name'] in ('__literal4', '__literal8', '__literal16'):
            size = int(section['name'].removeprefix('__literal'))
            alignment = size
        elif section['name'] == '__const' and not target['name'].startswith('_'):
            assert not section['relocation_count']
            end = min([s['value'] for s in symbols if s['section'] == target['section']
                       and s['value'] > target['value']] + [section['address'] + section['size']])
            size, alignment = end - target['value'], 1
            assert 0 < size <= 4096, 'Unbounded unnamed constant span'
        else:
            assert target['name'] in linked and linked[target['name']]['section']
            value = linked[target['name']]['value']
            page_candidates[index] = {value}
            return page_candidates[index]
        start = section['offset'] + target['value'] - section['address']
        expected = ob[start:start + size]
        assert len(expected) == size
        candidates = set()
        for destination_section in es:
            if destination_section['segment'] != '__TEXT' or destination_section['name'] not in ('__const','__literal4','__literal8','__literal16'):
                continue
            data = eb[destination_section['offset']:destination_section['offset'] + destination_section['size']]
            offset = data.find(expected)
            while offset >= 0:
                value = destination_section['address'] + offset
                if value % alignment == 0:
                    candidates.add(value)
                offset = data.find(expected, offset + 1)
        assert candidates, ('No exact linked constant target', target['name'])
        page_candidates[index] = candidates
        return candidates

    high_mask, low_mask = (3 << 29) | (0x7ffff << 5), 0xfff << 10
    for at, info in pending.items():
        kind, index = info >> 28, info & 0xffffff
        if kind not in (3, 4):
            continue
        relative, length, external = (info >> 24) & 1, (info >> 25) & 3, (info >> 27) & 1
        assert (relative, length, external) == (int(kind == 3), 2, 1)
        before = struct.unpack_from('<I', original, at)[0]
        after = struct.unpack_from('<I', pristine, at)[0]
        candidates = exact_targets(index)
        if kind == 3:
            assert before & 0x9f000000 == 0x90000000
            assert before & high_mask == 0, 'Unsupported nonzero page addend'
            assert after & ~high_mask == before
            immediate = ((after >> 5 & 0x7ffff) << 2) | (after >> 29 & 3)
            page = ((address + at) & ~0xfff) + ((immediate ^ 0x100000) - 0x100000) * 4096
            page_candidates[index] = {v for v in candidates if v & ~0xfff == page}
            struct.pack_into('<I', actual, at, after & ~high_mask)
            constraint = dict(linked_page=page)
        else:
            assert before & low_mask == 0, 'Unsupported nonzero page-offset addend'
            assert after & ~low_mask == before
            if before & 0xffc00000 == 0x91000000:  # ADD X, unshifted immediate
                scale = 1
            elif before & 0x3b000000 == 0x39000000:  # unsigned-offset LDR/STR
                # Literal references are loads only. SIMD Q encodes its width
                # separately from the scalar size bits.
                assert before & (1 << 22), 'Unsupported page-offset store'
                scale = 16 if before & 0xffc00000 == 0x3dc00000 else 1 << (before >> 30)
            else:
                raise AssertionError(('Unsupported page-offset opcode', hex(before)))
            low = (after >> 10 & 0xfff) * scale
            assert low < 4096
            page_candidates[index] = {v for v in candidates if v & 0xfff == low}
            struct.pack_into('<I', actual, at, after & ~low_mask)
            constraint = dict(linked_page_offset=low, scale=scale)
        assert page_candidates[index], ('Inconsistent PAGE target', symbols[index]['name'], at)
        consumed.add(at)
        page_records.append(dict(offset=at, symbol=symbols[index]['name'], relocation_kind=kind,
                                 verified=True, **constraint))
    for record in page_records:
        index = pending[record['offset']] & 0xffffff
        record['exact_linked_targets'] = sorted(page_candidates[index])
        relocations.append(record)
    for at, info in pending.items():
        if at in consumed:
            continue
        index, relative, length, external, kind = (
            info & 0xffffff, (info >> 24) & 1, (info >> 25) & 3,
            (info >> 27) & 1, info >> 28)
        # Taking the generated entry's address uses GOT_LOAD_PAGE21/PAGEOFF12.
        # Apple's linker may replace the indirect LDR with ADD. Verify the
        # decoded pointer or exact ADD target before normalizing that rewrite.
        if kind in (5, 6):
            high = at if kind == 5 else at - 4
            low = high + 4
            hi, lo = pending[high], pending[low]
            assert hi >> 28 == 5 and lo >> 28 == 6
            assert hi & 0x0fffffff == (index | (1 << 24) | (2 << 25) | (1 << 27))
            assert lo & 0x0fffffff == (index | (2 << 25) | (1 << 27))
            target = symbols[index]
            assert target['section'] and linked[target['name']]['section']
            before_hi, before_lo = struct.unpack_from('<II', original, high)
            after_hi, after_lo = struct.unpack_from('<II', actual, high)
            high_mask, low_mask = (3 << 29) | (0x7ffff << 5), 0xfff << 10
            assert before_hi & 0x9f000000 == 0x90000000
            assert before_lo & 0xffc00000 == 0xf9400000  # LDR X
            assert before_hi & high_mask == before_lo & low_mask == 0
            assert after_hi & ~high_mask == before_hi
            assert before_hi & 31 == before_lo >> 5 & 31
            imm = ((after_hi >> 5 & 0x7ffff) << 2) | (after_hi >> 29 & 3)
            page = ((address + high) & ~0xfff) + ((imm ^ 0x100000) - 0x100000) * 4096
            if after_lo & 0xffc00000 == 0x91000000:
                assert after_lo & ~low_mask == 0x91000000 | (before_lo & 0x3ff)
                resolved = page + (after_lo >> 10 & 0xfff)
                relaxation = 'LDR_X_to_ADD_exact_defined_symbol'
            else:
                assert after_lo & ~low_mask == before_lo
                got_address = page + (after_lo >> 10 & 0xfff) * 8
                got_section = next(s for s in es if s['address'] <= got_address and
                                   got_address + 8 <= s['address'] + s['size'])
                got_offset = got_section['offset'] + got_address - got_section['address']
                resolved = struct.unpack_from('<Q', eb, got_offset)[0]
                relaxation = None
            assert resolved == linked[target['name']]['value']
            struct.pack_into('<II', actual, high, before_hi, before_lo)
            consumed.update((high, low))
            relocations.append(dict(offset=high, paired_offset=low, symbol=target['name'],
                                    resolved_target=resolved, verified=True,
                                    pointer_load_relaxation=relaxation))
            continue
        assert at >= 0 and kind == 2 and relative and external and length == 2, (
            'Unrecognized relocation; no bytes are silently ignored', at, info)
        target = symbols[index]
        assert target['section'] != 0 and linked[target['name']]['section'] != 0
        before = struct.unpack_from('<I', original, at)[0]
        after = struct.unpack_from('<I', actual, at)[0]
        opcode = before & 0xfc000000
        assert opcode in [0x94000000, 0x14000000] and after & 0xfc000000 == opcode
        signed = lambda word: ((word & 0x3ffffff) ^ 0x2000000) - 0x2000000
        expected = linked[target['name']]['value'] + signed(before) * 4
        resolved = address + at + signed(after) * 4
        assert resolved == expected, (target['name'], hex(resolved), hex(expected))
        struct.pack_into('<I', original, at, opcode)
        struct.pack_into('<I', actual, at, opcode)
        relocations.append(dict(offset=at, symbol=target['name'],
                                resolved_target=resolved, verified=True))
    assert original == actual, 'Native text differs beyond independently verified relocation fields'
    # Read-only identity/binding exports need not be referenced by generated
    # text: the Rust launcher consumes them. Verify their complete linked bytes
    # separately, including the descriptor digest's object-owned expected value.
    def exported_bytes(data, sections, symbol, size):
        section = sections[symbol['section'] - 1]
        relative = symbol['value'] - section['address']
        assert relative >= 0 and relative + size <= section['size']
        return data[section['offset'] + relative:section['offset'] + relative + size]
    object_pins = []
    for symbol in symbols:
        if not symbol['name'].startswith('_mech_canonical_program_identity_v1_'):
            continue
        if symbol['name'].endswith('_compiled_module_v1'):
            # The module has relocations, unlike pointer-free identity/image
            # exports. Resolve and verify each pointer; never mask unknown data.
            size = 72
            expected = exported_bytes(ob, os, symbol, size)
            section = os[symbol['section'] - 1]
            assert section['name'] == '__const' and section['segment'] == '__DATA'
            assert expected[:16] == b'MECHMODULEPIN\0\0\0'
            assert struct.unpack_from('<QQ', expected, 16) == (1, 64)
            identity_name = symbol['name'].removesuffix('_compiled_module_v1')
            targets = {32: identity_name + '_native_metadata_v1',
                       48: identity_name,
                       56: identity_name + '_native_metadata_binding_v1',
                       64: entry}
            data_symbol = source[targets[32]]
            data_header = exported_bytes(ob, os, data_symbol, 13)
            assert data_header[:8] in (b'MECHTVI1', b'MECHTVI2', b'MECHTVI3', b'MECHTVI4', b'MECHTVI5')
            assert int.from_bytes(expected[40:48], 'little') == 13 + int.from_bytes(data_header[9:13], 'little')
            module_start = symbol['value'] - section['address']
            checked = {}
            for ordinal in range(section['relocation_count']):
                at, info = struct.unpack_from('<iI', ob, section['relocations'] + ordinal * 8)
                relative_at = at - module_start
                if not 0 <= relative_at < size:
                    continue
                assert relative_at in targets and relative_at not in checked
                index, relative, length, external, kind = (info & 0xffffff, (info >> 24) & 1,
                                                          (info >> 25) & 3, (info >> 27) & 1, info >> 28)
                assert (relative, length, external, kind) == (0, 3, 1, 0)
                assert symbols[index]['name'] == targets[relative_at]
                assert expected[relative_at:relative_at+8] == bytes(8), 'Unsupported module relocation addend'
                checked[relative_at] = targets[relative_at]
            assert checked == targets, 'Module must bind exactly its own metadata, identity, binding and entry'
            if symbol['name'] not in linked:
                object_pins.append(dict(symbol=symbol['name'], bytes=size, linked=False,
                                        all_four_compiler_module_relocations_verified=True))
                continue
            linked_section = es[linked[symbol['name']]['section'] - 1]
            assert linked_section['segment'] == '__DATA_CONST' and linked_section['name'] == '__const'
            actual_module = bytearray(exported_bytes(eb, es, linked[symbol['name']], size))
            resolved = []
            for at, target in targets.items():
                assert struct.unpack_from('<Q', actual_module, at)[0] == linked[target]['value']
                resolved.append(dict(offset=at, symbol=target, address=linked[target]['value']))
                actual_module[at:at+8] = bytes(8)
            assert bytes(actual_module) == expected
            object_pins.append(dict(symbol=symbol['name'], bytes=size, linked=True,
                                    complete_bytes_equal_except_four_verified_relocations=True,
                                    linked_data_is_read_only=True, linked_segment='__DATA_CONST',
                                    all_four_compiler_module_relocations_verified=True, relocations=resolved))
            continue
        native_binding = symbol['name'].endswith('_native_metadata_binding_v1')
        native_data = symbol['name'].endswith('_native_metadata_v1')
        json_binding = symbol['name'].endswith('_descriptor_binding_v1')
        binding = native_binding or json_binding
        if native_data:
            header = exported_bytes(ob, os, symbol, 13)
            assert header[:8] in (b'MECHTVI1', b'MECHTVI2', b'MECHTVI3', b'MECHTVI4', b'MECHTVI5')
            size = 13 + int.from_bytes(header[9:13], 'little')
            assert 13 <= size <= 64*1024*1024
        else:
            size = 64 if binding else 192
        expected = exported_bytes(ob, os, symbol, size)
        object_section = os[symbol['section'] - 1]
        assert object_section['segment'] == '__TEXT' and object_section['name'] in ('__const', '__mech_meta') and not object_section['relocation_count']
        if symbol['name'] not in linked:
            # Legacy JSON and independent loaders need not retain the optional
            # typed-data/binding exports. The chosen launch profile separately
            # requires every data export it actually uses.
            assert binding or native_data
            object_pins.append(dict(symbol=symbol['name'], bytes=size, linked=False))
            continue
        observed = exported_bytes(eb, es, linked[symbol['name']], size)
        linked_section = es[linked[symbol['name']]['section'] - 1]
        assert linked_section['name'] == object_section['name'] and linked_section['segment'] == '__TEXT'
        assert expected == observed, 'Object identity or descriptor-binding data differs after linking'
        if binding:
            assert expected[:16] == b'MECHIMAGEPIN\0\0\0\0'
            suffix = '_native_metadata_binding_v1' if native_binding else '_descriptor_binding_v1'
            descriptor_path = object_path.with_name('native-metadata.bin' if native_binding else 'descriptor.json')
            if descriptor_path.exists():
                descriptor = descriptor_path.read_bytes()
                identity_name = symbol['name'].removesuffix(suffix)
                identity_bytes = exported_bytes(ob, os, source[identity_name], 192)
                if native_binding:
                    emitted = source[identity_name + '_native_metadata_v1']
                    assert exported_bytes(ob, os, emitted, len(descriptor)) == descriptor
                length = len(descriptor).to_bytes(8, 'little')
                digest = hashlib.sha256(b'mech.checked-object.descriptor.v1\0' + length + identity_bytes + descriptor).digest()
                assert expected[16:24] == length and expected[24:32] == (1).to_bytes(8, 'little')
                assert expected[32:] == digest, 'Descriptor does not match the Object-owned binding'
        object_pins.append(dict(symbol=symbol['name'], bytes=size, linked=True, complete_bytes_equal=True,
                                object_data_has_no_relocations=True, linked_data_is_read_only=True))
    pin = lambda path: dict(path=str(path.resolve()), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    return dict(schema='mech.related-post-gen.native-link-audit.v9', status='passed',
                object=pin(object_path), executable=pin(executable_path),
                object_text=text, linked_text_address=address, linked_text_file_offset=offset,
                entry_symbol=entry, defined_native_function_count=len(functions),
                every_native_symbol_has_exact_object_relative_address=True,
                all_text_equal_except_independently_verified_relocation_fields=True,
                all_page_references_bound_to_exact_literals_or_defined_symbols=True,
                all_got_pointer_loads_or_relaxations_bind_to_exact_defined_symbols=True,
                normalized_text_sha256=hashlib.sha256(actual).hexdigest(),
                object_owned_data_exports=object_pins,
                relocations=relocations,
                scope='All native object text bytes compared. ARM64 BRANCH26 displacements '
                      'bind to exact named defined symbols. Every PAGE21/PAGEOFF12 relocation '
                      'constrains its symbol to consistent exact literal/constant bytes or a defined address. '
                      'GOT address loads, including checked LDR-to-ADD linker relaxation, '
                      'bind to exact defined function addresses. Nonzero page addends '
                      'and other relocation forms fail closed.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--object', type=Path, required=True)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.object, args.executable)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ['status', 'defined_native_function_count',
                                             'normalized_text_sha256']}))


if __name__ == '__main__':
    main()
