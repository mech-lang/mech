"""Test reducing repeated address arithmetic in the pinned call-free scorer.

This diagnostic rewrites one exact AArch64 scorer atom. It validates every input
byte through the object hash and edits only the identified instruction/data
relocations. The compact emitter must eventually derive equivalent facts from
prepared IR before a result can be used outside this experiment.
"""

from argparse import ArgumentParser
from collections import defaultdict
from hashlib import file_digest
from importlib import import_module
import json
from pathlib import Path
import re
from struct import pack, pack_into, unpack_from
import subprocess
import sys

EXPECTED = "7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b"
SYMBOL = "_mech_compact_numeric_3009_no_growth_composed"
COST_TABLE = "_mech_compact_cost_codes_0"
OBSERVED = "branch-on-zero-aligned-3009-no-growth-composed"
HOME = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")


def digest(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def add_shifted(destination, base, index, shift):
    return 0x8B000000 | (index << 16) | (shift << 10) | (base << 5) | destination


def sub_shifted(destination, base, index, shift):
    return 0xCB000000 | (index << 16) | (shift << 10) | (base << 5) | destination


def add_imm(destination, base, value):
    assert 0 <= value < 4096
    return 0x91000000 | (value << 10) | (base << 5) | destination


def ldur(opcode, destination, base, displacement):
    assert -256 <= displacement <= 255
    return opcode | ((displacement & 511) << 12) | (base << 5) | destination


def ldr_indexed(opcode, destination, base, index):
    return opcode | (index << 16) | (base << 5) | destination


def rewrite_field(word, reg, shift):
    return (word & ~(31 << shift)) | (reg << shift)


LDR_UNSIGNED_X = 0xF9400000
LDR_X_SCALED = 0xF8607800
LDR_W_SCALED = 0xB8607800
LDRB_EXTENDED_W = 0x38604800
LDUR_W = 0xB8400000
LDUR_Q = 0x3CC00000


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--qualifications", type=Path, default=HOME)
    parser.add_argument("--shrink", action="store_true", help="also reduce the atom's aligned extent")
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.mkdir(parents=True)
    q = args.qualifications.resolve()
    sys.path.insert(0, str(q))
    audit = import_module("count-matcher-instructions")
    macho = audit.macho
    branch_target = audit.layout.branch_target
    branch_opcode = audit.layout.branch_opcode
    obj_relocs = audit.layout.obj.relocs

    reference = q / "native-size-20261117/source-aligned"
    path = reference / "program.o"
    assert digest(path) == EXPECTED
    data, sections, symbols = macho.load(path)
    names = macho.text_symbols(sections, symbols)
    start = names[SYMBOL]
    extent = min(pc for pc in names.values() if pc > start)
    literal = start + 1120
    assert start == 0x20A0 and literal == 0x2500 and extent == 0x2520
    _, text_address, text_size, text_file, _, _ = sections[macho.TEXT]
    assert text_address == 0
    relocs = dict(obj_relocs(data, sections[macho.TEXT]))
    local_relocs = {pc: bits for pc, bits in relocs.items() if start <= pc < extent}
    assert set(local_relocs) == {0x21DC, 0x21E0}
    for pc, kind in ((0x21DC, 5), (0x21E0, 6)):
        bits = local_relocs[pc]
        assert bits >> 28 == kind and bits >> 27 & 1
        assert symbols[bits & 0xFFFFFF][0] == COST_TABLE

    disassembly = subprocess.check_output(["/usr/bin/otool", "-tvV", path], text=True)
    instructions = {}
    for line in disassembly.splitlines():
        match = re.fullmatch(r"([0-9a-f]{16})\s+(\S+)\s*(.*)", line)
        if match and start <= (pc := int(match[1], 16)) < literal:
            instructions[pc] = (match[2], match[3])
    assert set(instructions) == set(range(start, literal, 4))
    assert not any(op in {"bti", "bl", "blr", "br", "braa", "brab", "adr", "prfm"}
                   or re.search(r"\b(?:[wx]1[67]|ip[01]|[vqdsbh]1[89])\b", operands)
                   for op, operands in instructions.values())
    assert [pc for pc, (op, _) in instructions.items() if op == "adrp"] == [0x21DC]
    raw = {pc: unpack_from("<I", data, text_file + pc)[0] for pc in instructions}
    assert raw[0x24F8] == 0xD65F0FFF and raw[0x24FC] == 0
    branches = {}
    literal_reads = {}
    for pc, (operation, _) in instructions.items():
        field = audit.layout.branch_field(raw[pc])
        if field is not None:
            assert operation in {"b", "cbz", "cbnz", "tbnz", "tbz"} or operation.startswith("b.")
            target = branch_target(pc, raw[pc], field)
            assert target in instructions and target != 0x24FC
            branches[pc] = (target, field)
        if raw[pc] & 0x3B000000 == 0x18000000:
            target = branch_target(pc, raw[pc], (5, 19))
            assert operation == "ldr" and target in (literal, literal + 16)
            literal_reads[pc] = target
    assert literal_reads == {0x237C: 0x2500, 0x2394: 0x2510}

    # A step either appends a raw instruction, a relocated intra-atom branch,
    # an intra-atom literal load, or an instruction with one original Mach-O
    # relocation. Direct entries at changed PCs still identify the first
    # replacement for that PC. An entry may never target a deleted PC.
    edits = {}
    reasons = {}

    def edit(pc, words, reason):
        assert pc in raw and pc not in edits
        edits[pc] = words
        reasons[pc] = reason

    def control(word, target, field=(5, 19)):
        return ("branch", word, target, field)

    def literal_load(old, register):
        return ("literal", rewrite_field(raw[old], register, 0), literal_reads[old])

    def relocated(old, instruction):
        return ("mach", instruction, old)

    def remove(pcs, reason):
        for pc in pcs:
            edit(pc, [], reason)

    # x16/x17 and v18/v19 are unmodified across this leaf. All address and
    # table sources are identical. Load the immutable slot and shuffle bases
    # once before the outer loop, after reading the packet as before.
    reason = "Keep the invariant cost subtable base, slot-table address, and vector masks"
    uses_x3 = [pc for pc, (_, operands) in instructions.items() if re.search(r"\bx3\b", operands)]
    assert uses_x3 == [0x20F0, 0x20F4, 0x20F8, 0x2120, 0x221C]
    edit(0x2120, [
        raw[0x2120],
        add_imm(3, 3, 0x80C),
        relocated(0x21DC, rewrite_field(raw[0x21DC], 16, 0)),
        relocated(0x21E0, LDR_UNSIGNED_X | (16 << 5) | 16),
        literal_load(0x237C, 18), literal_load(0x2394, 19),
    ], reason)
    remove([0x21DC, 0x21E0, 0x221C, 0x237C, 0x2394], reason)
    edit(0x2218, [ldr_indexed(LDRB_EXTENDED_W, 7, 16, 25)], reason)
    edit(0x2220, [ldr_indexed(LDR_W_SCALED, 26, 3, 7)], reason)
    edit(0x2380, [rewrite_field(raw[0x2380], 18, 16)], reason)
    edit(0x2398, [rewrite_field(raw[0x2398], 19, 16)], reason)

    reason = "Fuse countdown compare against zero with its conditional edge"
    assert branches[0x2128] == (0x218C, (5, 19))
    assert branches[0x241C] == (0x2444, (5, 19))
    assert instructions[0x2124] == instructions[0x2418] == ("cmp", "x4, #0x0")
    edit(0x2124, [control(0xB5000004, 0x218C)], reason)
    edit(0x2418, [control(0xB4000004, 0x2444)], reason)
    remove([0x2128, 0x241C], reason)

    reason = "Reuse the literal index already computed at its range check"
    assert raw[0x219C] == raw[0x21A8]
    remove([0x21A8], reason)

    # Preserve the current node's address in x17; construct it as integer
    # arithmetic before the zero-length check, and preserve the check's
    # selected zero arm. The old latest-cost load supplied exactly these
    # four bytes. The store uses the same address with the same eight bytes.
    reason = "Reuse the current node address for forward and reverse reads and the final store"
    edit(0x21B0, [add_shifted(17, 11, 0, 3), control(raw[0x21B0], *branches[0x21B0])], reason)
    edit(0x21B4, [ldur(LDUR_W, 21, 17, -8)], reason)
    remove([0x21B8, 0x21BC], reason)
    edit(0x224C, [sub_shifted(14, 17, 28, 3)], reason)
    edit(0x2250, [LDR_UNSIGNED_X | (14 << 5) | 14], reason)
    # Keep the biased length in x28 through the initial candidate. Use
    # w23 for the partial sum and w14 for the wrapping total. Only their
    # low halves matter. The unsigned comparison uses the same values.
    edit(0x2254, [rewrite_field(raw[0x2254], 23, 0)], reason)
    edit(0x2258, [0x0B000000 | (14 << 16) | (23 << 5) | 14], reason)
    edit(0x225C, [rewrite_field(raw[0x225C], 14, 5)], reason)
    edit(0x2260, [rewrite_field(raw[0x2260], 14, 5)], reason)
    remove([0x2264], reason)
    edit(0x2268, [rewrite_field(raw[0x2268], 28, 5)], reason)
    # The next reverse-cursor start is the address of the first candidate
    # just evaluated. The following body pre-decrements that cursor.
    edit(0x2278, [sub_shifted(27, 17, 28, 3), add_imm(28, 28, 1)], reason)
    remove([0x227C, 0x2280, 0x2284, 0x2288], reason)
    # After the vector interval, backward address = current - (index+2)*8.
    edit(0x2308, [add_imm(14, 21, 2), sub_shifted(27, 17, 14, 3)], reason)
    remove([0x230C, 0x2310, 0x2314], reason)
    edit(0x2350, [add_shifted(7, 10, 23, 2)], reason)
    remove([0x2354], reason)
    edit(0x2358, [ldur(LDUR_Q, 1, 7, 12)], reason)
    # Fold the common reverse base in the vector group. The two Q loads
    # still issue from the original adjacent initialized byte spans.
    edit(0x235C, [sub_shifted(7, 17, 23, 3)], reason)
    remove([0x2360, 0x2364], reason)
    edit(0x2368, [ldur(LDUR_Q, 30, 7, -48)], reason)
    edit(0x236C, [ldur(LDUR_Q, 31, 7, -32)], reason)
    edit(0x23B4, [0xF9000000 | (17 << 5) | 12], reason)

    reason = "Update the low-word cost directly"
    edit(0x21C8, [rewrite_field(raw[0x21C8], 21, 0)], reason)
    remove([0x21CC], reason)
    # add w6 zero-extends x6 on AArch64. The following same-register move
    # redundantly repeated that effect on the fast literal route.
    remove([0x2488], reason)

    reason = "Reuse the zero flag from masking the count field"
    for masked, compare in ((0x21C0, 0x21E4), (0x23B8, 0x23BC), (0x2428, 0x242C)):
        assert raw[masked] >> 29 == 4
        edit(masked, [raw[masked] | (3 << 29)], reason)
        remove([compare], reason)
    # Additionally, 0x21e4 compared x12 - count == x12, which is exactly
    # count == 0 even for wrapping subtraction. No retained instruction
    # sets flags between the mask and its equal branch.
    assert raw[0x21E4] == 0xEB0C01BF

    reason = "Emit the scalar countdown's hot backedge as one conditional branch"
    assert branches[0x2348] == (0x21F4, (5, 19))
    assert branches[0x234C] == (0x2320, (0, 26))
    edit(0x2348, [control(0x54000001, 0x2320)], reason)
    edit(0x234C, [control(0x14000000, 0x21F4, (0, 26))], reason)

    deleted = {pc for pc, words in edits.items() if not words}
    assert not any(target in deleted for target, _ in branches.values())
    moved = {}
    relocation_pairs = {}
    work = []
    for pc in instructions:
        moved[pc] = start + len(work) * 4
        if pc in edits:
            work.extend(edits[pc])
        elif pc in branches:
            work.append(control(raw[pc], *branches[pc]))
        else:
            assert pc not in literal_reads and pc not in local_relocs
            work.append(raw[pc])

    emitted_code = len(work) * 4
    new_literal = (start + emitted_code + 15) & -16 if args.shrink else literal
    assert start + emitted_code <= new_literal <= literal
    body = bytearray()
    for operation in work:
        pc = start + len(body)
        if isinstance(operation, int):
            word = operation
        elif operation[0] == "branch":
            _, original, target, field = operation
            word = branch_opcode(original, pc, moved[target], field)
            assert branch_target(pc, word, field) == moved[target]
        elif operation[0] == "literal":
            _, original, target = operation
            relocated_target = new_literal + target - literal
            word = branch_opcode(original, pc, relocated_target, (5, 19))
            assert branch_target(pc, word, (5, 19)) == relocated_target
        else:
            assert operation[0] == "mach"
            _, word, old_pc = operation
            relocation_pairs[old_pc] = pc
        body.extend(pack("<I", word))

    assert set(relocation_pairs) == set(local_relocs)
    pad = (new_literal - start - len(body)) // 4
    body.extend(pack("<I", 0xD503201F) * pad)
    body.extend(data[text_file + literal:text_file + extent])
    output = destination / "program.o"
    new_text = audit.replace_atom(
        path, output, start, extent, bytes(body), moved_relocs=relocation_pairs,
    )
    result, candidate_sections, candidate_symbols = macho.load(output)
    candidate_names = macho.text_symbols(candidate_sections, candidate_symbols)
    assert set(names) == set(candidate_names)
    assert candidate_names[SYMBOL] == start
    assert not args.shrink or new_text < text_size
    if not args.shrink:
        assert text_size == new_text and names == candidate_names
        target_data = sections[macho.TEXT][3]
        assert data[target_data:target_data + start] == result[target_data:target_data + start]
        candidate_text_at = candidate_sections[macho.TEXT][3]
        assert candidate_text_at == target_data
        assert data[target_data + extent:target_data + text_size] == result[
            target_data + extent:target_data + text_size]
    candidate_relocs = dict(obj_relocs(result, candidate_sections[macho.TEXT]))
    for prior, changed in relocation_pairs.items():
        assert candidate_relocs[changed] == local_relocs[prior]

    counts = json.loads((q / f"matcher-dynamic-blocks/{OBSERVED}/counts.json").read_text())
    assert counts["symbol"] == SYMBOL and counts["object_sha256"] == EXPECTED
    cases = {}
    for name, case in counts["cases"].items():
        visits = defaultdict(int)
        for block in case["blocks"]:
            address = int(block["address"], 16)
            for i in range(len(block["instructions"])):
                visits[address + i * 4] += block["visits"]
        totals = defaultdict(int)
        for pc, words in edits.items():
            totals[reasons[pc]] += visits[pc] * (1 - len(words))
        cold_exit = visits[0x2348] - visits[0x234C]
        totals[reasons[0x234C]] += visits[0x234C] - cold_exit
        baseline_count = case["counts"].get("instructions", 0)
        assert baseline_count == sum(visits.values())
        cases[name] = {
            "baseline_composed_scorer_calls": case["calls"],
            "baseline_composed_scorer_instructions": baseline_count,
            "projected_reduction_for_unchanged_branches": sum(totals.values()),
            "breakdown": dict(totals),
        }

    report = {
        "reference_object_sha256": EXPECTED,
        "candidate_object_sha256": digest(output),
        "reference_text_bytes": text_size,
        "candidate_text_bytes": new_text,
        "reference_atom_bytes": extent - start,
        "candidate_atom_bytes": len(body),
        "original_bytes_before_literals": literal - start,
        "candidate_bytes_before_padding": emitted_code,
        "positions_preserved": not args.shrink,
        "relocations_moved": {hex(pc): hex(dest) for pc, dest in relocation_pairs.items()},
        "predicted_instruction_removals": cases,
        "retained_checks": [
            hex(pc) for pc in sorted(branches) if pc not in edits and pc < literal
        ],
    }
    (destination / "object-proof.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "retained_checks"}, indent=2))


if __name__ == "__main__":
    main()
