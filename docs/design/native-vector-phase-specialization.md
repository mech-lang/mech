# R245: guarded vector phase specialization

Retain the compiler experiment. Periodic33 improves in both balanced cohorts; all25 existing point/block1.05 gates pass, and eight literals stays below the user1x ceiling. Eleven cases remain faster than Roc. Parity is incomplete; eleven pooled block means are adverse and are listed in full.

Git parent `cd826b5e85dfb9ce9ec4c8e51ab8a94181838ce2`; retained control R242 `6fe370c1cedb7a0ea26b11501e242f89d6aa852c`. The measured compiler is the six-file compiler.patch plus the hashed complete source closure. Ordinary R215 `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5` and IR/work/ABI/MBC215/107/39/14 remain unchanged. The final published head is recorded in the PR and publication receipt.

The patch applies to the archived R242 experimental compiler source closure, not directly to the maintained IR215 tree. The complete tested compiler source and patch are published in the evidence packet; #973 tracks their provenance and measurements. This round does not integrate the compact compiler into the ordinary entry.

## All25 matched complete compression timings

Two fixed24-balanced-block cohorts use all four executable permutations, equal200ms warmup, fixed level12/runtime inputs, matched live output ownership, pinned allocator and the same Rust clock. Full compression, scratch construction, output transfer and cleanup are timed. Preparation/validation/logging/previous-output drop stay outside. No builds/tests/profiles overlap primary or counter cohorts. Before=R242, after=R245, historical component=R237, reference=pinned Roc. Ratios use pooled observation medians; changes use medians of balanced block means. New reference denominators do not rewrite historical results.

| Case | Bytes | R242 / Roc | R245 / Roc | Block change | Initial | Confirmation |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -3.9764% | -4.0799% | -4.4165% |
| one-stored | 1 | 1.976190x | 1.976190x | -3.1882% | -3.3122% | -3.1538% |
| seven-stored-prefix | 7 | 2.000000x | 1.976190x | -5.1893% | -4.7741% | -5.5860% |
| eight-literals | 8 | 0.504641x | 0.495073x | -1.7804% | +1.3976% | -4.1671% |
| eight-overlap | 8 | 0.490414x | 0.493131x | +0.8276% | +0.8960% | +0.2018% |
| 320-overlap | 320 | 0.559828x | 0.557606x | -0.4831% | +0.0645% | -0.3110% |
| eight-literals-prefix | 8 | 0.513316x | 0.506138x | -0.7094% | +0.9506% | -1.3762% |
| 128-biased-dynamic | 128 | 0.516865x | 0.518163x | +1.0346% | +0.5450% | +0.6095% |
| 32-high-entropy-stored | 32 | 0.546704x | 0.549802x | +0.5212% | +1.0319% | +0.1168% |
| 16384-distribution-change | 16384 | 2.738145x | 2.734700x | -0.1258% | -0.1831% | -0.0263% |
| 33027-periodic-window | 33027 | 1.620851x | 1.582628x | -2.3582% | -2.2240% | -2.5527% |
| random-64 | 64 | 0.572755x | 0.569464x | +1.4666% | -1.6210% | -0.4987% |
| random-256 | 256 | 0.664982x | 0.671579x | +1.3265% | +1.5183% | +0.2163% |
| random-1024 | 1024 | 0.770629x | 0.763209x | -0.2430% | +0.0565% | -1.7006% |
| repeated-1024 | 1024 | 0.659273x | 0.648246x | -1.9465% | -1.4531% | -2.2859% |
| random-32769 | 32769 | 2.757593x | 2.729007x | -1.0366% | -0.9177% | -1.1399% |
| repeated-32769 | 32769 | 1.523382x | 1.499992x | -1.5354% | -1.4484% | -1.4834% |
| reymont-prefix-1024 | 1024 | 0.928711x | 0.923515x | -0.4684% | +0.6074% | -0.9674% |
| reymont-prefix-16384 | 16384 | 2.185549x | 2.199768x | +0.6506% | +0.5040% | +0.6351% |
| reymont-prefix-32769 | 32769 | 2.230560x | 2.240494x | +0.4454% | +0.2591% | +0.6157% |
| heldout-window-32767 | 32767 | 2.227567x | 2.239588x | +0.5396% | +0.4317% | +0.6187% |
| heldout-window-32768 | 32768 | 2.225616x | 2.237501x | +0.5340% | +0.7196% | +0.3527% |
| heldout-slides-65537 | 65537 | 2.301132x | 2.304383x | +0.1413% | +0.0337% | +0.1358% |
| heldout-random-65537 | 65537 | 3.019573x | 2.983332x | -1.2002% | -1.2425% | -1.1746% |
| heldout-mixed-131073 | 131073 | 2.469678x | 2.502989x | +1.3488% | +1.1447% | +1.4916% |

