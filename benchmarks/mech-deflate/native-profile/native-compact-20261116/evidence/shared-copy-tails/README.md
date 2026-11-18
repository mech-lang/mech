# Shared checked-root frame copies

## Compiler-emitted variant

The verified Rust source emits the exact focused-test object and, with the same
fixed linker inputs, the exact timed worker. The compiler serializes the same
metadata as compact JSON. The focused prototype writes formatted JSON. Each
serialization's hash is recorded separately and their JSON values match.
The maintained recipe checks the source-emitted artifacts and its own
optional linked worker passes 162 exact calls. All 13 selected backend tests
passed, including the new frame-copy, relocation, and call-site check.
[Source and link identity](source-proof/exact-source-identity.json) · [Maintained rebuild](source-proof/rebuild.json)

| Matched Apple M1 source through final link | Time | Encoder text |
| --- | ---: | ---: |
| Mech control | 3.82900 s | 71,768 B |
| Mech with frame copies shared | 3.82835 s | 71,192 B |
| Roc | 2.98408 s | 57,696 B |
| libdeflate | 0.47948 s | 37,080 B |

The complete build takes **1.283× Roc** and **7.984× libdeflate** time.
Both Mech builds use the same prebuilt compiler with fixed link inputs.
[Raw matched build orders and phases](source-proof/timing.json.gz)

The predeclared distinct-control study bounds the 4 KiB random follow-up at
**+0.093%** with a 90% interval from **+0.064% to +0.123%**. The original
adverse reverse order remains in the combined replay. The focused candidate
remains **Unresolved** under v1 in the historical 17-case campaign.
[Duplicate-control replay](repeatability/combined-replay.json)

The full campaign records **81/81 Roc** in both orders and **57/81 then
61/81 libdeflate** for this candidate. The exact control records 57 and
60 libdeflate wins. The candidate preserves every individual control win.
The full runtime validator reports **Reject** after the established
`prefix-periodic-16384` slowdown. The unchanged control's shift from the
previous full orders and the adverse order remain in the archive.
[Both full raw orders and validator replay](../frame-copies-71192-runtime/README.md)

Rebuild from the repository root on Apple ARM64.

~~~sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-frame-copies --variant frame-copies --compiler-opt-level 3 \
  --direct-source --preselect-root-inlining
~~~

Seven checked-root suffixes perform the same physical frame loads and stores
as a sequence that already falls into their common continuation. The
prototype uses that fallthrough. It moves an existing unconditional branch
from the end of each removed suffix to its entry. Explicit branch targets
can enter the shared copies directly. The 152 removed instructions carry no
relocation, call, or checked origin. The original copy order, frame size,
calls, checked statuses and origins remain exact. All 498 relocation
records are present.

| Apple M1 encoder | Text bytes | × Roc | × libdeflate |
| --- | ---: | ---: | ---: |
| Exact control | 71,768 | 1.244 | 1.935 |
| Shared frame copies | 71,192 | 1.234 | 1.920 |

The root has 50,632 instruction bytes, down from 51,240. Its 3,728-byte frame
and all 162 measured stack observations match the control, with a 5,280-byte
whole-call peak. Both workers match on all 20 non-timing fields from 162 exact
linked calls, including memory counters. Six final allocation-failure cases
match the status, origin, payload counters, alias checks, and cleanup.
[Object scan](static-report.json) · [Linked checks](checks/linked-checks.log)
· [Stack observations](checks/stack.json)

Both frozen 17-case orders record 17 Roc wins and 13 libdeflate wins, preserving
every individual control win. Sixteen cases meet the declared 0.5% bound.
Empty input is **1.511% faster** with a 90% interval of **0.936% to 2.080%
faster**. Seven-stored-prefix is **1.063% faster** with a **0.524% to 1.600%**
interval. The prefix-random-4096 estimates disagree by order. Forward is
−0.267% with bounds [−0.594%, +0.058%]. Reverse is +1.459% with bounds
[+0.838%, +2.106%]. The v1 decision is **Unresolved**. Both raw orders
preserve the conflict for replay.
[Predeclared plan](plan.json) · [Focused report](report.json)

Reproduce the static object from the verified fresh-owner-counts-cold-tails-zero-branches
encoder on Apple AArch64. Supply the folder containing its program.o, metadata
and receipt. The portable probe reuses the reviewed Mach-O relocation remapper.
Then replay the two focused timing orders from this benchmark packet.

~~~sh
python3 -B evidence/shared-copy-tails/prototype/shared_copy_tails.py \
  /tmp/mech-shared-copies --reference /tmp/mech-current/native
python3 -B evidence/shared-copy-tails/replay-archive.py
~~~
