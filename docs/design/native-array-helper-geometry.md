# R243: successful helper geometry, with measured register-lifetime costs

**Keep R242 as the periodic baseline. Archive this one-file compiler experiment and preserve its small-case wins.** Periodic gets slower in both fixed cohorts: 1.624810x -> 1.648663x Roc, +1.4680% in pooled balanced block means. All11 larger-case block means are slower. Eight improves 0.491448x -> 0.485602x. Both variants remain below1x on the same11 of25 cases; no win is lost or gained.

The result is not noise inferred from code size: periodic's separate instruction and cycle counters also rise. Do not adopt it merely because the executable shrinks, all old1.05 gates pass, or the all25 descriptive average improves. The predeclared periodic-primary criterion fails. No small-case gain or adverse row is discarded.

Git baseline a2dc7c4d635780c274d7d4ce2fec6910d897f4fa contains only the scoreboard above R242; the actual compiler control is R242 at6fe370c1cedb7a0ea26b11501e242f89d6aa852c. The candidate source manifest and one-file patch pin the tested variant. Ordinary R21542c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14, remains unchanged. Preserved R237 is measured as the fourth comparator; no source/algorithm change.

## All25 complete compression timings

Two fixed24-balanced-block cohorts, all four executable permutations, equal200ms warmup, fixed level12/runtime inputs, matched live output ownership, pinned allocator and shared Rust clock. Compression, scratch creation, output transfer and cleanup stay inside; preparation/validation/logging/old-output drop stay outside. Builds/tests/profiles do not overlap timing or counter cohorts. Ratios use observation medians with a fresh shared reference denominator; changes use medians of balanced block means. Earlier published reference ratios remain historical, not substituted for current controls.

| Input | R242 / Roc | Candidate R243 / Roc | R237 / Roc | Pooled block change | Initial change | Confirmation change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 1.976190x | 1.976190x | 1.976190x | -0.7217% | +0.5206% | -1.4504% |
| one-stored | 1.976190x | 1.976190x | 1.976190x | -1.4206% | -1.5116% | -1.8751% |
| seven-stored-prefix | 2.000000x | 1.976190x | 2.000000x | -1.2994% | -0.9740% | -2.1668% |
| eight-literals | 0.491448x | 0.485602x | 0.498717x | -1.3725% | -0.7180% | -2.6588% |
| eight-overlap | 0.487586x | 0.473116x | 0.487942x | -3.4076% | -3.3037% | -3.4883% |
| 320-overlap | 0.550974x | 0.545754x | 0.565814x | -0.6030% | -0.5243% | -0.5388% |
| eight-literals-prefix | 0.493698x | 0.477545x | 0.497379x | -3.9672% | -3.9989% | -3.9614% |
| 128-biased-dynamic | 0.503152x | 0.496414x | 0.510465x | -1.0094% | -0.8686% | -1.3254% |
| 32-high-entropy-stored | 0.526721x | 0.514179x | 0.534988x | -2.0524% | -3.4960% | -0.5230% |
| 16384-distribution-change | 2.751893x | 2.824210x | 2.772895x | +2.6279% | +2.8940% | +2.9514% |
| 33027-periodic-window | 1.624810x | 1.648663x | 1.745734x | +1.4680% | +1.3040% | +1.2855% |
| random-64 | 0.560933x | 0.544895x | 0.567703x | -3.3415% | -2.8672% | -3.6436% |
| random-256 | 0.649137x | 0.630132x | 0.660654x | -3.2230% | -3.1840% | -2.9623% |
| random-1024 | 0.744782x | 0.732794x | 0.763368x | -1.3891% | -1.5129% | -1.8611% |
| repeated-1024 | 0.641187x | 0.640936x | 0.670027x | +0.4820% | +0.6823% | -0.4468% |
| random-32769 | 2.763567x | 2.831847x | 2.864417x | +2.4707% | +2.8561% | +2.0947% |
| repeated-32769 | 1.531830x | 1.560767x | 1.646315x | +1.8891% | +1.5956% | +1.9568% |
| reymont-prefix-1024 | 0.918168x | 0.906952x | 0.917426x | -1.1547% | -0.8332% | -1.5667% |
| reymont-prefix-16384 | 2.179456x | 2.182403x | 2.089900x | +0.1352% | +0.1763% | -0.1063% |
| reymont-prefix-32769 | 2.224053x | 2.233209x | 2.145140x | +0.4117% | +0.2845% | +0.4060% |
| heldout-window-32767 | 2.224811x | 2.235202x | 2.142692x | +0.4670% | +0.2634% | +0.5504% |
| heldout-window-32768 | 2.223193x | 2.234112x | 2.143226x | +0.4911% | +0.4843% | +0.5728% |
| heldout-slides-65537 | 2.299134x | 2.305577x | 2.231031x | +0.2802% | +0.1939% | +0.4267% |
| heldout-random-65537 | 3.005702x | 3.094068x | 3.180219x | +2.9399% | +2.9544% | +2.8274% |
| heldout-mixed-131073 | 2.465124x | 2.523425x | 2.532310x | +2.3650% | +2.3698% | +2.3565% |

