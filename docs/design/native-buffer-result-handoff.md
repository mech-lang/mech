# Transfer held ownership through successful buffer results (R221)

**The experimental compiler now hands one dead source reference to a successful same-owner buffer result. All20 unchanged elapsed gates pass; balanced block means improve6.899%. Instructions fall on19 cases and cycles on17. Normal compilation remains R215/IR215.**

## Revisions and scope

Base `9b1f27fdac88de32ea703cab4a876788cd0c84ac`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Source base is retained R220, plus two archived compiler files. Existing draft#973/branch only; no LLVM, runtime, allocator or compressor source change. Same nightly2026-03-03, Cranelift0.131.3 and mimalloc2.1.7.

Normal IR/work/ABI/MBC215/107/39/14 remains. Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Algorithm/tables/depth300/nice258/near-EOF rules/source allocation sites/real262144-U16 tree,1500513-U32 cache,305257-U64 path capacity and eight-byte output capacity13 remain. Real input/prepared-output aliases stay live.

**These are whole-native DIAGNOSTIC results. Ordinary exact admission/accounting/canonical publication/recovery is still omitted. No normal-entry speedup or parity is claimed.**

## Why this operation

R220 removed only13 nonnull copied references per natural32 compression. A separate reached-site census now finds357738 same-owned successful buffer results on natural32 and440192 on random32. The existing emitter retained each result and separately retired its consumed source even when both owned the same backing. This repeats actual memory-dependent reference updates in generated code.

The pinned reference U32 list-set shim at0x10003060c calls makeUnique, stores the word, then directly returns its owned three-word view. No additional retain/decrement pair appears in that shim; makeUnique keeps its actual uniqueness check. This is a static reference pattern, not a dynamic reference-call ledger.

## Ownership rule and checked order

`OwnerFlow::can_finish_source` reuses existing owner-field liveness and current-predicate last-use facts. It requires a held source token, excludes current definitions and output-port uses, and proves no reached continuation needs that reference. Only array-source/array-result operations qualify; constructors with scalar sources retain their original handling.562 SOURCE sites qualify, separately from reached events.

After the ORIGINAL buffer operation has completed all checks, result stores and fallback handling, compare the result owner with the saved source owner. If equal, omit acquiring a new reference and zero the old source home; the existing token belongs to the result. If different, retain the fresh result and leave the source to its original cleanup. Old destination cleanup remains ordered. Later retirement sees zero for a moved token.

Moving one reference is valid with shared backing. Mutation still uses the original current refcount/capacity gate. Genuine aliases, fresh/growth fallback, unselected paths, previous publication and failure recovery remain. No new alias analysis, source-name specialization, heap scan or cross-call permission reuse is introduced.

Actual before/after minimal Objects show the mechanism: the old indexed-store path increments the result owner and decrements the old source. The new path compares source/result and selects zero for both reference-update operands on the equal edge; both memory updates are bypassed. Zero is materialized in an ordinary register, so the initial xzr-only search missed this valid pattern. Actual excerpts are retained. Two owner comparisons still survive in that small Object; the change is not presented as their elimination.

| Input | Before same-owned results | Actual after nonnull handoffs | Shared same-owned before |
|---|---:|---:|---:|
| reymont-prefix-32769 | 357738 | 357738 | 527 |
| 33027-periodic-window | 213483 | 213483 | 0 |
| repeated-32769 | 210820 | 210820 | 0 |
| random-32769 | 440192 | 440192 | 782 |
| 16384-distribution-change | 264537 | 264537 | 815 |
| eight-literals | 12495 | 12495 | 0 |

Before and after observer images execute80 complete calls in total. Counts are per one compression; two calls agree per case. Actual handoffs reconcile exactly with reached permitted same-owned results. All observer allocation/copy/zero/peak counts match their primary variants; neither observer image supplies primary timings.

## All20 matched elapsed results

NormalR215, retained experimentalR220 and R221 run sequentially in24 balanced order blocks.512 calls per block for clock-limited stored cases,64 for other inputs<=1024B,one for larger inputs. The batching stays outside unchanged per-call timers; original1.05 gates remain.161832 complete Mech calls/161712 retained, plus5120 fresh reference compressions. Every call checks exact raw bytes and independent inflate.

