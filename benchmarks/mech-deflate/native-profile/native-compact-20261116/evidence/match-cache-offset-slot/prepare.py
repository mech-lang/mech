"""Produce a small source-only DEFLATE match-cache representation experiment."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess

root = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP")
here = Path(__file__).resolve().parent
head = "78b064c5bb70937db72db21ebd3ab2fd556816c3"
source = "benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec"
old = subprocess.check_output(["git", "show", f"{head}:{source}"], cwd=root).decode()
tables = re.search(r"^deflate-table-offset-slots := \[(.*)\]$", old, re.M)
table = [int(v.removesuffix("u8")) for v in tables[1].split()]
assert len(table) == 256
value = old


def replace(before, after, count=1):
    global value
    assert value.count(before) == count, before
    value = value.replace(before, after)


replace(
    "bt-child-slot(node<u64>)",
    """// Match records use bits 0..8 for length and 9..13 for the offset slot.
// Literal/count headers still use the full low half of bt-put-entry.
bt-put-match(cache<[u32]>, ptr<u64>, length<u64>, offset<u64>) => (out<[u32]>) :=
  cached-offset := bits/and(offset, 65535u64)
  slot := optimal-offset-slot(cached-offset, deflate-table-offset-slots)
  packed-length := bits/or(length, bits/shift-left-wrap(slot, 9u64))
  out := bt-put-entry(cache, ptr, packed-length, cached-offset).
