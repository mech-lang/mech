# Shared total word collections in the native diagnostic (R210)

**Production remains R199 /IR214.** The complete diagnostic now retains the existing proved pointwise scalar/vector lowering for tree rebasing. Periodic/repeated elapsed falls23.57%/23.65%, random32 falls12.67%, natural32 falls9.27%. All20 unchanged gates pass versus R209; six production gates still fail. This is experimental progress, not production adoption, parity or a cost-of-contract floor.

## Exact change and fidelity

Base `94df733966ed70416be7382c2e7ec027b3a945af`; production `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. IR/work/ABI/MBC **214/107/39/14**. Compressor source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Reference package `671077457ff7ef3a3d44caf715a3fc595addf379`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, mimalloc2.1.7 unchanged. Existing draft#973 only; one commit per measured round.

The ordinary preparer already proves two total unsigned pointwise collections,18/19 at headers877/2432. The diagnostic had lost that existing specialization and executed each scalar element through general collection growth, view transport and owner handling. The new adapter reuses both that exact proof and the same scalar/vector emitter. Only a dense array of current scalar captures crosses the private kernel boundary. No source-numbered home bank, interpretation, generic operation dispatch, or compressor-specific selector is added.

The selected route requires a fresh builder/cursor and matching current source count; otherwise the original source CFG remains. Empty collections allocate nothing. Nonempty results retain the original minimum-four/power-of-two capacity, and shape-preserving maps retain actual matrix dimensions. The existing total-body proof permits folding physical growth into one real output allocation, as the ordinary admitted map already does. Bodies with reachable assertions/overflow remain on the checked source route. The ordinary resource/admission/recovery path is unchanged. This is a physical compiler allocation optimization, not removal of a source allocation site or a reduction in scratch sizes.

Five files change from R209; the complete archived prototype has29 changed compiler files. Full patch SHA-256 `339bc0179bc7cd3ec48a633f8333c6c04cf9089d73871aa05f4891aa789ecf84`. No compiler source or timed artifact changes after primary; the patch applies cleanly to the reported source base.

Algorithm, tables, depth300/nice258, original EOF/search/skip rules,262144-U16 tree,1500513-U32 cache,305257-U64 path capacity, source allocation sites and eight-byte output capacity13 are pinned. The complete-call boundary includes actual construction/allocation/copying/output transfer/private cleanup; preparation/clear/prefix/import/rebind/output slots/validation/inflate/drop remain outside. Ordinary exact accounting, canonical recovery/publication and complete external admission are still omitted here.

## Matched elapsed across all twenty inputs

8,616 complete Mech calls /8,496 retained primary observations, twelve balanced permutation blocks per case, plus5,120 verified reference compressions. Production is freshly measured unchanged R199; previous diagnostic is R209. No favorable unchanged rerun replaces any observation.

| Input | Production / Roc | R209 diagnostic / Roc | R210 diagnostic / Roc | Diagnostic change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.364150× | 5.454456× | 4.949050× | -9.266% |
| 33027-periodic-window | 3.456198× | 6.926624× | 5.294152× | -23.568% |
| repeated-32769 | 2.944210× | 6.403544× | 4.889077× | -23.650% |
| random-32769 | 5.152546× | 19.114018× | 16.691392× | -12.675% |
| 16384-distribution-change | 6.411033× | 14.946163× | 14.934072× | -0.081% |
| eight-literals | 4.489037× | 2.222221× | 2.224709× | +0.112% |
| reymont-prefix-16384 | 4.078370× | 5.644617× | 5.649551× | +0.087% |
| empty | — | — | — | -28.425% |
| one-stored | — | — | — | -18.122% |
| seven-stored-prefix | — | — | — | -18.122% |
| eight-overlap | 4.326245× | 2.135995× | 2.163991× | +1.311% |
| 320-overlap | 4.148718× | 2.315316× | 2.332137× | +0.727% |
| eight-literals-prefix | 4.505658× | 2.220238× | 2.252792× | +1.466% |
| 128-biased-dynamic | 3.818537× | 2.083113× | 2.103945× | +1.000% |
| 32-high-entropy-stored | 4.117298× | 2.468090× | 2.482761× | +0.594% |
| random-64 | 4.258583× | 2.676937× | 2.713248× | +1.356% |
| random-256 | 4.154670× | 3.347714× | 3.355290× | +0.226% |
| random-1024 | 3.889191× | 3.993708× | 3.998671× | +0.124% |
| repeated-1024 | 4.142262× | 2.683530× | 2.705357× | +0.813% |
| reymont-prefix-1024 | 4.614536× | 4.089992× | 4.090056× | +0.002% |

**Eight medians improve/twelve worsen; equal-case-weight geometric change−6.894416%.** All twenty prior-diagnostic1.05 gates pass; fourteen production gates pass. Smaller adverse pairs remain, including eight literals+0.11%, eight overlap+1.31% and eight-literals-prefix+1.47%. Empty/one/seven-byte reference elapsed rounds to zero, so no multiples are defined. No independent rounds or cohorts are compounded.

## Instructions, cycles and real memory

2,400 complete Mech calls supply separate CPU/heap cohorts. Fresh same-input reference counters contain1,280 events /10,240 verified compressions, with the exact forty SHA-checked binaries and pinned sources/allocator. Counts include unadjusted probe overhead; CPU-probe allocation zeros are uninstrumented.

| Input | R209 instructions | R210 instructions | Change | R210 instructions / Roc | Cycle change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 285,779,991.5 | 259,022,909.5 | -9.363% | 5.444318× | -9.504% |
| 33027-periodic-window | 111,077,446.0 | 85,350,891.5 | -23.161% | 4.430384× | -24.003% |
| repeated-32769 | 111,379,089.5 | 84,085,073.5 | -24.506% | 4.397363× | -24.187% |
| random-32769 | 225,876,456.0 | 197,511,864.5 | -12.558% | 16.805543× | -13.300% |
| 16384-distribution-change | 136,927,678.0 | 136,441,951.0 | -0.355% | 14.564057× | -0.531% |
| eight-literals | 5,189,545.0 | 5,156,715.5 | -0.633% | 3.779552× | -0.433% |
| reymont-prefix-16384 | 143,225,191.5 | 142,636,604.0 | -0.411% | 5.747064× | -0.303% |

**All twenty instruction medians decrease.** Twelve cycle medians decrease/eight increase; contrary cycle and elapsed cohorts are retained separately. Timer samples are not a retired-instruction partition.

The generic262144-entry U16 control changes17 payload allocations→1,1,048,568 requested payloadB→524,288,524,280 initialized growth-copyB→0, with identical final capacity262144. In the full compressor exactly four inputs execute one selected word-map call: natural32/periodic33 use header2432; repeated32/random32 use877. The other sixteen execute none. This comes from a separate observer image and supplies no timing claim.

For each of those four complete calls, full allocator requests fall32, requested bytes fall525,048, and incremental usable-peak heap falls262,192B. The32 requests are16 removed payload requests plus16 owner boxes; the byte delta is524,280 payloadB plus768 owner-boxB. Natural32 requests9,224→9,192 /requested12,057,256→11,532,208B /incremental peak10,013,032→9,750,840B. Periodic932→900 /10,262,803→9,737,755 /9,972,104→9,709,912. All sixteen other cases retain equal request/byte/reallocation/incremental-peak medians; eight remains498 /9,091,628 /9,193,416.

Requested capacity, initialized copies, usable heap, RSS and physical peaks are distinct. The initialized-copy counter tracks concrete COW helpers, not all memory traffic. All raw process-memory cohorts remain. Capacity/alias checks do not establish physical final-output backing reuse or zero input copying.

Generated text including phases and word kernels grows **7,668,288→7,669,096B** (+808). Root stack84,736→84,672B; root-plus-maximum-direct-kernel envelope85,120→85,056B. These exclude allocator/host stack and RSS. The two actual word kernels are276B /16B stack each. Their dense capture boundary has four words each. Original, decoded and observer generation times are separately recorded build observations, not a matched compile-time benchmark.

Timed Object `155db6419bac33b1927fa66512e2de9e760a1d05eaa05dfc75a67823c8315a54`; worker `2016efeeffda1dc415b9f4e4ee0babaee73c831665cb19734e5489faac4ef644`.

## Fresh executable inspection and next target

Six fresh wall-stack profiles execute10,370 correct complete calls; six exact-PC profiles execute24,866. The exact timed default executable is used, not the observer image. All seven numeric phases and both word kernels count as generated code.

| Input | Generated exclusive | Host below native | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 97.122% | 1.962% | 0.916% |
| 33027-periodic-window | 96.680% | 2.359% | 0.961% |
| eight-literals | 80.652% | 12.652% | 6.696% |
| random-32769 | 98.256% | 1.090% | 0.654% |
| repeated-32769 | 96.949% | 2.092% | 0.959% |
| reymont-prefix-32769 | 98.079% | 1.266% | 0.655% |

The actual new kernels use unsigned saturating vector subtraction and direct loads/stores. Fresh analogous C/Rust Objects and224 linked controls cover both changing thresholds and the pinned32768 threshold. Their constant-threshold code also uses saturating halfword vectors; the widened dynamic versions use more unpack/compare/mask work. These are pointwise mechanism controls, not complete C/Rust compressor timing claims. The exact current reference executable is SHA-checked and its candidate vector instruction contexts are archived without inventing source-function or dynamic-count attribution.

The word kernels now have small timer weight. Random32 and distribution-change share the same hottest root PC0x37af4, at adjacent owner increments/decrements surrounded by stack reloads. Current repeated/periodic root code retains similar transport; natural also retains significant reduction-body/finish time. These samples do not prove every root instruction is removable. The next bounded experiment must trace the aggregate lanes, establish a minimal alias/error/retry witness, and test transferring a dead source owner into its destination using existing liveness/path facts. Preserve live aliases, old-target release, error precedence and cleanup; do not assert a speedup before the next full matrix.

## Qualification, failures and remaining work

- **1,480 new map cases /32 forms** pass across R209/R210 and original/decoded: U8/U16/U32/U64, changing captures, vector boundaries/tails, rectangle/implicit shape preservation, repeated calls, real262144-entry buffers, source assertions and arithmetic failures, unchanged input/prior publication, exact old/new errors/origins/capacities, cleanup and retry.928 resident value and96 resident error agreements are explicit; larger cases use the independent scalar oracle and exact previous native outcomes. No allowance or expected output increases.
- Ordinary Bool/U64/I64/F64 scalar and total-map Object/metadata/descriptor controls are byte-identical before/after and original/decoded (eight ordinary images). This is an image-equivalence control, not a new broad production-suite or latency claim.
- Fresh320 fixed-buffer cases,192 origin,48 phase,1,724 matching and40 builder cases pass, plus120 original/decoded/observer whole-stream smokes. The same142 matching default16Mi denials and24 fixed-witness default denials remain, with seven inherited native exclusions. All33 original/decoded Object/metadata pairs match byte-for-byte.
- Candidate library/linked consumers build opt3 with warnings-as-errors, nightly2026-03-03, Cranelift0.131.3 and unchanged R203 sparse-SSA frontend; recorded features are aot/jit/canonical-native/program/runtime. All29 compiler files pass format; maintained module/native-build layout checks and source-patch application pass. No fresh claim for historical R206677 native tests/260 selections,15 architecture checks or132 workflow tests.

All required local commands finish successfully, with no introduced compiler or harness failure in this round. Full dependency/all-feature rebuilding, multi-MB timing, the separate ten-workload suite and another maintained production-suite run remain skipped. Ordinary exact-head CI is cancelled under the standing instruction, review automation remains enabled and #973 stays draft.

Preserve this diagnostic improvement and every adverse observation externally. Production stays R199. The full brief remains incomplete: exact resource accounting, canonical error-prefix/recovery, canonical ownership/publication and full external admission/import still require measured restoration. The production exclusive-host operation ledger and complete initialized-copy census also remain open. Prior rejected R205/R206 production experiments are not relabeled as new wins.

## Reproduction

Principal wrappers: measure-three.py, measure-counter-cohorts.py, qualify-maps.py, qualify-ordinary.py, qualify-patterns.py, qualify-constant-patterns.py, qualification-pipeline.py and profile-current.py. Exact commands/features/source and artifact hashes accompany all raw receipts. From an extracted packet, heap-summary.py then analyze.py reproduce numerical results without compiling or executing a compressor. Native reproduction needs the pinned toolchain/dependencies. No native binaries, bytecode, executable descriptors or raw trace stores are uploaded.

[Download the exact compiler source, five-file delta, all20 measurements, generic controls, actual assembly and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r210-shared-word-collections-d56e614db03f.tar.gz) (3095725 bytes;1319 files). SHA-256 `d56e614db03f544268fde266f6aaa29cab8372bb0294c27236de2a4f3e441523`. Server digests, every file, fresh download and byte-identical numerical replay verify.