Periodic block elapsed improves 2.358217%; its instruction/cycle reductions are 0.903877%/2.212633%. All25 descriptive geometric block elapsed improves 0.630614%; large11 improves 0.241935%. These are equal-case descriptive means, not a workload-weighted throughput promise. The eight-literal initial adverse pair remains; pooled elapsed falls while its instructions/cycles rise slightly. Tiny0/1/7-byte observations are near clock resolution and do not establish a precise2x implementation gap.

### Absolute times from the same pooled observations

| Case | Roc ms | R242 ms | R245 ms | Historical R237 ms |
|---|---:|---:|---:|---:|
| empty | 0.000042 | 0.000083 | 0.000083 | 0.000083 |
| one-stored | 0.000042 | 0.000083 | 0.000083 | 0.000083 |
| seven-stored-prefix | 0.000042 | 0.000084 | 0.000083 | 0.000084 |
| eight-literals | 0.291791 | 0.147250 | 0.144458 | 0.147125 |
| eight-overlap | 0.291166 | 0.142792 | 0.143583 | 0.145084 |
| 320-overlap | 0.300167 | 0.168042 | 0.167375 | 0.169958 |
| eight-literals-prefix | 0.290187 | 0.148958 | 0.146875 | 0.148417 |
| 128-biased-dynamic | 0.289083 | 0.149417 | 0.149792 | 0.149229 |
| 32-high-entropy-stored | 0.289042 | 0.158020 | 0.158916 | 0.158125 |
| 16384-distribution-change | 0.840563 | 2.301584 | 2.298688 | 2.331125 |
| 33027-periodic-window | 1.263958 | 2.048688 | 2.000375 | 2.204500 |
| random-64 | 0.291209 | 0.166792 | 0.165833 | 0.167646 |
| random-256 | 0.296875 | 0.197417 | 0.199375 | 0.199541 |
| random-1024 | 0.306000 | 0.235812 | 0.233542 | 0.237792 |
| repeated-1024 | 0.326875 | 0.215500 | 0.211895 | 0.222333 |
| random-32769 | 1.070604 | 2.952291 | 2.921687 | 3.060208 |
| repeated-32769 | 1.348521 | 2.054312 | 2.022771 | 2.221563 |
| reymont-prefix-1024 | 0.336667 | 0.312667 | 0.310917 | 0.310458 |
| reymont-prefix-16384 | 2.172833 | 4.748833 | 4.779729 | 4.553771 |
| reymont-prefix-32769 | 4.372416 | 9.752937 | 9.796375 | 9.403583 |
| heldout-window-32767 | 4.363917 | 9.720917 | 9.773375 | 9.370896 |
| heldout-window-32768 | 4.368229 | 9.722000 | 9.773917 | 9.384500 |
| heldout-slides-65537 | 8.855479 | 20.377626 | 20.406416 | 19.755416 |
| heldout-random-65537 | 1.943563 | 5.868729 | 5.798292 | 6.198605 |
| heldout-mixed-131073 | 8.398000 | 20.740354 | 21.020103 | 21.283771 |

## Mechanism and architectural scope

The pinned reference matching assembly uses direct one-dimensional list addressing. Its rbit/clz mismatch extension already exists in Mech; it is not a new opportunity. The retained matching2624 still has56 static udiv/msub/madd matrix projection sequences. Those divide blocks are skipped on vectors; the target is repeated shape dispatch/addressing and code/register pressure, not56 executed divides on each call.

A private opaque VectorContract is constructed only by the compact phase caller. The caller checks the actual current rows/columns before selecting an additional vector body: fixed/read-only views require one row or one column, append-capable views require one row. Failed geometry selects the original general native body; failed ownership/capacity/source-window gates preserve the existing root fallback. Both bodies use the same checked numeric emitter. Source bounds still precede addressing; indexed read/store use the same physical element helper, and element stride stays independent of geometry. No function name, compressor field, input size or compression-path selector participates.

