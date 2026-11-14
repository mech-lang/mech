# R247: phase-only threading isolates a natural-text gain and a periodic penalty

Archive this candidate and retain R245. Natural32 improves in both cohorts, but periodic still regresses in both. All25 historical1.05 timing gates pass in both cohorts and the eight-byte result remains below1x. These gates do not override the predeclared periodic-primary criterion. Preserve the useful natural-text component; it has not been discarded.

Git parent `a08088fa87defbcf47f6efcb29d4cbdf459cedcb`; performance control is retained R245 `085dce0ee59b2c798a9ed542c62273759414f0cc`. Ordinary R215 `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14, remains unchanged. The tested experimental compiler closure is external: one policy file changes over archived R246, or nine files over archived R245. Those patches apply to their declared closures, not directly to the maintained tree.

## Bounded change and machine identity

MECH_THREAD_PHASES_ONLY=1 suppresses the root invocation of the existing R237/R246 threader while MECH_THREAD_LOOP_EDGES=1 still enables the identical pass in outlined numeric phases. The three existing R245 private flags remain. There is no new proof algorithm, source operation, input-specific selection or compressor source change.

The root restores R245's 5630944-byte machine body and 55136-byte frame exactly. All 101817 symbolic root relocation offsets/types/targets also match. Every270 outlined original/map/vector body matches R246 byte-for-byte. Relative to R245, four original path bodies (12419,15296,26895,31086) shrink2040->1968B;132 other original numeric/map bodies and all seven selected vector bodies remain identical. The four path bodies contain three threaded edges each. Disabled entire Object and metadata match R245. Root-only source images are wholly identical in normal/observed original/decoded forms; the separate real path witness still selects three phase edges.

Object text 5860960->5860960B; linked worker text 6178864->6178864B; file 6640168->6640168B. Unselected variant compilation remains inherited. Raw code/relocation identity preserves intended root semantics; linked addresses, placement and cache behavior may still differ. Static identity alone is not a timing prediction.

The pass follows only proved current SSA Boolean/tag control through total scalar operations, transports current typed values, and refuses effects, loads/stores/calls, checked arithmetic/debits and unsafe merges/cycles. Snapshot-bound facts prevent old predicates from being reinterpreted under new recursive/loop arguments. The exact threader and seven proof tests remain those of R237. The experimental runtime retains its pre-existing omitted ordinary resource/admission/accounting/canonical-publication/recovery contract; this round does not restore it.

## Every complete compression case

Before=R245; after=phase-only R247; the transport field named retained=archived full R246, not the default compiler. All four actors, including pinned fixed-level12 runtime-input Roc, run in two fixed24-block cohorts covering every ordering. Each gets200ms warmup. Output ownership, pinned allocator, Rust clock and validation placement match. Full compression, scratch construction, output transfer and cleanup are timed; preparation/validation/logging/previous-output drop are excluded. No build/test/profile overlaps timing or counters. Ratios are pooled observation medians; percentage changes are medians of balanced block means. Historical cohorts are preserved with their original denominators.

| Case | Bytes | R245 / Roc | R247 / Roc | Full R246 / Roc | Block change | Initial | Confirmation |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | 1.976190x | +1.5294% | +0.4775% | -1.2136% |
| one-stored | 1 | 1.976190x | 1.976190x | 1.976190x | -0.7924% | -0.0946% | -2.2067% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | 1.976190x | +0.3884% | +0.9896% | -0.0932% |
| eight-literals | 8 | 0.503905x | 0.507425x | 0.495672x | +0.8161% | -1.2774% | -22.1298% |
| eight-overlap | 8 | 0.508717x | 0.511321x | 0.511684x | +0.1454% | -2.4550% | +1.4458% |
| 320-overlap | 320 | 0.578674x | 0.575161x | 0.581272x | -0.2121% | -3.0533% | +1.0513% |
| eight-literals-prefix | 8 | 0.503448x | 0.508766x | 0.517386x | +1.1239% | -0.9144% | +2.3809% |
| 128-biased-dynamic | 128 | 0.520970x | 0.521617x | 0.527115x | +0.9321% | -2.9886% | +3.1799% |
| 32-high-entropy-stored | 32 | 0.546296x | 0.542923x | 0.538615x | +0.7785% | -2.8915% | +3.0379% |
| 16384-distribution-change | 16384 | 2.741508x | 2.724473x | 2.707563x | -0.6214% | -0.1798% | -0.8702% |
| 33027-periodic-window | 33027 | 1.593197x | 1.632102x | 1.648828x | +2.4420% | +3.0344% | +2.3515% |
| random-64 | 64 | 0.569897x | 0.570611x | 0.575472x | +0.3530% | -0.4003% | +1.2906% |
| random-256 | 256 | 0.662601x | 0.674516x | 0.665405x | +1.8982% | +0.7251% | +1.1883% |
| random-1024 | 1024 | 0.742282x | 0.756571x | 0.749156x | +1.7790% | +0.4979% | +2.3079% |
| repeated-1024 | 1024 | 0.641270x | 0.652534x | 0.653799x | +1.8469% | +0.1836% | +2.7494% |
| random-32769 | 32769 | 2.732096x | 2.806148x | 2.790419x | +2.7105% | +2.7105% | +2.6647% |
| repeated-32769 | 32769 | 1.506688x | 1.541355x | 1.556664x | +2.3009% | +2.0097% | +2.4650% |
| reymont-prefix-1024 | 1024 | 0.927292x | 0.909146x | 0.906615x | -1.7196% | -2.4216% | -0.6721% |
| reymont-prefix-16384 | 16384 | 2.213907x | 2.099295x | 2.092803x | -5.1769% | -5.2673% | -4.9326% |
| reymont-prefix-32769 | 32769 | 2.248360x | 2.145959x | 2.145357x | -4.5545% | -4.4967% | -4.7067% |
| heldout-window-32767 | 32767 | 2.251295x | 2.146943x | 2.145632x | -4.6352% | -4.5745% | -4.6044% |
| heldout-window-32768 | 32768 | 2.244875x | 2.142933x | 2.142139x | -4.5411% | -4.6045% | -4.5033% |
| heldout-slides-65537 | 65537 | 2.306372x | 2.215506x | 2.210606x | -3.9398% | -3.9810% | -3.9780% |
| heldout-random-65537 | 65537 | 2.985697x | 3.055233x | 3.055083x | +2.3290% | +2.3857% | +2.4405% |
| heldout-mixed-131073 | 131073 | 2.506422x | 2.511497x | 2.509036x | +0.2025% | +0.0727% | +0.1573% |

Natural32 block elapsed changes -4.554498%; periodic +2.441976%, each direction agreeing in both cohorts. Eight is 0.507425x. The same11 cases beat the reference; no win is gained or lost. There are16 adverse pooled block means, all shown above. All25 geometric block elapsed changes -0.214917%; large11 changes -1.277865%. Tiny0/1/7-byte calls remain near clock resolution. Ratios can move with a fresh reference denominator even when R245 is unchanged.

### Current retained wins, all of them

| Case | Bytes | Retained R245 / Roc |
|---|---:|---:|
| eight-literals-prefix | 8 | **0.503448x** |
| eight-literals | 8 | **0.503905x** |
| eight-overlap | 8 | **0.508717x** |
| 128-biased-dynamic | 128 | **0.520970x** |
| 32-high-entropy-stored | 32 | **0.546296x** |
| random-64 | 64 | **0.569897x** |
| 320-overlap | 320 | **0.578674x** |
| repeated-1024 | 1024 | **0.641270x** |
| random-256 | 256 | **0.662601x** |
| random-1024 | 1024 | **0.742282x** |
| reymont-prefix-1024 | 1024 | **0.927292x** |

### Absolute times and byte-normalized retained work

| Case | Roc ms | R245 ms | R247 ms | R245 ns/input byte | Roc ns/input byte |
|---|---:|---:|---:|---:|---:|
| empty | 0.000042 | 0.000083 | 0.000083 | — | — |
| one-stored | 0.000042 | 0.000083 | 0.000083 | 83.0000 | 42.0000 |
| seven-stored-prefix | 0.000042 | 0.000083 | 0.000083 | 11.8571 | 6.0000 |
| eight-literals | 0.296063 | 0.149188 | 0.150229 | 18648.4375 | 37007.8125 |
| eight-overlap | 0.287979 | 0.146500 | 0.147250 | 18312.5000 | 35997.4375 |
| 320-overlap | 0.296583 | 0.171625 | 0.170583 | 536.3281 | 926.8219 |
| eight-literals-prefix | 0.290000 | 0.146000 | 0.147542 | 18250.0000 | 36250.0000 |
| 128-biased-dynamic | 0.288125 | 0.150105 | 0.150291 | 1172.6914 | 2250.9766 |
| 32-high-entropy-stored | 0.290250 | 0.158563 | 0.157583 | 4955.0781 | 9070.3125 |
| 16384-distribution-change | 0.845125 | 2.316917 | 2.302520 | 141.4134 | 51.5823 |
| 33027-periodic-window | 1.260542 | 2.008292 | 2.057334 | 60.8076 | 38.1670 |
| random-64 | 0.291500 | 0.166125 | 0.166333 | 2595.7031 | 4554.6875 |
| random-256 | 0.297250 | 0.196958 | 0.200500 | 769.3672 | 1161.1328 |
| random-1024 | 0.309125 | 0.229458 | 0.233875 | 224.0801 | 301.8799 |
| repeated-1024 | 0.329230 | 0.211125 | 0.214834 | 206.1768 | 321.5132 |
| random-32769 | 1.070187 | 2.923854 | 3.003105 | 89.2262 | 32.6585 |
| repeated-32769 | 1.347312 | 2.029980 | 2.076687 | 61.9482 | 41.1155 |
| reymont-prefix-1024 | 0.337542 | 0.313000 | 0.306875 | 305.6641 | 329.6309 |
| reymont-prefix-16384 | 2.169458 | 4.802980 | 4.554334 | 293.1506 | 132.4132 |
| reymont-prefix-32769 | 4.358250 | 9.798917 | 9.352625 | 299.0301 | 132.9992 |
| heldout-window-32767 | 4.353292 | 9.800541 | 9.346270 | 299.0979 | 132.8560 |
| heldout-window-32768 | 4.357230 | 9.781438 | 9.337250 | 298.5058 | 132.9721 |
| heldout-slides-65537 | 8.864896 | 20.445750 | 19.640229 | 311.9726 | 135.2655 |
| heldout-random-65537 | 1.947437 | 5.814458 | 5.949876 | 88.7202 | 29.7151 |
| heldout-mixed-131073 | 8.398834 | 21.051021 | 21.093646 | 160.6053 | 64.0775 |

## Independent CPU and memory evidence

Separate CPU/allocation cohorts contain5200 observed/4800 retained calls. Instructions decrease on22 cases, increase on0, and are equal on3. Cycles decrease on12/increase on13. All25 payload and heap request/byte/reallocation/copy/zero/peak medians are exactly unchanged. Requested capacity is not initialized copying. RSS/physical/lifetime peaks are separate process measures and remain in raw counter JSON.

| Case | R245 instructions | R247 instructions | Roc instructions | Instruction change | Cycle change | R247 heap requests | Roc heap requests | Private payload peak B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| empty | 6,308.0 | 6,308.0 | 5,668.0 | +0.0000% | +0.0630% | 4 | 1 | 5 |
| one-stored | 6,451.0 | 6,451.0 | 5,791.0 | +0.0000% | +0.9414% | 4 | 1 | 6 |
| seven-stored-prefix | 6,502.0 | 6,502.0 | 5,691.0 | +0.0000% | +2.9034% | 4 | 1 | 14 |
| eight-literals | 1,235,828.0 | 1,235,326.0 | 1,361,848.0 | -0.0406% | -3.0696% | 230 | 96 | 8982372 |
| eight-overlap | 1,214,532.0 | 1,213,943.5 | 1,358,271.0 | -0.0485% | -6.5642% | 230 | 96 | 8982372 |
| 320-overlap | 1,643,574.0 | 1,620,431.5 | 1,519,617.0 | -1.4081% | +1.9352% | 230 | 96 | 8982372 |
| eight-literals-prefix | 1,235,808.0 | 1,235,305.5 | 1,361,806.0 | -0.0407% | -1.7606% | 230 | 96 | 8982372 |
| 128-biased-dynamic | 1,334,714.0 | 1,327,182.5 | 1,367,428.0 | -0.5643% | +0.1319% | 210 | 86 | 8982372 |
| 32-high-entropy-stored | 1,422,702.0 | 1,420,501.5 | 1,379,518.0 | -0.1547% | +0.8173% | 226 | 94 | 8982372 |
| 16384-distribution-change | 33,991,491.0 | 32,387,720.5 | 9,262,390.0 | -4.7182% | -0.6877% | 522 | 234 | 8998776 |
| 33027-periodic-window | 39,327,562.0 | 38,133,058.0 | 19,394,800.0 | -3.0373% | +2.3764% | 208 | 84 | 9500860 |
| random-64 | 1,553,212.5 | 1,548,708.5 | 1,401,380.0 | -0.2900% | +0.2743% | 226 | 94 | 8982372 |
| random-256 | 2,151,791.5 | 2,133,417.5 | 1,507,510.0 | -0.8539% | -1.8570% | 226 | 94 | 8982372 |
| random-1024 | 2,966,143.0 | 2,910,858.0 | 1,682,333.5 | -1.8639% | +1.1157% | 208 | 85 | 8982372 |
| repeated-1024 | 2,610,413.0 | 2,536,433.0 | 1,964,611.0 | -2.8340% | -0.0504% | 232 | 97 | 8982372 |
| random-32769 | 47,304,614.0 | 45,534,435.0 | 11,554,026.0 | -3.7421% | +2.3507% | 276 | 115 | 9500860 |
| repeated-32769 | 38,920,960.0 | 37,735,624.0 | 19,250,997.0 | -3.0455% | +2.2315% | 208 | 84 | 9500860 |
| reymont-prefix-1024 | 4,102,903.0 | 3,981,368.0 | 2,233,910.0 | -2.9622% | -2.4686% | 266 | 114 | 8982372 |
| reymont-prefix-16384 | 65,037,049.0 | 61,269,460.5 | 24,840,641.0 | -5.7930% | -4.6518% | 650 | 300 | 8998776 |
| reymont-prefix-32769 | 130,007,659.5 | 122,838,919.5 | 47,717,652.0 | -5.5141% | -4.2594% | 608 | 277 | 9533536 |
| heldout-window-32767 | 129,763,785.5 | 122,593,790.0 | 47,613,797.0 | -5.5254% | -4.1101% | 606 | 277 | 9015174 |
| heldout-window-32768 | 129,761,815.5 | 122,594,858.0 | 47,614,255.0 | -5.5232% | -4.1016% | 606 | 277 | 9015177 |
| heldout-slides-65537 | 266,783,071.5 | 253,244,475.5 | 95,130,200.0 | -5.0748% | -3.6125% | 864 | 394 | 9566339 |
| heldout-random-65537 | 93,592,999.0 | 90,054,515.0 | 21,044,070.0 | -3.7807% | +2.7406% | 256 | 109 | 9500860 |
| heldout-mixed-131073 | 317,696,788.5 | 299,848,134.5 | 101,045,474.5 | -5.6181% | +0.5282% | 950 | 440 | 9631947 |

Periodic and random64 again retire fewer instructions while taking more cycles. Restoring the root did not remove this behavior. This narrows the question to changed phase execution/interaction/placement; it does not distinguish dependence chains, register allocation, branches or cache effects. The natural gain survives almost unchanged. Neither CPU-time sample shares nor independent elapsed cohorts are multiplied into instruction counts.

## Fresh profiles and controlled-prefix verification

- R247 33027-periodic-window: _mech_compact_closed_v2 2372/5539, _mech_compact_numeric_2624_vector 1601/5539, _mech_compact_numeric_15296 849/5539, _mech_compact_numeric_5538_vector 169/5539.
- R247 eight-literals: _mech_compact_closed_v2 1663/5451, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 65/5451, _mech_compact_numeric_17556 47/5451, _mech_compact_numeric_16010 43/5451.
- R247 heldout-random-65537: _mech_compact_closed_v2 3331/5548, _mech_compact_numeric_15296 850/5548, _mech_compact_numeric_15737 603/5548, _mech_compact_numeric_1279_vector 391/5548.
- R247 reymont-prefix-32769: _mech_compact_numeric_15296 2494/5365, _mech_compact_numeric_1279_vector 1259/5365, _mech_compact_closed_v2 1246/5365, _mech_compact_numeric_15737 81/5365.
- retained R245 heldout-random-65537: _mech_compact_closed_v2 3431/5503, _mech_compact_numeric_15296 722/5503, _mech_compact_numeric_15737 614/5503, _mech_compact_numeric_1279_vector 380/5503.

The missing random64 profile now uses each exact measured executable and its own current symbols/Object addresses. R245 random64 is dominated by the native root, followed by path15296, tally15737 and search1279_vector. These Timer Fired/Running samples include harness and validation, and locate work rather than establish removable percentages. Focused assembly windows and aggregate PCs are retained; raw traces/stacks are excluded.

All six existing natural-text fixtures (1024,16384,32767,32768,32769,65537) were freshly hash-checked and proved exact prefixes of the same65537-byte stream. Thus their existing size comparisons are controlled with respect to input contents. Random32 is not a prefix of held-out random64; a new frozen-prefix random sweep is still needed. The requested256KiB/optional1MiB sweep and fresh logical-work comparison are next work, not completed by this round. Historical R239 counts are not relabeled current.

## Qualification, configurations and failure disposition

710 maintained tests and seven threading tests pass. The unchanged seven inherited exclusions remain explicit in maintained-native-tests.json. Compileropt3/test-rootopt1, -Dwarnings, nightly2026-03-03/aarch64, Cranelift0.131.3 and six features: aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime. Exact rustc/dependency commands and five enabled flags are recorded in build/command receipts. Flags: MECH_PRIVATE_INDEXED_CHAINS=1, MECH_PHASE_OWNER_TRANSFER=1, MECH_PHASE_VECTOR_FAST=1, MECH_THREAD_LOOP_EDGES=1, MECH_THREAD_PHASES_ONLY=1.

7076 fresh native source controls:4640 nested-dispatch,320 actual-path,1700 candidate matrix and416 phase boundary cases. Dispatch has1160 fresh resident calls; matrix1700.1700 parent matrix rows are hash-pinned R245 reuse.40 old path resident rows are reused, including34 inherited default-work denials; native path oracles/expectations remain unchanged. Fanout retains12 inherited resident default-cap denials and its unsupported native selection. All33 newly emitted small original/decoded Object/metadata pairs match.50 immediate full smokes and every timing/counter/profile call retain exact raw bytes, independent inflate, input/output aliases, capacity and zero final private payload/owners. No expected value or allowance changed.

The unchanged runtime ABI controls are inherited by hash; no fresh ABI run or full source re-encoding/rebuild is claimed. Full revision14 decoded bytecode, fourteen pinned reference source files, algorithm/tables/depth300/nice258/nearEOF,262144U16 tree,1500513U32 cache,305257U64 path capacity, allocation sites, allocator, output13 clear/reuse and exact eight-byte output[75,76,74,78,73,77,75,207,0,0] remain fixed.

An initial harness invocation passed relative Python paths to run.py, which deliberately uses the PR worktree. Those three setup commands returned exit2 and were corrected to absolute paths before timing. All reruns pass; this was not a compiler/program failure. The periodic performance criterion fails as an introduced candidate tradeoff. No unresolved introduced semantic/build/resource failure remains. Inherited module-layout debt, seven exclusions, resident cap denials, standalone contract omissions and unused variant generation remain assigned. Multi-MB timing, full Cargo/math rebuilds and the separate ten-workload suite are not claimed. Hosted CI is cancelled by user instruction; #973 remains draft pending green CI and clean exact-head review.

Main commands (full expansions/features in receipts):

```text
python3 run-spec.py candidate-gpu candidate-producer candidate-tests
python3 qualify-dispatch.py {before,after} {observed,normal}
python3 qualify-path.py {before,after} {observed,normal}
python3 qualify-writes.py
python3 qualify-exits.py
python3 run-spec.py mech-worker after-cpu after-memory
python3 compare-isolation.py
python3 run-measurements.py
python3 replay.py
```

## Next bounded work and pins

Refresh logical work counts and controlled prefix scaling on retained R245; use the fresh random64 root profile to select one bounded ownership/transport experiment. First count eligible phase-success transitions; do not assume a returned owner is unique.

This completes the phase-only isolation and current random64 profiling portions of the new brief. The work-normalized cross-language census, expanded prefix sweep and subsequent measured ownership/transport improvement remain outstanding. A phase success-edge capability requires current identity, uniqueness and no escape; an owner identity transfer alone is not such a capability. Count actual eligible transitions before implementing it.

- object: `417d2d637df748a704f37426fcd57a2d11a71dbde97e8bf719dca6a0b46a2710`.
- worker: `cb3f028b56a1753ba3cab8b51434208fa4b3c58a8dd968742e7cfdf02b620080`.
- incremental_patch_over_R246: `a84546373db30eb01ee4e23b92db25fe13188416ece692a12acbe99e84331581`.
- combined_patch_over_R245: `8f99668c7e8d065e9c823ac61866f334bd20463e15438faedef2fbb6b6be4c13`.
- compiler_source_manifest: `1a11e5e831f39c69b6889512e3c66988c6a3191c5f520be8c35d150956fbbe8f`.
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- qualification_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`.
- reference_assembly: `acc956027028d0d4a6deb4baf69635d2d1b84c20ef45d9d508e427b4d50442b1`.

The external archive contains the820-file tested compiler closure, incremental/combined patches, exact configurations, proof witnesses, every25-case observation, portable fixtures, aggregate counters/profiles, root relocation audit and focused assembly. No native binary/Object, bytecode, full executable metadata/descriptor, raw trace/stack, secret or private environment export is included. verify-packet.py checks every hash; replay.py recomputes five byte-identical numerical/report outputs without compiling or compressing.

## Public evidence

[Complete phase-only isolation evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r247-phase-only-threading-539f1187d629.tar.gz) (`sha256:539f1187d629f4d172f9cd6237a0304a0e467326d9c0f8549e7d3a45a4f5fd5f`, 6001772 bytes). All2117 downloaded hashes, five byte-identical numerical replays and820-file incremental source reconstruction verify. The one-file policy patch over R246 and nine-file combined patch over R245 are both included.
