# Contiguous source access in the syntax compiler

One-piece snapshots supply the current retained slice for byte reads, chunk
lookups, and grapheme scanning. The scanner consumes the same logical work
allowances. Metered storage lookups and multipart snapshots use the existing
retained traversal. A test cross-checks every byte offset and UTF-8 boundary
on an edited slice against the metered path.

Standalone parser time moved from **2.04447 to 1.97762 s**,
a **3.35% paired median reduction** across eight orders. The probes
produce the same tree, 718,353 events, and 3,967,302 parser steps.
[Raw parser trials](probe-timings/report.json)

| Apple M1 | Source through final executable link | Encoder text |
| --- | ---: | ---: |
| Mech pooled-parser control | 4.03290 s | 71,768 B |
| Mech contiguous source | 3.93889 s | 71,768 B |
| Roc | 3.04136 s | 57,696 B |
| libdeflate | 0.48541 s | 37,080 B |

The paired complete build improved by **2.23%**, with
8 of eight orders faster. It uses prebuilt compilers, fixed runtimes,
and adapters. Median parse time inside that build moved from
2.08591 to
2.00690 s. Compiler peak RSS was
401,522,688 B. Reconstructing the compiler
reused dependencies and took 241.55 s with four Cargo jobs.
[Complete stages, sizes, and construction](report.json) · [Raw matched trials](matched/timing.json.gz)

The compiler reproduces the bytecode, encoder object, complete metadata,
executable, and benchmark worker byte for byte. The exact worker passes
162 linked checks and has **81/81 Roc and 61/81 libdeflate** wins in both
fully bound orders, with every individual win preserved. Seven-stored-prefix
duplicate-control repeatability still exceeds 0.5%.
[Full runtime replay](../bound-71768-runtime/README.md)

All **534 syntax tests** pass and all **60 syntax snapshots** match, covering
edited source pieces, UTF-8 boundaries, malformed input, diagnostics, and finite
fuel and event allowances. Three small programs reproduce their bytecode.
[Syntax proof](validation/receipt.json) · [Small inputs](small/bytecode-and-timing.json)

Run the pinned source build from the PR root.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-contiguous-source --variant fresh-owner-counts-cold-tails-zero-branches \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining
```
