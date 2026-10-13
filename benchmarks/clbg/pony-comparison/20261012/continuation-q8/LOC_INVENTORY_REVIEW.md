Read-only actual compiler-input role review, 2026-10-12.

This note does not execute the normalizer, supply source-unit counts or admit
canonical LOC. The inventory stage completed with exact8 fixtures; its immutable
compiler-input proposal is source-counts/compiler-input-v1/compiler-input-proposals.json,
whose saved terminal pin is
008340b7e28051a6b042c15de713c6e35462cb8a2f108ed5114ed9342d52fb92.
It binds actual private-v8 report
4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159.
These are values read from retained receipts, not newly calculated here.

All twenty final scored forms retain their original main.pony package and original
compiler/build-receipt source identities. Actual loaded authored packages are:

| Workload | Compact main origin | Optimized main origin | Additional counted authored package |
| --- | --- | --- | --- |
| regexredux | private-v1 root source | private-v1 root source | none |
| revcomp | private-v1 root source | private-v1 root source | none |
| pidigits | private-v3 root source | private-v3 root source | entire support-v2/ponytest, aliases.pony |
| spectralnorm | private-v3 root source | private-v3 root source | none |
| mandelbrot | private-v3 root source | private-v3 root source | none |
| fasta | private-v3 root source | private-v3 root source | none |
| knucleotide | private-v4 precedence-v4 | private-v6 shift-types-v5 | none |
| binarytrees | private-v6 precedence-v4 | private-v6 precedence-v4 | none |
| nbody | private-v6 root source | private-v6 root source | none |
| fannkuchredux | private-v6 root source | private-v6 root source | none |

The Pi helper aliases are authored application glue despite the package label
ponytest; they are counted for each Pi compiled bundle. Generic compiler stdlib
pony_test is a distinct actual loaded vendor path. No authored support is inferred
from a search path: support/search was available but was never a Building record.
The inventory includes every direct authored .pony/.c file in each loaded package;
the actual final bundles contain Pony only. Generic regex and GMP wrappers and
compiler standard packages are separately pinned vendor source, not application
kernel substitutes. No unknown loaded package or unresolved role remains.

The frozen replay stage copies the qualified LOC18 neutral/Pony grammar and pins
Pygments2.19.2 plus Python3.14.8 sources under the common mutex. It reuses the exact8
inventory fixture receipt and does not compile or rerun native cases. Proposed
command for root review/launch (absolute paths may be used):

python3.14 -B benchmarks/clbg/pony-comparison/20261012/revisions/loc-inputs-v1/qualify_loc.py --stage replay --inventory benchmarks/clbg/pony-comparison/20261012/source-counts/compiler-input-v1/compiler-input-proposals.json --inventory-sha256 008340b7e28051a6b042c15de713c6e35462cb8a2f108ed5114ed9342d52fb92 --output benchmarks/clbg/pony-comparison/20261012/source-counts/loc18-replay-v1

The resulting counts, owner events, expansions and all diagnostics require a
complete per-source semantic review before an overlay. This note reviews roles
and replay readiness only. It neither waives diagnostics nor invents main-only
counts. All work inputs and source-count packet remain frozen.
