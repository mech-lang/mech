"""Insert a reduced-state unmatched run into the original Mech collector."""

from hashlib import sha256
import json
from pathlib import Path

study = Path(__file__).resolve().parent
packet = study.parent / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
original = (packet / "program.mec").read_bytes()
assert sha256(original).hexdigest() == json.loads((packet / "recipe.json").read_text())["source_sha256"]
source = original.decode()

helpers = r'''
// The old collector visits its checkpoints after each position. Bound a run
// before the first checkpoint or limit so that step still makes the decision.
optimal-unmatched-end(position<u64>, base<u64>, next-slide<u64>, pointer<u64>, begin<u64>, maximum-end<u64>, end<u64>, cache-length<u64>, number<u64>) => (out<u64>) :=
  remaining := bt-saturating-sub(end, position)
  since-begin := bt-saturating-sub(position, begin)
  since-slide := bt-saturating-sub(position, base)
  fresh-needed := bt-saturating-sub(512u64, number)
  distance-needed := bt-saturating-sub(5000u64, since-begin)
  need := ((fresh-needed > distance-needed)? | true => fresh-needed | false => distance-needed)
  checkpoint := ((need == 0u64)? | true => 1u64 | false => need)
  checkpoint-possible := checkpoint <= bt-saturating-sub(remaining, 5000u64)
  block-room := bt-saturating-sub(bt-saturating-sub(maximum-end, position), 1u64)
  record-room := bt-saturating-sub(bt-saturating-sub(1500000u64, pointer), 1u64)
  room := bt-minimum(block-room, bt-minimum(record-room, bt-saturating-sub(cache-length, pointer)))
  word-room := bt-minimum(bt-saturating-sub(remaining, 4u64), bt-saturating-sub(next-slide, position))
  count-room := bt-saturating-sub(18446744073709551615u64, number)
  safe := bt-minimum(room, bt-minimum(word-room, bt-minimum(count-room, bt-saturating-sub(32768u64, since-slide))))
  steps := (checkpoint-possible? | true => bt-minimum(safe, checkpoint - 1u64) | false => safe)
  out := bits/wrapping-add(position, steps).

<optimal-unmatched> := <{tables<[u16]> cache<[u32]> next-hash3<u64> next-hash4<u64> next-position<u64> cache-ptr<u64> fresh-observations<[u32]>}>

// The checked caller establishes all three table partitions and the two
// output ranges. No comparison past a stale root can produce a match.
optimal-record-unmatched(tables<[u16]>, cache<[u32]>, input<[u8]>, observations<[u32]>, position<u64>, cur<u64>, pointer<u64>, slot3<u64>, slot4<u64>, node3<u16>) => (out<optimal-unmatched>) :=
  next-sequence := prefix-readable(input, position + 1u64)
  seq3 := bt-narrow32(bits/and(bt-word32(next-sequence), 16777215u64))
  next-hash3 := lz-hash(seq3, 16u64)
  next-hash4 := lz-hash(next-sequence, 16u64)
  pos := bt-narrow16(cur + 32768u64)
  ~mf := tables
  mf[slot3 + 1u64] = pos
  mf[slot3 + 2u64] = node3
  mf[slot4 + 1u64] = pos
  child := bt-child-slot(cur)
  next-tables := bt-clear(mf, child, child + 1u64)
  literal := byte-readable(input, position)
  next-cache := bt-put-entry(cache, pointer, 0u64, literal)
  kind := bits/or(bits/and(bits/shift-right-wrap(literal, 5u64), 6u64), bits/and(literal, 1u64))
  next-observations := optimal-increment-counter(observations, kind)
  out := {tables: next-tables, cache: next-cache, next-hash3: next-hash3, next-hash4: next-hash4, next-position: position + 1u64, cache-ptr: pointer + 1u64, fresh-observations: next-observations}.

optimal-unmatched-step(tables<[u16]>, cache<[u32]>, hash3<u64>, hash4<u64>, input<[u8]>, position<u64>, base<u64>, pointer<u64>, observations<[u32]>) => (out<optimal-unmatched>) :=
  cur := position - base
  slot3 := 131072u64 + 2u64 * bits/and(hash3, 65535u64)
  slot4 := 65536u64 + bits/and(hash4, 65535u64)
  node3 := tables[slot3 + 1u64]
  node4 := tables[slot4 + 1u64]
  stale := (bt-word16(node3) <= cur) && (bt-word16(node4) <= cur)
  out := (stale?
    | true => optimal-record-unmatched(tables, cache, input, observations, position, cur, pointer, slot3, slot4, node3)
    | false => {tables: tables, cache: cache, next-hash3: hash3, next-hash4: hash4, next-position: position, cache-ptr: pointer, fresh-observations: observations}).

optimal-unmatched-run(tables<[u16]>, cache<[u32]>, hash3<u64>, hash4<u64>, input<[u8]>, position<u64>, base<u64>, pointer<u64>, observations<[u32]>, through<u64>) => (out<optimal-unmatched>) :=
  step := ((position < through)?
    | true => optimal-unmatched-step(tables, cache, hash3, hash4, input, position, base, pointer, observations)
    | false => {tables: tables, cache: cache, next-hash3: hash3, next-hash4: hash4, next-position: position, cache-ptr: pointer, fresh-observations: observations})
  out := ((step.next-position > position && step.next-position < through)?
    | true => optimal-unmatched-run(step.tables, step.cache, step.next-hash3, step.next-hash4, input, step.next-position, base, step.cache-ptr, step.fresh-observations, through)
    | false => step).

'''
position = source.index("\noptimal-block-loop(")
source = source[:position + 1] + helpers + source[position + 1:]
old = "  advanced := optimal-advance-position(tables, cache, hash3, hash4, input, position, base, next-slide, skip, pointer, fresh-frequencies, fresh-observations, number-fresh, next-observation, minimum)\n"
new = '''  // Aligned probes amortize the extra admission when matches are frequent.
  probe := (bits/and(position, 15u64) == 0u64) && (skip == 0u64) && (position == next-observation)
  valid := probe && (minimum > 0u64) && (begin <= position) && (base <= position) && (sequence/length(tables) >= 262144u64) && (sequence/length(fresh-observations) >= 8u64)
  through := (valid?
    | true => optimal-unmatched-end(position, base, next-slide, pointer, begin, maximum-end, sequence/length(input), sequence/length(cache), number-fresh)
    | false => position)
  run := ((through > position)?
    | true => optimal-unmatched-run(tables, cache, hash3, hash4, input, position, base, pointer, fresh-observations, through)
    | false => {tables: tables, cache: cache, next-hash3: hash3, next-hash4: hash4, next-position: position, cache-ptr: pointer, fresh-observations: fresh-observations})
  updates := run.next-position - position
  resumed-observation := ((updates != 0u64)? | true => run.next-position | false => next-observation)
  advanced := optimal-advance-position(run.tables, run.cache, run.next-hash3, run.next-hash4, input, run.next-position, base, next-slide, skip, run.cache-ptr, fresh-frequencies, run.fresh-observations, number-fresh + updates, resumed-observation, minimum)
'''
assert source.count(old) == 1
candidate = source.replace(old, new)
for span in (helpers, new):
    assert span in candidate
(study / "control.mec").write_bytes(original)
(study / "candidate.mec").write_text(candidate)
(study / "pinned-source.json").write_text(json.dumps({
    "control": sha256(original).hexdigest(),
    "candidate": sha256(candidate.encode()).hexdigest(),
    "scope": "Private source experiment with a reduced-state unmatched sequence. All constants are collector limits or the probe stride.",
}, indent=2) + "\n")