All25 geometric block elapsed changes -0.3764%, while large11 changes +1.4080%. Twelve pooled block means are adverse. Every initial, confirmation and pooled historical point/block1.05 gate passes; the primary acceptance criterion still fails. Periodic absolute medians are 2.0549790 -> 2.0851460ms, with reference 1.2647500ms. Empty/one/seven-byte samples are close to timer resolution; their about2x ratios are not precise throughput estimates.

## Independent hardware and memory evidence

5200 observed/4800 retained complete-call CPU/allocation measurements are separate from the primary elapsed cohorts. Instructions fall on17 cases/rise on8; cycles fall18/rise7. All25 payload request/requested-byte/copy/zero/private-peak and whole-heap request/requested-byte/reallocation/usable-peak fields are exactly equal. No allocation, copy or streaming improvement is claimed. Complete RSS/physical-footprint counters remain in counter-results.json and are process/lifetime measurements rather than copied-byte counts.

| Input | Instructions before | Instructions after | Instruction change | Cycle change | After instructions / Roc |
|---|---:|---:|---:|---:|---:|
| empty | 6,360.0 | 6,344.0 | -0.2516% | -7.0569% | 1.119266x |
| one-stored | 6,503.0 | 6,490.0 | -0.1999% | -4.2012% | 1.120705x |
| seven-stored-prefix | 6,554.0 | 6,541.0 | -0.1984% | -3.6207% | 1.149359x |
| eight-literals | 1,232,277.5 | 1,226,212.0 | -0.4922% | -4.3639% | 0.900543x |
| eight-overlap | 1,211,028.5 | 1,206,812.0 | -0.3482% | -2.6645% | 0.888562x |
| 320-overlap | 1,645,593.0 | 1,645,892.5 | +0.0182% | -2.0208% | 1.083244x |
| eight-literals-prefix | 1,232,274.0 | 1,226,136.0 | -0.4981% | -6.2388% | 0.900512x |
| 128-biased-dynamic | 1,333,029.0 | 1,330,569.5 | -0.1845% | -1.6545% | 0.973193x |
| 32-high-entropy-stored | 1,418,864.5 | 1,385,339.0 | -2.3628% | -2.9593% | 1.004371x |
| 16384-distribution-change | 34,262,485.5 | 34,531,262.0 | +0.7845% | +1.6541% | 3.728174x |
| 33027-periodic-window | 39,689,290.5 | 40,227,210.0 | +1.3553% | +1.6650% | 2.074139x |
| random-64 | 1,550,172.5 | 1,506,570.5 | -2.8127% | -2.8149% | 1.075219x |
| random-256 | 2,151,121.5 | 2,065,252.0 | -3.9918% | -3.4355% | 1.370322x |
| random-1024 | 2,983,512.5 | 2,928,472.5 | -1.8448% | -3.6848% | 1.741113x |
| repeated-1024 | 2,626,635.0 | 2,640,615.0 | +0.5322% | -0.6935% | 1.344351x |
| random-32769 | 48,107,822.0 | 48,685,628.0 | +1.2011% | +2.4077% | 4.213852x |
| repeated-32769 | 39,277,153.5 | 39,825,245.0 | +1.3954% | +1.2551% | 2.068760x |
| reymont-prefix-1024 | 4,132,670.5 | 4,085,249.0 | -1.1475% | -0.7258% | 1.829056x |
| reymont-prefix-16384 | 66,409,256.5 | 66,074,300.5 | -0.5044% | -0.0233% | 2.659950x |
| reymont-prefix-32769 | 133,708,298.0 | 133,133,627.0 | -0.4298% | -0.0024% | 2.790147x |
| heldout-window-32767 | 133,464,692.5 | 132,889,315.0 | -0.4311% | -0.1639% | 2.791001x |
| heldout-window-32768 | 133,461,707.5 | 132,887,041.0 | -0.4306% | -0.0449% | 2.790839x |
| heldout-slides-65537 | 275,977,593.0 | 274,915,819.5 | -0.3847% | +0.1857% | 2.889959x |
| heldout-random-65537 | 94,789,591.0 | 95,892,890.0 | +1.1639% | +2.7988% | 4.556811x |
| heldout-mixed-131073 | 321,802,355.5 | 323,116,792.5 | +0.4085% | +2.4540% | 3.197772x |

