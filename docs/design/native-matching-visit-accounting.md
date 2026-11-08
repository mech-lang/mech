# Native matching visit accounting (R206)

The compiler candidate is **not retained**. It removes real matching-loop work, but complete elapsed is effectively flat and sixteen of twenty whole-call instruction medians rise. Production remains **R199, IR/work/ABI/MBC214/107/39/14**. The new stride/error/recovery regression test and all contrary observations are retained. Small demonstrated wins remain eligible; this is a mixed result, not a rejection based on a minimum percentage.

## Checkpoint and mechanism

Base `71beaa5aafbb83c87a1a7b1112933852e04f7e30`; retained production `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Candidate IR218/work107/ABI39/MBC14 is archived, then reverted. Existing draft #973 remains the only PR.

The existing funded-child proof already protects the current induction, initial carrier, limit and stride from interior writes other than the proven latch. The candidate keeps the original maximum-visit funding/counter-space gate and checked tight-funding fallback. On admission it saves the current induction. At every observable normal/error/child-initialization exit it derives actual completed visits from current minus last-settled induction divided by the proved stride. It settles the original work/comparison prices and counter, then advances the saved induction to prevent double settlement. The first source header is already paid; an uncompleted trailing comparison is not counted as a full chunk. No per-header counter or balance update remains on the admitted loop.

Nine static funding sites select the candidate, all with stride8. These are eligibility counts, not dynamic visits. Shared code/semantic proofs are used; no compressor function names or site-specific authority are added. Source, algorithm, expected output, allocation sites, capacities, prices and limits are unchanged. `candidate-source.patch` and seven changed compiler files preserve the unadopted implementation.

## Complete-call elapsed and independent counters

All **20 unchanged1.05 gates pass**. Ten elapsed medians fall and ten rise; geometric elapsed changes **+0.138561%**. No favorable unchanged rerun replaces these observations. The table separates primary complete-call elapsed from retired instructions in an independent cohort.

| Input | Retained / Roc | Candidate / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.323912× | 3.342472× | +0.558% | +0.698% |
| random-32769 | 4.708848× | 4.743827× | +0.743% | +0.191% |
| 16384-distribution-change | 6.187342× | 6.165139× | -0.359% | +0.083% |
| repeated-32769 | 2.879536× | 2.873670× | -0.204% | -5.408% |
| 33027-periodic-window | 3.392393× | 3.390149× | -0.066% | -5.013% |
| eight-literals | 4.428090× | 4.407688× | -0.461% | +0.047% |
| reymont-prefix-16384 | 4.044180× | 4.028059× | -0.399% | +0.558% |
| empty | — | — | -0.374% | +0.022% |
| one-stored | — | — | -0.242% | +0.010% |
| seven-stored-prefix | — | — | +0.035% | +0.015% |
| eight-overlap | 4.485317× | 4.460646× | -0.550% | +0.049% |
| 320-overlap | 4.071851× | 4.055350× | -0.405% | -0.110% |
| eight-literals-prefix | 4.399625× | 4.417885× | +0.415% | +0.052% |
| 128-biased-dynamic | 3.784309× | 3.773017× | -0.298% | +0.048% |
| 32-high-entropy-stored | 4.035953× | 4.096427× | +1.498% | +0.052% |
| random-64 | 4.107341× | 4.146121× | +0.944% | +0.050% |
| random-256 | 4.133207× | 4.181327× | +1.164% | +0.043% |
| random-1024 | 3.784314× | 3.801948× | +0.466% | +0.046% |
| repeated-1024 | 4.026170× | 4.029384× | +0.080% | -0.687% |
| reymont-prefix-1024 | 4.651923× | 4.663974× | +0.259% | +0.108% |

Primary has3,968 observations /4,048 complete calls in eight balanced blocks per case, with fresh pinned reference elapsed for each case. Empty, one and seven-byte reference elapsed rounds to zero in the existing clock harness, so no elapsed multiple is computed. Necessary compression allocation/copying stays inside `evaluate_into`; preparation/clear/prefix/import/rebind, output slots, validation, independent inflate and drop stay outside identically. The pinned mimalloc2.1.7 is unchanged. There are1,440 separate Mech CPU/heap calls; these are not pooled with primary elapsed.

Four instruction medians fall and sixteen rise; fourteen cycle medians fall and six rise. Repeated instructions **66,841,269.5→63,226,786.5 (−5.407562%)**; periodic **72,488,199→68,854,226.5 (−5.013192%)**. Natural32 rises **208,523,059.5→209,979,448 (+0.698430%)**. Periodic cycle median rises despite its instruction reduction and tiny primary elapsed improvement. The adverse observations are retained; sampled time, cycles and instructions are distinct quantities.

## Fresh reference instructions and memory

This round closes the missing same-input reference-counter measurement. The exact fourteen-file package is recovered from historical Git objects at6ab75037bc5d4ec138d735c7dcfe1c4b653d31fa; all fourteen source hashes match the pinned reference package671077457ff7ef3a3d44caf715a3fc595addf379. Compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d and allocator object remain pinned. The recovered common Bench harness is byte-identical. Only the separate CPU/heap probe wrappers are added; primary elapsed continues to use the original pinned non-diagnostic executables.

Forty builds and forty cohorts complete:32 counter observations per case/mode, **1,280 observations and10,240 verified complete compressions**, including unrecorded repetitions. CPU and heap runs are separate. Each performs the same256 level12 repetitions with input/prefix/preparation outside its clock interval and complete compression inside; exact output and independent inflate remain checked outside. Full commands, inputs/expected/source/compiler/allocator/binary hashes and raw observations are in `reference-counters/build-index.json`, `reference-counters/{cpu,memory}/`, `reference-restore/` and `fixture-payloads.json`.

| Input | Retained instructions / Roc | Candidate instructions / Roc | Reference instructions |
|---|---:|---:|---:|
| reymont-prefix-32769 | 4.382859× | 4.413470× | 47,576,947.0 |
| random-32769 | 6.042113× | 6.053665× | 11,752,781.0 |
| 16384-distribution-change | 6.904665× | 6.910404× | 9,368,403.0 |
| repeated-32769 | 3.495571× | 3.306546× | 19,121,704.0 |
| 33027-periodic-window | 3.762729× | 3.574097× | 19,264,792.0 |
| eight-literals | 9.111289× | 9.115554× | 1,364,372.0 |
| reymont-prefix-16384 | 4.789038× | 4.815746× | 24,819,040.0 |

Counts are unadjusted whole windows, not a per-function ledger. Separate101-empty-window calibrations observe reference5,707 instructions/1,479 cycles and Mech5,659/1,492. Calibration is outside primary timing and collected before sampled compression starts; nothing is subtracted. CPU probe heap fields are uninstrumented zeros, not evidence of zero allocation. The separate heap cohort supplies allocation metrics.

All20 Mech allocation-request, requested-byte, reallocation and incremental usable-peak medians are exactly equal before/candidate. Natural32 remains **6,449 requests /24,793,116 requested B /18,722,528 incremental usable-peak B**, versus reference **276 /9,115,076 /9,257,328**. Eight literals remain **3,354 /17,643,969 /15,279,752**, versus reference **95 /9,021,923 /9,257,328**. Ninety-five is an observed result, not a semantic minimum. Requested bytes are not initialized-copy volume; usable heap is not process RSS.

The262144-entry U16 tree,1500513-word U32 cache,305257-entry U64 path capacity, depth300/nice258, original EOF rules, source allocation sites and eight-byte output capacity13 remain unchanged. Capacity protocol and live aliases pass; physical output backing reuse is still not established.

## Actual executable and current profiles

Generated text **8,152,436→8,150,284 B** (−2,152); reported stack **16,480→16,352 B** (−128). The actual admitted eight-byte equal edge shrinks **15→11 machine instructions**, preserving both64-bit loads and no hot stack reload. Actual Object window0x6940c..0x6942c and equal backedge0x6947c..0x69480 retain the range comparison, loads, XOR/comparison and induction advance; accounting is on exits. The existing outlined phases remain unchanged. Assembly, Object/worker hashes and source eligibility metadata are archived.

Four fresh stack profiles execute **6,758 correct complete calls**. Disjoint native-exclusive weights are natural78.61%, repeated71.13%, periodic64.20%, eight literals9.51%; host below native is18.64%,24.35%,31.39%,78.27%. These are timer weights with children subtracted, not retired-instruction fractions or removable time.

Fresh exact-PC profiles execute **6,814 complete calls**. Natural has5,355 running timer samples,2,313 in phase15296 and1,853 in root entry; the hottest phase offsets remain3,288 and3,264. Repeated/periodic still sample the real matching loop heavily. No historical address map is applied to changed code. This again identifies the natural path-selection phase as a distinct cost; removing loop accounting does not solve it. A fresh reference executable disassembly is pinned with38 static candidate word-mismatch contexts; those are not dynamic visits or new proven function attribution.

## Qualification, failures and remaining coverage

- **677 candidate native tests**, all **260 actual CI selections executed**; five focused funding/stride/handoff controls also pass. The restored production source freshly passes the same677/260.
- **1,998 resource-policy observations /4,002 native invocations per side**, with stride8, stride12 and fallible8, original/decoded, normal/restored entry. Exact successful reports, errors, prior publication/backing, aliases, cleanup and successful retry agree across actual compiler revisions. No cross-configuration work-equality claim is made.
- The retained new test exercises strides8/12/16/24, three starts and ten mismatch positions, original/decoded and normal/restored native execution, checked/funded versions, independent resident results, finite/zero/unlimited policies and recovery.
- **1,724 real matching controls** pass. There are1,566 successful resident value agreements,142 resident default-cap denials and16 ordered source-error controls. The existing16,777,216 resident cap is not increased. Independent expected native results, aliases, failure publication and retry pass for every control.
- Unchanged **236 components /eight compositions /one real-capacity path-phase mechanism**, plus40 complete smoke calls, pass. Expected values and allowances do not change.
- **15 architecture/catalog/warning/format contracts and132 workflow/inventory tests** pass; final restored format/module/native-layout/diff checks pass. GPU/library opt3, test root opt1, nightly2026-03-03, Cranelift0.131.3 dense frontend, aot/jit/canonical-native/program/runtime features and `-D warnings` remain. Frozen dependency pins and all commands are archived.

Development failures are preserved and classified in `failure-disposition.json`: initial zero-test inventory required a fresh test-root build; the new three-repetition resident witness exhausted an inherited default cap and was bounded to one repetition without altering existing tests; reference-cache denial needed ordinary cache access; a missing local Bench was recovered byte-identically; real-match fixtures had to be staged before their standalone consumer. Clean R199 reproduces the resident-cap family (not an identical required-work number). All final receipts succeed. The candidate has no unresolved correctness failure on the exercised controls; its performance tradeoff remains the reason for reversion.

The same seven inherited native exclusions remain:

- `batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch`
- `batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries`
- `batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity`
- `batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing`
- `batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable`
- `batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch`
- `native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`

Ordinary exact-head hosted CI is cancelled under the standing instruction, while review automation stays enabled; #973 stays draft. A full dependency/all-features rebuild, multi-MB runs and the separate ten-workload suite remain skipped. Numerical replay runs archived-observation scripts, not a compiler or compressor.

## Remaining brief and next bounded step

Both requested production experiments now have complete measured outcomes: R205 direct reduction return and R206 induction-derived settlement are correct on their controls but unadopted. R203/R204 already produce complete raw compact streams, yet they discard retained packed/numeric specializations and remain slower on large inputs. They are not a cost-of-contract floor. Exact resource, precise error/recovery, current ownership/publication and external import/admission restoration remain unfinished. The current production exclusive host-family ledger and initialized-copy census also remain open.

The next bounded target returns to the primary milestone: preserve the existing qualified compact numeric phase and packed reads through whole compact emission, then repeat complete small-stream qualification and all20 measurements before restoring contracts. Production profiles justify targeting the path-selection phase; measured R193 kernel savings cannot be substituted for a whole-call result. The zero-displacement settlement idea needs dynamic eligible/executed counts before another candidate, and the old universal XOR comparison rewrite remains a rejected experiment rather than an unexplored target.

Parity and the complete brief remain unfinished. Production sources outside test-only additions match R199 byte-for-byte. Fresh matched retained observations above describe the current compiler; rejected candidate figures are not presented as a new production speedup.

## Immutable review packet

[Download exact compiler patch, all20 observations, fresh reference counters, commands and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r206-matching-visit-accounting-7fe9d934a38c.tar.gz) (3,487,108 bytes;959 files). SHA-256 `7fe9d934a38cb77984c7154afdd1de2616e1b7fd1d9805cb1f153de80ced44b5`. Server digests, every file and a fresh release download verify; `heap-summary.py` and `analyze.py` reproduce archived outputs byte-for-byte. The direct unauthenticated URL first returned a gateway504; release-API download succeeds, with both receipts retained outside the sealed packet. No native binaries, bytecode, executable descriptors or raw trace stores are uploaded.
