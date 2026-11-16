# R244: constant-only helper geometry, with retained-baseline comparison

**Keep R242 as the periodic baseline. Archive this constant-only component and preserve all measured tradeoffs.** It removes most of R243's periodic penalty, but does not establish a target win over retained R242: 1.617537x -> 1.625343x Roc, +0.4826% in pooled block means. The first cohort is adverse and confirmation is nearly tied. Eight is0.493434x. All three variants beat Roc on the same11 of25 cases; no below1x case is gained or lost.

Git parent e21d1415fe03138b3fbb0f1f5ee7ddb9c0b6b8dd records R243. This candidate is the exact R243 source plus one-file constant-only policy; combined-compiler.patch also records the full difference from retained R2426fe370c1cedb7a0ea26b11501e242f89d6aa852c. Ordinary R21542c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14, remains unchanged.

## All25 matched complete compression timings

Two fixed24-balanced-block cohorts, all four executable permutations, equal200ms warmup, level12/runtime inputs, matched live output ownership, pinned allocator and shared Rust clock. Compression, scratch construction, output transfer and cleanup remain timed; preparation/validation/logging/previous-output drop stay outside. No builds/tests/profiles overlap primary or counter cohorts. Before=archived R243, after=R244, retained=R242, reference=pinned Roc. Ratios use pooled observation medians against the common fresh reference; changes use medians of balanced block means. Do not mistake recovery relative to the slower R243 component for improvement over the retained baseline.

| Input | Retained R242 / Roc | R243 / Roc | Candidate R244 / Roc | R244 block vs R242 | Initial vs R242 | Confirmation vs R242 |
|---|---:|---:|---:|---:|---:|---:|
| empty | 1.976190x | 1.976190x | 1.976190x | -1.0784% | -0.6391% | -1.3377% |
| one-stored | 1.976190x | 1.976190x | 1.976190x | -0.4084% | +3.0233% | -2.2688% |
| seven-stored-prefix | 2.000000x | 2.000000x | 2.000000x | -0.7628% | -0.1781% | -1.4645% |
| eight-literals | 0.486901x | 0.495208x | 0.493434x | +0.2959% | +2.3575% | -1.5967% |
| eight-overlap | 0.488129x | 0.493422x | 0.486842x | -0.2318% | +2.2555% | -2.6048% |
| 320-overlap | 0.557249x | 0.562240x | 0.552951x | -0.6848% | +2.0640% | -3.4925% |
| eight-literals-prefix | 0.495990x | 0.499856x | 0.494271x | +0.7518% | +3.1498% | -2.7751% |
| 128-biased-dynamic | 0.507794x | 0.504919x | 0.505855x | -0.3521% | +0.9395% | -3.0729% |
| 32-high-entropy-stored | 0.527496x | 0.529853x | 0.528493x | -0.9779% | +0.0462% | -1.6370% |
| 16384-distribution-change | 2.742602x | 2.834176x | 2.740287x | -0.0844% | -0.1922% | +0.0418% |
| 33027-periodic-window | 1.617537x | 1.654599x | 1.625343x | +0.4826% | +0.6519% | -0.0061% |
| random-64 | 0.559874x | 0.554677x | 0.559730x | -0.0734% | +3.3641% | -3.3841% |
| random-256 | 0.658935x | 0.652929x | 0.655863x | -1.1164% | +2.0147% | -3.5078% |
| random-1024 | 0.747988x | 0.747581x | 0.742712x | -1.0504% | +0.6232% | -3.8927% |
| repeated-1024 | 0.650560x | 0.658024x | 0.646894x | -0.1696% | +1.1404% | -1.5406% |
| random-32769 | 2.746343x | 2.838488x | 2.746032x | -0.0113% | -0.1565% | +0.4864% |
| repeated-32769 | 1.523366x | 1.562943x | 1.528157x | +0.3145% | +0.4570% | +0.6062% |
| reymont-prefix-1024 | 0.925885x | 0.924520x | 0.925885x | +0.3271% | +1.5095% | -1.5384% |
| reymont-prefix-16384 | 2.181973x | 2.188586x | 2.182951x | +0.0448% | -0.1694% | +0.4100% |
| reymont-prefix-32769 | 2.222992x | 2.229990x | 2.223049x | +0.0026% | -0.1090% | +0.3346% |
| heldout-window-32767 | 2.226325x | 2.238016x | 2.229299x | +0.1336% | +0.2860% | +0.0851% |
| heldout-window-32768 | 2.225359x | 2.233924x | 2.226467x | +0.0498% | +0.1239% | +0.0377% |
| heldout-slides-65537 | 2.299736x | 2.305275x | 2.296964x | -0.1206% | -0.3551% | +0.0515% |
| heldout-random-65537 | 3.016067x | 3.103847x | 2.998607x | -0.5789% | -0.6652% | -0.4111% |
| heldout-mixed-131073 | 2.464219x | 2.523235x | 2.458190x | -0.2447% | -0.2825% | -0.2449% |