The contract preserves geometry through supported indexed writes/copies/child reductions. Existing append capacity/growth refreshes the backing pointer; no pointer or owner is cached across a growth operation. Current arguments, lexical error order, first change, aliases, success-only output and cleanup/retry behavior remain. Existing source algorithms, tables, depth300/nice258/nearEOF limits,262144U16 tree,1500513U32 cache,305257U64 path capacity, allocation sites and timing boundaries are unchanged. Eight-literal raw output remains [75,76,74,78,73,77,75,207,0,0], capacity13 with clear/reuse.

MECH_PHASE_VECTOR_FAST=1 explicitly enables the experiment, alongside MECH_PRIVATE_INDEXED_CHAINS=1 and MECH_PHASE_OWNER_TRANSFER=1. Disabled full Object and metadata are byte-identical to R242. All136 original numeric/map bodies and their original metadata records remain unchanged; seven additional vector bodies are selected by current-view guards. No new source-operation or phase-selection class is claimed.

The ordinary admitted compiler remains R215. The standalone still omits its additional exact admission/resource/accounting/canonical publication/recovery contract; this experiment does not restore it. Scalar Bool/U64/I64/F64 direct checked emission and ordinary finite admission remain maintained.

## Code and frame costs

Matching2624 shrinks10120->8228B and354->242 static conditional branches; its vector body has zero matrix divides versus56 in the general body. Root code grows 5630692->5630944B; root frame falls 55152->55136B. Search1279/3835 stack grows384->400B; skip2624 stays304B. The static counts are not a dynamic instruction partition.

Object text 5749220->5860960B; linked worker text 6151092->6178864B; complete worker file 6606776->6640168B. The producer currently emits 134 candidate vector bodies, selecting only7; 75764B of unused candidate code remains in the Object and is largely removed by linking. Avoiding duplicate compilation is a follow-up build-cost opportunity, not a measured runtime gain. Disabled/enabled generation receipts are descriptive, not matched build-time benchmarks.

## Independent full-call instruction and memory evidence

Separate cohorts contain 5200 observed/4800 retained calls. Instructions fall on18 cases and rise on7; cycles fall on17 and rise on8. All25 payload/heap request, requested-byte, copied/zeroed-byte and live-peak fields are exactly equal before/after. Source allocation behavior has not been traded for speed. Process RSS/physical/lifetime peaks include loaded code, metadata, harness and allocator retention; they are not private payload or memcpy volume. Candidate process footprint is higher and is retained explicitly in counter-results.json.

