# R236: narrow pure masked wrapping arithmetic

Retain this five-file compiler experiment under the predeclared full-matrix gates, with a **mixed 0.179% pooled geometric elapsed improvement**. Initial and confirmation cohorts improve only **0.0619%** and **0.0727%**. Ten pooled block means regress, including periodic/repeated/random/heldout-random by roughly 2.5–2.7%. Instructions decrease on 17 cases and increase on eight; cycles decrease on ten and increase on fifteen. This does not establish uniform larger-input progress or a statistically established general speedup.

Outcome-log base: `f9bd1f10f0bc0fbf1f746a1d3d77a30cd335491b`; retained compiler parent: R234 `eab2b19251c9ac6a221b862f1354bb5ec45a4d34`; PR base: `92f75db84e2523693648084bea16cd52ad60777a`. R235 input-packet flags and R225 source bindings stay uncombined. Normal R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14.

## Compiler behavior

The real scalar cost/hash/offset operations use wrapping arithmetic and then retain only low 32-bit bits. Actual retained ARM64 uses full-width arithmetic and masks; local unsigned C/Rust controls emit W-register operations. A generic data-only pass replaces sole-use masked I64 add/sub/mul with I32 arithmetic and zero extension. It recognizes no source names, sites, fields, contents or input sizes. Checked-overflow observers, additional full-width and edge uses, multi-result arithmetic, floating operations, loads and other masks retain their original operations. Pure wrapping work can move to the mask; effects, checks and errors keep their order. The existing CfgCleanup option disables it.

Existing compact root/numeric adapters select the experiment. The ordinary exact admission/accounting/publication/recovery contract and ordinary scalar pipeline stay unchanged. No new operation class or alternate unchecked arithmetic path is added.

90 root and 88 phase arithmetic operations qualify. Actual root text is 5,649,872→5,646,388 B; root stack 55,088→55,184 B (**+96 B**). Thirty-three numeric bodies change; 101 numeric and two map bodies stay identical. Four path bodies shrink 2,048→2,016 B. The actual hot scalar sequence shrinks from five instructions to three, including a surviving W move. Packet geometry and source bounds/ownership/widening/read-reuse reports remain unchanged. Static counts are not dynamic savings.

## Complete matched comparison

Exactly two predeclared complete cohorts, each with 24 balanced blocks across retained parent, candidate and Roc; same pinned clock, fixed level 12, runtime input, 200ms warmup, validation placement and live borrowed output conditions. All 25 unchanged point/block 1.05 gates pass in both cohorts and pooled48. No timing is retried or discarded. The table shows pooled point medians and balanced-block changes; instruction deltas come from a separate hardware cohort. Fresh reference denominators are retained alongside history.