bt-child-slot(node<u64>)""",
)
replace("bt-put-entry(cache, cache-ptr, extended,", "bt-put-match(cache, cache-ptr, extended,")
replace("bt-put-entry(cache, cache-ptr, 3u64, distance)", "bt-put-match(cache, cache-ptr, 3u64, distance)")
replace(
    "bits/and(bt-word32(next-cache[next-pointer]), 65535u64)",
    "bits/and(bt-word32(next-cache[next-pointer]), 511u64)",
)
replace("  best := bits/and(entry, 65535u64)\n", "  best := bits/and(entry, 511u64)\n")
replace(
    "        offset := bits/shift-right-wrap(optimal-word32(optimal-get32(cache, m)), 16u64)\n",
    "        entry := optimal-word32(optimal-get32(cache, m))\n"
    "        offset := bits/shift-right-wrap(entry, 16u64)\n",
    count=2,
)
replace(
    "        this-length := bits/and(optimal-word32(optimal-get32(cache, m)), 65535u64)\n",
    "        this-length := bits/and(entry, 511u64)\n",
    count=2,
)
replace(
    "        offset-cost := optimal-get32(costs, bits/wrapping-add(optimal-offset-slot(offset, slots), 515u64))\n",
    "        slot := bits/and(bits/shift-right-wrap(entry, 9u64), 31u64)\n"
    "        offset-cost := optimal-get32(costs, bits/wrapping-add(slot, 515u64))\n",
)
replace(
    "number<u64>, costs<[u32]>, slots<[u8]>, best<u32>, item<u32>) => <optimal-matches>",
    "number<u64>, costs<[u32]>, best<u32>, item<u32>) => <optimal-matches>",
)
replace(
    "#PackedOptimalPathMatches(nodes, cache, cp, number, costs, slots, best, item)",
    "#PackedOptimalPathMatches(nodes, cache, cp, number, costs, best, item)",
)
replace(
    "#PackedOptimalPathMatches(nodes, cache, header-ptr, number, costs, slots, best, item)",
    "#PackedOptimalPathMatches(nodes, cache, header-ptr, number, costs, best, item)",
)
replace(
    "cache<[u32]>, costs<[u32]>, slots<[u8]>) => (out<([u64],u64,u64)>)",
    "cache<[u32]>, costs<[u32]>) => (out<([u64],u64,u64)>)",
)
replace(
    "cache<[u32]>, costs<[u32]>, slots<[u8]>) => (out<[u64]>)",
    "cache<[u32]>, costs<[u32]>) => (out<[u64]>)",
)
replace("optimal-packed-path-position(nodes, cur, cp, cache, costs, slots)",
        "optimal-packed-path-position(nodes, cur, cp, cache, costs)")
replace("optimal-packed-path-scan(next-nodes, next-cur, next-cp, cache, costs, slots)",
        "optimal-packed-path-scan(next-nodes, next-cur, next-cp, cache, costs)")
replace(
    "nodes0<[u64]>, costs<[u32]>, slots<[u8]>) => (out<optimal-path>)",
    "nodes0<[u64]>, costs<[u32]>) => (out<optimal-path>)",
)
replace("optimal-packed-path-scan(nodes, block-length, cache-end, cache, costs, slots)",
        "optimal-packed-path-scan(nodes, block-length, cache-end, cache, costs)")
replace("optimal-find-packed-cost-path(length, cache, cache-end, nodes, model, offset-slots)",
        "optimal-find-packed-cost-path(length, cache, cache-end, nodes, model)")

# Header decoders, copy/rewind, and the previous-match cursor keep their old
# formats. All four current match readers use the nine-bit length.
assert value.count("optimal-checked-match-length(this-length) - 2u64") == 2
assert value.count("bt-put-entry(next-cache, next-pointer, bits/wrapping-sub(next-pointer, pointer), byte(input, position))") == 2
assert value.count("optimal-get32(cache, header-ptr)), 65535u64)") == 2
assert value.count("optimal-get32(cache, header-ptr)), 16u64)") == 2
assert value.count("optimal-counter(cache, header-at)), 65535u64)") == 1
assert value.count("bits/and(bt-word32(next-cache[next-pointer]), 511u64)") == 1
assert value.count("  best := bits/and(entry, 511u64)\n") == 1
assert value.count("  this-length := bits/and(entry, 511u64)\n") == 2
assert value.count("  searched-match := bits/or(bits/shift-left-wrap(best, 16u64), bits/shift-right-wrap(entry, 16u64))") == 1
assert value.count("bits/and(previous-match, 65535u64)") == 1
assert value.count("bits/shift-right-wrap(previous-match, 16u64)") == 1
assert value.count("optimal-get32(cache, m)") == 2


def old_slot(offset):
    n = ((256 - offset) & 0xffff_ffff) >> 29
    at = ((offset - 1) % (1 << 64)) >> n
    return (table[at] if at < len(table) else 0) + 2 * n


def arithmetic_slot(distance):
    if distance == 0:
        return 0
    if distance > 32768:
        return 14
    at = distance - 1
    high = at.bit_length() - 1
    return at if at < 4 else 2 * high + ((at >> (high - 1)) & 1)


for offset in range(1 << 16):
    slot = old_slot(offset)
    assert 0 <= slot < 32
    assert slot == arithmetic_slot(offset)
    for length in (0, 2, 3, 4, 255, 256, 258, 259, 511):
        packed = (offset << 16) | (slot << 9) | length
        assert packed >> 16 == offset
        assert packed & 511 == length
        assert (packed >> 9) & 31 == slot

path = here / "offset-cache-slot.patch"
path.write_text("".join(unified_diff(old.splitlines(keepends=True), value.splitlines(keepends=True),
                                    fromfile=f"a/{source}", tofile=f"b/{source}")))
(here / "program.mec").write_text(value)

lookup = "  slot := optimal-offset-slot(cached-offset, deflate-table-offset-slots)\n"
assert value.count(lookup) == 1
arith = value.replace(lookup, "  slot := bt-cached-offset-slot(cached-offset)\n").replace(
    "bt-put-match(cache<[u32]>",
    """bt-cached-offset-slot(offset<u64>) => (out<u64>) :=
  ordinal := bits/wrapping-sub(offset, 1u64)
  high := bits/wrapping-sub(63u64, bits/leading-zeros(ordinal))
  second := bits/and(bits/shift-right-wrap(ordinal, bits/wrapping-sub(high, 1u64)), 1u64)
  regular := bits/or(bits/shift-left-wrap(high, 1u64), second)
  large := ((offset == 0u64)? | true => 0u64 | false => 14u64)
  out := ((ordinal < 4u64)? | true => ordinal | false => ((ordinal < 32768u64)? | true => regular | false => large)).
bt-put-match(cache<[u32]>""",
    1,
)
(here / "offset-cache-slot-arithmetic.patch").write_text("".join(unified_diff(
    old.splitlines(keepends=True), arith.splitlines(keepends=True),
    fromfile=f"a/{source}", tofile=f"b/{source}",
)))
(here / "program-arithmetic.mec").write_text(arith)
receipt = {
    "source_head": head, "path": source,
    "control_source_sha256": sha256(old.encode()).hexdigest(),
    "candidate_source_sha256": sha256(value.encode()).hexdigest(),
    "all_stored_u16_offsets_round_trip": True,
    "arithmetic_equivalence_stored_offset_range": [0, 65535],
    "arithmetic_source_sha256": sha256(arith.encode()).hexdigest(),
    "checked_source_length_interval": [3, 258],
    "compile_link_runtime": "unverified",
}
(here / "proof.json").write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps(receipt, indent=2))