| Input | Normal R215 / Roc | Retained R220 / Roc | R221 / Roc | Block-mean change vs R220 |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.434753× | 2.744180× | 2.648136× | -3.500% |
| 33027-periodic-window | 3.652188× | 2.618478× | 2.412664× | -7.860% |
| repeated-32769 | 3.123293× | 2.423684× | 2.234148× | -7.820% |
| random-32769 | 5.087730× | 4.461423× | 3.976770× | -10.863% |
| 16384-distribution-change | 6.573909× | 4.119779× | 3.725016× | -9.582% |
| eight-literals | 4.439333× | 0.805750× | 0.753081× | -5.928% |
| reymont-prefix-16384 | 4.200982× | 2.758366× | 2.644105× | -4.142% |
| empty | unresolved | unresolved | unresolved | -2.522% |
| one-stored | unresolved | unresolved | unresolved | -8.040% |
| seven-stored-prefix | unresolved | unresolved | unresolved | -5.947% |
| eight-overlap | 4.172852× | 0.770051× | 0.718717× | -7.039% |
| 320-overlap | 4.118551× | 0.886974× | 0.822847× | -8.341% |
| eight-literals-prefix | 4.285878× | 0.776553× | 0.731993× | -4.928% |
| 128-biased-dynamic | 3.631707× | 0.722603× | 0.684433× | -5.560% |
| 32-high-entropy-stored | 3.990019× | 0.832611× | 0.776981× | -6.216% |
| random-64 | 4.062644× | 0.886317× | 0.830953× | -7.053% |
| random-256 | 4.093959× | 1.059966× | 0.969172× | -8.373% |
| random-1024 | 3.814171× | 1.152731× | 1.041737× | -9.313% |
| repeated-1024 | 3.967515× | 1.024462× | 0.951336× | -7.477% |
| reymont-prefix-1024 | 4.532128× | 1.342997× | 1.246235× | -7.024% |

All20 original per-call-median and all20 descriptive block-mean gates pass.18 elapsed medians fall/two equal/none rise. Equal-case geometric block-mean ratio0.931015 gives6.899% lower elapsed; raw median geometric ratio0.938605 is separately retained. No allowance, expected output or timer endpoint changes. Three reference stored-case medians round to zero; their multiples remain unresolved. A single matched cohort does not guarantee these ratios on every machine.

## Independent counters and memory

Separate3120 Mech CPU/heap calls and1280 fresh reference windows/10240 compressions. No counter overhead subtraction or multiplication of independent cohorts.

| Input | Instructions before → after | Instruction change | Cycle change | Requests |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 160,547,053.5 → 159,773,922.5 | -0.482% | -3.441% | 608 |
| 33027-periodic-window | 53,694,533.5 → 53,102,000.5 | -1.104% | -7.919% | 208 |
| repeated-32769 | 53,160,140.5 → 52,571,535.0 | -1.107% | -8.028% | 208 |
| random-32769 | 68,847,425.5 → 68,595,945.5 | -0.365% | -10.928% | 276 |
| 16384-distribution-change | 46,660,817.5 → 46,512,456.5 | -0.318% | -9.187% | 522 |
| eight-literals | 1,977,333.5 → 1,946,014.0 | -1.584% | -13.155% | 230 |

Instructions fall on19 cases/rise on one; cycles fall on17/rise on three. The three adverse cycle cases are empty,one-stored,seven-stored-prefix. Their exact raw medians remain. The much larger elapsed gains than instruction reductions are consistent with removing dependent reference-memory traffic, but this is an inference, not a measured per-instruction stall attribution.

All20 allocation-request/requested-byte/realloc/incremental-usable-peak metrics and payload/copy/zero/peak counters are unchanged. Natural32 keeps608 requests,9687324 requestedB,9750840 incremental usable-peakB and14932 tracked writable-copyB. Requested capacity, initialized copying, live peak/RSS and stack are different quantities; writable-copy counters exclude generated stores/direct memmove.

Reported root code5604324→5611688B (+7364); root stack61296→61552B (+256). The extra compare/select/SSA handling has a static cost; the measured full-call gain is retained with it. Physical output backing reuse remains unestablished.

