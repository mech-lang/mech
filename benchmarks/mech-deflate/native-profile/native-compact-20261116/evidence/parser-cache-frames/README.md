# Reusable parser cache frames

Each parser keeps one subtree sink while it processes the cache queue. A
partial request retains its sink state and frame stack. On completion the
cache takes ownership of the root and the next request reuses the empty
stack. Request order, event charges, IDs and the final diagnostic sink
follow the same paths.

Separate instrumentation counted **706,474 frame stack growths before and
one after**. Both executions push the same 706,474 frames, with maximum
depth one on this input. The pooled parser went from **2.09175 to 2.04431 s**
in eight balanced parser-only pairs with identical trees, events, and
parser steps. A separate three-page cache window experiment ran slower
and is recorded with its source patch.
[Frame census](frames/receipt.json) · [Parser trials](scratch-timings/report.json)
· [Page window screen](probe-timings/report.json)

| Apple M1 | Source through final link | Encoder text |
|---|---:|---:|
| Mech control | 4.01608 s | 71,768 B |
| Mech pooled frames | 3.95160 s | 71,768 B |
| Roc | 2.98682 s | 57,696 B |
| libdeflate | 0.48054 s | 37,080 B |

The measured median paired build reduction is **1.60%**. All eight orders
and the two unusually slow control builds remain in the receipt. Median
parsing within the complete build moves from **2.11619 to 2.04808 s**.
Median compiler RSS moves from 401,465,344 B to 402,432,000 B. The candidate
build is **1.323× Roc** and **8.223× libdeflate**, with prebuilt compilers and
fixed runtime, allocator and link adapters. Compiler reconstruction reused
dependencies and took 3 min 46 s with four Cargo jobs.
[Matched raw trials](matched/timing.json.gz) · [Build and size report](report.json)

The new compiler produces identical bytecode, object, complete metadata,
and benchmark executable. The pinned benchmark worker also relinks to
`674d88602946…` byte for byte and passes 162 exact-output, independent
inflation and alias smoke calls. Its previously recorded full orders were
**81/81 Roc and 61/81 libdeflate**. Earlier missing run bindings leave their
qualification unresolved. The 5,280-byte measured native stack and runtime
private-payload receipts apply to that exact worker.
[Exact runtime identity and qualification](../branch-on-zero/README.md)

All **533 syntax unit tests** pass. The 60 identical syntax snapshots cover
UTF-8 source pieces, malformed input, diagnostic anchors and finite fuel
and event limits. Three small frontend builds reproduce their bytecode.
[Syntax checks](validation/receipt.json) · [Small inputs](small/bytecode-and-timing.json)

Rebuild the fixed object from the PR root with the checked-in source.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py   /tmp/mech-frame-pool --variant fresh-owner-counts-cold-tails-zero-branches   --compiler-opt-level 3 --direct-source --preselect-root-inlining
```