## The generic compiler change

compact/arrays.rs is the only changed compiler file. After a successful concrete allocate call, reuse its guaranteed length0, requested capacity, row1 and column0 instead of reloading them from the six-word return slot. After successful writable backing, reuse the source's current length/rows/columns. Pointer, owner and writable capacity remain fresh helper results, so copying/growth cannot leave stale backing. Calls, allocation sites, first-change/alias decisions, failure edges and ordering remain. The private MECH_ARRAY_GEOMETRY_FACTS switch explicitly enables the experiment; disabled complete Object and metadata match R242 byte-for-byte.

The real compressor reaches these operations at output reserve and the tree/cache/path initialization (source1901-1904), plus subsequent replacement/append/clear. Keep262144U16 tree entries,1500513U32 cache words and305257U64 path capacity. Full source, tables, depth300/nice258, search/near-EOF limits, allocation behavior, output13 clear/reuse and timing endpoints are unchanged. The change supplies no canonical authority and does not restore the standalone's omitted ordinary admission/resource/accounting/publication/recovery contract.

## Why smaller code is insufficient

Root text shrinks5,630,692 ->5,509,508B (-121,184B); root stack55,152 ->51,264B (-3888B). Four existing path bodies shrink16B each; the other132 numeric/map bodies are byte-identical. All134 phase/two map selections,12 private-write intervals,170 phase owner-transfer pairs and certified wide equality groups remain.

The full root has1,407,673 ->1,377,377 static instructions including padding,523,055 ->514,251 loads and418,014 ->409,930 SP-based loads. Those are static counts, not dynamic execution weights. Periodic instead retires39,689,290.5 ->40,227,210 instructions (+1.3553%) and cycles rise1.6650%.

Actual path15296 disassembly shows changed register assignment and additional stack traffic for live scalar values in the hot loop after dynamic geometry is kept live across cold growth. phase15296-instruction-diff.txt preserves the normalized comparison. This establishes a concrete liveness cost, not a proved per-function instruction partition or complete attribution of the whole-call regression. Four body changes and whole-root register allocation both remain possible contributors. Reusing a true value can still lose speed if it lengthens that value's live range.

Next test one bounded restriction: reuse only already-constant helper geometry, so useful constants survive without carrying unknown dynamic values through cold calls. Compare the resulting combination against both R243 and retained R242 in the same balanced matrix. Keep later fixed-view phase propagation separate. No speedup or coverage is claimed for that unimplemented experiment.

## Fresh current-executable profile

33027-periodic-window: 5533 current-PC samples; _mech_compact_closed_v2 39.36%, _mech_compact_numeric_2624 31.09%, _mech_compact_numeric_15296 16.63%, _mech_compact_numeric_5538 2.78%.