## Validation and actual profiles

-2560 newly executed compiled before/after original/decoded controls across16 fixtures, including self/swap/overlap/fanout, metadata/predicate changes, private/shared/duplicate data, current outputs, error precedence, prior publication, cleanup and retry.5380 nonnull handoffs execute in the candidate controls. Values/statuses/origins/capacities agree.1456 resident-value,640 resident-error and464 inherited default-denial outcomes remain explicit.
-48 before/after original/decoded generic buffer controls pass without altering hardcoded values or allocation/copy allowances. Real305257-U64 reserve,append,clear/reuse,13-byte U8 output,sublist/reserve,growth,aliases and failures execute.
-Bool/U64/I64/F64 scalar-control Objects are byte-identical before/after and original/decoded, with zero unresolved native calls. This is actual code identity, not a new scalar timing claim.38 small original/decoded image pairs match.
-40 full-stream smokes plus80 separate census calls and all primary/counter/profile calls match exact reference bytes and independent inflate. Final private payload bytes/owner records are zero; saved outputs and aliases survive rejection/retry.
-Fresh exact-executable profiles execute23052 wall and72088 PC calls on six inputs. Large generated-exclusive weights94.29–97.16%,host below native1.57–3.10%; eight literals54.38% generated/29.29% host/16.33% other. Timer weights are not instruction shares or wholly removable overhead. The inherited symbol resolver handles1029 missing-label frames from the start; no wrong R221100%-outside summary is claimed.
-Warnings-as-errors remain. Maintained normal code stays R215. Historical678 normal tests/seven exclusions,52 linked checks and15 architecture/132 workflow checks are not claimed newly repeated here. Fresh maintained report/layout/index checks are recorded separately. Ordinary exact-head CI is cancelled under standing direction; review automation stays enabled and the PR remains draft.
-One introduced analysis receipt-name adaptation error was resolved; it did not affect native execution. The xzr-only inspection and copied descriptive symbol-resolution label were corrected. No tested semantic or elapsed-gate failure remains. No multi-MB timing, ten-workload suite or new full dependency/all-feature rebuild. Full streams use strict decoded MBC14; original/decoded execution is on the explicit small controls.

## Retention and next bounded experiment

Retain R221 experimentally: all20 original gates pass, balanced elapsed falls6.899%,19 instruction medians and17 cycle medians fall. Preserve the three adverse short cycle cohorts and the larger code/stack. NormalR215 remains the qualified compiler; whole-native ordinary guarantees and parity remain open.

The new exact profile still places most large-case time in generated code. The existing prepared descriptor contains247 numeric regions,184 without a nested phase. The compact selector currently requires proof.phase and more than one loop. Two real hot root updates already belong to complete numeric proofs: site12928/header12868 (65 sites,U32 writable array plus read-onlyU8 input,Array result) and site15815/header15737 (199 sites,two U32 writable arrays with read-onlyU64/U8 arrays,Frame result). Both have zero cold exits and no appends. This does not prove all184 are suitable for compact entry.

Next test a compact adapter for the existing complete non-appending SINGLE-LOOP numeric class, using these exact source requirements and a minimal update-loop witness. Reuse existing proof/emission and current-owner/geometry gates; keep checked fallback and source errors. Verify selection/real visits,private/shared/shape/error/retry/original-decoded controls, then the same20-case elapsed/instruction/cycle/heap/profile inventory. Do not invent a new proof engine or specialize on these source IDs. The existing ordinary admission/accounting/funding qualification remains necessary before adoption there.

Replay: `python3 heap-summary.py`, `python3 analyze-census.py`, `python3 analyze-samples.py`, then `python3 analyze.py`. They only replay retained records. Native commands/source pins are in the build specifications and receipts.

## Public evidence

[Exact compiler source,all20 matched timings/counters,2560 ownership and48 buffer controls,before/after census and current profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r221-buffer-result-handoff-d3a0e20b5df5.tar.gz) (3918343 bytes;1200 files). SHA256 `d3a0e20b5df517343d120a543e7e92cd4d5eb7882cc956f868dee1af50233962`. Server/download/all-file hashes and four numerical replays are verified in the publication receipt.