Nine pooled block means are slower than R242. All25 descriptive geometric block elapsed changes-0.2230%; large11 changes-0.0014%, effectively tied. Pooled historical1.05 point/block gates pass25/25 against R242 and R243; all component-relative initial/confirmation gates also pass. The prospective primary criterion against R242 still fails. Eight's initial adverse and confirmation favorable measurements remain visible. Timer-limited empty/one/seven-byte calls retain their near-resolution limitation. Older reference denominators remain historical, not substituted here.

## Independent whole-call counters and memory

5200 observed/4800 retained complete-call CPU/allocation measurements are separate from primary elapsed. R242, R243 and R244 have exactly equal payload request/requested-byte/copy/zero/private-peak and whole-heap request/requested-byte/reallocation/usable-peak fields on every case. No allocation, copying or streaming reduction is claimed. RSS/physical footprints in counter-results.json are process/lifetime measurements. Whole-call instruction/cycle counters are not partitioned by CPU-time sample shares.

| Input | R242 instructions | R244 instructions | Instruction change vs R242 | Cycle change vs R242 |
|---|---:|---:|---:|---:|
| empty | 6,360.0 | 6,360.0 | +0.0000% | +2.2793% |
| one-stored | 6,503.0 | 6,503.0 | +0.0000% | +0.5908% |
| seven-stored-prefix | 6,554.0 | 6,554.0 | +0.0000% | +0.9942% |
| eight-literals | 1,232,276.0 | 1,233,778.0 | +0.1219% | +0.0696% |
| eight-overlap | 1,210,953.5 | 1,212,683.5 | +0.1429% | +1.4764% |
| 320-overlap | 1,645,434.0 | 1,648,998.5 | +0.2166% | +1.6698% |
| eight-literals-prefix | 1,232,080.0 | 1,233,685.5 | +0.1303% | +1.6018% |
| 128-biased-dynamic | 1,332,951.5 | 1,333,466.0 | +0.0386% | +1.3601% |
| 32-high-entropy-stored | 1,418,776.5 | 1,420,241.0 | +0.1032% | +0.9271% |
| 16384-distribution-change | 34,262,530.0 | 34,194,347.5 | -0.1990% | -0.2223% |
| 33027-periodic-window | 39,686,297.5 | 39,883,565.0 | +0.4971% | +0.3563% |
| random-64 | 1,550,040.0 | 1,550,848.0 | +0.0521% | -0.1913% |
| random-256 | 2,151,014.0 | 2,150,106.0 | -0.0422% | +2.8969% |
| random-1024 | 2,983,388.5 | 2,979,014.0 | -0.1466% | +1.4133% |
| repeated-1024 | 2,626,546.0 | 2,634,243.0 | +0.2930% | +2.6296% |
| random-32769 | 48,107,809.5 | 47,958,210.5 | -0.3110% | +0.3344% |
| repeated-32769 | 39,277,151.0 | 39,473,304.5 | +0.4994% | -0.2843% |
| reymont-prefix-1024 | 4,132,580.5 | 4,129,100.5 | -0.0842% | +2.3578% |
| reymont-prefix-16384 | 66,409,370.0 | 66,324,922.5 | -0.1272% | -0.1956% |
| reymont-prefix-32769 | 133,711,886.0 | 133,530,011.0 | -0.1360% | -0.0458% |
| heldout-window-32767 | 133,462,085.0 | 133,280,501.5 | -0.1361% | +0.0333% |
| heldout-window-32768 | 133,465,671.5 | 133,283,307.5 | -0.1366% | +0.0425% |
| heldout-slides-65537 | 275,980,155.0 | 275,608,723.5 | -0.1346% | +0.0050% |
| heldout-random-65537 | 94,791,116.5 | 94,485,726.5 | -0.3222% | -0.1653% |
| heldout-mixed-131073 | 321,805,805.0 | 321,509,687.5 | -0.0920% | -0.4643% |

Periodic instructions are39,686,297.5 ->39,883,565.0, +0.4971%; cycles+0.3563%. Relative to R243, instructions recover from40,227,136.0, but remain above R242. Across25, instructions fall12/rise10; cycles fall7/rise18. Contrary random256 elapsed/cycle directions remain separate.

## Generic change and machine evidence

compact/arrays.rs restricts R243 helper-value substitution to current SSA values that resolve to an integer literal. Unknown parameters, expressions and joins continue to load actual returned geometry. This is a compiler cost restriction, not source specialization by compressor/function/site/input size. Successful-call facts remain exact; helpers, source checks, error precedence, backing/capacity/owner refresh, alias decisions, allocation sites and cleanup stay. MECH_ARRAY_GEOMETRY_FACTS=1 plus MECH_ARRAY_GEOMETRY_CONSTANTS_ONLY=1 explicitly enable the private experiment. Disabling the second switch produces an entire Object and metadata byte-identical to R243.

All136 numeric/map bodies are byte-identical to R242, restoring the four changed R243 path bodies. Thus no changed inner matching/path kernel is claimed relative to R242. Root text5,630,692 ->5,629,248B (-1444B) and stack55,152 ->55,136B (-16B). Those static savings do not establish a speedup. Relative to R243, the code/stack shrink is mostly undone. The current root register allocation/layout still has a measurable whole-call cost; no exact source-site or per-function instruction attribution is inferred.