| Case | R242 instructions | R245 instructions | Roc instructions | R245/Roc instructions | Instruction change | Cycle change | R245 heap requests | Private payload peak B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| empty | 6,360.0 | 6,308.0 | 5,668.0 | 1.112915x | -0.8176% | -1.3925% | 4 | 5 |
| one-stored | 6,503.0 | 6,451.0 | 5,791.0 | 1.113970x | -0.7996% | -0.5387% | 4 | 6 |
| seven-stored-prefix | 6,554.0 | 6,502.0 | 5,691.0 | 1.142506x | -0.7934% | -4.9980% | 4 | 14 |
| eight-literals | 1,232,276.0 | 1,235,828.0 | 1,361,848.0 | 0.907464x | +0.2882% | +0.0450% | 230 | 8982372 |
| eight-overlap | 1,211,030.5 | 1,214,532.0 | 1,358,272.0 | 0.894174x | +0.2891% | +0.1439% | 230 | 8982372 |
| 320-overlap | 1,645,468.0 | 1,643,575.0 | 1,519,720.0 | 1.081499x | -0.1150% | +0.4262% | 230 | 8982372 |
| eight-literals-prefix | 1,232,156.5 | 1,235,807.0 | 1,361,806.0 | 0.907477x | +0.2963% | -0.3224% | 230 | 8982372 |
| 128-biased-dynamic | 1,332,919.5 | 1,334,606.0 | 1,367,428.0 | 0.975997x | +0.1265% | +0.5285% | 210 | 8982372 |
| 32-high-entropy-stored | 1,418,775.5 | 1,422,723.5 | 1,379,569.5 | 1.031281x | +0.2783% | +0.2766% | 226 | 8982372 |
| 16384-distribution-change | 34,262,407.0 | 33,991,519.5 | 9,262,340.0 | 3.669863x | -0.7906% | +0.4874% | 522 | 8998776 |
| 33027-periodic-window | 39,686,273.0 | 39,327,558.0 | 19,394,800.0 | 2.027737x | -0.9039% | -2.2126% | 208 | 9500860 |
| random-64 | 1,550,060.5 | 1,553,283.0 | 1,401,484.5 | 1.108313x | +0.2079% | -1.4311% | 226 | 8982372 |
| random-256 | 2,151,010.5 | 2,151,751.5 | 1,507,510.0 | 1.427355x | +0.0344% | -0.6286% | 226 | 8982372 |
| random-1024 | 2,983,381.0 | 2,966,149.0 | 1,682,335.0 | 1.763114x | -0.5776% | -1.6157% | 208 | 8982372 |
| repeated-1024 | 2,626,545.0 | 2,610,413.0 | 1,964,714.0 | 1.328648x | -0.6142% | -0.5083% | 232 | 8982372 |
| random-32769 | 48,107,788.5 | 47,304,637.5 | 11,553,920.0 | 4.094250x | -1.6695% | -0.6542% | 276 | 9500860 |
| repeated-32769 | 39,277,155.5 | 38,920,909.0 | 19,250,995.0 | 2.021761x | -0.9070% | -2.1563% | 208 | 9500860 |
| reymont-prefix-1024 | 4,132,688.0 | 4,102,904.5 | 2,233,911.5 | 1.836646x | -0.7207% | +0.3999% | 266 | 8982372 |
| reymont-prefix-16384 | 66,409,338.5 | 65,038,563.5 | 24,840,537.0 | 2.618243x | -2.0641% | -1.0091% | 650 | 8998776 |
| reymont-prefix-32769 | 133,708,397.5 | 130,010,885.0 | 47,714,304.0 | 2.724778x | -2.7654% | -1.2003% | 608 | 9533536 |
| heldout-window-32767 | 133,479,065.5 | 129,782,794.0 | 47,623,566.5 | 2.725180x | -2.7692% | -0.9161% | 606 | 9015174 |
| heldout-window-32768 | 133,481,152.5 | 129,780,462.5 | 47,623,973.0 | 2.725108x | -2.7724% | -1.1110% | 606 | 9015177 |
| heldout-slides-65537 | 276,018,437.0 | 266,799,349.5 | 95,142,947.0 | 2.804195x | -3.3400% | -1.3142% | 864 | 9566339 |
| heldout-random-65537 | 94,802,017.0 | 93,603,177.0 | 21,047,342.5 | 4.447268x | -1.2646% | -0.2880% | 256 | 9500860 |
| heldout-mixed-131073 | 321,839,549.5 | 317,720,100.0 | 101,062,036.0 | 3.143813x | -1.2800% | +0.7639% | 950 | 9631947 |

## Current executable profiles

- 33027-periodic-window: _mech_compact_closed_v2 2344/5696, _mech_compact_numeric_2624_vector 1675/5696, _mech_compact_numeric_15296 1018/5696, _mech_compact_numeric_12868_vector 184/5696.
- eight-literals: _mech_compact_closed_v2 1606/5301, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 72/5301, _mech_compact_numeric_17556 58/5301, _mech_compact_numeric_16010 41/5301.
- reymont-prefix-32769: _mech_compact_numeric_15296 2654/5430, _mech_compact_closed_v2 1241/5430, _mech_compact_numeric_1279_vector 1189/5430, _mech_compact_numeric_15737 83/5430.

These are fresh Timer Fired/Running current-PC samples from the exact measured executable, including harness/validation, mapped through its own symbols/Object. They are CPU-time samples, not retired-instruction attribution. The periodic vector matching symbol actually executes. Natural32 now spends most sampled time in path15296; periodic still spends substantial time in the generated root. Focused current assembly windows are retained.

## Qualification and commands

703 maintained native tests pass with the same seven inherited exclusions. Compileropt3/test-rootopt1, six native features, -Dwarnings, nightly-2026-03-03-aarch64-apple-darwin and all three experimental flags are explicit in receipts. Seven unchanged-runtime ABI tests were freshly rerun using the retained R244 ABI client against the identical runtime; this is not a new ABI implementation. They include51 allocation/240 writable geometry cases, scalar/packed strides, row/column/2D/empty views, aliases, growth, rejection and retry.

3400 matrix controls/17 sources/four parent-candidate original-decoded forms agree against fresh resident values/errors.416 boundary controls retain selected growth/live-alias paths and inherited unselected fanout fallback. The frozen fixed/dynamic seed witnesses add104 parent and104 candidate controls; they already have static unit geometry and correctly need no new vector dispatch. All50 small original/decoded Object/metadata pairs match. The source-shaped dispatch trace records1372 general native visits,100 vector visits and52 same-invocation shared fallback->vector cases, with every current geometry branch checked. Publication, source aliases, current arguments, overflow/check/load precedence and recovery are exercised.

