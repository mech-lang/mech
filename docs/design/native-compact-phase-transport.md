# Effect-aware native phase transport (R212)

**Experiment recorded; retain R211 as the diagnostic default. Production remains R199.** The full20 primary cohort improves geometrically by0.740150%, but periodic and repeated input regress about6.1%, failing two of the unchanged1.05 gates. Separate large-case instruction/cycle medians improve. These contrary results are preserved; fewer interface words are not proof of elapsed improvement.

## Provenance and scope

Base `efd9069b589512630891810191e7de35d57325d8` (R211); fixed diagnostic comparison `2b5049786feb3dfece5b7c743dc45a0ff352009f` (R210); retained production `7748204bd12e57a20369ce7970f31f8d15c3901f` (R199); PR base `92f75db84e2523693648084bea16cd52ad60777a`. IR/work/ABI/MBC214/107/39/14. Only existing draft#973, one measured commit, no new branch/PR. No LLVM change.

Source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same full pinned level12 algorithm/tables, depth300/nice258, original EOF limits and source allocation sites. Real262144-U16 tree,1500513-U32 cache and305257-U64 path capacity; output capacity13 for eight literals. Exact bytes and independent inflate on all20. No source rewrite, substituted compressor or allowance increase.

Same M1/8GiB host, nightly2026-03-03 and pinned mimalloc2.1.7. Complete evaluate_into-equivalent diagnostic boundary includes physical allocations/copies, growth, native compression, output transfer and private cleanup; preparation/clear/prefix/import/rebind/output slots/validation/inflate/drop excluded. Physical final-output backing reuse and complete memory-traffic totals remain unestablished.

## Design and actual mechanism

The existing numeric Plan proves array origins and whether an array appends. Fixed indexed updates cannot change its descriptor. Keep that descriptor with the caller, return scalar lanes and six-word replacement descriptors only for growth-capable arrays. Preserve writable current-owner uniqueness/capacity gates, checked source fallback and actual growth. Standalone diagnostic and ordinary contracts retain their existing interfaces.

Retain the existing array-origin source/result edges and pass them to the SAME OwnerFlow transfer selector used by root aggregates. Entry liveness establishes a held reference; continuation liveness proves whether its source is dead. Transfer each source once, retain fan-out references, exclude overwritten sources and same-site post-uses. All result RHS fields are snapshotted before destination writes. Genuine aliases remain; reference transfer adds no uniqueness requirement. Mutation retains its existing exclusivity gate. Cold errors keep the exact reached source origin and do not commit result ownership.

Three archived compiler files change: numeric_minimal.rs, compact/phases.rs and compact/owner_flow/transfers.rs. The archived full prototype remains30 changed compiler files relative to the maintained production tree. Frozen source and timed artifacts are verified after qualification. This is still a diagnostic with exact ordinary accounting/canonical publication/recovery/full external admission omitted.

| Phase header | Input words before → after | Output words before → after | Growing descriptors returned |
|---|---:|---:|---:|
| 1279 | 31 → 27 | 24 → 4 | 0 |
| 3835 | 31 → 27 | 24 → 4 | 0 |
| 2624 | 22 → 20 | 13 → 3 | 0 |
| 12419 | 28 → 28 | 12 → 7 | 1 |
| 15296 | 28 → 28 | 12 → 7 | 1 |
| 26895 | 28 → 28 | 12 → 7 | 1 |
| 31086 | 28 → 28 | 12 → 7 | 1 |

These are logical interface words, not measured physical byte traffic. Keeping views live across calls may change spills/register allocation. Current exact Object/PC excerpts accompany the profile evidence; no entire root or host bucket is labeled removable.

## All twenty elapsed results

11,488 complete Mech calls /11,328 retained observations,24 balanced permutations per case across four variants.5,120 fresh reference compressions. The first17 case cohorts completed before an expired reference path interrupted the harness; only the three unfinished cases were resumed. All earlier pairs and the interrupted receipt remain. Two primary reference executables were rebuilt from verified pinned source/compiler/allocator/harness; their fresh hashes are recorded.

