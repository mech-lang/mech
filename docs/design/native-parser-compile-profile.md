# Canonical parser compilation profile

The [predictive-dispatch follow-up](native-parser-prediction-profile.md) now
qualifies 2.452–2.494-second parsing and identical program output. This report
records the preceding parser optimization.

On the Apple M3 Air, the level-12 compressor's parser falls from
**10.13–10.29 seconds to 3.47–3.48 seconds**, a **2.94× speedup**.
Fresh source-to-Object compilation falls from **57.89–58.56 seconds to
50.84–51.00 seconds**, a 12.55% reduction in the mean compiler phase total.
The sub-10-second compilation goal remains open.

The [execution-work follow-up](native-parser-work-amplification.md) counts
129 million base/operator continuation phases for this source and identifies
failed alternatives and transactional bookkeeping as the next parser targets.

Source change `47a500d758d1273dbd8ab73f6b61d25ae7bdca7f` preserves the existing
canonical grammar. Terminal dispatch is generated from the same specification,
with references into the original public terminal table. Base and operator
continuations reuse compact frame stacks and scanner storage. Parser journals
store four entries per persistent page and reuse unpublished storage; retained
views keep their original contents through copy-on-write updates.

A bounded 4,096-slot cache reuses completed literal scans against sealed input.
Its keys include the literal, source offset, consume bound and Unicode context
bound. Cached results charge the original scanner work. Fusing the surrounding
terminal phases also reserves their original charges; smaller allowances follow
the existing resumable phases. Unicode boundary decisions continue to come from
the original scanner. Edits discard the owning parser state and its cache.

| Balanced run order | Parse | Frontend | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 10.133 s | 8.971 s | 39.349 s | 58.565 s | 58.781 s |
| Parser 1 | 3.475 s | 8.380 s | 39.066 s | 51.004 s | 52.690 s |
| Parser 2 | 3.472 s | 8.334 s | 38.957 s | 50.842 s | 51.023 s |
| Control 2 | 10.293 s | 8.239 s | 39.253 s | 57.893 s | 58.153 s |

Each run starts a new compiler process with the same 51 flags and source. No
builds, tests or other compiler measurements ran concurrently. The first launch
of the candidate executable adds about 1.4 seconds before its first phase;
whole-process measurements retain that cost. Compiler bootstrap is excluded.
The cohort contains four balanced individual builds.
The control compiler is `88ce9e278c`; the candidate incorporates native changes
through `e310ea239d`, with the subsequent append-pair flag disabled in this
comparison. Isolated parser probes independently confirm the parser improvement.

All four runs reproduce the source, revision-14 bytecode, Object and descriptor
**byte-for-byte**. The Object is
`c04be2f8f15d2cdcfd05f5bf88ffa93a89dc082a73bfa007928ec2da9c1b53d2`.
The final full syntax suite passes **759 tests in 45 suites**, including streaming,
incremental parsing, grammar isolation, Unicode, recovery and resource limits.
A separate **1,667-case differential comparison** against the original parser
preserves complete trees, node/token identities, diagnostics, statistics and
restart entries exactly. Its two serialized outputs have SHA-256
`d5ad4ed9d87c64d3295f31eed97d167f6ba5b41ac1a3852b3e00ff1af8e8b2e7`.
Identical emitted program bytes preserve runtime speed without a new runtime
optimization or compression campaign.

The latest qualified configuration adds `MECH_PAIR_CHECKED_APPENDS=1`. It compiles
in **51.113 seconds**, or **51.226 seconds** for the whole process, and reproduces
the current qualified Object
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`.
The separate sampled run takes 50.777 seconds. Its native phase breaks down as
follows; these adjacent stage intervals are disjoint.

| Native compiler work | Sampled elapsed |
| --- | ---: |
| Canonical lowering | 3.858 s |
| Preparation | 13.797 s |
| Numeric helpers through typed layout | 5.438 s |
| Flow proof | 0.045 s |
| Ownership/operation recipes | 0.717 s |
| Source CFG and SSA construction | 7.886 s |
| Finalize SSA | 0.322 s |
| CFG cleanup | 4.891 s |
| Machine-code emission | 1.931 s |

Parsing now accounts for about 7% of compilation. The remaining large costs are
frontend schema work, native preparation, helper construction and SSA/CFG work.
CPU samples retain dimension-environment canonicalization and boxed dimension
parameter cloning, old-value reachability, and Cranelift SSA work. The latest
clean process reaches **7,760,019,456 bytes of maximum resident memory**. This
makes repeated schema/analysis storage a useful next target; the peak alone does
not identify its allocation sites.

The [fresh peer comparison](native-compressor-size-compile-comparison.md) now
uses the exact pinned Roc compiler on this M3. C spends most measured compiler
time in LLVM optimization/emission. Roc spends most in specialization/LIR passes
and LLVM. Mech spends most before machine-code emission, and emits substantially
more native code than either peer.

The raw archive retains the journal-only improvement, isolated parser profiles,
boxed/inline-stack trials, journal page sizes, cache-size/hash trials, rejected
rule memoization, unsuccessful setup attempts, all test logs and every primary
observation. A four-second Roc sampling attempt outlived the process and had no
usable stacks; a one-second window captured use-order and range-proof paths.
The final Mech sample attempt raced process exit after three usable windows.

The [compact result](../../benchmarks/mech-deflate/native-profile/parser-peer-compile-apple-m3-20261113.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/parser-peer-compile-apple-m3-20261113.archive.json)
pin [all raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-parser-peer-compile-apple-m3-20261113.tar.gz):
4,355 files, SHA-256 `4a4d3ad01f553a97a22d88ad08bf22fc6347b7876cd507e9a99db9ce727cc253`.
Run `python3 replay.py` after verified extraction; no compiler is needed.
