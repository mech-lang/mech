"""Reuse a checked root's existing fallthrough copy sequence.

This static diagnostic compares physical registers, frame offsets, instruction
order, and final continuation. A replaced tail already jumps, and the retained
copy falls directly into that continuation. Checked calls and their failure
edges precede the copies. Ordinary lowering should identify the corresponding
allocated copy blocks rather than hard-code any of these addresses.
"""

from argparse import ArgumentParser
from collections import defaultdict
from hashlib import sha256
import json
from pathlib import Path
import re
from struct import pack
import sys

root_outline = Path(__file__).resolve().parents[2] / "cold-origin-tables/prototype/outline-root.py"


def pinned_program(reference, output):
    """Read the already reviewed Mach-O relocation and branch remapper."""
    scope = {"__file__": str(root_outline), "__name__": "__main__"}
    old_args = sys.argv
    sys.path.insert(0, str(root_outline.resolve().parent))
    try:
        sys.argv = [str(root_outline), "unused", str(output), str(reference)]
        source = root_outline.read_text().split('\nif __name__ == "__main__":\n')[0]
        exec(compile(source, str(root_outline), "exec"), scope)
    finally:
        sys.argv = old_args
        sys.path.pop(0)
    return scope


def copy_instruction(opcode, operands, frame):
    """Recognize scalar copies that neither change SP nor read caller state."""
    if opcode == "mov":
        return re.fullmatch(r"[wx](?:[0-9]|[12][0-8]), (?:[wx](?:[0-9]|[12][0-8]|zr)|#0x[0-9a-f]+)", operands) is not None
    if opcode not in ("ldr", "ldur", "str", "stur", "ldp", "stp"):
        return False
    parts = re.fullmatch(
        r"(x(?:[0-9]|[12][0-8]|zr))(?:, (x(?:[0-9]|[12][0-8]|zr)))?, \[sp(?:, #(0x[0-9a-f]+))?\]",
        operands,
    )
    if not parts:
        return False
    paired = opcode in ("ldp", "stp")
    if paired != bool(parts[2]):
        return False
    offset = int(parts[3] or "0", 0)
    width = 16 if paired else 8
    return offset % 8 == 0 and offset + width <= frame


