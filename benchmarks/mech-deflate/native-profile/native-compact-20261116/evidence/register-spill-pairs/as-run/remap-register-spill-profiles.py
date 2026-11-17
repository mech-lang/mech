"""Carry a checked leaf profile across AArch64 spill-pair emission."""
from importlib import import_module
import json
from pathlib import Path
import re
from hashlib import file_digest
import struct
import sys

q = Path(__file__).resolve().parent
trace = import_module("count-matcher-instructions")
macho, layout = trace.macho, trace.layout
study = q / "runtime-matchfinder-20261117"


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def blocks(folder, symbol):
    path = folder / "program.o"
    data, sections, table = macho.load(path)
    start = macho.text_symbols(sections, table)[symbol]
    frame = json.loads((folder / "metadata.json").read_text())
    extent = trace.phase_metadata(frame, symbol)[2]["code_bytes"]
    end = start + extent
    assert not any(start <= pc < end for pc, _ in layout.obj.relocs(data, sections[macho.TEXT]))
    asm = trace.instructions(path, start, end)
    _, address, _, offset, *_ = sections[macho.TEXT]
    assert address == 0
    boundary = {start}
    raw = {}
    edges = {}
    for pc, (op, _) in asm.items():
        raw[pc] = struct.unpack_from("<I", data, offset + pc)[0]
        field = layout.branch_field(raw[pc])
        if field:
            target = layout.branch_target(pc, raw[pc], field)
            assert target in asm
            boundary.add(target)
            edges[pc] = target
            bit, width = field
            raw[pc] &= ~(((1 << width) - 1) << bit)
        if pc + 4 < end and (field or op in {"ret", "retab", "brk", "udf"}):
            boundary.add(pc + 4)
    heads = sorted(boundary)
    ids = {at: i for i, at in enumerate(heads)}
    assert asm[end - 4][0] in {"ret", "retab", "brk", "udf", "b"}
    return {
        at - start: [(raw[pc], ids[edges[pc]] if pc in edges else None, *asm[pc])
                     for pc in range(at, limit, 4)]
        for at, limit in zip(heads, [*heads[1:], end], strict=True)
    }


scalar_stack = re.compile(r"(x[0-9]+), \[sp(?:, #(0x[0-9a-f]+|[0-9]+))?\]")
pair_stack = re.compile(r"(x[0-9]+), (x[0-9]+), \[sp(?:, #(0x[0-9a-f]+|[0-9]+))?\]")


def scalar(instruction):
    _, branch, op, args = instruction
    match = scalar_stack.fullmatch(args)
    if branch is None and op in ("ldur", "ldr", "stur", "str") and match:
        return op[:2], match[1], int(match[2] or "0", 0)
    return None


def equivalent(before, after):
    cursor = iter(before)
    changes = 0
    for entry in after:
        old = next(cursor)
        if old[:2] == entry[:2]:
            continue
        _, edge, op, args = entry
        pair = pair_stack.fullmatch(args)
        assert edge is None and op in ("ldp", "stp") and pair, (old, entry)
        offset = int(pair[3] or "0", 0)
        assert {scalar(old), scalar(next(cursor))} == {
            (op[:2], pair[1], offset), (op[:2], pair[2], offset + 8)
        }, (old, entry)
        changes += 1
    assert next(cursor, None) is None
    return changes


baseline = q / "stale-cache-accounting/pinned-smaller"
before = study / "unprofiled-control"
after = study / "unprofiled-paired"
flag = json.loads((baseline / "runtime.json").read_text())["flags"]["MECH_COMPACT_PHASE_BLOCK_LAYOUT"]
result = []
proofs = []
for item in flag.split(";"):
    symbol, filename = item.split("@", 1)
    plan = Path(filename)
    old = json.loads(plan.read_text())
    assert old.get("literal_offset") is None
    archived = blocks(before, "_" + symbol)
    candidate = blocks(after, "_" + symbol)
    assert len(old["order"]) == len(archived) == len(candidate), (symbol, len(old["order"]),len(archived),len(candidate))
    assert sorted(old["order"]) == list(archived)
    moves = dict(zip(archived, candidate, strict=True))
    changes = sum(equivalent(archived[x], candidate[y]) for x, y in moves.items())
    output = study / f"{symbol}-paired-block-layout.json"
    result.append(f"{symbol}@{output}")
    write = {
        "order": [moves[address] for address in old["order"]],
        "source_profile_sha256": sha(plan),
        "block_count": len(moves),
        "verified_integer_spill_pairs": changes,
        "prelayout_control_object_sha256": sha(before / "program.o"),
        "prelayout_candidate_object_sha256": sha(after / "program.o"),
        "mapping": list(moves.items()),
    }
    output.write_text(json.dumps(write, indent=2) + "\n")
    proofs.append({"symbol": symbol, "profile": str(output), "blocks": len(moves), "pairs": changes})
    print(symbol, len(moves), "blocks,", changes, "paired register moves")
(study / "spill-pair-profile-flags.txt").write_text(";".join(result) + "\n")
(study / "spill-pair-profile-proof.json").write_text(json.dumps(proofs,indent=2)+"\n")
