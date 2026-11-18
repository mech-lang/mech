# Reuse dominating MechDeflate definitions in native SSA

The source graph proposes typed scalars with one definition. The census
records their physical reads and writes after generated routes are in place.
The exact control produced identical object, bytecode, and metadata with
the census enabled. It proved **2,471** scalar homes covering **3,511** reads.
Direct mode reuses the assigned SSA value at those reads. The compiler
verifies exactly one emitted write, no earlier read, and that the write
block dominates every read on the finalized native graph.
[Scalar census](scalar/census-summary.json) · [Compiler patch](scalar/compiler.patch.gz)

Combining this with empty-jump forwarding reduced values allocated before
compaction from **1,171,123 to 1,128,603**. It emitted **67,272 B** of
ARM64 text and measured **4,400 B** peak stack. The same compiler with
forwarding alone emits **67,656 B**. The matcher, scorer, and checked region
proofs retained their pinned identities. Each linked comparison covered
**162** output and resource observations and six checked error exits.
[Linked proof](scalar/linked-result.json.gz) ·
[Verified encoder replays](verified-replays.json)

The field census proved **6,831** additional non-owning fields and **8,404** reads. It includes ordinary scalar product fields and array views. The proof excludes owner tokens and tagged payloads.
The combined variant creates **831,612** values before compaction and emits **68,424 B**. All 24 controller windows survived. The compiler emits **2** previously shared continuations inline. Excluding the **20** fields visible in those continuation graphs produced the same encoder bytes in both variants.
[Field proof summary](fields/census-summary.json) · [Compiler patch](fields/compiler.patch.gz)
[Continuation rebinding](fields/size-results.json) · [Field exclusion](fields/continuation-exclusion.json)


## Source through final link

Fresh processes compiled the same supplied MechDeflate source on Apple
M1, macOS 15.6.1, with fixed support prebuilt. The scalar session used
twelve matched orders. Each session includes its own Roc and libdeflate
controls. Phase medians and paired ratios are available in the deployment
receipts.

| Session | Build | Source through link | Native phase |
| --- | --- | ---: | ---: |
| Scalar | Mech size control | 2.430 s | 1.346 s |
| Scalar | Mech with forwarded jumps | 2.383 s | 1.299 s |
| Scalar | Scalar definitions | 2.422 s | 1.336 s |
| Scalar | Scalar definitions with forwarded jumps | 2.375 s | 1.290 s |
| Scalar | Roc | 2.987 s | — |
| Scalar | libdeflate | 0.480 s | — |
| Fields | Mech with forwarded jumps | 2.385 s | 1.299 s |
| Fields | Non-owning field definitions | 2.323 s | 1.237 s |
| Fields | Scalar and non-owning field definitions | 2.305 s | 1.219 s |
| Fields | Roc | 2.986 s | — |
| Fields | libdeflate | 0.481 s | — |

The scalar build with forwarded jumps takes **2.375 s** from
source through link. [Scalar deployment measurements](scalar/deployment.json)

## Compression runtime

Both scalar experiments lost `320-overlap` and
`prefix-periodic-1024` to libdeflate in both focused orders.
Each focused order covers 15 cases. Complete orders cover 81.
Natural 8 KiB is paired compression time relative to libdeflate.

| Experiment and order | Roc wins | libdeflate wins | Natural 8 KiB |
| --- | ---: | ---: | ---: |
| [Scalar definitions focus forward](scalar/direct-small-original-windows/focus-screen.json) | 15/15 | 10/15 | 1.00135× |
| [Scalar definitions focus reverse](scalar/direct-small-original-windows/focus-screen.json) | 15/15 | 10/15 | 1.00645× |
| [Scalar definitions with forwarded jumps focus forward](scalar/direct-fast-original-windows/focus-screen.json) | 15/15 | 10/15 | 1.00318× |
| [Scalar definitions with forwarded jumps focus reverse](scalar/direct-fast-original-windows/focus-screen.json) | 15/15 | 10/15 | 1.00463× |
| [Non-owning field definitions focus forward](fields/direct-fields-rebound/focus-screen.json) | 15/15 | 12/15 | 0.99305× |
| [Non-owning field definitions focus reverse](fields/direct-fields-rebound/focus-screen.json) | 15/15 | 12/15 | 1.00182× |
| [Scalar and non-owning field definitions focus forward](fields/direct-fields-and-scalars-rebound/focus-screen.json) | 15/15 | 13/15 | 0.99449× |
| [Scalar and non-owning field definitions focus reverse](fields/direct-fields-and-scalars-rebound/focus-screen.json) | 15/15 | 12/15 | 1.00143× |
| [Scalar and non-owning field definitions full forward](fields/direct-fields-and-scalars-rebound/full-screen.json) | 81/81 | 62/81 | 0.99726× |
| [Scalar and non-owning field definitions full reverse](fields/direct-fields-and-scalars-rebound/full-screen.json) | 81/81 | 62/81 | 0.99596× |

The complete v3 orders produced **Retain** for scalar and non-owning field definitions. [Full qualification record](fields/direct-fields-and-scalars-rebound/full-screen.json)

[Qualification policy](../../../qualification-policy-v3.json)

## Replay

Use the independent compiler patch for the appropriate session. Apply it
in an isolated checkout at `6fce1ff9ab9caa38805116cbeb0565f135721e6b`.
Rebuild from source with that patch. The replay scripts verify the
encoder object, canonical bytecode, and metadata by digest. Each recipe
contains the exact scalar or field selection used in its measurement.

~~~sh
# Set study to this evidence directory and run from the isolated Git checkout.
gzip -dc "$study/scalar/compiler.patch.gz" | git apply
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-definition-control --variant frame-copies --compiler-opt-level 3 \
  --direct-source --preselect-root-inlining --skip-refused-general-ssa \
  --target-dir /tmp/mech-definition-target
python3 -B "$study/scalar/direct-fast-original-windows/replay.py" \
  /tmp/mech-definition-control /tmp/mech-direct-definition
~~~
