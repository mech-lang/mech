# Native verification and ownership search

The pinned encoder profile showed repeated CFG integrity checks and guarded
ownership searches. The verifier now compares two ordered successor streams
and two ordered predecessor-instruction streams for each block. Equal streams
prove equal edge sets without materializing temporary sets. Mismatches still
take the original diagnostic path. Dominator and instruction checks run as
before.

The ownership analysis owns one reusable table and frontier per function.
Each query keeps its own facts and the original traversal order. Clearing the
visited slots and pending frontier also handles early returns. The emitter
uses the existing consumption decision when checking whether a held owner
reference can move. Runtime exclusivity remains a heap check.
[Original emitter sample](sample/sample.txt.gz) · [Source identities](baseline.json)

## Matched Apple M1 measurements

| Configuration | Encoder text | Bytecode through native object | Source through final link |
| --- | ---: | ---: | ---: |
| Mech retained control | 72,520 B | 1.69794 s | 4.18188 s |
| Mech retained with work reuse | 72,520 B | 1.59066 s | 4.07447 s |
| Mech smaller control | 71,960 B | 1.70433 s | 4.18568 s |
| Mech smaller with work reuse | 71,960 B | 1.59661 s | 4.07374 s |
| Roc | 57,696 B | | 2.98297 s |
| libdeflate | 37,080 B | | 0.48050 s |

Eight balanced native-only orders show a **6.32% reduction** for
the smaller recipe. The paired change is -6.42% with a 90%
interval of -6.58% to -6.29%.
Six balanced source-through-link orders show a **2.67% reduction**.
The paired whole-build change is -2.64% with a 90% interval
of -2.76% to -2.53%. All complete builds
use the same prebuilt Mech frontend, fixed runtimes and adapters.

The 71,960-byte encoder emits **1.24723× Roc** and
**1.94067× libdeflate** text. Its complete build takes
**1.36567× Roc** and **8.47816× libdeflate** time.
Parsing accounts for 2.109 s of the 2.434 s
frontend. The matched build records 1.598 s of native emission
and 0.041 s for final linking. The matched C object has 37,080
bytes of encoder text. Raw identity receipts preserve the earlier helper's
41,112-byte input value. [Report](report.json) · [Native trials](native-only/timing.json.gz) ·
[Complete build trials](matched/timing.json.gz) · [Build profiles](matched/profiles/)

## Verification

- The rebuilt verifier matches the original library on a valid graph, a stale
  successor, a stale predecessor, and invalid return instructions.
- **43 release tests** passed across guard
  state, old-value reads, fresh owner counts and native cold tails. The guard
  harness now exercises contradictions across packed words and checks that
  the conflicting Boolean home remains stable.
- Every full timing trial checks the canonical bytecode, object and complete
  metadata. Within each Mech recipe the linked outputs match byte for byte.
- The newly built emitter also reproduces the retained, smaller and aligned-spill
  benchmark workers byte for byte. Those workers pass **486 exact calls**.
  The retained and smaller workers carry their recorded **81/81 Roc** and
  **61/81 libdeflate** wins. The aligned-spill variant retains its earlier
  per-case rejection and opt-in status.
- The maintained rebuild independently emits the pinned smaller object and
  metadata. Its separately linked worker passes **162 exact calls**.

[CFG diagnostics](cfg-verifier/receipt.json) · [Tests](tests.log.gz) ·
[Benchmark worker identities](native-work-identities/receipt.json) ·
[Maintained rebuild](recipe-rebuild/rebuild.json)

Rebuild the smaller recipe on Apple ARM64 with the compiler source in this PR.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py   /tmp/mech-native-work-reuse   --variant fresh-owner-counts-cold-tails   --compiler-opt-level 3 --preselect-root-inlining
```
