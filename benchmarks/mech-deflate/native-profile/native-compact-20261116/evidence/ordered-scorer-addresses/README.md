# Ordered scorer address reuse

The pinned AArch64 scorer repeatedly computes the same address parts and
reloads immutable constants. The diagnostic keeps the offset-cost subtable
base and two vector masks live, shares the current node address, and emits a
single conditional scalar backedge on the continuing route. It retains every
range check, the original wrapping costs, strict earliest-winner order, and
the original stack frame. The object hash, direct branches, literal addresses
and two exact Mach-O relocations bind the transformation to the original
encoder. [Portable object reproduction](prototype/rebuild-scorer.py)

The original measured block visits project **908,028 fewer scorer
instructions (11.7%)** on 8 KiB natural text and **421,770 fewer (11.9%)** on
4 KiB natural text. The scorer has zero visits on the two recorded random
cases and empty input. [Original block counts](original-counter-visit.json)
· [Reduced object proof](identities/shrink/object-proof.json)

| Apple M1 | Control | Smaller prototype |
| --- | ---: | ---: |
| Encoder text | 71,768 B | 71,704 B |
| Encoder text / Roc | 1.24390× | 1.24279× |
| Encoder text / libdeflate | 1.93549× | 1.93376× |
| Linked executable text | 474,692 B | 474,628 B |
| Linked executable file | 956,056 B | 956,056 B |

The smaller prototype ran **1.556% faster** than the simultaneous control on
8 KiB natural text. The 90% paired interval was **−1.682% to −1.419%** for
candidate/control change. The prototype took **1.01539×** and **1.02201×**
libdeflate's time in the two timing orders. The 4, 16, and 32 KiB natural
cases also improved in each order. Both orders preserve all individual
peer wins at **11/11 Roc and 8/11 libdeflate**.
[Timing ratios and per-case bounds](report.json)

Policy v1 rejects the smaller prototype. Empty input slowed **1.722%**
(90% interval **+1.130% to +2.329%**) and one-stored slowed **1.369%**
(**+0.758% to +2.000%**). The seven-stored-prefix forward order is adverse
and the two orders together leave that case unresolved. Each timing order
used 96 balanced blocks per case with the original timing clock, pinned
workloads, and exact worker identities.
[Both raw orders](raw/) · [Pinned plan](shrink/plan.json) · [Replay](shrink/focus-replay.json)

Both linked prototypes passed 162 exact byte, capacity, alias and
independent inflation checks with the original metadata and empty cleanup.
The reduced body moves 28 native symbols by a whole number of 64-byte
blocks. A separate address-preserving link leaves all other linked text and
all symbol addresses identical for a layout-controlled follow-up.
[Reduced worker checks](identities/shrink/scorer-invariants-qualification.json)
· [Address-preserving worker checks](identities/preserve/scorer-invariants-qualification.json)
· [Linked layout proof](linked-layout-proof.json)

Obtain the 71,768-byte source-emitted object with the
fresh-owner-counts-cold-tails-zero-branches recipe from the PR.
The build and object roles for that recipe are described in the
[checked-root evidence](../branch-on-zero/README.md).

~~~sh
evidence=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/ordered-scorer-addresses
python3 -B "$evidence/prototype/rebuild-scorer.py" /tmp/retained/native/program.o /tmp/scorer-shrink --shrink
python3 -B "$evidence/replay.py"
~~~
