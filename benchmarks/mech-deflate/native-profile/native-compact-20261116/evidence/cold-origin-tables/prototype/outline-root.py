"""Diagnostic AArch64 outlining on the pinned compact root's finalized code."""
from bisect import bisect_right
from collections import Counter, defaultdict
from hashlib import sha256
import json
from pathlib import Path
import re
from struct import pack, pack_into, unpack_from
import subprocess
import sys

q = Path(__file__).resolve().parent
sys.path.insert(0, str(q / "matched-source-builds/pr973-compact/benchmarks/mech-deflate/native-profile/compact-size"))
import compact_object as obj
import mach_o as mach

reference = q / "cranelift-literal-arguments-root-fold-native/final-candidate"
if __name__ == "__main__" and len(sys.argv) == 4:
    reference = Path(sys.argv[3]).resolve(strict=True)
source = reference / "program.o"
metadata = json.loads((reference / "metadata.json").read_text())
receipt = {"object_sha256": "7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b"}
text_section = mach.TEXT
data_section = (b"__TEXT", b"__const")
base, sections, symbols = mach.load(source)
assert sha256(base).hexdigest() == receipt["object_sha256"]
assert set(sections) == {text_section, data_section}
section_id, text_addr, text_size, text_offset, _, _ = sections[text_section]
assert text_addr == 0
names = mach.text_symbols(sections, symbols)
start = names[mach.ROOT]
extent = min(addr for addr in names.values() if addr > start)
end = start + metadata["code_bytes"]
assert end <= extent
raw = {pc: unpack_from("<I", base, text_offset + pc)[0] for pc in range(0, text_size, 4)}
reloc = defaultdict(list)
for at, bits in obj.relocs(base, sections[text_section]):
    reloc[at].append(bits)

asm = subprocess.check_output(["/usr/bin/otool", "-tvV", source], text=True)
instructions = {
    int(m[1], 16): (m[2], m[3]) for line in asm.splitlines()
    if (m := re.fullmatch(r"([0-9a-f]{16})\s+(\S+)\s*(.*)", line))
}
assert instructions[start] == ("pacibsp", "")
assert instructions[start + 4] == ("stp", "x29, x30, [sp, #-0x10]!")
assert instructions[start + 8] == ("mov", "x29, sp")
assert instructions[start + 28][0] == "stp"
assert instructions[start + 32][0] == "sub"
assert instructions[start + 32][1].startswith("sp, sp, #")
first = start + 36

def branch_field(word):
    if word & 0x7c00_0000 == 0x1400_0000:
        return 0, 26
    if word & 0xff00_0010 == 0x5400_0000 or word & 0x7e00_0000 == 0x3400_0000:
        return 5, 19
    if word & 0x7e00_0000 == 0x3600_0000:
        return 5, 14
    return None

def branch_target(pc, word, field):
    low, width = field
    displacement = (word >> low) & ((1 << width) - 1)
    if displacement >> (width - 1):
        displacement -= 1 << width
    return pc + 4 * displacement

def branch_opcode(word, pc, target, field):
    low, width = field
    displacement, unaligned = divmod(target - pc, 4)
    assert not unaligned and -(1 << (width - 1)) <= displacement < (1 << (width - 1))
    mask = (1 << width) - 1
    return (word & ~(mask << low)) | ((displacement & mask) << low)

targets = set(names.values())
root_branches = {}
function_heads = sorted(set(names.values()))
for pc, (op, operands) in instructions.items():
    if pc in reloc:
        continue
    if start <= pc < end:
        assert op not in ("br", "braa", "brab", "adr", "adrp")
        assert not (op.startswith(("ldr", "prfm")) and "[" not in operands)
    if op in ("b", "bl", "cbnz", "cbz", "tbnz", "tbz") or op.startswith("b."):
        field = branch_field(raw[pc])
        assert field
        target = branch_target(pc, raw[pc], field)
        targets.add(target)
        # Every unrelocated branch stays in its original function atom.
        assert bisect_right(function_heads, pc) == bisect_right(function_heads, target)
        if start <= pc < end:
            assert start <= target < end
            root_branches[pc] = (target, field)

unsafe_regs = re.compile(r"\b(?:[wx](?:16|17|18|29|30)|fp|lr)\b")
alu = {
    "add", "and", "asr", "bic", "clz", "cmp", "csel", "cset", "eor", "lsl", "lsr",
    "madd", "mov", "movk", "movn", "movz", "msub", "mul", "neg", "orr", "sbfiz", "sxtb",
    "sxtw", "sub", "subs", "ubfiz", "ubfx", "uxtb", "uxth", "uxtw", "tst",
}
stack = {"ldr", "ldrh", "ldrb", "ldur", "ldurh", "ldurb", "ldp",
         "str", "strh", "strb", "stur", "sturh", "sturb", "stp"}
