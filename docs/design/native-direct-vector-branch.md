# Native vector truth without Boolean materialization — R231

Retain the two-file compiler experiment over R230. A private branch now tests the OR of two U64 vector lanes instead of producing a normalized Boolean. No source operation, algorithm, allocation site, original guard or mismatch/tail behavior changes.

Both predeclared complete 24-block cohorts and all 48 pooled blocks pass all 25 unchanged 1.05 gates. Pooled geometric block means improve **1.550%**. Periodic 33,027 B improves **3.977%**, repeated 32,769 B **3.751%**, and natural text 32,769 B **0.595%**. Retain the adverse one-stored block mean (+0.413%) and its cycle increase (+2.724%). Tiny clock quantization and separate cohort movement remain visible; no additional eight-byte algorithmic work saving is claimed.

## Complete-call timing

| Case | R230 / Roc | R231 / Roc | Pooled block change | First cohort | Second cohort |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.313231× | 2.299463× | -0.595% | -0.660% | -0.587% |
| 33027-periodic-window | 1.785572× | 1.714551× | -3.977% | -4.231% | -4.055% |
| repeated-32769 | 1.680338× | 1.617311× | -3.751% | -3.942% | -3.688% |
| random-32769 | 2.804937× | 2.787642× | -0.617% | -0.111% | -0.725% |
| 16384-distribution-change | 2.786859× | 2.771643× | -0.546% | -0.131% | -1.144% |
| eight-literals | 0.513047× | 0.499426× | -2.555% | -1.609% | -3.583% |
| reymont-prefix-16384 | 2.252878× | 2.245777× | -0.315% | -0.073% | -0.751% |
| empty | 1.976190× | 1.976190× | -0.038% | +1.895% | -1.085% |
| one-stored | 1.976190× | 1.976190× | +0.413% | +1.344% | -1.217% |
| seven-stored-prefix | 2.000000× | 2.000000× | -0.635% | -0.220% | -0.653% |
| eight-overlap | 0.510147× | 0.495034× | -2.805% | -1.256% | -2.891% |
| 320-overlap | 0.576091× | 0.562553× | -1.855% | -0.650% | -3.915% |
| eight-literals-prefix | 0.511800× | 0.497983× | -2.934% | -2.454% | -3.924% |
| 128-biased-dynamic | 0.521438× | 0.507048× | -2.817% | -2.170% | -3.094% |
| 32-high-entropy-stored | 0.550680× | 0.530465× | -3.796% | -2.092% | -5.350% |
| random-64 | 0.578644× | 0.563516× | -2.695% | -1.187% | -3.738% |
| random-256 | 0.675883× | 0.663255× | -2.114% | -1.576% | -3.694% |
| random-1024 | 0.765921× | 0.755734× | -1.712% | -0.950% | -2.062% |
| repeated-1024 | 0.687419× | 0.668831× | -2.447% | -2.410% | -2.800% |
| reymont-prefix-1024 | 0.956751× | 0.944990× | -0.866% | -0.367% | -1.923% |
| heldout-window-32767 | 2.309903× | 2.304299× | -0.243% | -0.247% | -0.185% |
| heldout-window-32768 | 2.314856× | 2.300908× | -0.603% | -0.666% | -0.495% |
| heldout-slides-65537 | 2.385689× | 2.377007× | -0.364% | -0.292% | -0.476% |
| heldout-random-65537 | 3.131275× | 3.127984× | -0.105% | -0.259% | -0.238% |
| heldout-mixed-131073 | 2.588772× | 2.574488× | -0.552% | -0.482% | -0.654% |

324,444 complete primary calls / 324,144 retained observations. Same 20 original and five held-out runtime inputs through 131,073 B, fixed level 12, held input/output aliases, equal 200 ms warmup, balanced three-actor order, pinned allocator and shared Rust clock. Full compression, real scratch construction, output transfer and cleanup remain inside the timing boundary; validation, preparation, logging and old output drop remain outside. Exact reference streams and independent inflation agree on every observation. Final nonreturned private payload and owner records are zero. No compilation, test or profile overlaps timing/counter collection. Historical ratios from other cohorts are not substituted for this matched comparison.

## Compiler change and exact machine evidence

Only `codegen/wide_equal.rs` and its focused test change. Keep R230's private current-window certificate, sole changing induction phi, pure two-load equality cycle, original readable guards and original scalar mismatch fallback. Sixteen bytes remain admitted on both sides before either load. The only new choice is I64X2 loads, XOR, extraction of both U64 lanes, OR, and branch on the resulting nonzero word.

The OR is zero iff every bit of both lanes is zero. Signedness and lane order do not affect this predicate. It is a private machine branch condition with no canonical/source Bool observer or storage destination. Ordinary Bool normalization, scalar vocabulary, resource accounting and admission remain unchanged. No source identity, table name, compression path or input size selects the rule.

Actual ARM64 changes each vector probe from pairwise reduction / lane transfer / comparison / cset / byte normalization / conditional branch to two lane transfers / OR / conditional branch. The q loads and vector XOR remain. Nine existing static probes lose two instructions each: **18 instructions / 72 bytes** across the same three numeric bodies. The complete 5,649,872 B root and 133 other specialized bodies remain byte-identical. No new phase or source operation becomes native. Static eligibility is not an observed dynamic probe count.

