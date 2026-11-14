# Sparse SSA definition lookup

Fresh level-12 source-to-Object compilation falls from **46.861–47.233 seconds
to 41.859–41.883 seconds** on the Apple M3 Air, an **11.00% mean reduction**.
Source CFG construction falls from **7.776–7.834 seconds to 2.462–2.543 seconds**,
or **67.94%**. Whole-compiler retired instructions fall **8.68%**. Source commit
`7c30083b39b25e7e3945bdbb532653370fb9765c` changes private SSA bookkeeping;
the source language, native instructions and complete metadata stay unchanged.

The SSA builder already admits dense storage for sufficiently occupied prefixes
near block zero. Variables used in later regions often keep their definitions
in sparse storage. The focused source-generation sample shows substantial work
in tree lookups, insertions and movement of those entries. Sparse storage now
uses Cranelift's existing `FxHashMap`. Dense-prefix admission remains unchanged,
and sparse or extreme block IDs still avoid storage proportional to a complete
block/variable product. No dependency or feature selection changes.

Definition enumeration retains ascending block order, including explicit
None-valued entries. This preserves the observable sequence used by stack-map
propagation. Sorting is confined to enumeration; point lookups and insertions
need no ordering. SSA predecessor traversal, cycle handling, placeholder values,
block parameters and instruction creation retain their existing algorithms.

| Balanced run order | Parse | Frontend | Native | Source CFG within native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.334 s | 8.489 s | 35.963 s | 7.776 s | 46.861 s | 47.011 s |
| Candidate 1 | 2.463 s | 8.453 s | 30.862 s | 2.462 s | 41.859 s | 43.045 s |
| Candidate 2 | 2.437 s | 8.373 s | 30.996 s | 2.543 s | 41.883 s | 42.190 s |
| Control 2 | 2.449 s | 8.290 s | 36.412 s | 7.834 s | 47.233 s | 47.452 s |

Both pairs improve. These are four serial ABBA observations with the same
52 flags, toolchain and source. No builds, tests, counters or CPU sampling
overlap primary timing. Bootstrap takes about 79 seconds and is excluded.
Source CFG uses independently stamped stage-message boundaries and is included
in native time. The first candidate launch has about 0.98 seconds of additional
process startup outside compiler phase timers. Four observations do not establish
a statistical population or attribute unrelated phase variation to this change.

Cleanup is an adverse phase observation: **4.828–4.930 seconds** in control
versus **4.984–5.009 seconds** in the candidate, a **2.41% higher mean**.
Its cause is not isolated. Preparation remains around 11.3 seconds. Candidate
maximum RSS is **7,593,771,008–7,595,196,416 bytes**, compared with
**7,600,635,904–7,737,819,136 bytes** in control. The cause of the process-peak
difference is not isolated; no causal whole-compiler memory saving is claimed.

All four source, canonical bytecode, native Object and complete metadata outputs
are byte-identical. The Object retains SHA-256
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata retains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Runtime source and generated instructions are unchanged. No new compression
runtime timing campaign is claimed for this compiler-only storage change.

All **61 tests across five focused suites** pass, including **14 SSA tests**.
The storage differential checks 50,000 mixed insertions against the ordered-map
reference, preserving lookup results, replacement results, occupancy bounds,
keys and ordered values. It includes sparse/extreme block IDs and explicit
None-valued definitions. Existing tests retain undefined entries, predecessor
cycles, joins, queried-block caching and generation wrap. The final explanatory
comment correction was rebuilt to a byte-identical compiler; both build/patch
receipts remain in the packet.

Three separate CPU windows from the retained control cover numeric generation,
source CFG construction and cleanup. Their elapsed is diagnostic only. Numeric
generation also reveals repeated fresh definition inventories, which remain an
optimization candidate after this change.

The [compact result](../../benchmarks/mech-deflate/native-profile/ssa-sparse-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/ssa-sparse-compile-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-ssa-sparse-compile-apple-m3-20261113.tar.gz):
102 files, 5,858,020 compressed bytes, SHA-256
`06b7de6c030cb24d40ebac29a7f2dc6727661bf98dd2f4073be6b0397e4bbf5d`.
Independent public retrieval verifies every manifest entry and reproduces the
compact result byte-for-byte with `python3 replay.py`. Compiler/test executables
stay local; source, Object and metadata evidence remain retrievable.

The [preceding ownership change](native-owner-read-compile-profile.md) reduced
preparation. Native compilation now takes about 31 seconds, frontend work about
8.4 seconds and parsing about 2.5 seconds. The sub-10-second goal remains open.