| Case | Mech µs | Roc µs | Before / Roc | After / Roc | Block time Δ | Instructions Δ |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 9832.583 | 4366.229 | 2.254849× | 2.251962× | -0.1280% | -0.6803% |
| 33027-periodic-window | 2170.062 | 1265.438 | 1.670354× | 1.714871× | +2.6651% | -0.0030% |
| repeated-32769 | 2188.479 | 1349.208 | 1.581205× | 1.622047× | +2.5830% | -0.0034% |
| random-32769 | 3034.229 | 1063.562 | 2.783315× | 2.852892× | +2.4998% | -0.1651% |
| 16384-distribution-change | 2332.375 | 832.417 | 2.760086× | 2.801933× | +1.5161% | -0.3412% |
| eight-literals | 139.542 | 290.042 | 0.499641× | 0.481110× | -3.8135% | +0.0415% |
| reymont-prefix-16384 | 4764.270 | 2166.292 | 2.196447× | 2.199275× | +0.1287% | -0.6770% |
| empty | 0.083 | 0.042 | 1.976190× | 1.976190× | -0.2791% | -0.0157% |
| one-stored | 0.083 | 0.042 | 1.976190× | 1.976190× | +0.6038% | -0.0154% |
| seven-stored-prefix | 0.084 | 0.042 | 2.000000× | 2.000000× | -2.2601% | -0.0153% |
| eight-overlap | 143.645 | 290.834 | 0.509312× | 0.493910× | -3.7749% | +0.0678% |
| 320-overlap | 169.709 | 297.459 | 0.566954× | 0.570529× | +0.6068% | +0.0488% |
| eight-literals-prefix | 143.250 | 288.167 | 0.506217× | 0.497108× | -1.4940% | +0.0418% |
| 128-biased-dynamic | 147.333 | 287.833 | 0.525478× | 0.511870× | -2.4266% | +0.0112% |
| 32-high-entropy-stored | 152.938 | 288.625 | 0.537317× | 0.529883× | -0.6576% | +0.0428% |
| random-64 | 161.084 | 290.896 | 0.561341× | 0.553752× | -0.5503% | +0.0224% |
| random-256 | 192.917 | 297.959 | 0.658370× | 0.647463× | -2.1569% | -0.0565% |
| random-1024 | 229.895 | 308.125 | 0.750372× | 0.746111× | -0.5825% | -0.0853% |
| repeated-1024 | 221.542 | 328.438 | 0.669838× | 0.674533× | +0.7324% | +0.0282% |
| reymont-prefix-1024 | 314.875 | 336.375 | 0.948593× | 0.936083× | -0.9489% | -0.2914% |
| heldout-window-32767 | 9805.833 | 4365.188 | 2.253874× | 2.246372× | -0.3329% | -0.6843% |
| heldout-window-32768 | 9784.271 | 4358.917 | 2.251869× | 2.244656× | -0.3203% | -0.6802% |
| heldout-slides-65537 | 20556.250 | 8850.104 | 2.333555× | 2.322713× | -0.4646% | -0.6559% |
| heldout-random-65537 | 6174.271 | 1938.438 | 3.101445× | 3.185179× | +2.6998% | -0.1749% |
| heldout-mixed-131073 | 21683.270 | 8402.041 | 2.527841× | 2.580715× | +2.0917% | -0.3760% |

Natural32 instructions decrease0.6803%, with elapsed only0.1280% lower. Eight-byte elapsed is3.8135% lower while its instruction median increases0.0415% and cycles rise2.7877%; these separate cohorts must not be multiplied or used as a proved causal attribution. The full adverse cycle/elapsed rows remain in the evidence.

## Memory and code costs

All 25 before/after payload requests, requested payload bytes, initialized bytes copied, zeroed bytes and private payload peaks are equal. Separate heap request/reallocation/requested-byte/incremental-usable-peak medians are also equal. Requested bytes are not copied bytes or live memory. No allocation, streaming or whole-worker RSS/physical-memory improvement is claimed. The stack growth above is a real cost.

| Case | Whole-call requests, Mech / Roc | Requested bytes, Mech / Roc | Incremental peak usable, Mech / Roc | Instructions, Mech / Roc |
|---|---:|---:|---:|---:|
| eight-literals | 230 / 96 | 9,030,708 / 9,021,944 | 9,192,488 / 9,257,360 | 1,235,854.0 / 1,361,820.0 |
| reymont-prefix-32769 | 608 / 277 | 9,687,324 / 9,147,888 | 9,750,840 / 9,298,288 | 134,433,397.5 / 47,714,242.0 |
| 33027-periodic-window | 208 / 84 | 9,578,211 / 9,045,231 | 9,709,912 / 9,298,288 | 42,780,808.0 / 19,394,740.0 |
| random-32769 | 276 / 115 | 9,595,155 / 9,061,951 | 9,709,912 / 9,298,288 | 49,065,078.0 / 11,553,920.0 |

## Qualification and current profile

696 maintained tests pass with `-D warnings` and the unchanged single six-feature profile: `aot`, `jit`, `canonical-native`, `canonical-native-runtime`, `canonical-native-program`, `canonical-native-program-runtime`. Seven inherited exclusions remain. Four new arithmetic tests run focused and in the suite: 591,588 JIT value checks/run across the three operations and mask forms with full 64-bit high-bit/wrap edges, 60 additional checked/branch/full-use calls/run, negative IR controls, actual I32 arithmetic, disabled optimization and idempotence. Existing six read-reuse tests and 1,057,536 wide-loop value controls run focused and in the suite without allowance changes.

