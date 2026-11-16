# R251: zero-trip state and enclosing recurrence diagnosis

**Retain R250. No functional optimization or new speedup is claimed in R251.** The new brief is now grounded in the exact current image: paired source witnesses, source/CLIF/machine attribution, exact zero-route counts on all 81 configurations, and current enclosing-region proof refusals. Identity-result forwarding, empty-reduction forwarding, multi-position cold handoff and the R247 placement control remain unimplemented.

Git parent `4783e9c4d8855119b6fd24e31a917c6cf3eab33a`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. The diagnostic patch changes three files in the archived 815-file R250 GPU closure, plus its external producer. It does not apply directly to the maintained tree. Ordinary R215 stays `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC 215/107/39/14. The benchmark source and bytecode, allocator, arrays, algorithm and timing boundary stay pinned.

## Paired current lowering

The conditional and always-entered fixtures are semantically equivalent. Their whole-root sizes differ: public-record always/conditional are 2,768/3,020 bytes (416/400-byte frames); private-record always/conditional are 3,300/3,580 bytes. The private forms select the supported leaf phase at headers 4/9; the escaping record forms do not select a numeric phase. These totals include both positive and zero paths and are not executed instruction counts or performance evidence. The separate CLIF and machine excerpts identify surviving zero-route work.

## What actually survives the zero-trip route

The source unconditionally enters optimal-skip-run after each searched position. Its current header is 2398, with a 27-word carrier: two writable views, the read-only input view and nine scalar fields. Its returned position record contains the two writable views and nine scalar fields. Header 837 is the enclosing block collector; header 2624 is the matching skip subtree. These identities are recovered from current canonical Tail ancestry and word recipes, not inferred from the historical header alone.

Most packs, projections and owner-token transfers on the zero arm are already virtual. There is no claim of a record allocation or a native helper call for every zero entry. However, actual current ARM64 code at Object addresses 0x63ad4–0x63b2c contains a relay of **22 instructions: 11 stack loads, 10 stack stores and one branch**. Repeated skip-condition comparisons also survive. The packet includes metadata/scalars, not copies of input payload. The actual disassembly and selected source ranges accompany the packet.

The enclosing root continuation also contains a separate 97-instruction corridor at 0x533458–0x5335dc: **46 loads, 50 stores and one branch**. This is a concrete state-transport target. It is not proved wholly removable, nor charged to the skip identity transformation. A forwarding experiment must remove the relevant continuation work as a unit and check that it has not moved into other spills.

## Exact route counts, including the near-EOF distinction

Two observed complete calls for each of all 81 configurations give identical route counts, exact reference output, independent inflate, unchanged input/borrowed-output aliases, unchanged capacity and final zero private ownership. The source observer is restricted to eight explicitly requested sites; it is absent from timing and hardware measurements. Its 162 calls are correctness/eligibility evidence, not performance samples.

| 1 MiB family | Zero-skip helper entries | Nonzero helper visits | Qualified matching skips | Nonzero visits outside that readable-window scope |
|---|---:|---:|---:|---:|
| natural | 1,048,572 | 4 | 0 | 4 |
| random | 1,048,576 | 0 | 0 | 0 |
| periodic | 4,106 | 1,044,470 | 1,044,466 | 4 |
| repeated | 4,067 | 1,044,509 | 1,044,505 | 4 |

Every helper-header count equals the independently recorded cache-header count from R248, on all 81 configurations. The earlier matching census counts the readable matching operations, whereas this observer counts helper visits even at near EOF. In particular, natural text has four nonzero helper visits despite zero qualified tree skips. Keep the original near-EOF behavior and both counter scopes; do not rewrite either count to make them appear identical.

## Mapping the measured current executable

The source-location annotations and IR/disassembly dumps reproduce the **entire R250 Object and metadata byte-for-byte**. With diagnostics disabled, the final compiler repeats the same complete-image equality. Object SHA-256 is `12127a7982dd982db282d993895035656da96eace5f151426436cbd5ac2c290f`. Therefore the existing exact R250 worker profiles can be attributed without measuring a changed executable. No new hardware cohort is presented as a speedup.

| Current profile | All running samples | Generated root | Root samples with default/unavailable source location | Samples in the 97-instruction latch corridor |
|---|---:|---:|---:|---:|
| 33027-periodic-window | 5583 | 2350 | 1337 | 6 |
| eight-literals | 5524 | 1658 | 832 | 1 |
| heldout-random-65537 | 5470 | 3333 | 2361 | 455 |
| reymont-prefix-32769 | 5353 | 1160 | 713 | 125 |

Default source locations frequently cover register-allocation moves and gaps. Do not assign all of that weight to one source operation. The random64 latch corridor has 455/5,470 running samples, but samples include validation/harness and are neither retired-instruction counts nor removable-time budgets. The published aggregate PC/source ranges and narrow assembly excerpts support the attribution without publishing raw stack traces or the full executable descriptor.

## Current proof boundary, rather than a guessed missing loop class

| Current region | Complete proof outcome | Selected descriptive proof cold exits | Compact native phase selected |
|---|---|---:|---|
| 837 | numeric phase contains a non-Tail child | 52 | no |
| 2398 | numeric phase contains a non-Tail child | 25 | no |
| 1279 | complete | 0 | yes |
| 2624 | complete | 0 | yes |

The exact full-program complete-proof refusal is `numeric phase contains a non-Tail child`. The selected partial proofs retain exact cold exports; the compact phase selector requires no exits and excludes checked children. Collections such as the real window slide remain an actual boundary. Supporting several positions must preserve current state at these exports and hand back before unsupported work, as requested by the brief. No selection or refusal rule is relaxed here.

A minimal fixed-buffer outer recurrence with a writable child and header store has correct execution but exposes a second distinction. The child itself has a complete numeric proof. A decreasing-count child makes the enclosing complete proof decline with `numeric child has no proved bounded recurrence`. Replacing only that diagnostic child counter with an increasing checked counter reaches the exact next guard: `mutable children require current-account visit funding`. Its selected descriptive proof has two ChildInitialize/ChildHeader handoffs; the compact native phase list is empty. This is not evidence that generic nested computation is wholly absent, and not permission to remove a funding check. It is a small acceptance witness for the relevant parent/child boundary. The source algorithm in the compressor was not changed.

`check-nested-native-selection.py` intentionally fails on this current inherited selection boundary, with the exact reason. This remains an open optimization acceptance test, not a semantic regression or an unresolved broken compressor run. Any follow-up must identify the matching current compiler operation and preserve ordinary finite funding, or use an independently proved private boundary without weakening it.

## Qualification and failure disposition

- 3,500 fresh semantic calls: 250 independent cases across seven execution variants, each original/decoded, including 700 zero-trip visits. Six source programs cover always-entered versus conditional entry, top-level versus private record representation, nested mutable children and the counted variant; one source is also run with a phase observer. Values, rectangular indexing, wrapping counters, bound failures, input aliases, actual prior published backing, failure output sentinels, cleanup and retry agree across all variants. These are repeated controls across representations, not 3,500 distinct inputs.
- All 81 complete configurations have two observed route calls (162 total), with independent output/inflate/alias/capacity/cleanup checks and reconciliation to the existing work ledger.
- Normal/source-located full Objects and metadata restore R250 exactly. Small original/decoded images agree. Final GPU/producer builds keep warnings-as-errors, opt3 and the same six native features. The 703 maintained tests and seven exclusions are inherited from the freshly tested R250 change, not relabeled as fresh R251 execution. No broad suite or full CI rerun is claimed.
- Introduced diagnostic setup failures are preserved: a missing Debug formatter; then a diagnostic closure edit matched the older metadata method and was rejected by warnings-as-errors. The latter shell continued using the old count-only producer; those outputs are explicitly labeled stale, the owned stale run was stopped, the old ordinary metadata method restored exactly, and the replacement orchestration checks every build/command result. None overlaps primary measurements.
- The first source-location dump hit the environmental disk limit. The corrected run streams its large pre-cleanup text directly into compression; complete image equality then passes. Verified redundant replay extractions were removed and old executables/builds compressed losslessly with hashes. Source and measured results remain intact.
- The first new test harness failed because it dropped borrowed input buffers while retaining returned borrowed views; it now retains both buffers with the prior publication. A generated private-wrapper fixture initially inserted an output into the wrong function and was rejected by the existing semantic checker; the corrected source passes. An initial observed nested test assumed selection before proving eligibility; the assertion was separated from semantic correctness. An unaligned disassembly request was discarded and repeated at aligned addresses. These are diagnostic/test setup corrections, not parent semantic defects.
- The current enclosing-native acceptance failure is inherited and deliberately exposed, as above. No new production semantic failure remains. The ordinary extra exact admission/accounting/resource/canonical-publication/recovery contract remains omitted by this standalone compact entry, unchanged.

## Latest retained complete-compression timings

Timings are R250’s matched two-cohort results; R251 emits the same normal executable and adds no new timing claim. Ratios use pooled observation medians. Balanced changes are measured separately against the remeasured R249 parent. All 81 existing point/block 1.05 gates pass in both R250 cohorts; all 25 adverse rows remain in the retained report.

| Input | Mech / Roc |
|---|---:|
| Eight literal bytes | 0.502581x |
| Random 64 KiB + 1 | 2.940361x |
| Natural 1 MiB | 2.244741x |
| Random 1 MiB | 3.217791x |
| Periodic 1 MiB | 1.774171x |
| Repeated 1 MiB | 1.659128x |

## Next bounded step

First implement a generic identity-exit/continuation proof against the demonstrated zero path, keeping the current condition/argument evaluation, word mapping, logical aliases, token balance, error ordering and only the selected branch. Compare actual emitted relay/continuation code; do not count virtual records as savings or relocate the same traffic. If the experiment lacks meaningful eligibility or fails confirmation, preserve it and move on.

Next inspect the empty outer OptimalPathMatches reduction using the same attribution machinery, rather than repeating a broad census. Separately use the current 2398/837 refusal and the fixed-view witnesses to plan a multi-position region with exact cold handoff. The R247 footprint/placement control is still outstanding and must remain a separate measured experiment. No throughput gain from these unimplemented transformations is predicted. All 81 cases, the eight-byte ceiling, fixed source allocation sites and real scratch capacities remain acceptance controls.

## Verified public evidence

[Evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r251-zero-trip-state-ca9af7a2eee1.tar.gz), 2,492,862 bytes, SHA-256 `ca9af7a2eee136d0a92f7b1f6ff776cca2887e456d3cb09d7fc031aa8e34d3e4`. The public download verifies all 1,212 file hashes and five byte-identical offline diagnostic reports. The three-file patch applies to R250 and reconstructs all 815 tested GPU source files. Native binaries, bytecode, complete executable descriptors/IR/source maps and raw traces are excluded.
