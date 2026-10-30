# Round 37: qualified layout experiment and executable profile

The helper-layout candidate is qualified but **not a confirmed elapsed-time
improvement**. No further compiler optimization has started. The next decision
uses the [executable scorecard and allocation backtraces](../deep-profile/REPORT.md).

Base `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`; starting published head
`4686620853914517474a23e9d8a6ff1cf11db5cf`; previous confirmed compiler
`9047e634e5964ad23ae19449bc425c2a3cf325b2`; measured candidate
`3a54f1a1d64db9eee01db97763a3f95a138db93a`. Existing ABI/IR/work/codec remain
37/131/95/14. The source diff is confined to the existing invocation helper and
one private sibling module.

Four existing canonical fallback bodies move into non-inlined functions. Common
validation and selected routes stay at their original positions. Body-equivalence
receipts compare both sides after the same Rust formatting; there is no changed
predicate, error path, ledger, owner, descriptor selection or source algorithm.
The earlier action-retirement probe was not enabled; its instrumentation was
removed and its useful negative evidence retained.

| Four-pair complete eight-byte result | Before | Candidate |
| --- | ---: | ---: |
| Pooled median | 4.6680415 ms | 4.6539165 ms |
| Shared measured reference 0.282 ms = 1× | 16.55333865× | 16.50325× |
| Fixed historical reference 0.300 ms = 1× | 15.56013833× | 15.513055× |

The pooled reduction is 0.302589%, with **two improving and two regressing worker
pairs**, and 47/64 improving descriptive blocks. Keep that uncertainty explicit;
blocks share four prepared pairs. All 3,984 observations (3,968 primary) are
preserved, with the original harness, allocator, source, reference bytes,
inflation, aliases, capacity 13, work 44,443,887,150 and final zero cleanup.

The shared helper body shrinks 6,294→4,901 static instructions and its Rust entry
frame 2,416→1,952 bytes. Outlined functions retain their code; the worker's total
text grows 3,148 bytes. Actual retired instructions change only
50,728,073.5→50,663,871 (0.126562%); allocation requests/bytes/peak are unchanged.
Code-layout reduction alone has not removed the main executed work.

Validation before primary timing:

- 311 maintained runtime tests pass, excluding the unchanged reduced-catalog
  knucleotide case; 31 work/memory/value tests pass. Exact commands:
  `gpu-unit-tests native_artifact::program::runtime:: --skip complete_unchanged_knucleotide_body_uses_the_owned_emitted_map --nocapture --test-threads=1`
  and `gpu-unit-tests native_artifact::work:: native_artifact::memory:: native_artifact::values:: --nocapture --test-threads=1`.
- All 12 protected scalar original/decoded controls retain requests/bytes, zero
  allocation slopes and the unchanged 1.05 timing ceiling; maximum ratio
  1.021687395. Eight managed-frame original/decoded controls retain requests,
  bytes, work and zero allocation slopes. No allowance or expectation changed.
- Ten architecture/catalog/factory/warning/unsafe/format checks pass. Exact
  commands/results are in `premeasurement-qualification/receipts.tar.gz`:
  resident routing, source catalog, host catalog, build-analysis layout, Rust
  module layout, runtime factory safety, warnings policy, unsafe boundaries,
  `cargo +nightly-2026-03-03 fmt --all -- --check`, and `git diff --check`.
- Fresh Object, descriptor and metadata emission is byte-identical to round 36;
  source, bytecode and the seven non-GPU library hashes remain pinned. The final
  comparison is in `../deep-profile/generated-object/receipts.tar.gz`.

Profiles use the maintained selected-operation configuration. Release libraries
are opt 3; maintained diagnostic test roots retain opt 1; warnings are errors,
incremental compilation is off and builds are locked/offline with one job. This
layout change did not rerun the 233 component cases or linked/AOT recovery case:
their round-36 receipts remain retained rather than relabeled as fresh. Full
Core/Engine/all-feature suites, unrelated targets, planning-quarantine and CI
inventory checks are not newly claimed. The inherited inventory repair and its
132 passing Python checks remain separately reviewable in `921cd9194`.

No new runtime correctness failure remains from this candidate. Setup/check
dispositions and the unresolved preparation-memory attribution are explicit in
the executable report. The complete source program and all benchmark algorithms,
tables, allocations and timing boundaries remain unchanged. Only eight-byte
complete compression was measured; no larger-input or parity result is claimed.

PR #973 remains draft pending green CI and clean review. Hosted CI is cancelled
by user direction; do not wait for it or describe cancelled coverage as passing.