| Input | Production / Roc | Fixed R210 / Roc | Retained R211 / Roc | Candidate R212 / Roc | Change vs R211 |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.446408× | 5.008961× | 2.753557× | 2.694534× | -2.144% |
| 33027-periodic-window | 3.702072× | 5.431022× | 2.609612× | 2.767952× | +6.068% |
| repeated-32769 | 3.175580× | 4.996167× | 2.401427× | 2.547385× | +6.078% |
| random-32769 | 5.192118× | 16.665025× | 4.522979× | 4.464918× | -1.284% |
| 16384-distribution-change | 6.750736× | 15.169346× | 4.206599× | 4.171343× | -0.838% |
| eight-literals | 4.585681× | 2.242738× | 0.835752× | 0.828347× | -0.886% |
| reymont-prefix-16384 | 4.246616× | 5.778247× | 2.802983× | 2.732306× | -2.521% |
| empty | unresolved | unresolved | unresolved | unresolved | -0.240% |
| one-stored | unresolved | unresolved | unresolved | unresolved | +0.000% |
| seven-stored-prefix | unresolved | unresolved | unresolved | unresolved | -14.237% |
| eight-overlap | 4.477993× | 2.201806× | 0.821009× | 0.810593× | -1.269% |
| 320-overlap | 4.245763× | 2.340747× | 0.902895× | 0.886936× | -1.768% |
| eight-literals-prefix | 4.550773× | 2.220500× | 0.824666× | 0.823937× | -0.088% |
| 128-biased-dynamic | 3.877281× | 2.110203× | 0.766859× | 0.764207× | -0.346% |
| 32-high-entropy-stored | 4.072416× | 2.427014× | 0.860873× | 0.853808× | -0.821% |
| random-64 | 4.248611× | 2.683186× | 0.943642× | 0.947586× | +0.418% |
| random-256 | 4.294146× | 3.396825× | 1.163478× | 1.153131× | -0.889% |
| random-1024 | 3.872517× | 3.947917× | 1.278767× | 1.276558× | -0.173% |
| repeated-1024 | 4.012078× | 2.610044× | 1.033742× | 1.057836× | +2.331% |
| reymont-prefix-1024 | 4.568075× | 4.023979× | 1.409940× | 1.400909× | -0.641% |

15 lower medians, 4 higher, 1 equal. Gates vs R211:18/20; vs production:20/20; vs fixedR210:20/20. Three tiny stored-only reference medians round to zero, leaving their multiples unresolved. No favorable replacement cohort and no gate/allowance changes.

## Separate instructions, cycles and heap

4,160 separate Mech CPU/heap calls,24 retained observations per case/variant/cohort. Fresh same-input reference counters:1,280 windows /10,240 compressions from40 SHA-checked executables. Unadjusted counter overhead remains; CPU allocation zeros are uninstrumented. Do not compound ratios from independent cohorts or turn time samples into per-function instruction counts.

| Input | R211 instructions | R212 instructions | Instruction change | R212 / Roc instructions | Cycle change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 161,628,763.0 | 157,438,102.0 | -2.593% | 3.308853× | -1.588% |
| 33027-periodic-window | 53,546,183.5 | 52,184,795.5 | -2.542% | 2.708803× | -2.947% |
| repeated-32769 | 52,932,058.5 | 51,579,811.5 | -2.555% | 2.697448× | -2.934% |
| random-32769 | 69,930,145.0 | 69,411,808.0 | -0.741% | 5.905990× | -1.321% |
| 16384-distribution-change | 47,419,038.5 | 47,326,750.0 | -0.195% | 5.051741× | -0.637% |
| eight-literals | 2,006,335.0 | 2,006,803.0 | +0.023% | 1.470862× | +0.433% |
| reymont-prefix-16384 | 82,432,626.0 | 80,702,425.5 | -2.099% | 3.251621× | -1.222% |

Instruction medians lower/higher:10/10; cycle medians lower/higher:16/4. All20 request counts, requested bytes and incremental usable-heap peaks match R211. Tracked COW copy counters match; they are not a complete memcpy/initialization ledger. Requested capacity, actual initialized copying, live heap and process RSS remain separate.

| Input | R212 requests | Requested bytes | Incremental usable peak |
|---|---:|---:|---:|
| reymont-prefix-32769 | 5,184 | 10,731,644 | 9,750,840 |
| 33027-periodic-window | 502 | 9,658,395 | 9,709,912 |
| repeated-32769 | 228 | 9,581,410 | 9,709,912 |
| random-32769 | 4,262 | 10,603,819 | 9,709,912 |
| 16384-distribution-change | 3,176 | 9,765,214 | 9,213,016 |
| eight-literals | 368 | 9,059,092 | 9,193,416 |
| reymont-prefix-16384 | 5,360 | 10,216,224 | 9,213,768 |

Generated text including7 phases/2 maps:5,637,108→5,636,428B. Root stack:61,120→60,512B; root-plus-maximum-kernel static stack:61,504→60,832B. These exclude host/allocator stack and process RSS.

## Fresh profiles and dynamic selection

Separate observer image:40 full calls; original numeric/map/pack execution counts remain equal to R211. Root reference moves and phase-boundary moves have disjoint tags. Phase moves are actual reached non-null references, not static eligibility. Primary image has no observer.

