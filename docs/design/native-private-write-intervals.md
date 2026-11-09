# Guarded private indexed-write intervals — R240

**The R234-based component qualifies, but does not replace retained R237.** Across the predeclared 11 large cases its balanced-block geometric elapsed improves **1.0771%** against R234; the all25 improvement is **0.2291%**. Both complete24-block cohorts and the pooled48 pass all25 unchanged1.05 point/block gates. Against separately measured R237, the large-case portfolio is **+0.2132%**. Preserve both qualified implementations; do not claim this variant is uniformly better or discard the small gain.

## Scope and revisions

Parent outcome `dd04883faa58b304fc7ad94d80208aeb457c3a66`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Main computational control R234 (`eab2b19251c9ac6a221b862f1354bb5ec45a4d34`), retained standalone R237 (`f92415a31dbf0398ae641e66c419f435a3e3cb7c`), ordinary compiler R215 (`42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; IR/work/ABI/MBC215/107/39/14). The full source, exact four-file patch and all source hashes accompany this report. The timed producer precedes only a later cfg(test) addition; every non-test source byte is verified against the prospectively frozen manifest.

Standalone compact native compiler experiment. Ordinary exact resource/admission/accounting/canonical publication/recovery contract remains omitted; ordinaryR215 unchanged.

The pinned source/MBC hashes are `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Full source and decoded bytecode independently emit the identical complete candidate Object. Turning the experiment off emits R234’s complete Object byte-for-byte. Candidate Object `209953c08278efba5755767ecdbe3e9ce940fbb9f01f1edb105dd2b92a0335cd`.

No compressor source/algorithm/table/search/pass/allocation-site/timing-boundary change. Level12, depth300, nice258, original near-EOF checks,262144 U16 tree,1500513 U32 cache,305257 U64 path capacity, output13 clear/reuse, held input/output aliases and pinned allocator remain. No new PR or branch.

## Repeated work and proof

R239 identifies high-frequency consecutive root writes that already arrive with unique backing. This bounded generic proof selects a single-entry linear interval containing at least two consumed non-growing replacements of the same exact view. Intermediate actions must be owner-free scalar/aggregate operations. Every replacement retains the existing can-finish-source proof and has an empty equality-dependent prechange-retirement list. Calls, owned uses/alias construction, effects, other arrays, interior joins, phase/pack boundaries and unsupported actions decline. The32-site analysis bound only declines optimization; it is not a program resource denial.

A nonallocating current nonnull/refcount==1 guard admits the interval on each reached visit. Shared backing executes the unchanged compare-before-copy fallback. The sealed admission witness authorizes the existing array emitter only after its original element, bounds and current-schema canonical linear-ordinal checks. It emits direct packed stores and carries unchanged pointer/length/capacity/geometry/owner words. Checked scalar arithmetic remains ordered between writes.

The root’s checked action and owner-settlement emission is factored once and reused by both paths. Transfers, retirement, error origins and cleanup remain the existing implementations. Each cloned failure block reports that exact source site. Explicit join predecessor accounting precedes SSA sealing; the Cranelift verifier runs before cleanup. No source/function/site-name specialization, eager shared clone, speculative growth, new ABI or per-array permission cache.

Four compiler files change: compact/private_chains.rs (derivation, sealed witness, eight proof/refusal tests), compact/emit.rs (shared action, guarded interval and joins), compact/arrays.rs (same checked indexing plus admitted store), compact/mod.rs (registration). Twelve two-write intervals qualify, including the real1038→1040 head pair. Compatible child-return continuity and zero-trip transport are still open.

## Generated code and memory tradeoff

Only the root changes: **5,649,872→5,654,028 B** (+4,156); stack **55,088→55,152 B** (+64). All134 numeric and two map bodies remain byte-identical. All25 payload requests, requested bytes, initialized copies, zeroing, private peaks and separate heap allocation/reallocation/usable-peak counters match R234. This is an instruction/control optimization, not an allocation reduction.

Removing equality tests changes memory traffic. In the already recorded R239/R234 control census, random32’s selected all-unique intervals contain117036 old-value reads and77297 equal writes. The candidate predicts154594 additional identical-value store bytes. Old changed-only writable-owner inspections39739 become58518 per-interval inspections; the equal-clear path therefore increases ownership inspections even though other repeated control disappears. Natural32 predicts76930 removed reads,28602 extra store bytes and62629→38465 owner inspections. These are explicit control-census predictions, not fresh hardware store counters or a claim that every ownership decision decreases. Focused observer controls separately prove actual private selection and shared fallback.

## Complete-call elapsed, multiples of Roc

Four executables share each of24 balanced permutations per cohort; two prospectively fixed cohorts,200ms warmup per case/variant. The11 inputs≥16KiB are primary; no tiny-case compensation. Validation uses exact raw bytes and independent inflate outside the matched complete-call timer; scratch construction, output transfer and cleanup remain included. Historical comparison records remain unchanged.

| Input | R234 / Roc | Candidate / Roc | R237 / Roc | Candidate block mean change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 1.976190× | 1.976190× | 1.976190× | -1.1032% | -0.0471% | -15.7957% |
| one-stored | 1.976190× | 1.976190× | 1.976190× | -1.8332% | -0.0461% | -17.0782% |
| seven-stored-prefix | 2.000000× | 2.000000× | 2.000000× | -1.4843% | -0.0458% | +7.6038% |
| eight-literals | 0.493649× | 0.500856× | 0.495716× | +1.4511% | -0.1387% | -1.3466% |
| eight-overlap | 0.490997× | 0.498143× | 0.493283× | +2.6517% | -0.1258% | -1.0556% |
| 320-overlap | 0.555664× | 0.559657× | 0.561305× | +0.4221% | -0.6328% | -0.0093% |
| eight-literals-prefix | 0.497674× | 0.497814× | 0.495885× | -0.5145% | -0.1383% | +1.4703% |
| 128-biased-dynamic | 0.498927× | 0.504793× | 0.505796× | +1.6845% | -0.3565% | -1.0747% |
| 32-high-entropy-stored | 0.525543× | 0.533033× | 0.522116× | +1.0995% | -0.1497% | -1.1994% |
| 16384-distribution-change | 2.789916× | 2.751299× | 2.783506× | -1.3842% | -1.3412% | -1.4216% |
| 33027-periodic-window | 1.688896× | 1.669372× | 1.738908× | -1.1560% | -2.1646% | -1.0247% |
| random-64 | 0.557070× | 0.564027× | 0.561968× | +1.5906% | -0.2219% | -1.7209% |
| random-256 | 0.660889× | 0.668730× | 0.659305× | +1.4769% | -0.4511% | +0.5573% |
| random-1024 | 0.747823× | 0.748090× | 0.755062× | +0.1314% | -0.9819% | +0.4984% |
| repeated-1024 | 0.655449× | 0.659494× | 0.669871× | +0.5696% | -1.1147% | +1.7152% |
| random-32769 | 2.804555× | 2.769391× | 2.864031× | -1.2538% | -1.7999% | -1.7976% |
| repeated-32769 | 1.597679× | 1.580126× | 1.653402× | -1.0987% | -2.1698% | -1.5298% |
| reymont-prefix-1024 | 0.933044× | 0.934529× | 0.914355× | +0.1618% | -0.7377% | -0.0383% |
| reymont-prefix-16384 | 2.219911× | 2.205243× | 2.105314× | -0.6607% | -0.7038% | -0.6837% |
| reymont-prefix-32769 | 2.261704× | 2.248339× | 2.145702× | -0.5909% | -0.6935% | -0.1958% |
| heldout-window-32767 | 2.261982× | 2.252875× | 2.151384× | -0.4026% | -0.6945% | -0.2250% |
| heldout-window-32768 | 2.263640× | 2.255829× | 2.151223× | -0.3451% | -0.7043% | -0.4150% |
| heldout-slides-65537 | 2.339916× | 2.331941× | 2.235444× | -0.3408% | -0.6874% | -0.5482% |
| heldout-random-65537 | 3.105705× | 3.016150× | 3.186167× | -2.8836% | -1.8998% | -3.5412% |
| heldout-mixed-131073 | 2.529816× | 2.486746× | 2.535176× | -1.7025% | -1.1739% | -1.7557% |

Family geometric block means: natural -0.6258%, repetitive -1.1273%, random_distribution -1.8433%, mixed -0.6995%. Both initial and confirmation large-case means improve separately. All ten adverse pooled block means remain in the table; eight-literals rises1.4511%, eight-overlap2.6517%. R237’s natural32 remains faster: 2.145702× versus candidate2.248339×.

## Independent counters and fresh profiles

All25 retired-instruction medians fall. Cycles fall20/rise5; preserve adverse cases rather than multiplying independent cohorts. Separate CPU and memory cohorts execute5200 complete observed calls,4800 retained. Natural32 instructions135356079→134417441.5 (−0.6935%); random32 49146244.5→48261641.5 (−1.7999%). These remain about2.817× and4.177× the separate reference instruction counts.

Fresh six-case wall profiling executes25468 correct calls; actual-PC profiling executes103717. Natural32 wall weights are97.7729% generated exclusive and0.9170% host-under-generated. Its5462 running-PC samples place2582 in path15296,1326 in matching1279 and1199 in the root. Random32 has3199/5342 in the root. Those are sampled time contexts, not retired-instruction partitions or percentages of removable work. Actual hot ARM64 windows and disjoint weighted summaries accompany the evidence.

## Qualification and failures

700 compatible maintained tests pass (eight new proof tests included), warnings denied, six native feature flags, nightly2026-03-03/aarch64. Same seven inherited exclusions. The focused eight proof tests also run separately. Frozen Core/Engine/math/runtime/dependency libraries are reused rather than rebuilding the math catalog; exact rustc invocations and hashes remain in receipts.

59028 native controls pass:6000 parent/candidate original/decoded two-write oracle calls;52800 current row/column/rectangular/zero-geometry and packed-record calls;152 held-publication/retry/Boolean calls plus76 observer calls.58968 have independent source-operation oracles;60 raw-Boolean observations establish exact clean-parent agreement. Full standalone external admission is omitted: malformed nonzero assertion inputs can be accepted by that existing raw entry, while invalid exhaustive Boolean matches reject. This does not weaken the maintained canonical entry.

Selected/unselected aliases, equal/changed stores, first-bounds-before-later-overflow, later checked overflow, assertion rejection, changing arguments, exact current views and packed8-byte stride pass. Previously retained backing and failed result ports stay unchanged; cleanup and fresh retry succeed. All25 complete streams pass independent inflate, held aliases, exact capacities and zero final private payload/owner records.

Introduced harness failures are repaired and preserved: a null owner dereference in a zero-capacity bootstrap (before the first program call); an incorrect new geometry oracle that assumed an explicit row comprehension preserved input shape; and a deterministic report/receipt filename collision. Correct current-view witnesses use the existing checked COW operation. A packed-record comprehension fixture is refused by the clean parent (inherited unsupported collected element kind); the reachable packed constructor/write path needs no new compiler work. No expected value or allowance of an existing qualification changes.

The unchanged experimental owner_flow.rs/file-directory collision remains assigned to ordinary integration. Maintained layout, architecture and formatting checks are separate from that experimental debt. Same seven exclusions, full Cargo dependency/all-features rebuild, multi-MB runs and separate ten-workload suite remain skipped. Hosted CI stays cancelled by standing instruction; PR stays draft pending green hosted CI and clean exact-head review. No current clean-review claim.

## Disposition and next step

Preserve the fully qualified R234-based private-write implementation and both contrary timing cohorts. Keep R237 as the retained executable: candidate is0.2132% slower on its large portfolio, despite useful per-family improvements against the declared R234 control. A later combination must be measured, not inferred.

The brief is not complete. This closes short linear private-view intervals, not collector-wide/child-return continuity. Direct ordered-reduction completion, whole-phase append reserve, comparison-result reuse, remaining mismatch/recompute/zero-trip/stack census and ordinary integration remain open. The next shared-emitter target is direct reduction completion, preserving terminal payload assignments, tie rules, exact funding/error prefixes and fallback in both entry paths.

Replay primary and independent counter arithmetic with `python3 replay.py` after extracting the evidence packet. It does not compile or compress. Executables, Objects, bytecode, full executable metadata, raw stack samples and raw traces are excluded from the public archive.

## Public evidence

[Source, exact patch, controls, full timing/counter data and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r240-private-write-e303b38b68d6.tar.gz) (`sha256:e303b38b68d60bb564a59ae07a01a22fcfbb4a863e453182a42c7526c3be10b9`, 7,038,753 bytes). All 1,561 downloaded file hashes, server digest and seven byte-identical numerical replays verify. No native executable, Object, bytecode, full executable metadata or raw trace is included.