plain_stack = re.compile(r"\[sp(?:, #(?:0x[\da-f]+|\d+))?\]$")

def candidates(kinds, start_at=first):
    groups = defaultdict(list)
    for pc in range(start_at, end, 4):
        words = []
        for at in range(pc, min(end, pc + 48), 4):
            if at != pc and at in targets:
                break
            op, operands = instructions[at]
            word = raw[at]
            rel = tuple(reloc.get(at, ()))
            if unsafe_regs.search(operands):
                break
            if op == "b" and (edge := root_branches.get(at)) and words and "jump" in kinds:
                groups["jump", (*words, (edge[0], ()))].append(pc)
            elif op == "bl" and rel and words and "call" in kinds:
                assert word == 0x94000000 and len(rel) == 1 and rel[0] >> 28 == 2
                groups["call", (*words, (word, rel))].append(pc)
            elif op == "blr" and not rel and words and "call" in kinds:
                assert word & ~0x3e0 == 0xd63f0000
                groups["call", (*words, (word, ()))].append(pc)
            if rel:
                # A Mach-O GOT lookup carries its exact symbol and relocation.
                # Keep indirect calls and arbitrary heap loads out of a prefix.
                if len(rel) != 1 or (op, rel[0] >> 28) not in (("adrp", 5), ("ldr", 6)):
                    break
            elif not (op in alu and not re.search(r"\bsp\b", operands)
                      or op in stack and plain_stack.search(operands)):
                break
            words.append((word, rel))
            if len(words) >= 3 and "leaf" in kinds:
                groups["leaf", tuple(words)].append(pc)
    return groups

def select(groups):
    def gain(kind, width, count):
        if kind == "jump":
            return (count - 1) * (width - 1)
        return count * (width - 1) - width - int(kind == "leaf")

    used = set()
    selected = []
    priorities = sorted(groups, key=lambda k: (gain(k[0], len(k[1]), len(groups[k])), len(k[1])),
                        reverse=True)
    for kind, words in priorities:
        matched, sites = [], set()
        width = len(words)
        for pc in groups[kind, words]:
            occupied = set(range(pc, pc + 4 * width, 4))
            if used.isdisjoint(occupied) and sites.isdisjoint(occupied):
                sites.update(occupied)
                matched.append(pc)
        savings = gain(kind, width, len(matched))
        if len(matched) >= 2 and savings > 0:
            used.update(sites)
            selected.append((kind, words, matched, 4 * savings))
    return selected

def machine_code(chosen, rewrites=None):
    # An entry to a removed suffix is replaced by a branch. No interior
    # address may be referenced. The remainder of the function is relocated.
    substitutions = {}
    occupied = set()
    for index, (kind, words, sites, _) in enumerate(chosen):
        target = sites[0] if kind == "jump" else ("outline", index)
        for site in sites:
            occupied.update(range(site, site + 4 * len(words), 4))
        for site in sites[int(kind == "jump"):]:
            substitutions[site] = (kind, target, len(words))
    rewrites = {pc: word for pc, word in (rewrites or {}).items() if pc not in occupied}
    assert all(first <= pc < end and pc not in reloc and pc not in root_branches for pc in rewrites)
    out = []
    moved = {}
    pc = start
    while pc < end:
        moved[pc] = start + 4 * len(out)
        if pc in substitutions:
            kind, to, width = substitutions[pc]
            word = 0x14000000 if kind == "jump" else 0x94000000
            out.append((word, (), (to, (0, 26))))
            pc += 4 * width
        else:
            word = rewrites.get(pc, raw[pc])
            if word is not None:
                out.append((word, tuple(reloc.get(pc, ())), root_branches.get(pc)))
            pc += 4
    assert instructions[end - 4][0] in ("b", "retab", "udf")
    for index, (kind, words, _, _) in enumerate(chosen):
        if kind == "jump":
            continue
        moved[("outline", index)] = start + 4 * len(out)
        if kind == "leaf":
            out.extend((word, rel, None) for word, rel in words)
            out.append((0xd65f03c0, (), None))
        else:
            out.extend((word, rel, None) for word, rel in words[:-1])
            word, rel = words[-1]
            tail = 0x14000000 if rel else 0xd61f0000 | (word & 0x3e0)
            out.append((tail, rel, None))
    body = bytearray()
    rels = []
    for index, (word, rel, edge) in enumerate(out):
        pc = start + 4 * index
        if edge is not None:
            target, field = edge
            word = branch_opcode(word, pc, moved[target], field)
        body.extend(pack("<I", word))
        rels.extend((pc, flags) for flags in rel)
    assert len(body) == end - start - sum(v[3] for v in chosen) - 4 * sum(
        word is None for word in rewrites.values())
    return body, rels

