# Stable owner-record evaluation and complete payload-site census (R216)

**The brief’s requested experiments and ordinary integration checkpoint are addressed. The owner pool is evaluated but unretained: two of the unchanged twenty 1.05 elapsed gates fail. Normal compilation remains R215/IR215; retained whole-native diagnostic remains R211. No parity claim.**

Base/normal head `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; retained diagnostic `efd9069b589512630891810191e7de35d57325d8`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Normal IR/work/ABI/MBC215/107/39/14 is unchanged. Only the existing draft PR973 and existing branch are used.

## Mechanism and limits

The diagnostic Heap already has stable owner slot indices and a free list. Reuse each slot’s existing 48-byte boxed owner record after its payload retires. Keep its address stable through slot-vector growth; never reuse an active record. Payload allocation/deallocation Layout and the final Vec::from_raw_parts pointer/capacity transfer stay exact. Heap destruction frees retained records once, within the original complete-call timing. No cross-invocation owner/permission or output backing reuse is introduced. The pool patch and eight tests are in the evidence, rather than adopted in maintained compiler/runtime code.

The primary workers link the EXACT retained R211 native Object: SHA256 `f4542d9e15d0d0b4e4e19345c7ddcde78321bcbdd7460a5dd4c2e4c1fba9394e`, 6451496 bytes. Generated metadata reports5597664 code bytes and61120-byte root stack, unchanged. Only the concrete runtime differs. Normal R215 and both diagnostic variants are freshly measured sequentially; compiler activity is excluded.

Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict revision14 MBC SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same mimalloc 2.1.7 and nightly 2026-03-03/Cranelift0.131.3. No source changes, altered level 12 algorithm/tables/search limits/EOF behavior, or changed allocation sites/capacities. Real262144-U16 tree,1500513-U32 cache,305257-U64 path capacity and eight-byte output capacity13 remain. Borrowed input and prepared-output aliases remain live.

The diagnostic omits ordinary exact resource/admission/accounting/canonical publication/recovery contracts. It is not NoLimits or a qualified normal replacement. A small diagnostic speedup cannot be reported as an ordinary compiler speedup. Normal evaluate_into and disclosed diagnostic complete-call boundaries retain their existing preparation/verification/output-drop exclusions; actual allocation/copy/construction/final transfer/private cleanup stay included.

## Complete timing matrix

24 balanced blocks, all six side orders repeated four times;192 retained observations/side for inputs<=1024 bytes,24 otherwise. Every call checks exact bytes and independent inflate. Fresh reference256 compressions/case. These are independently retained primary observations, not hardware-counter or profile timings.

| Input | Normal R215 / Roc | Retained R211 / Roc | Pool candidate / Roc | Candidate vs R211 | Gate |
|---|---:|---:|---:|---:|---|
| reymont-prefix-32769 | 3.453993× | 2.746509× | 2.742236× | -0.156% | pass |
| 33027-periodic-window | 3.657685× | 2.598603× | 2.589837× | -0.337% | pass |
| repeated-32769 | 3.116981× | 2.392766× | 2.386398× | -0.266% | pass |
| random-32769 | 5.093416× | 4.495676× | 4.488699× | -0.155% | pass |
| 16384-distribution-change | 6.673419× | 4.202160× | 4.168347× | -0.805% | pass |
| eight-literals | 4.547004× | 0.837793× | 0.835453× | -0.279% | pass |
| reymont-prefix-16384 | 4.209018× | 2.790679× | 2.772212× | -0.662% | pass |
| empty | unresolved | unresolved | unresolved | +24.551% | FAIL |
| one-stored | unresolved | unresolved | unresolved | +39.234% | FAIL |
| seven-stored-prefix | unresolved | unresolved | unresolved | +0.000% | pass |
| eight-overlap | 4.248720× | 0.806527× | 0.776381× | -3.738% | pass |
| 320-overlap | 4.209323× | 0.908022× | 0.903133× | -0.538% | pass |
| eight-literals-prefix | 4.435816× | 0.804293× | 0.794261× | -1.247% | pass |
| 128-biased-dynamic | 3.828598× | 0.786350× | 0.772874× | -1.714% | pass |
| 32-high-entropy-stored | 4.109848× | 0.875073× | 0.863783× | -1.290% | pass |
| random-64 | 4.210598× | 0.951242× | 0.930265× | -2.205% | pass |
| random-256 | 4.145171× | 1.156546× | 1.120462× | -3.120% | pass |
| random-1024 | 3.802951× | 1.248028× | 1.223085× | -1.999% | pass |
| repeated-1024 | 4.022549× | 1.040460× | 1.029979× | -1.007% | pass |
| reymont-prefix-1024 | 4.566566× | 1.424111× | 1.391392× | -2.298% | pass |

17 elapsed medians fall, two rise and one equals; equal-case geometric elapsed is **1.664% higher**. Empty:167→208ns; one-stored:209→291ns. Their reference medians round to zero, so no reference multiple is invented. The larger-case improvements and tiny-case regressions both remain. The candidate is not adopted and no gate is waived.

## Reconciled allocation/copy census

A separate observed Object tags only concrete allocation/writable helper sites, including inside numeric phases. It changes neither the primary Object nor primary runtime counters. Each successful nonzero payload allocation records exact Plan site, current canonical origin, constructor/update/collection operation, capacity/stride/alignment, requested bytes, zeroing and initialized bytes copied. Zero-size views allocate nothing. All 151 executed sites are resolved; all 20 cases have zero unattributed requests. Header allocation sites are explicitly collection sinks, not guessed scalar operations or human source lines.

Both observed sides execute80 complete calls. Their payload event sequences are IDENTICAL except whether an owner record is freshly allocated. All payload counts, requested bytes, copy/zero volume and payload peaks match the frozen primary workers for every call. The initialized-copy counter covers runtime writable-helper copies; generated initialization stores and direct memmove work are not an all-traffic census. Global allocation counts reconcile exactly with payloads + owner records + the unchanged other requests. Removed requested bytes equal48 times removed owner records.

| Input | Payload requests | Owner records before→pool | Global requests before→pool | Instruction change | Cycle change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2588 | 2588→29 | 5184→2625 | -0.093% | +0.589% |
| 33027-periodic-window | 247 | 247→29 | 502→284 | -0.034% | +0.684% |
| repeated-32769 | 110 | 110→29 | 228→147 | -0.013% | +0.655% |
| random-32769 | 2127 | 2127→28 | 4262→2163 | -0.259% | +0.447% |
| 16384-distribution-change | 1584 | 1584→29 | 3176→1621 | -0.284% | -0.166% |
| eight-literals | 180 | 180→29 | 368→217 | -0.645% | -9.194% |
| reymont-prefix-16384 | 2676 | 2676→29 | 5360→2713 | -0.278% | +0.067% |

Natural32 requested bytes10731644→10608812; incremental usable heap peak9750840→9751224 (+384B). Some other usable peaks also rise slightly because inactive records remain until invocation end; payload peaks do not change. Counts/capacity/live-usable/RSS/copy/stack are different metrics.

Natural32’s2588 payload requests resolve to2306 indexed updates (949428 initialized bytes copied),266 collection sinks,14 capacity constructors,one singleton matrix andone output byte append. Random32:1998/2127 payload requests are indexed updates; distribution16:1340/1584. This identifies the next ownership/copy target without pretending every copy is removable. Source sites17323 and17008 alone account for754×128B and260×1152B copies on natural32. Genuine alias requirements must be established before another change.

17 instruction medians fall/three rise;12 cycle medians fall/eight rise. These are separate whole-call counter cohorts, not estimates derived from sampled percentages. In particular, natural32 cycles rise0.589% despite a0.156% primary elapsed improvement. Fresh reference CPU/heap1280 windows /10240 complete compressions are retained.

## Current exact-executable profiles

| Input | Generated exclusive | Host under native | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 94.249% | 2.897% | 2.853% |
| 33027-periodic-window | 95.144% | 2.625% | 2.231% |
| eight-literals | 56.280% | 25.120% | 18.601% |
| random-32769 | 96.120% | 2.267% | 1.613% |
| repeated-32769 | 94.226% | 3.500% | 2.275% |
| reymont-prefix-32769 | 95.983% | 2.882% | 1.135% |

Six wall profiles22486 correct calls; six actual-PC profiles65027 correct calls. Native symbol inventory, exact executable/Object digests and hot ARM64 excerpts come from the actual candidate. Large-case samples remain94–96% generated code; reducing allocator requests alone cannot eliminate the dominant native work. Timer weights include surrounding worker/verification code and are not per-function instruction shares.

## Validation, failures and coverage

- Eight focused runtime tests pass: stable reuse/double-release, live alias through vector growth, copy-on-write, real U16/U32 initialization versus uninitialized U64 reserve, clear capacity, final Vec layout and survival through record reuse/Heap drop, failed physical size, detached prior output, abort and recovery.
-640 freshly compiled before/after original/decoded controls pass across private,alias,duplicate,duplicate-alias modes. Exact statuses/origins/values/capacities agree; assertions precede failing arithmetic, saved aliases/prior publication survive failure, cleanup and retry succeed. Original/decoded small Objects/metadata are byte-identical.364 resident-value /160 resident-error /116 inherited resident default-denial outcomes are retained. Diagnostic comparison is not qualification of ordinary guarantees.
-8616 full primary calls /8496 retained;3120 separate Mech CPU/heap calls;80 census calls; all 20 source-pinned streams match exact reference bytes and independent inflate. Largest input33027B.
- Normal maintained code is unchanged fromR215, which established678 passing tests,52 linked ordinary controls,15 architecture/catalog checks and132 workflow checks with the same seven inherited exclusions. Those historical Rust counts are not claimed newly rerun inR216. Fresh checks after this report update pass: all15 architecture/catalog commands,132 workflow tests, JSON index validation and git diff --check; the publication receipt records them separately.
- Three development harness errors are resolved: build wrapper expected a name, the observed-worker link flag lost its link-arg prefix, and the copied historical profile driver requested a missing analysis script. Final builds use warnings as errors. Failed profile receipts and all dispositions are retained; no production workaround.
- Two introduced elapsed acceptance failures remain, so the pool stays an archived unretained experiment. There is no unresolved semantic regression in the tested cases. Remote ordinary CI is cancelled by standing direction, review automation remains enabled and the PR stays draft. No green-CI/clean exact-head review claim.
- No multi-MB run, separate ten-workload suite, full dependency/all-feature rebuild or second full-MBC Object emission. Original/decoded controls are explicit small images; full compression uses strict decoded MBC14.

## Brief closure and next bounded work

R211 addresses aggregate owner transfer and actual hot code; R212 evaluates phase-boundary transport (unretained after regressions); R213/R214 qualify an ordinary phase and preserve its home facts; R215 integrates it into the maintained normal compiler. R216 completes the separate owner-pool evaluation and payload-site census. These requested work items are addressed; the brief does not require every experiment to be adopted.

The maintained whole-native replacement remains unqualified, and parity remains unachieved. Next use the exact indexed-update allocation sites to prove or reject redundant current-owner copies, or remove duplicated canonical/packet transport in the existing ordinary class. Preserve the rejected pool evidence; do not reintroduce its two gate failures as an accepted default. No broader allocator or source rewrite was started.

Replay: `python3 heap-summary.py` then `python3 analyze.py`. This only replays retained observations. Native reproduction uses the commands/features/source pins in build specifications, the unchanged R211 compiler source linked in source-pins.json and the allocation-observer.patch. Public evidence excludes native binaries/Objects, bytecode, executable descriptors, raw traces, credentials and environment exports.

## Public evidence

[Exact pool and observer source patches, all 20 observations,640 compiled controls, source-site census and fresh profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r216-owner-pool-census-ee2be6268dde.tar.gz) (2941444 bytes;728 files). SHA256 `ee2be6268dde56788558bb6bef2daa9a02a5caa5f01b46e0c51a3a620149559f`. [Publication verification and exact head receipt](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r216-owner-pool-census-publication.json) records the server/download/all-file/numerical replay checks.