| Input | Root moves | Phase-boundary moves |
|---|---:|---:|
| reymont-prefix-32769 | 4,950,603 | 54,138 |
| 33027-periodic-window | 1,476,578 | 33,112 |
| repeated-32769 | 1,448,988 | 32,892 |
| random-32769 | 6,428,859 | 14,027 |
| 16384-distribution-change | 4,226,438 | 4,249 |
| eight-literals | 164,833 | 4 |
| reymont-prefix-16384 | 2,962,521 | 24,987 |

Six wall-stack profiles:22,310 exact full calls. Six exact-PC profiles:65,215. Each uses the actual timed candidate executable.

| Input | Generated exclusive | Host under generated | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 94.017% | 2.707% | 3.275% |
| 33027-periodic-window | 95.184% | 3.196% | 1.620% |
| eight-literals | 55.706% | 24.172% | 20.122% |
| random-32769 | 96.597% | 1.963% | 1.440% |
| repeated-32769 | 95.457% | 2.534% | 2.010% |
| reymont-prefix-32769 | 97.686% | 0.742% | 1.572% |

## Qualification and failures

- 1,920 parent/candidate boundary controls /48 forms cover self/swap/fan-out, overlapping returns, metadata-only use, live aliases, predicate redefinition, ordered failures, exact owner reference counts, capacity, prior backing/publication, cleanup and retry. Predicate-redefinition shape explicitly stays on checked fallback. Another320 controls /8 forms execute a genuine phase with a shared readonly buffer; transfers require no unique backing.
- 1,920 explicit parallel aggregate controls establish R210/R211 behavior;1,920 more compare R211/R212.640 observation-record controls preserve the shared root selector. All original/decoded outputs, statuses, origins, capacities and cleanup match. Resident default16Mi denials match clean parents and are counted in receipts; no caps raised.
- Fresh1,480 map,320 fixed-buffer,192 origin,48 growing/alias phase,1,724 matching and40 builder cases pass.120 original/decoded/observer whole-stream smokes match reference bytes and independent inflate. Actual reference C/Rust map and ownership controls remain separate mechanism checks, not full-compressor timing claims.
- Ordinary scalar Bool/U64/I64/F64 and total-map Objects/metadata/descriptors are byte-identical across parent/candidate and original/decoded (eight images). Optimized library/consumer builds retain warnings-as-errors. Source formatting, maintained module/native build layout checks and patch application pass. Frozen dependencies reused; no whole math-library rebuild.
- Environmental disk exhaustion occurred while linking a clean parent control. Obsolete reproducible builds/raw profiles were removed with source/receipt retention, and only failed/missing tests retried. Completed current fixture binaries were subsequently reclaimed after receipt verification. Interrupted reference restoration required compiler-cache access; the authorized exact rebuild passed. The full successful cohorts are preserved.
- A readonly-return witness with no actual cache read was declined by the parent, so it did not exercise a phase boundary. Retaining a checked cache read produced the required native witness; compiler eligibility was not expanded for the test. Declined record-return shapes remain explicitly inventoried. No new compressor gap inferred from those test shapes.
- The same seven inherited native exclusions remain. Full dependency/all-feature rebuild, multi-MB runs, the separate ten-workload suite and fresh full maintained native/architecture/workflow suites remain skipped. Historical suite counts are not fresh results. Ordinary exact-head CI is cancelled by instruction, with review automation preserved; draft#973 is not marked ready.

## Remaining brief and next bounded work

Qualify one ordinary managed numeric phase with a dense live input/result packet. Reuse the current numeric proof and exact boundary snapshot words; keep canonical owners and current funding in the existing runtime. Cold first-change, selected arithmetic/bounds/assertion failure, unfunded fallback, publication and retry retain their existing paths. The diagnostic result does not authorize weaker accounting or raw-Heap authority in ordinary execution.

Production integration is not complete. The next checkpoint restores one compact live interface inside ordinary admission/funding/ownership with exact error-prefix/publication/retry. It does not wait for all diagnostic cases to win. Stable owner-record pooling, preserving final Vec layout, and a source-site payload census remain separate follow-ups. R205/R206 rejected experiments are preserved and not reproposed unchanged.

Replay heap-summary.py then analyze.py against the extracted archive to recompute numerical results. Native reproduction needs pinned toolchain/dependencies and exact command receipts. No native binaries, bytecode, executable descriptors or raw trace stores are uploaded.

## Public evidence

[Exact compiler patch, all20 timings/counters, semantic controls, current Object excerpts and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r212-phase-transport-13b85fe8dad1.tar.gz) (6472632 bytes;3020 files). SHA256 `13b85fe8dad147ff9e75a7c7d99262f39c8a92f2d662a9ce81e7f9b67d315374`. Fresh server digest, downloaded bytes, all file hashes and byte-identical numerical replay verified.