def replace(destination, body, body_relocs):
    commands, headers = obj.tables(base)
    assert set(commands) == {0x19, 2, 11}
    assert not any(unpack_from("<12I", base, commands[11] + 32))
    const_id, data_addr, data_size, data_offset, _, data_relocs = sections[data_section]
    assert not data_relocs
    align = 1 << unpack_from("<I", base, headers[text_section] + 52)[0]
    assert start % align == extent % align == 0 and align % 4 == 0
    padded = (len(body) + align - 1) & -align
    delta = start + padded - extent
    assert delta < 0
    old_relocations = obj.relocs(base, sections[text_section])
    outside = [(at + (delta if at >= extent else 0), flags) for at, flags in old_relocations
               if not start <= at < extent]
    fixed = sorted(outside + body_relocs, key=lambda pair: -pair[0])
    symtab = commands[2]
    symbols_at, n, strings_at, strings_len = unpack_from("<IIII", base, symtab + 8)
    const_end = data_offset + data_size
    old_rel = sections[text_section][4]
    assert old_rel == (const_end + 7) & -8
    assert old_rel + len(old_relocations)*8 == symbols_at
    assert symbols_at + 16*n == strings_at and strings_at + strings_len == len(base)
    new_const_end = const_end + delta
    new_rel = (new_const_end + 7) & -8
    new_symtab = new_rel + 8*len(fixed)
    header = bytearray(base[:text_offset])
    pack_into("<Q", header, commands[0x19] + 32, data_addr + data_size + delta)
    pack_into("<Q", header, commands[0x19] + 48, new_const_end - text_offset)
    pack_into("<Q", header, headers[text_section] + 40, text_size + delta)
    pack_into("<II", header, headers[text_section] + 56, new_rel, len(fixed))
    pack_into("<Q", header, headers[data_section] + 32, data_addr + delta)
    pack_into("<I", header, headers[data_section] + 48, data_offset + delta)
    pack_into("<I", header, symtab + 8, new_symtab)
    pack_into("<I", header, symtab + 16, new_symtab + 16*n)
    symbol_records = bytearray(base[symbols_at:strings_at])
    for index, (_, _, section, address) in enumerate(symbols):
        if section == section_id:
            assert not start < address < extent
        if section == const_id or section == section_id and address >= extent:
            pack_into("<Q", symbol_records, 16*index + 8, address + delta)
    destination.write_bytes(b"".join((
        header, base[text_offset:text_offset+start], body,
        pack("<I", 0xd503201f)*((padded-len(body))//4), base[text_offset+extent:const_end],
        bytes(new_rel-new_const_end), b"".join(pack("<II", a, b) for a,b in fixed),
        symbol_records, base[strings_at:],
    )))
    _, final_sections, final_symbols = mach.load(destination)
    assert final_sections[text_section][2] == text_size + delta
    assert mach.text_symbols(final_sections, final_symbols)[mach.ROOT] == start
    return text_size + delta, len(fixed)

if __name__ == "__main__":
    for policy, modes in (
        ("jump", ("jump",)), ("call", ("call",)), ("leaf", ("leaf",)),
        ("call-jump", ("call", "jump")), ("all", ("call","jump","leaf")),
    ):
        selected = select(candidates(modes))
        print(policy, sum(v[3] for v in selected), Counter(v[0] for v in selected), flush=True)
        if len(sys.argv) < 2 or sys.argv[1] != policy:
            continue
        target = Path(sys.argv[2])
        target.mkdir(parents=True, exist_ok=True)
        body, rels = machine_code(selected)
        size, count = replace(target/"program.o", body, rels)
        (target/"report.json").write_text(json.dumps({
            "old_sha256": receipt["object_sha256"],
            "object_bytes":size, "root_bytes":len(body), "relocations":count,
            "selected":[{"kind":k,"instructions":len(w),"sites":s,"bytes_saved":g}
                        for k,w,s,g in selected],
        },indent=2)+"\n")