class Root:
    def __init__(self, scope, metadata):
        self.scope = scope
        self.start, self.end = scope["start"], scope["end"]
        self.instructions = scope["instructions"]
        self.reloc = scope["reloc"]
        self.branches = scope["root_branches"]
        self.words = scope["raw"]
        self.heads = {self.start}
        self.pred = defaultdict(set)
        for pc in range(self.start, self.end, 4):
            opcode, _ = self.instructions[pc]
            edge = self.branches.get(pc)
            if edge is not None:
                destination = edge[0]
                assert self.start <= destination < self.end
                self.heads.add(destination)
                self.pred[destination].add((pc, "branch"))
            if opcode not in ("b", "ret", "retaa", "retab", "brk", "udf"):
                assert pc + 4 < self.end
                self.pred[pc + 4].add((pc, "fallthrough"))
            else:
                self.heads.add(pc + 4)

        # Keep all proven copies within the fixed frame well before restoration.
        # SP from this established prologue stays fixed between it and each
        # matching epilogue. Reject unfamiliar prologue/epilogue forms.
        prologue = self.start + 32
        assert self.instructions[prologue][0] == "sub"
        setup = self.instructions[prologue][1]
        frame_bytes = metadata["stack_bytes"]
        payload = int(setup.removeprefix("sp, sp, #"), 0)
        assert frame_bytes >= payload + 16
        assert all(
            pc <= prologue or (
                self.instructions[pc][0] == "add" and self.instructions[pc][1] == setup
            )
            for pc in range(self.start, self.end, 4)
            if re.match(r"^sp,", self.instructions[pc][1])
        )
        self.first = prologue + 4
        self.copies = {
            pc for pc in range(self.first, self.end, 4)
            if pc not in self.reloc and copy_instruction(*self.instructions[pc], payload)
        }
        assert not (self.copies & self.branches.keys())
        assert not (self.copies & self.reloc.keys())

    def preceding_copies(self, end):
        copies = []
        while end - 4 in self.copies:
            end -= 4
            copies.append(end)
            if end in self.heads:
                break
        return list(reversed(copies))

    def select(self, minimum):
        targets = defaultdict(list)
        for pc, (continuation, _field) in sorted(self.branches.items()):
            if self.instructions[pc][0] == "b" and self.preceding_copies(pc):
                if self.pred[pc] == {(pc - 4, "fallthrough")}:
                    targets[continuation].append(pc)

        families = []
        for continuation, jumps in targets.items():
            canonical = self.preceding_copies(continuation)
            if len(canonical) < minimum:
                continue
            by_width = defaultdict(list)
            for jump in jumps:
                suffix = self.preceding_copies(jump)
                width = 0
                for a, b in zip(reversed(canonical), reversed(suffix)):
                    if self.words[a] != self.words[b]:
                        break
                    width += 1
                if width >= minimum:
                    by_width[width].append(jump)
            for width, sites in by_width.items():
                canonical_start = canonical[-width]
                for jump in sites:
                    dup_start = jump - 4 * width
                    # Only a fallthrough predecessor needs the replacement B.
                    # Explicit branch predecessors can target the shared copy.
                    via_fallthrough = (dup_start - 4, "fallthrough") in self.pred[dup_start]
                    if not self.pred[dup_start]:
                        continue
                    families.append((width, dup_start, jump, canonical_start, continuation, via_fallthrough))

        blocked = set()
        chosen = []
        for width, begin, jump, canonical, target, keeps_branch in sorted(families, reverse=True):
            touched = set(range(begin, jump + 4, 4))
            preserved = set(range(canonical, target + 4, 4))
            if touched & (blocked | preserved) or preserved & set(self.removed(chosen)):
                continue
            assert target - canonical == jump - begin == width * 4
            assert all((pc - 4, "fallthrough") in self.pred[pc] for pc in range(begin + 4, jump + 4, 4))
            assert all(pc not in self.heads for pc in range(begin + 4, jump + 4, 4))
            assert self.branches[jump][0] == target
            assert [self.words[pc] for pc in range(begin, jump, 4)] == [
                self.words[pc] for pc in range(canonical, target, 4)
            ]
            blocked.update(touched)
            blocked.update(preserved)
            chosen.append((width, begin, jump, canonical, target, keeps_branch))
        return sorted(chosen, key=lambda g: g[1])

    @staticmethod
    def removed(chosen):
        return {
            pc for _, begin, jump, _, _, keep in chosen
            for pc in range(begin + 4 * keep, jump + 4, 4)
        }

    def rewrite(self, chosen):
        removed = self.removed(chosen)
        assert removed.isdisjoint(self.reloc)
        aliases = {begin: canonical for _, begin, _, canonical, _, _ in chosen}
        stubs = {begin: canonical for _, begin, _, canonical, _, keep in chosen if keep}
        rank = {
            pc: self.start + 4 * index
            for index, pc in enumerate(
                pc for pc in range(self.start, self.end, 4) if pc not in removed
            )
        }
        assert all(pc in rank for pc in aliases.values())
        assert all(
            begin in aliases
            for begin in self.heads & removed
        )
        output = []
        for old, new in rank.items():
            word = self.words[old]
            branch = self.branches.get(old)
            if old in stubs:
                word, branch = 0x1400_0000, (stubs[old], (0, 26))
            if branch:
                target, field = branch
                new_target = rank[aliases.get(target, target)]
                word = self.scope["branch_opcode"](word, new, new_target, field)
                assert self.scope["branch_target"](new, word, field) == new_target
            output.append(word)
        # Every preserved non-stub copy remains byte-identical. All removed
        # transfer words occur in one of the proved equal sequences above.
        assert all(output[(new - self.start) // 4] == self.words[old]
                   for old, new in rank.items()
                   if old not in self.branches and old not in stubs)
        assert len(output) * 4 == self.end - self.start - len(removed) * 4
        relocs = [
            (rank[pc], bits) for pc, original in self.reloc.items() if self.start <= pc < self.end
            for bits in original
        ]
        return bytearray().join(pack("<I", word) for word in output), relocs


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--minimum", type=int, default=3)
    args = parser.parse_args()
    args.output.mkdir()
    ref = args.reference.resolve(strict=True)
    metadata = json.loads((ref / "metadata.json").read_text())
    scope = pinned_program(ref, args.output)
    root = Root(scope, metadata)
    chosen = root.select(args.minimum)
    assert chosen
    body, relocs = root.rewrite(chosen)
    root_bytes = len(body)
    padding = (scope["extent"] - root.start - len(body)) % 64
    body.extend(pack("<I", 0xd503_201f) * (padding // 4))
    candidate = args.output / "program.o"
    total, relocations = scope["replace"](candidate, body, relocs)
    assert padding % 4 == 0
    metadata["code_bytes"] = root_bytes
    (args.output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    # Mach-O text ends on a 64-byte-congruent position; no following helper
    # changes its alignment. No new relocations, calls, or stack slots.
    report = {
        "reference_sha256": sha256((ref / "program.o").read_bytes()).hexdigest(),
        "candidate_sha256": sha256(candidate.read_bytes()).hexdigest(),
        "reference_text_bytes": scope["text_size"],
        "candidate_text_bytes": total,
        "root_instruction_bytes": root_bytes,
        "root_frame_bytes": metadata["stack_bytes"],
        "relocations": relocations,
        "groups": [
            {"candidate": hex(begin), "copy_words": width, "shared_copy": hex(canonical),
             "continuation": hex(target), "fallthrough_branch_replaces_terminal_branch": keep}
            for width, begin, _, canonical, target, keep in chosen
        ],
    }
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
