# Canonical parser storage

The literal parser follows the base parser's reusable storage design. It keeps
the suspended base or literal scanner in a reusable slot outside the ordinary
frame stack. Reading a smaller frame preserves the same grammar transition and
work charge. A suspended scanner remains owned by its continuation. A completed
continuation returns its storage to the parser for the next literal.

The one-shot subtree cache copies ranges of at most 16 events into one
request-order queue. These requests release their source pages immediately.
Longer requests and streaming snapshots retain the existing persistent journal.
Both paths replay the same event values and can refresh a cached child when
the live marker identity proves it is the same child. The bounded capture
copies at most 16 events from at most two pages. The materialization loop
uses the same work allowances and releases the capture before publication.

The current compiler profile found event page copying and literal frame moves.
The census counted **1,078,128** short
ranges among **1,414,476** requests. In eight
balanced standalone orders, pooling alone reduced the parser median by
**9.11%** and bounded captures reduced it by
**6.32%**. Borrowing the cache in place by itself was
**0.87% slower**. All ablations kept the same tree fingerprint,
parser steps and event count. [Window census](census/receipt.json) · [Raw parser trials](probe-timings/report.json.gz)

## Complete builds on Apple M1

| Configuration | Encoder text | Parsing | Source through final link |
| --- | ---: | ---: | ---: |
| Mech retained control | 72,520 B | 2.45758 s | 4.53348 s |
| Mech retained with both parser changes | 72,520 B | 2.10879 s | 4.17810 s |
| Mech smaller control | 71,960 B | 2.45876 s | 4.53898 s |
| Mech smaller with both parser changes | 71,960 B | 2.10957 s | 4.18694 s |
| Roc | 57,696 B | | 2.98712 s |
| libdeflate | 37,080 B | | 0.48026 s |

The six complete orders use prebuilt compilers, a fixed Cranelift emitter,
fixed runtimes and the same link adapters. The 71,960-byte encoder builds
**7.76% faster**, with parsing **14.20% faster**.
The paired whole-build change is -7.79% with a 90%
interval of -7.90% to -7.69%.
Mech takes **1.40167× Roc** and **8.71812×
libdeflate** time. Its encoder text is **1.24723× Roc** and
**1.94067× libdeflate**. [Report](report.json) · [Raw builds](matched/timing.json.gz)

The smaller configuration spends 2.438 s in the
frontend, including 2.110 s parsing. Native emission takes
1.710 s and final linking takes 0.040 s.
The raw trials keep compiler RSS, object size and linked executable size
alongside the timing results. [Build profiles](matched/profiles/)

The size ratios use the matched 37,080-byte libdeflate text section. The raw
identity receipts preserve the earlier helper's 41,112-byte input value.
The compiler construction log records the separate Rust release rebuild.
[Compiler construction](integrated-cargo.jsonl.gz)

## Identity and verification

- **533 syntax unit tests** passed. The cache test covers copied and retained
  ranges across a page boundary, stale marker identities, and small resumed grants.
- **60 exact snapshot comparisons** cover
  malformed, Unicode, split-source, finite-fuel and finite-event cases. The
  comparisons include the tree, node and restart indexes, diagnostics and statistics.
- The complete benchmark source keeps its tree fingerprint and canonical
  bytecode. Every timed Mech build checks its object, metadata and executable.
- The newly built compiler reproduces the retained, smaller and aligned-spill
  benchmark workers byte for byte, with **486 exact linked calls** across
  those workers. The retained and smaller workers keep their recorded
  **81/81 Roc and 61/81 libdeflate** wins in both full orders.
  The aligned-spill worker remains an opt-in diagnostic with its earlier
  per-case rejection.
- Three smaller executable inputs also keep their exact bytecode.

[Syntax receipts](syntax-tests/receipt.json) · [Artifact identities](identity/receipt.json) · [Small inputs](small/report.json)

The maintained Cranelift rebuild takes the parser changes directly from
this PR. Run it from the repository root on Apple ARM64.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py   /tmp/mech-parser-storage   --compiler-opt-level 3 --preselect-root-inlining
```

Add `--variant fresh-owner-counts-cold-tails` to check the smaller recipe.
The rebuild's bytecode, object and metadata checks pin those outputs.
