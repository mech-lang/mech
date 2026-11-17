"""Attribute a Mach-O executable's file bytes and linked text using its linker map."""

from collections import Counter, defaultdict
import json
from pathlib import Path
import re
import struct
import sys


def file_regions(data):
    magic, cpu, _, _, count, command_bytes = struct.unpack_from("<6I", data)
    assert (magic, cpu) == (0xFEEDFACF, 0x0100000C)
    ranges = [(0, 32 + command_bytes, "header and load commands")]
    sections = {}

    def record(offset, length, kind):
        if length:
            ranges.append((offset, offset + length, kind))

    offset = 32
    for _ in range(count):
        cmd, size = struct.unpack_from("<II", data, offset)
        if cmd == 0x19:  # LC_SEGMENT_64
            section_count, = struct.unpack_from("<I", data, offset + 64)
            for index in range(section_count):
                at = offset + 72 + index * 80
                section, segment, start, length, file_at, _, reloc, reloc_count, flags = struct.unpack_from(
                    "<16s16sQQ5I", data, at)
                segment, section = (item.rstrip(b"\0").decode() for item in (segment, section))
                sections[segment, section] = (start, length)
                if flags & 0xff not in (1, 12, 18):  # zero-fill sections
                    record(file_at, length, segment + "," + section)
                record(reloc, reloc_count * 8, "section relocations")
        elif cmd == 2:  # LC_SYMTAB
            sym, count, names, names_size = struct.unpack_from("<4I", data, offset + 8)
            record(sym, count * 16, "symbols")
            record(names, names_size, "symbol names")
        elif cmd == 11:  # LC_DYSYMTAB
            indirect, count, external, external_count, local, local_count = struct.unpack_from("<6I", data, offset + 56)
            record(indirect, count * 4, "indirect symbols")
            record(external, external_count * 8, "external relocations")
            record(local, local_count * 8, "local relocations")
        elif cmd in (0x22, 0x80000022):  # LC_DYLD_INFO and LC_DYLD_INFO_ONLY
            labels = ("rebases", "binding", "weak binding", "lazy binding", "exports trie")
            for index, label in enumerate(labels):
                start, length = struct.unpack_from("<II", data, offset + 8 + index * 8)
                record(start, length, label)
        elif cmd in (0x1D, 0x26, 0x29, 0x80000033, 0x80000034):
            names = {0x1D: "signature", 0x26: "function starts", 0x29: "data in code",
                     0x80000033: "exports trie", 0x80000034: "chained fixups"}
            start, length = struct.unpack_from("<II", data, offset + 8)
            record(start, length, names[cmd])
        offset += size
    assert offset == 32 + command_bytes
    sizes = Counter()
    previous_end = 0
    for start, end, name in sorted(ranges):
        assert previous_end <= start < end <= len(data), (name, start, end, previous_end)
        sizes[name] += end - start
        previous_end = end
    sizes["padding and other unassigned bytes"] = len(data) - sum(sizes.values())
    return sections, dict(sizes)


def family(path):
    name = Path(path.split("(", 1)[0]).name
    if name in {"program.o", "encoder.o"} or "encoder.a(roc_app_" in path:
        return "encoder"
    if name == "mimalloc.o":
        return "allocator"
    if "/mech-adapter/" in path or name.endswith("-adapter.o"):
        return "benchmark adapter objects"
    if name.startswith(("libminiz_oxide-", "libadler2-")):
        return "inflation support"
    if name.startswith(("libserde", "libmemchr-", "libitoa-", "libzmij-")):
        return "JSON and serialization libraries"
    if name.startswith(("libmech_gpu_runtime", "libmech_engine", "libdeflate-")):
        return "fixed language or encoder runtime"
    if "/rustlib/" in path:
        return "Rust standard libraries"
    return "other fixed inputs"


def text_attribution(map_path, text):
    start, size = text
    object_pattern = re.compile(r"^\[\s*(\d+)\]\s+(.*)$")
    symbol_pattern = re.compile(r"^(0x[0-9A-Fa-f]+)\s+(0x[0-9A-Fa-f]+)\s+\[\s*(\d+)\]\s+(.*)$")
    objects = {}
    symbols = []
    for line in map_path.read_text(errors="surrogateescape").splitlines():
        if line == "# Dead Stripped Symbols:":
            break
        if match := object_pattern.match(line):
            objects[int(match[1])] = match[2]
        if match := symbol_pattern.match(line):
            address, length = int(match[1], 16), int(match[2], 16)
            if start <= address < start + size and length:
                symbols.append((address, length, objects[int(match[3])], match[4]))
    by_family = Counter()
    by_object = Counter()
    by_size = defaultdict(list)
    previous_end = start
    for address, length, path, symbol in sorted(symbols):
        assert previous_end <= address < address + length <= start + size, (path, symbol)
        by_family[family(path)] += length
        by_object[path] += length
        by_size[family(path)].append((length, symbol))
        previous_end = address + length
    by_family["text padding and unassigned bytes"] = size - sum(by_family.values())
    return {
        "by_family": dict(by_family.most_common()),
        "by_object": dict(by_object.most_common()),
        "largest_symbols_by_family": {group: sorted(entries, reverse=True)[:8] for group, entries in by_size.items()},
    }


if __name__ == "__main__":
    exe, link_map, encoder = map(Path, sys.argv[1:])
    sections, file_breakdown = file_regions(exe.read_bytes())
    _, encoder_breakdown = file_regions(encoder.read_bytes())
    print(json.dumps({
        "executable_file_breakdown": file_breakdown,
        "encoder_object_file_breakdown": encoder_breakdown,
        "linked_text": text_attribution(link_map, sections["__TEXT", "__text"]),
    }, indent=2))
