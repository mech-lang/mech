# Collector-only packets and the checked scorer fallback

The experimental LLVM-free encoder emits **68,840 B** from the same
Mech source and canonical bytecode. This is **2,352 B** smaller than the
maintained 71,192 B encoder, **1.193× Roc**, and **1.857× libdeflate**.
The compiler patch and the generated object are pinned in
[candidate.json](candidate.json).

The collector's general geometry already refers to the source
continuation. That makes it possible to use its proven smaller call
interface without requiring the other numeric phases to adopt it.
The maximum simultaneous phase input and output space falls from
**440 + 616 B to 232 + 136 B**. Both areas remain disjoint and synchronous.
Read and write views, their owners, and the caller's checked continuations
keep their existing contracts. The compiler re-proves all mapped
regions on the new graph. The previous sorter and the existing
near-parity scorer have the same instructions and relocations.
[Packet and code measurements](packet-and-code.json) ·
[Controller mapping](controller-window-map.json) ·
[Continuation mapping](continuation-window-map.json) ·
[Matcher layout proof](matcher-profile-proof.json)

The no-growth scorer uses the original capacity guard and goes to the
checked continuation on refusal. Only the private packet has been
written at that point. The separate growing scorer becomes unreachable
from the exported entry. Moving the refusal by itself still measured
5,296 B peak. With collector-only packing the checked-entry frame is
3,072 B and the measured peak is **4,624 B**, down **656 B**. The linked
validation covered **162 exact calls** and six checked failures.
Another 162 exact calls on the restricted-scratch fixture exercise
growth with the compact interface active. [Linked checks](linked-checks.json) ·
[Growth fixture](small-scratch.json)

## Runtime and build comparisons

| 15-case screen | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: |
| Collector-only, forward | 15/15 | 12/15 | 1.00064× |
| Collector-only, reverse | 15/15 | 12/15 | 1.00262× |

The collector-only variant recovers both 1 KiB random-input wins lost
by packing all eligible phases. It preserves the control's wins on
all 15 cases in each order. Natural 8 KiB remains just over parity in
each order. Full qualification requires 81/81 against Roc and at
least 62/81 against libdeflate in each complete order. This result is
a focused experiment. [Focused results](focused.json) ·
[All-phase packing results](unscoped-focused.json)

The earlier bottom-tested scorer reduced the collector-only object to
68,808 B, with natural 8 KiB at 1.00058× and 1.00723× libdeflate.
Reusing the first reverse address on continuation emitted 68,824 B,
with natural 8 KiB at 1.00376× and 1.00472×. Deferring item packing
while retaining the incoming minimum saved 137,916 OR operations in
six 8 KiB invocations and added 51,450 total instructions. That variant
lost the case against its near-parity control. Encoding the initial item
with a reversible XOR also grew the composed scorer, from 1,152 B to
1,296 B. [Bottom-tested results](bottom-tested-focused.json) ·
[Cursor results](shared-cursor-focused.json) ·
[Deferred item counters](deferred-winner-counts.json)

In eight fresh-process M1 trials per compiler, the scoped encoder built
from source through final link in **2.434 s**. The maintained
encoder took **2.443 s**, Roc **2.988 s**, and libdeflate
**0.483 s**, with the same fixed deployment support prebuilt. The
candidate's paired ratios are **0.814× Roc** and **5.046×
libdeflate**. Its stripped executable is **276,056 B**.
[Matched deployment measurements](deployment.json)

## Replay

Apply [compiler.patch.gz](compiler.patch.gz) in a working copy of the
review branch, then rebuild with the documented direct frame-copy
recipe. With the experimental flags disabled, the compiler produces
the maintained encoder, bytecode and metadata. The replay applies the
recorded packet and layout settings to that compiler and checks the
three candidate artifact hashes.

```sh
python3 -B replay.py /tmp/mech-frame-copies-71192 /tmp/mech-scoped-packed
```
