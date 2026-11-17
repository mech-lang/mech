"""Static cold-origin packing probe for the pinned Apple AArch64 root."""

from collections import defaultdict
from hashlib import sha256
import json
from pathlib import Path
import re
from struct import pack
import sys

reference_dir, output_dir = (Path(arg).resolve() for arg in sys.argv[1:])
here = output_dir
here.mkdir(parents=True)
ref = reference_dir
outline = Path(__file__).resolve().parent / "outline-root.py"
scope = {"__file__": str(outline), "__name__": "__main__"}
sys.argv = [str(outline), "unused", str(here), str(ref)]
exec(compile(outline.read_text().split('\nif __name__ == "__main__":\n')[0],
             str(outline), "exec"), scope)
start, end, raw, instructions, branches, reloc = (
    scope[key] for key in ("start", "end", "raw", "instructions", "root_branches", "reloc")
)
branch_opcode = scope["branch_opcode"]
branch_target = scope["branch_target"]
returns = {"ret", "retab", "retaa", "udf", "brk"}
epilogues = [
    pc for pc in range(start, end - 4, 4)
    if instructions[pc] == ("ldp", "x29, x30, [sp], #0x10") and instructions[pc + 4][0] == "retab"
]
assert len(epilogues) == 3
assert all(pc in (start + 4, *epilogues)
           for pc in range(start, end, 4)
           if re.search(r"\bx30\b", instructions[pc][1]))


def move_imm(word, reg):
    return word & ~0x1fffe0 in (0xd2800000 | reg, 0x52800000 | reg)


def reaches_restore(lead):
    """Admit only an exact cold return of x0 status and x1 origin."""
    for pc in range(lead, min(end, lead + 12), 4):
        inst, arg = instructions[pc]
        if inst == "add" and arg == instructions[start + 32][1]:
            return any(offset >= 0 and offset <= 32 and offset % 4 == 0 and all(
                instructions[i][0] == "ldp" for i in range(pc + 4, epilogue + 4, 4)
            ) for epilogue in epilogues for offset in (epilogue - pc,))
        if inst == "b" and pc in branches:
            return reaches_restore(branches[pc][0]) if branches[pc][0] < lead else False
        if not (move_imm(raw[pc], 0) or inst == "movk" and arg.startswith("w0,")):
            return False
    return False


preds = defaultdict(list)
reachable = {start}
work = [start]
while work:
    pc = work.pop()
    inst, _arg = instructions[pc]
    edges = []
    if inst not in returns and inst != "b":
        edges.append((pc + 4, "fallthrough"))
    if pc in branches:
        edges.append((branches[pc][0], "branch"))
    for target, kind in edges:
        assert start <= target < end
        preds[target].append((pc, kind))
        if target not in reachable:
            reachable.add(target)
            work.append(target)

groups = defaultdict(list)
for pc in sorted(reachable):
    if pc <= epilogues[-2] + 4 or pc in reloc or not move_imm(raw[pc], 1):
        continue
    if not preds[pc] or any(kind != "branch" for _, kind in preds[pc]):
        continue
    if preds[pc + 4] != [(pc, "fallthrough")] or instructions[pc + 4][0] != "b":
        continue
    target = branches[pc + 4][0]
    if reaches_restore(target):
        groups[target].append((pc, (raw[pc] >> 5) & 0xffff))
groups = {target: rows for target, rows in sorted(groups.items()) if len(rows) > 12}
assert [len(rows) for rows in groups.values()] == [133, 19, 139]
removed = {pc + shift for rows in groups.values() for pc, _ in rows for shift in (0, 4)}
assert not removed.intersection(reloc)
assert all(target not in removed for target in groups)


def adr(register, pc, target):
    delta = target - pc
    assert -(1 << 20) <= delta < 1 << 20
    delta &= (1 << 21) - 1
    return 0x10000000 | (delta & 3) << 29 | (delta >> 2) << 5 | register


body = bytearray()
moved = {}
for pc in range(start, end, 4):
    if pc not in removed:
        moved[pc] = start + len(body)
        body.extend(pack("<I", raw[pc]))
assert instructions[end - 4][0] in ("b", "retab", "udf")
decoded = []
group_layout = []
for old_target, sites in groups.items():
    first = start + len(body)
    decoder = first + 4 * len(sites)
    table = decoder + 24
    group_layout.append({"return_tail": hex(old_target), "sites": len(sites),
                         "first_stub": hex(first), "decoder": hex(decoder), "table": hex(table)})
    for i, (pc, origin) in enumerate(sites):
        moved[pc] = at = start + len(body)
        assert at == first + 4 * i and at + 4 - (first + 4) == 4 * i
        body.extend(pack("<I", branch_opcode(0x94000000, at, decoder, (0, 26))))
        assert branch_target(at, int.from_bytes(body[-4:], "little"), (0, 26)) == decoder
        decoded.append((pc, at, decoder, table + 2 * i, origin, old_target))
    body.extend(pack("<6I", adr(1, decoder, first + 4), 0xcb0103c1, 0xd341fc21,
                     adr(30, decoder + 12, table), 0x78616bc1,
                     branch_opcode(0x14000000, decoder + 20, moved[old_target], (0, 26))))
    for _, origin in sites:
        body.extend(pack("<H", origin))
    if len(body) % 4:
        body.extend(bytes(2))

for old, (target, field) in sorted(branches.items()):
    if old not in removed:
        dest = moved[old] - start
        body[dest:dest + 4] = pack("<I", branch_opcode(raw[old], moved[old], moved[target], field))
for _, _, _, entry, origin, _ in decoded:
    assert int.from_bytes(body[entry - start:entry - start + 2], "little") == origin

# Keep the helper addresses congruent with the control.
raw_bytes = len(body)
align = (scope["extent"] - start - len(body)) % 64
body.extend(pack("<I", 0xd503201f) * (align // 4))
assert len(body) % 64 == (scope["extent"] - start) % 64
rs = [(moved[at], flag) for at, flags in reloc.items() if start <= at < end for flag in flags]
candidate = here / "program.o"
text, _ = scope["replace"](candidate, bytes(body), rs)
report = {
    "reference": str(ref / "program.o"),
    "reference_sha256": sha256((ref / "program.o").read_bytes()).hexdigest(),
    "candidate_sha256": sha256(candidate.read_bytes()).hexdigest(),
    "reference_root_bytes": end - start,
    "candidate_root_before_alignment_bytes": raw_bytes,
    "candidate_text_bytes": text,
    "reference_text_bytes": scope["text_size"],
    "groups": group_layout,
    "link_and_runtime": "not tested; static object prototype",
}
(here / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
assert report["candidate_sha256"] == "66899f7928c031d7e9e882149f8d428c1b13ca22f1ca50ccdbefad8cb07d706a"
