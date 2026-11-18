"""Check the source pass's raw instruction predicates against the pinned image."""

from collections import Counter
from pathlib import Path
import json
import sys
from shared_copy_tails import Root, pinned_program

reference = Path(sys.argv[1]).resolve(strict=True)
result = reference
scope = pinned_program(reference, result)
metadata = json.loads((reference / "metadata.json").read_text())
root = Root(scope, metadata)
original = root.select(minimum=3)
words = [root.words[pc] for pc in range(root.start, root.end, 4)]
edges = {(at-root.start)//4: (to-root.start)//4 for at,(to,_) in root.branches.items()}
external = {(at-root.start)//4 for at in root.reloc if root.start <= at < root.end}
ends = {
    index for index, word in enumerate(words)
    if word & 0xfffffc1f == 0xd65f0000
    or word in (0xd65f0bff,0xd65f0fff)
    or word & 0xffe0001f == 0xd4200000
    or word & 0xffff0000 == 0
}
protected = external | {
    index for index, word in enumerate(words)
    if word & 0xffe0001f == 0xd4200000 or word & 0xffff0000 == 0
}
barriers = external | {index for index, word in enumerate(words) if word & 0xfffffc1f == 0xd63f0000}
heads = set(edges.values()) | {ex+1 for ex in edges.keys() | barriers | ends if ex+1 < len(words)} | {0}


def frame_copy(word):
    return (
        word & 0xffc003e0 in (0xf90003e0, 0xf94003e0)
        and word & 31 < 29
        and (((word >> 10) & 0xfff) + 1) * 8 <= metadata["stack_bytes"] - 16
    )


branch_entries = Counter(edges.values())
candidates = []
first = (root.first-root.start)//4
for jump in range(first, len(words)):
    if jump not in edges or words[jump] & 0xfc000000 != 0x14000000 or jump in protected or jump in heads:
        continue
    target = edges[jump]
    copies = 0
    while copies < 64:
        here, retained = jump-copies, target-copies
        if (here <= first or retained <= first or here in heads or copies and retained in heads
            or here-1 in protected or retained-1 in protected or words[here-1] != words[retained-1]
            or not frame_copy(words[here-1])):
            break
        copies += 1
    if copies and branch_entries[jump-copies]>1:
        copies -= 1
    if copies >= 3:
        begin = jump-copies
        prior = begin-1
        prior_jump = prior in edges and words[prior] & 0xfc000000 == 0x14000000
        candidates.append((copies, begin, jump, target-copies, target, not prior_jump and prior not in ends))
occupied, removed, chosen = set(), set(), []
for copies, begin, jump, retained, target, keeps in sorted(
         candidates, key=lambda r: (r[0], r[1], r[3]), reverse=True):
    touched = set(range(begin, jump+1))
    kept = set(range(retained, target+1))
    if touched & kept or touched & occupied or kept & removed:
        continue
    occupied.update(touched | kept)
    removed.update(touched)
    chosen.append((copies, *(root.start+index*4 for index in (begin,jump,retained,target)), keeps))

chosen.sort(key=lambda r:r[1])
assert chosen == original, (chosen, original)
body, relocs = root.rewrite(chosen)
assert len(body) == 50632
assert sum(copies for copies, *_ in chosen) == 152
print("The proposed raw instruction predicates and branch selection find the exact seven measured substitutions.")
print("Root instruction bytes",len(body),"root relocations",len(relocs))