50 complete smokes and every primary/counter/profile observation match exact reference bytes, independently inflate, preserve input/output aliases/capacity and end with zero nonreturned private bytes/owners. Full emission starts from the pinned decoded revision14 bytecode; a second full-compressor re-encoding/rebuild was not repeated. Original/decoded reconstruction is freshly covered by the50 small pairs. No expected values or allowances increased.

From the evidence directory, exact command expansions are retained in the named JSON receipts:

```text
python3 run-spec.py candidate-gpu candidate-producer candidate-tests
python3 qualify-parent.py
python3 qualify-candidate.py
python3 qualify-writes.py
python3 qualify-exits.py
python3 run-spec.py mech-worker after-cpu after-memory
python3 run.py all25-immediate-smokes python3 smoke-all.py
env MECH_PRIVATE_INDEXED_CHAINS=1 MECH_PHASE_OWNER_TRANSFER=1 MECH_PHASE_VECTOR_FAST=1 candidate-tests --test-threads=1 [the seven exact --skip entries in maintained-suite-command.json]
python3 run-measurements.py
python3 replay.py
```

## Failure disposition and skipped coverage

Introduced setup failures were fixed before measurement: a private module path/unused local failed -Dwarnings; opaque-type re-export and removal fixed both. Initial diagnostic flat facts had no ordinary addressing consumer, and a later observer incorrectly required vector dispatch even for an already-unit-shaped identical body. The shared address wiring was added; dispatch is required only when the compiler actually emitted a different selected body. The frozen source/oracle were not changed. Static opcode counting was corrected to accept tab-separated objdump mnemonics. Initial failing receipts are preserved. A patch check initially used the maintained IR215 tree instead of its declared R242 experimental source base; checking the exact R242 source closure passes. These were setup/coverage failures, not observed wrong output/error results.

Two guessed read paths, unmatched shell globs and a broad restricted-directory inventory failed during inspection; exact files were located and read. No outstanding introduced compiler, semantic or benchmark resource failure remains. Twelve inherited resident default-cap denials in the unselected fanout controls remain explicit, without raising limits. The same seven inherited maintained exclusions, unsupported native fanout class and owner_flow file/directory layout debt remain. Full Cargo/math-library rebuilds, fresh5788 matching-component controls, multi-MB inputs and the separate ten-workload suite were not repeated. Hosted CI is cancelled under standing instruction and the existing PR remains draft pending green CI plus clean exact-head review; no hosted pass is claimed.

## Pins and reproduction

- object: `f74cd039e908a7bb502a9c4a703c53e3cc0103cc7ae3d6af7f91819a5b62d4c7`.
- worker: `58fa08ee1160db9febd35298f854874572741ba2b6f311b01eb1143d812e44ba`.
- compiler_patch: `cb300d85d35c95bc4a0e054677409ca118c6cee29203fabfd316716bbc383e3b`.
- compiler_source_manifest: `995dea7a6ec812ac69e9fdde0eeb9c6314f8f94932c87221eb71113c064410bf`.
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- qualification_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`.
- reference_assembly: `acc956027028d0d4a6deb4baf69635d2d1b84c20ef45d9d508e427b4d50442b1`.

The packet contains the tested source closure/patch, exact commands, all25 observations, portable fixtures, aggregate counter/profile reports and focused assembly. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/raw stack samples and private environment exports are excluded. replay.py recomputes primary/counter/decision/report outputs without running a compiler or compressor. verify-packet.py verifies every archived file hash before that replay.

Next inspect the dominant current path-scan/root PCs and pick one general proof or transport change using a minimal source witness. Preserve this baseline and all11 wins. The deferred whole-root constant-copy solver has not been implemented; this round chose the more direct, verified matching geometry opportunity.

## Published evidence

[Complete tested compiler source, six-file patch, every timing/counter observation, validation and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r245-vector-phase-e4e132a4499e.tar.gz). SHA256 `e4e132a4499e4080e7cbcc1fc00035961f298744403b1bc28c3ce8f9f4a361f1`; 5577780B. The public download verifies all2403 file hashes and five byte-identical numerical reports. The patch also reconstructs all815 tested source files from its declared R242 source base; the maintained R215 tree is unchanged.