Frames remain 400 / 400 / 304 B for the three bodies; root stack stays 55,088 B. The full worker remains 6,623,288 B including file padding. Heap requests, requested bytes, reallocations and incremental usable peaks match exactly on all 25 inputs. Source payload requests/bytes, initialized copying, zeroing and private peaks also match. No streaming or whole-worker RSS reduction is claimed.

## Hardware and profiles

3,900 separate CPU/heap observations. Sixteen instruction medians fall, three tie, six rise; twenty cycle medians fall and five rise. These cohorts are distinct from primary elapsed measurements and CPU sample attribution.

| Case | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 138,349,781.0 | 138,145,429.5 | -0.148% | -0.512% |
| 33027-periodic-window | 45,578,447.0 | 44,527,324.0 | -2.306% | -4.361% |
| repeated-32769 | 45,128,639.5 | 44,084,476.0 | -2.314% | -3.579% |
| random-32769 | 49,645,417.5 | 49,645,325.5 | -0.000% | +0.387% |
| 16384-distribution-change | 35,262,037.0 | 35,261,112.5 | -0.003% | -0.084% |
| eight-literals | 1,235,645.0 | 1,235,660.5 | +0.001% | -1.528% |
| reymont-prefix-16384 | 68,717,169.0 | 68,634,948.0 | -0.120% | -0.398% |

Fresh 25,338 wall-profile and 103,556 actual-PC-profile correct calls resolve the current executable's own symbol map. Derived samples and focused assembly are retained; raw traces/stack samples stay local. Samples are not per-function retired instruction counts or fractions of automatically removable work. The native matching loop still has both its original eight-byte range guard and the wider-range test on each successful vector traversal. A separately measured scalar peel plus dedicated vector header is the next bounded target; it is not combined here.

## Qualification and explicit reused evidence

686 maintained tests pass with the same six features and -Dwarnings, retaining the same seven inherited exclusions. The widened branch test now covers 149,760 distinct alignment/start/limit/mismatch/bit combinations, including low and high bits in both U64 lanes. It executes once in the focused run and again in the maintained suite: 299,520 native value checks in total. Existing negative patterns retain byte-identical unoptimized IR.

All 5,788 focused native source controls rerun: four source programs, parent/candidate and original/decoded forms, exact status/origin/value/shape agreement, live input/output aliases, old publication on failure, cleanup and successful retry. All 16 small images form eight equal original/decoded Object/metadata pairs. Phase selection is observed; fallible and changed-stride loops select no wider probe. Final managed bytes/nodes and private payload are zero.

The focused resident executions are **reused from R230, not counted as newly executed**. Each source program and resident engine library is byte-pinned; frozen source rows and hashes are included. Every new native row must equal that frozen row, and the independent source-operation oracle still runs for every current case. The broader maintained suite is rerun, including its resident controls. Existing resident compute rejections and the exact admitted single-visit controls remain documented in the frozen data; no allowance changes. This avoids repeating the unchanged, slow resident run while preserving its provenance and every native correctness check.

There are no introduced compiler build/test/semantic failures. The first archival export rejected obsolete required-client filenames; its receipt remains, and the inventory now requires the actually executed client and resident provenance. The prior packet/write census is not claimed freshly rerun. Ordinary full-compressor timing, exact dynamic vector-probe counts, and multi-MB runs remain outside this round. The complete image is emitted once from pinned strict MBC14; small original/decoded equivalence covers the changed class. Commands/build profiles, raw structured cohorts and failures are retained. Replay `analyze.py` uses only this packet's data, not a compiler/compressor execution.

## Pins and scope

- base: `aa1a55439d15f455f8155b8a35c412f76edb8022`
- experimental_parent: `aa1a55439d15f455f8155b8a35c412f76edb8022`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `3b839eefea20206135a81630f092b9361dc2161ee4cb8ef93b881a354efb0157`
- compiler_patch_sha256: `8c94849eb112a53714cab5c0a7c790412f92fd42cc6af74d3531698abb146516`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `ce51dd24b3d3d0e487f05f21c0a4d236dd324b9c82738db198018b94f42654b2`
- worker_sha256: `9d1b9f3cc6fd510c16c2a18aac8d446caf1a9f27f38b48473c395aef91e0a7fa`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

The maintained normal compiler remains R215 at 42c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC 215/107/39/14. This standalone experiment still omits its additional exact resource/admission/accounting/publication/recovery contract. No ordinary full-compressor speed or restored-contract claim. No source change, LLVM, new branch/PR, R225 combination or optional whole-corpus run.

Original source/bytecode pins, algorithm/tables/depth 300/nice 258/search and near-EOF limits, 262,144-entry U16 tree, 1,500,513-word U32 cache, 305,257-word U64 path capacity and output cap 13 with clear/reuse remain. All compiler changes are fully archived with the minimal patch. Full parity remains open.

## Verified evidence

[Compiler source, minimal patch, both complete cohorts, hardware/heap/profile observations and test receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r231-vector-truth-e5bdadf32544.tar.gz). SHA-256 `e5bdadf325448bd566a7fdad83b40a4b726bb5a09ad201f13d81cd32e28d6942`; 7128301 bytes. All 1,416 downloaded file hashes and numerical replay verify. Native binaries/Objects, MBC, full executable metadata/descriptors and raw traces/stack samples are excluded.