eight-literals: 5502 current-PC samples; _mech_compact_closed_v2 30.73%, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 1.04%, _mech_compact_numeric_17556 1.04%, _mech_compact_numeric_16152 0.78%.

reymont-prefix-32769: 5348 current-PC samples; _mech_compact_numeric_15296 47.70%, _mech_compact_numeric_1279 23.99%, _mech_compact_closed_v2 21.73%, _mech_compact_numeric_15737 1.50%.

These are CPU-time samples mapped with this exact executable/Object, not retired-instruction partitions or removable percentages. Independent reference counters use the unchanged pinned reference executable.

## Qualification and failure disposition

703 maintained tests pass, with the same7 inherited exclusions, six native features and-Dwarnings; test root opt1 and compiler/producer opt3. All three experimental flags are enabled during the maintained suite. Seven concrete-runtime ABI tests pass (three new plus four existing):51 allocation geometry cases including real buffers,240 writable cases spanning six scalar/packed strides, row/column/2x3/3x2/empty views, shared/private copying/reuse/growth, and rejected-helper unchanged-result/source/heap followed by recovery. The standalone test permits dead_code only for unused methods in the included runtime; compiler warnings remain errors.

3400 compiled matrix controls across17 sources and four parent/candidate original/decoded forms pass, with2924 fresh resident values and476 resident errors.416 boundary controls cover selected growth/live-alias exits and unchanged unselected fanout fallback. The latter include320 fresh resident calls (236 values,72 errors,12 inherited finite default denials). All42 small original/decoded image pairs are identical. Expected values, allowances, source programs and error order remain.50 complete smokes and every measured observation verify all25 exact reference streams, independent inflate, input/output aliases, capacity and final zero nonreturned private bytes/owners.

Introduced orchestration failures are retained in setup-failures.json and exact failed receipts: a relative test path under the wrapper's different cwd; missing unchanged relative runtime dependency in two client builds; and a wrapper label that overwrote an analyzer's same-named output. Fix the paths/dependency and use a distinct receipt label, then rerun unchanged controls. A read-only status ternary and nonexistent lookup paths also failed, with no semantic effect. There are no remaining semantic test failures or compressor resource denials. The recorded cleanup frees232,933,472B of completed reproducible old test binaries, preserving source/build specs, producers, Objects, workers and receipts.

Inherited exclusions, unselected fanout and owner_flow.rs plus owner_flow/ experimental layout debt remain assigned before ordinary integration. Hosted CI is cancelled under standing request; keep draft and do not claim green CI or clean exact-head review. Full Cargo/math rebuild, the multi-MB corpus, separate ten-workload suite and fresh5788 matching controls are skipped. Complete source-vs-MBC Object equivalence is reused from R240; strict MBC14 and this round's42 original/decoded small-image pairs execute anew. No new branch/PR or LLVM change.

## Pins and replay

- object: `f63922754b8a2e30454c41ddc96b9b1fe98c67bb1fbfe9fe22c45c156898b75d`
- worker: `ed3e6511439f497ad1c61f3c40c776a94f2d115752c25cc04a11047af9f1ec85`
- compiler_patch: `826fbf71e29514ffaa61d35e6c77d0adee5e2519c4d8728e9c0119a2675aec20`
- compiler_source_manifest: `601c489bb57353a9142d86e17389cf06924e4ffcfe3ac961042349ffee90a3bc`
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`
- helper_contract_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`
- qualification_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`

The evidence includes the tested compiler closure/patch, actual runtime-helper source, build/command specifications, every25-case observation, portable fixtures, focused assembly, raw counter logs, derived PC summaries and all failed/corrected receipts. analyze-primary.py, analyze-counters.py and make-report.py regenerate five byte-identical numerical reports; verify-packet.py verifies every file hash. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/raw stack samples, credentials and private environment exports are excluded.

[Complete tested source, patch, all25 observations and replay scripts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r243-array-geometry-40a9e631f27c.tar.gz). SHA-256`40a9e631f27cad373465a0884c3aba3e524ccc19dcd41f15dbb40c989470fb5d`;5328777B. All2,149 file hashes and five numerical report replays verify after public retrieval.