All134 numeric/two map selections,12 private-write intervals,170 phase ownership transfers and existing equality groups remain. No new source operation is lowered. The pinned full source, algorithm/tables/depth300/nice258/search/near-EOF rules,262144U16 tree,1500513U32 cache,305257U64 path capacity, output13 clear/reuse, allocation behavior, aliases, allocator and timing endpoints remain unchanged. Ordinary exact admission/resource/accounting/canonical publication/recovery guarantees remain omitted by the standalone path and are not counted as optimized away.

## Actual current-executable profiles

33027-periodic-window: 5557 current-PC samples; _mech_compact_closed_v2 41.16%, _mech_compact_numeric_2624 33.02%, _mech_compact_numeric_15296 14.14%, _mech_compact_numeric_12868 2.43%.

eight-literals: 5423 current-PC samples; _mech_compact_closed_v2 30.33%, _mech_compact_numeric_17556 1.07%, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 0.96%, _mech_compact_numeric_16010 0.66%.

reymont-prefix-32769: 5351 current-PC samples; _mech_compact_numeric_15296 48.12%, _mech_compact_numeric_1279 23.57%, _mech_compact_closed_v2 21.75%, _mech_compact_numeric_15737 1.50%.

These use each current executable's own symbols/Object; samples are CPU-time evidence, not retired-instruction partitions. The immutable reference's actual matching assembly uses the trailing-zero idiom already present in Mech source/lowering; do not propose a missing trailing-zero primitive as a new target.

The next bounded target is retaining useful constant facts after complete SSA/loop analysis without holding unknown dynamic metadata across cold calls. Current helper/phase return loads obscure exact view identities, and fixed-length tree/cache facts are lost across repeated search/skip visits. Establish a small fixed-buffer loop witness first, with changed lengths/shapes, growth, aliases and original/decoded failure controls; only then add proof reuse across certified identity edges. No implementation, eligible count or speedup is claimed for that next pass.

## Qualification and failures

703 maintained tests pass with the same7 inherited exclusions, six native features and-Dwarnings (compiler opt3/test root opt1). All four experimental flags are explicit in the command receipt. Seven unchanged-runtime ABI tests pass, including51 allocation/240 writable geometry cases with real buffers, packed strides,2D/empty views, alias copying/reuse/growth, rejection and recovery. Only unused included-runtime methods permit dead_code; compiler warnings remain errors.

3400 native matrix controls/17 sources/four parent-candidate original-decoded forms pass against fresh resident values/errors;416 boundary controls retain selected growth/live-alias paths and inherited unselected fanout fallback. All42 small original/decoded Object/metadata pairs match.50 complete smokes and every primary/counter/profile observation match exact reference bytes, independently inflate, preserve aliases/capacity and end with zero nonreturned private bytes/owners. No source expectation, allowance or work limit increases.

No introduced build, semantic test or resource-denial failure occurred. A read-only inspection guessed an absent census filename before finding it with the file inventory; it changed no program or result. Inherited fanout selection, seven test exclusions and experimental owner_flow.rs plus owner_flow/ layout debt remain explicitly assigned. Hosted CI is cancelled by standing request; keep draft and make no green-CI or clean exact-head review claim. Multi-MB inputs, separate ten-workload suite and full Cargo/math rebuild are skipped. Full source-vs-MBC Object equivalence is reused from R240; strict MBC14 and42 small original/decoded image pairs execute anew. No new branch/PR or LLVM change.

## Pins and replay

- object: `b22fe417f374d5ffdf09f1449fe2151eb5daa076c0dc321e93adfeeb437da120`
- worker: `8ed03ebc2a7693e3ebf2f755dea22791f11cdf6a0480003499ef010f3413499f`
- compiler_patch: `357a1d39174eab9d8889337aead960ce264479df267f7675f8bde042fc24be84`
- compiler_source_manifest: `5b3b182e45fc06b55dd390279c0dc9fb8967e927689e2fb7e8906058d9358564`
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`
- helper_contract_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`
- qualification_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`
- combined_compiler_patch: `a4e2d7b03c3c0484fcd906d71c90b4e69bccdbc5e67c252d58c7247d78f02391`

The packet includes the tested compiler closure, incremental/combined patches, exact commands, every25-case observation, portable fixtures, focused assembly, counters/profiles and qualification receipts. Replaying the analyzers and make-report.py reproduces five byte-identical reports; verify-packet.py checks every file hash. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/raw stack samples, credentials and private environment exports are excluded.

[Complete compiler source, incremental/combined patches and all25 observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r244-constant-geometry-4be083119d1b.tar.gz). SHA-256`4be083119d1bfc0dc856c386ac8d3f8c3c4a1da8f4a603bdafcad5fdd9433e2f`;5329692B. All2,129 file hashes and five numerical report replays verify after public retrieval.
