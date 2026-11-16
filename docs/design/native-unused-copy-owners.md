# Omit unused copied owner fields in native aggregates (R220)

**The experimental compiler removes transient references for copied owner fields that existing field liveness proves unread. All20 unchanged elapsed gates pass. The balanced block-mean inventory improves1.022%; the change is retained experimentally. Normal compilation remains R215/IR215.**

## Provenance and scope

Base `8791df325cb2ba766e8c580c794ece8e8651c37c`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. The source base is the retained R217 prototype after R219 qualification, with two compiler files changed. Same nightly2026-03-03, Cranelift0.131.3 and mimalloc2.1.7. Existing draft#973 only; no new branch, PR or LLVM change.

Normal IR/work/ABI/MBC215/107/39/14 remains unchanged. Strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`; compressor source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. All pinned level12 operations/tables/depth300/nice258/EOF behavior/source allocation sites/timing boundaries and real262144-U16 tree,1500513-U32 cache,305257-U64 path capacity/eight-byte output capacity13 stay. Genuine input/prepared-output aliases stay live.

**The whole-native diagnostic still omits ordinary exact admission/accounting/canonical publication/recovery. These are diagnostic results, not normal-entry speedups or parity.**

## Exact source-PC map before the change

Three diagnostic source-tag/export edits use existing Cranelift SourceLoc support. The complete mapped Object is BYTE-IDENTICAL to the actual retained R217 Object, SHA256 `90e6a726d58f8385345ba38e7d0c15491825dd534b22644e439a9c647860de99`. Original/decoded small Objects also match the untagged compiler with identical observer settings. No generated callback or executable work is added.

The independently retained six current-PC profiles join to the same41039 Plan origins. Explicit source locations cover2470/5108 distribution,3301/5236 periodic,1939/3365 eight,2371/5172 random,3230/5119 repeated and3194/5275 natural native samples. Untagged backend moves/gaps, prologues and word-map kernels remain UNMAPPED. No nearby source location is silently substituted. These are timer samples, not per-operation retired instructions.

The map identifies indexed read/write sites12920/12928/15815 in the root, packed match extension2744/2895 in phase2624, and nested match-cost selection15529 in phase15296. The source tag of a fused expression is not a claim that every machine instruction implements only the named scalar operation. No baseline offsets are applied to the changed candidate.

## Compiler change and safety

`OwnerFlow::unused_copy_owner` reuses its existing owner-copy edges, continuation liveness and current output-port uses. Only a pure copied destination owner that has no reached payload use qualifies. Fresh allocations and buffer-operation results do not qualify. Existing transferred tokens retain their prior handling; this experiment targets the otherwise additional retain.

After the SAME checked aggregate action succeeds, preserve scalar metadata but seed its unused destination owner with zero. Omit the retain that would otherwise be immediately followed by retirement. Old destination cleanup stays in its existing order; genuine aliases and source mutation exclusivity/capacity gates remain. All existing source checks finish before this change. No second ownership analysis, new permission cache or dynamic heap scan is introduced.

73 static owner lanes qualify. The separate complete-call observer shows natural32 reaches224 such events, only13 nonnull; periodic/repeated74/6, random89/6, distribution195/14 and eight75/7. These modest counts prevent claiming that this change removes the dominant ownership work. Small controls record1664 nonnull events across all candidate forms/calls, a different denominator.

## All20 matched elapsed results

Three variants run sequentially in24 balanced side-order blocks. Per-call timer endpoints and the original1.05 gate remain. Requests are batched outside the timer:512 per block for the three clock-limited stored cases,64 for other inputs<=1024B,one for larger inputs.161832 complete Mech calls/161712 retained observations plus5120 fresh reference compressions. Exact raw bytes and independent inflate are verified on every call.

| Input | Normal R215 / Roc | Retained R217 / Roc | Candidate R220 / Roc | Block-mean change vs R217 |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.435698× | 2.743081× | 2.723254× | -0.723% |
| 33027-periodic-window | 3.654878× | 2.590599× | 2.613494× | +0.884% |
| repeated-32769 | 3.135565× | 2.409801× | 2.420679× | +0.451% |
| random-32769 | 5.098611× | 4.479365× | 4.484920× | +0.124% |
| 16384-distribution-change | 6.591520× | 4.135232× | 4.129329× | -0.143% |
| eight-literals | 4.342408× | 0.814024× | 0.798199× | -1.473% |
| reymont-prefix-16384 | 4.199566× | 2.759247× | 2.755245× | -0.145% |
| empty | unresolved | unresolved | unresolved | -1.971% |
| one-stored | unresolved | unresolved | unresolved | -4.482% |
| seven-stored-prefix | unresolved | unresolved | unresolved | -2.310% |
| eight-overlap | 4.178419× | 0.799595× | 0.780997× | -2.053% |
| 320-overlap | 4.024353× | 0.882041× | 0.875142× | -1.006% |
| eight-literals-prefix | 4.449777× | 0.829464× | 0.820536× | -1.197% |
| 128-biased-dynamic | 3.670114× | 0.756310× | 0.751687× | -0.377% |
| 32-high-entropy-stored | 3.953745× | 0.849797× | 0.838287× | -1.548% |
| random-64 | 4.031685× | 0.909747× | 0.892192× | -1.973% |
| random-256 | 4.035493× | 1.062990× | 1.056118× | -0.318% |
| random-1024 | 3.708609× | 1.153398× | 1.142465× | -0.895% |
| repeated-1024 | 3.942173× | 1.040304× | 1.033879× | -0.837% |
| reymont-prefix-1024 | 4.480997× | 1.353915× | 1.347855× | -0.309% |

All20 original per-call-median gates and all20 descriptive block-mean gates pass. Median changes:15 lower/three higher/two equal. Equal-case geometric block-mean ratio0.989781 is the headline1.022% improvement. The quantized seven-byte median125→84ns exaggerates its block-mean improvement of2.310%; consequently the raw median geometric ratio0.974292 is retained but is not advertised as a2.571% speedup. Reference medians on three stored cases round to zero; their multiples remain unresolved.

Contrary large results remain: periodic+0.884%,repeated+0.451%,random+0.124%; natural32−0.723% and distribution−0.143%. A small overall win does not imply every input improved. This single matched cohort is not a cross-machine guarantee.

## Independent instructions, cycles and memory

Separate3120 Mech CPU/heap calls and1280 fresh reference windows/10240 compressions. No counter overhead subtraction or multiplication of independent cohorts.

| Input | Instructions vs R217 | Cycles vs R217 | Allocation requests |
|---|---:|---:|---:|
| reymont-prefix-32769 | +0.137% | -0.084% | 608 |
| 33027-periodic-window | +0.380% | +0.278% | 208 |
| repeated-32769 | +0.381% | +0.123% | 208 |
| random-32769 | +0.066% | -0.252% | 276 |
| 16384-distribution-change | -0.047% | -0.505% | 522 |
| eight-literals | +0.231% | -7.545% | 230 |

Instructions fall on one case, rise on18 and equal one. Cycles fall on18 and rise on two. This is a measured latency/cycle tradeoff, not an instruction-count reduction. All20 allocation requests/requested bytes/reallocations/incremental usable peaks and tracked payload/copy/zero/peak counters match R217. Natural32 retains608 global requests and14932 tracked writable-copy bytes; requested capacity is not initialized copying or live peak. Those writable-copy counters exclude generated stores/direct memmove.

Reported ROOT code5608036→5604324B (−3712); root stack61344→61296B (−48). These are compiler metadata, not process RSS or measured stack peak. Candidate Object SHA256 `5a2f11e2c7a083960d343de3b00b94e6948f7585fe16e10886bec86af8c91256`. Physical final-output backing reuse remains unestablished.

## Current profiles and validation

Fresh exact-candidate profiles execute22487 wall and67010 PC calls across six inputs. Large-input generated exclusive timer weights are94.97–98.52%; host below native0.48–3.27%. Eight literals is59.82% generated/22.94% host/17.24% other. These are disjoint wall weights, not retired-instruction shares or wholly removable overhead.

Three wall profiles initially contained local unknown symbol labels. The old analyzer misclassified their weighted trees as100% outside the worker. Resolve1490 unknown frames using the ACTUAL pinned worker symbol table and load addresses; preserve raw samples and the initial wrong summary. Totals remain unchanged, with zero unresolved local frames. This is a diagnostic-analysis repair, not new timings or a runtime change. Pure replay uses the derived weighted nodes and exact symbol inventory.

-2560 freshly executed compiled controls:16 private/alias/duplicate/self/swap/fanout/overlap/metadata/predicate fixtures ×before/after ×original/decoded ×40 calls. All statuses/origins/values/capacities match.1456 resident-value,640 resident-error and464 inherited default-denial outcomes remain explicit; denial is not a resident success. Original/decoded Objects/metadata match. Saved aliases/prior publication, ordered checks, cleanup and retry are verified.
-40 full-stream smokes and40 separate full observer calls match exact reference bytes/inflate. All primary/counter/profile calls also verify output. The observer matches primary payload/copy/zero/peak counts on all20 cases. Final private payload bytes/owner records are zero.
-All builds preserve warnings-as-errors. Normal maintained code is unchanged. R215678 tests/seven exclusions,52 linked checks and later15 architecture/132 workflow checks remain HISTORICAL unless separately identified as fresh report checks; this round does not claim a new ordinary Rust suite.
-Resolved harness errors: a relative producer path under a wrapper with another working directory; an invalid observer/nonobserver Object comparison; rejecting Cranelift default SourceLoc as a source site; missing-symbol wall classification. Each is resolved with exact matching configurations/current symbols. No compiler semantic failure or elapsed gate failure remains in the exercised candidate.
-No multi-MB timing, ten-workload suite, full dependency/all-feature rebuild or second full-MBC original/decoded Object pair. Full streams use strict decoded MBC14; small original/decoded coverage is explicit. Hosted ordinary CI is cancelled under standing direction, review automation stays enabled, and the PR remains draft.

## Retention and next bounded target

Retain R220 experimentally for its1.022% balanced elapsed point,18 lower cycle medians and all20 unchanged gates. Preserve R217 and every contrary instruction/timing result. Ordinary R215 remains the qualified baseline; full whole-native qualification and parity remain open.

Fresh inspection of the pinned reference U32 list-set shim at0x10003060c shows makeUnique, one U32 store and direct return of the three-word view; there is no additional retain/decrement pair in that shim. Its unique helper still checks the actual reference count. This static pattern is not a dynamic call-count or proof about every caller.

The next compiler experiment should transfer the existing ownership token through successful CONSUMING buffer updates. Current Arrays::emit returns a view, then the generic definition path retains its owner and separately retires the source—even when the checked update reuses the same backing. R220 only removes13 nonnull copied references on natural32, so repeating rare aggregate cleanup is a weaker target.

First count reached root buffer results separately and build the minimal unchanged/changing two-store witness. Preserve live aliases, same-site outputs, overwritten source/destination, shared and unique backing, growth, exact checks/error order and recovery. Move one token only after successful checks when the existing proof permits; fresh fallback results still need their new reference. Pure transfer requires no uniqueness; actual mutation keeps its original gate. Verify actual generated retain/release removal and all20 elapsed/instruction/cycle/heap/profile outcomes. No source/algorithm rewrite or broader compiler redesign is justified by this finding.

Replay numerical records with `python3 heap-summary.py`, `python3 analyze-observed.py`, `python3 analyze-samples.py`, `python3 map-pcs.py`, then `python3 analyze.py`. These do not execute a compiler or compressor. Native build/run commands and source hashes are retained in the build specifications and receipts.

## Public evidence

[Exact source patches, all20 timings/counters,2560 controls, full observer census and current profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r220-unused-copy-owners-a59448bfff37.tar.gz) (5528718 bytes;1159 files). SHA256 `a59448bfff370e3183db32c0c365cd08be678f58591e88546011bab8f9d410ec`. Server/download/all-file hashes and five numerical replays are verified in the publication receipt.