8,360 original/decoded parent/candidate source controls execute: 1,296 new arithmetic, 1,116 checked-read, 160 real-path and 5,788 matching. 603 resident calls are fresh: 324 arithmetic and 279 checked-read. All native outputs/statuses/origins agree with the independent oracle and the explicitly identified current/frozen resident results. The new source tests preserve assertion-before-arithmetic and arithmetic-before-assertion precedence, checked overflow, unselected paths, changing recursive visits, full-width multiple uses, input aliases, old output on failure, cleanup and retry. Older path/matching resident rows retain their hashes and 34 original default-work denials and are not counted as fresh.

All 25 full streams agree exactly and independently inflate; 50 immediate smoke calls precede measurement. Two elapsed cohorts contain 324,444 complete observed calls, of which 324,144 are retained primary observations, plus 3,900 separate counter calls. Fresh exact-worker profiles cover 25,270 wall and 106,514 actual-PC calls.

Natural32 wall weights: 97.6815% generated exclusive, 0.8311% host-under-generated, 1.4873% other/harness/system. Actual-PC samples: 2,570/5,362 in the two path scans, 1,273 in matching, 1,221 in root. These are sampled-time distributions, not retired-instruction partitions or removable-overhead estimates. Periodic wall weights are 92.4934% generated and 4.8288% host. Current profile attribution uses the exact worker/Object/symbol pins below.

Maintained module-layout/native-build-analysis checks and formatting of the five changed files pass. The extra experimental owner_flow file/directory audit fails identically in retained parent and candidate; its repair stays separately assigned before ordinary integration. No new compiler build/test/semantic failure or program/runtime resource denial occurs. Read-only inspection mistakes and the initial denied process-list read are recorded separately. CI is cancelled under the standing instruction, the existing PR stays draft, and no green CI or completed review is claimed.

## Next bounded target

The current path-scan body still executes repeated loop-state comparisons at its child-reduction boundaries. First construct a minimal source/IR witness and inspect every incoming SSA edge. A literal on one predecessor does not prove a joined condition constant. Test whether edge-specific pure-control threading can bypass redundant dispatch while retaining effects, source checks/debits, changing visits, error order and original/decoded behavior; then measure all 25 again. This is a nominated experiment, not an implemented gain. Input packet loads already occur in the actual path preheader, so repeated input-load hoisting is not a demonstrated hot-loop opportunity.

## Pins and limits

- base: `f9bd1f10f0bc0fbf1f746a1d3d77a30cd335491b`
- retained_compiler_parent: `eab2b19251c9ac6a221b862f1354bb5ec45a4d34`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `0dbf9381d10d29e36c7b1dc2cac55e72f176e2cf062611cce843f96448fd3940`
- compiler_patch_sha256: `8d528125588d0982a6971fe5f982528a06ffe37a551c7da60f9030efd974d7fa`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `47fa77edd30abc5a88558de3871b81b70b16d823cbee85d3774f4715a0876bf2`
- parent_object_sha256: `1561d3a0d98b1db3e8176fb23d5725f543e9accf0b3b383d765c0d80855573fc`
- worker_sha256: `67b40886cb780fbe8f9155a0863873be2fccf3d5f3e2ba10044ae1a19cd353e9`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

The algorithm, tables, depth 300 / nice 258 / pass 10 / cutoff 10000/near-EOF limits, source allocation sites, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, output cap 13 clear/reuse, allocator and timing boundaries are unchanged. The strict full MBC14 image emits once; small source pairs execute original/decoded forms. A second full-image emit, multi-MB inputs, historical packet/write censuses, additional feature profiles and ordinary full-compressor contract/timings are not newly claimed. All 25 sizes through 131073 B remain covered.

[Complete source, five-file patch, all 25 observations, tests, counters and current profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r236-low-word-arithmetic-89547d7d0da9.tar.gz). SHA-256 `89547d7d0da907395fd5074da400bb4bd3110c7a4eb6ecb0ce47438e70af1bd8`; 6871850 B. All 1,757 public hashes and numerical replay verify. Binaries/Objects, bytecode, full executable metadata/descriptors and raw traces/stack samples are excluded.
