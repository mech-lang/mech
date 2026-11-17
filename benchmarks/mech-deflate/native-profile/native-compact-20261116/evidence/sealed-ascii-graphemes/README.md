# Sealed ASCII grapheme scanning

Compiler source `c93ed0e1a726ff7f64164533593349f04fdfa12c` recognizes a bounded ASCII grapheme directly
when sealed input is contiguous and the next scalar is ASCII or reaches the
end. It handles CRLF, advances the Unicode cursor to the same boundary and
charges the same calls and bytes. Segmented input, Unicode lookahead and
insufficient grants use the existing resumable scanner.

| Standalone parser arm | Median |
| --- | ---: |
| Control | 918.1 ms |
| Direct scan for fixed literal spelling only | 876.1 ms |
| Direct scan for fixed literals and sealed source | 812.2 ms |

All six balanced orders produce the same lossless tree, 718,353 events and
3,967,302 charged parser steps. The syntax suite passes 536 tests. The focused
check compares the scanner to the Unicode cursor across subranges, Unicode
boundaries and incremental grants.
[Exact parser outputs and ablation](standalone/comparison.json) ·
[Syntax tests](standalone/candidate/tests.json)

| Measurement | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.59466 s | 2.97795 s | 0.48074 s |
| Source through strip | 2.60531 s | 2.98852 s | 0.49123 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable used in timing | 292,568 B | 191,912 B | 155,752 B |

Eight balanced fresh-process orders on Apple M1 save a paired median of
**98.5 ms** through the final link.
Mech's bytecode, object, metadata, stripped executable and qualification
worker reproduce exactly. The rebound worker passes 162 checks and matches
the worker that recorded 81/81 Roc and 61/81 libdeflate in both full orders.
[Compiler receipt](compiler-build.json) · [Matched plan](matched-plan.json) ·
[All matched observations](matched-observations.json.gz) ·
[Runtime qualification](../frame-copy-v2/README.md)

The separate phase run reports **812.7 ms Mech**
and **3.0 ms Roc**. Mech includes lossless parsing,
indexing and the strict check. Roc reports its own summed parser interval
with a single job and its compiler cache disabled.
[Parser phase plan](parser-phase-plan.json) · [Phase observations](parser-phase-observations.json.gz)

Use the frame-copy command in [CURRENT.md](../../CURRENT.md) to build the
maintained compiler and regenerate the exact encoder. The plans pin each
compiler, allocator and fixed deployment input. For another host, substitute
local paths for inputs with matching hashes and use fresh output directories.

The four post-change stack captures put 21.7% of parser samples in
base-rule continuation and 8.3% in structural replay.
[Post-change samples](residual/summary.json)
