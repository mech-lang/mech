# R246: loop-threading combination preserves a natural-text win but slows periodic input

Archive the combined candidate; R245 stays the retained experimental compiler. Natural32 improves in both cohorts, but periodic regresses in both, and the confirmation eight-byte historical1.05 gates fail. Preserve this component and its gain rather than discard it. No parity or uniform speedup is claimed.

Git parent `085dce0ee59b2c798a9ed542c62273759414f0cc`; compiler control is its R245 experimental source closure and worker. Ordinary R215 `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14, remains unchanged. The tested nine-file patch applies to archived R245, not directly to the maintained tree. The source closure, patch, commands and measurements are published in the evidence packet; #973 records their provenance.

## Actual opportunity and bounded combination

R245 current-PC sampling places2654/5430 natural32 weights in path15296. Its actual Object retains literal-tag dispatch immediately after a terminal result. R237 already proved a conservative loop-exit threader, but R241/R242/R245 descend from the periodic R234/R240 line and do not contain that pass. This round reuses the existing R237 threader, snapshot-bound fact mode and seven proof tests on R245. R236 low-word narrowing and R238 flag reuse remain uncombined. It is not a new threading algorithm or a new source-operation class.

The pass transports current typed values through repaired destination parameters. It follows only current SSA-proved Boolean/tag edges through total scalar dispatch, requires a backedge, and stops at loads/stores/calls/checked arithmetic/debits/unknown/exceptional edges. It refuses unsafe shared merges, pure cycles and shortcuts that bypass another repaired destination. Source words are snapshots; an old predicate is never reinterpreted under a new visit's arguments. No compressor name, field, source site or input size selects it.

MECH_THREAD_LOOP_EDGES=1 privately enables root and numeric-phase threading after the existing cleanup/wide-loop work. The original three R245 flags remain explicit. Disabled complete Object and metadata match R245 byte-for-byte. Five threader source files and the bound-value implementation match R237 exactly. All source algorithms, tables, depth300/nice258/nearEOF limits,262144U16 tree,1500513U32 cache,305257U64 path capacity, source allocation sites, pinned allocator and timing boundary stay fixed. Output13 clear/reuse and eight-byte raw bytes [75,76,74,78,73,77,75,207,0,0] remain.

Fresh C and Rust O3 controls compile the same checked-add Step/Done state loop to direct exits. Their source/assembly/commands are retained as analogues, not compression benchmarks or equal-contract timing comparisons. Pinned reference matching assembly/source provenance is retained. There is no new claim of a complete per-function reference instruction saving.

## All25 complete-call measurements

Two fixed24-balanced-block cohorts, all four executable orders, equal200ms warmup, fixed level12/runtime inputs, matched live output ownership, pinned allocator and one Rust clock. Compression, scratch construction, output transfer and cleanup are timed; validation/preparation/logging/previous-output drop stay outside. No build/test/profile overlaps primary or counter cohorts. Before=retained R245, after=R246, historical component=R237, reference=pinned Roc. Pooled ratios are observation medians; changes are medians of balanced block means. Historical denominators remain unchanged.

| Case | Bytes | R245 / Roc | R246 / Roc | Block change | Initial | Confirmation |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -0.7388% | -0.7144% | -0.7207% |
| one-stored | 1 | 1.976190x | 1.976190x | +1.5796% | +2.6881% | +1.1241% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -1.2639% | -2.5685% | +0.8450% |
| eight-literals | 8 | 0.513988x | 0.524576x | +3.0194% | -1.1071% | +5.9119% |
| eight-overlap | 8 | 0.510621x | 0.516733x | +0.8530% | +1.4720% | +1.5248% |
| 320-overlap | 320 | 0.559668x | 0.577550x | +2.7044% | +2.7632% | +3.7587% |
| eight-literals-prefix | 8 | 0.503478x | 0.515872x | +2.3792% | +0.4809% | +3.0375% |
| 128-biased-dynamic | 128 | 0.510730x | 0.516820x | +0.5566% | -0.0050% | +0.5756% |
| 32-high-entropy-stored | 32 | 0.540318x | 0.543990x | -0.1391% | +0.7987% | -1.2142% |
| 16384-distribution-change | 16384 | 2.741179x | 2.721304x | -0.7251% | -0.8488% | -0.7297% |
| 33027-periodic-window | 33027 | 1.599350x | 1.660024x | +3.7937% | +3.6347% | +4.5212% |
| random-64 | 64 | 0.568511x | 0.572825x | +0.5709% | +0.1068% | +2.7821% |
| random-256 | 256 | 0.658766x | 0.674708x | +2.3127% | +2.6766% | +1.1615% |
| random-1024 | 1024 | 0.747621x | 0.763402x | +1.7509% | +2.2108% | +1.1913% |
| repeated-1024 | 1024 | 0.649719x | 0.669698x | +2.9488% | +1.4874% | +2.9526% |
| random-32769 | 32769 | 2.747608x | 2.805254x | +2.0981% | +1.9697% | +2.8854% |
| repeated-32769 | 32769 | 1.509380x | 1.561093x | +3.4261% | +3.4116% | +3.5236% |
| reymont-prefix-1024 | 1024 | 0.934397x | 0.922571x | -1.3825% | -1.9826% | -1.2113% |
| reymont-prefix-16384 | 16384 | 2.200813x | 2.095515x | -4.7845% | -4.9875% | -4.4734% |
| reymont-prefix-32769 | 32769 | 2.239774x | 2.141492x | -4.3880% | -4.4049% | -4.4100% |
| heldout-window-32767 | 32767 | 2.248910x | 2.144908x | -4.6246% | -4.5832% | -4.8804% |
| heldout-window-32768 | 32768 | 2.245089x | 2.141979x | -4.5927% | -4.6080% | -4.5238% |
| heldout-slides-65537 | 65537 | 2.306179x | 2.216222x | -3.9007% | -3.9398% | -3.7947% |
| heldout-random-65537 | 65537 | 2.987819x | 3.062057x | +2.4847% | +2.5673% | +2.3398% |
| heldout-mixed-131073 | 131073 | 2.503778x | 2.508725x | +0.1976% | +0.2131% | +0.1996% |

Natural32 improves 4.388028% with both cohorts favorable. Periodic worsens 3.793705%, with both cohorts adverse. Eight stays below the user1x ceiling but its confirmation point/block1.05 gates fail; preserve both its favorable initial pair and adverse confirmation. Pooled gates pass25/25, initial25/25, confirmation24/25. All25 descriptive geometric block elapsed changes +0.128336%; large11 changes -1.059133%. Neither average overrides the predeclared periodic criterion. Both variants retain the same11 below-reference cases. Tiny0/1/7-byte calls remain near clock resolution.

### Absolute pooled observations

| Case | Roc ms | R245 ms | R246 ms | Historical R237 ms |
|---|---:|---:|---:|---:|
| empty | 0.000042 | 0.000083 | 0.000083 | 0.000083 |
| one-stored | 0.000042 | 0.000083 | 0.000083 | 0.000083 |
| seven-stored-prefix | 0.000042 | 0.000083 | 0.000083 | 0.000083 |
| eight-literals | 0.291187 | 0.149667 | 0.152750 | 0.150187 |
| eight-overlap | 0.286334 | 0.146208 | 0.147958 | 0.149458 |
| 320-overlap | 0.297125 | 0.166292 | 0.171605 | 0.171208 |
| eight-literals-prefix | 0.287417 | 0.144708 | 0.148270 | 0.144583 |
| 128-biased-dynamic | 0.287334 | 0.146750 | 0.148500 | 0.146958 |
| 32-high-entropy-stored | 0.289375 | 0.156355 | 0.157417 | 0.156084 |
| 16384-distribution-change | 0.835438 | 2.290083 | 2.273479 | 2.321167 |
| 33027-periodic-window | 1.261854 | 2.018146 | 2.094708 | 2.221709 |
| random-64 | 0.289792 | 0.164750 | 0.166000 | 0.166541 |
| random-256 | 0.295375 | 0.194583 | 0.199292 | 0.197208 |
| random-1024 | 0.306250 | 0.228959 | 0.233792 | 0.234416 |
| repeated-1024 | 0.325333 | 0.211375 | 0.217875 | 0.220625 |
| random-32769 | 1.071167 | 2.943147 | 3.004896 | 3.094124 |
| repeated-32769 | 1.344771 | 2.029771 | 2.099312 | 2.224333 |
| reymont-prefix-1024 | 0.334708 | 0.312750 | 0.308792 | 0.309833 |
| reymont-prefix-16384 | 2.170041 | 4.775855 | 4.547354 | 4.577479 |
| reymont-prefix-32769 | 4.364355 | 9.775166 | 9.346230 | 9.377438 |
| heldout-window-32767 | 4.354062 | 9.791895 | 9.339062 | 9.362959 |
| heldout-window-32768 | 4.356583 | 9.780917 | 9.331708 | 9.368208 |
| heldout-slides-65537 | 8.849813 | 20.409250 | 19.613146 | 19.720813 |
| heldout-random-65537 | 1.941124 | 5.799729 | 5.943833 | 6.192521 |
| heldout-mixed-131073 | 8.396187 | 21.022188 | 21.063729 | 21.270105 |

## Code and frame evidence

Root: 5630944->5628152B, frame 55136->55120B. Nine root edges bypass17 proved conditions with71 typed transport parameters. Four path bodies each shrink2040->1968B with three threaded edges apiece;132 other original numeric/map bodies and all seven selected R245 vector bodies remain byte-identical. Static12 phase edges are not a dynamic full-compressor count. Normal/observed source witnesses positively exercise the pass, and fresh current PCs establish execution of the actual changed bodies.

Object text 5860960->5858168B; linked worker text 6178864->6176072B; worker file 6640168->6640168B. Unused vector candidate compilation from R245 remains inherited. Root layout and typed transport change in this combination, but this round does not establish which change causes the periodic loss; a phase-only control is required. Build receipts are descriptive setup observations, not matched compilation benchmarks.

## Independent full-call counters and memory

Separate CPU/allocation cohorts contain5200 observed/4800 retained calls. Instructions decrease on22 cases/increase on3; cycles decrease on15/increase on10. All25 source payload and heap requests/bytes/reallocations/copy/zero/live-peak medians are exactly equal. Requested bytes are not initialized copies. Process RSS/physical/lifetime peaks include loaded code, metadata, harness and allocator retention; raw values remain in counter-results.json.

| Case | R245 instructions | R246 instructions | Roc instructions | R246 / Roc instructions | Instruction change | Cycle change | Heap requests | Private payload peak B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| empty | 6,308.0 | 6,310.0 | 5,668.0 | 1.113267x | +0.0317% | +4.6247% | 4 | 5 |
| one-stored | 6,451.0 | 6,453.0 | 5,791.0 | 1.114315x | +0.0310% | +2.8289% | 4 | 6 |
| seven-stored-prefix | 6,502.0 | 6,504.0 | 5,691.0 | 1.142857x | +0.0308% | +2.1349% | 4 | 14 |
| eight-literals | 1,235,827.0 | 1,233,863.0 | 1,361,848.0 | 0.906021x | -0.1589% | +1.3504% | 230 | 8982372 |
| eight-overlap | 1,214,532.5 | 1,212,525.0 | 1,358,272.0 | 0.892697x | -0.1653% | -1.4049% | 230 | 8982372 |
| 320-overlap | 1,643,574.5 | 1,622,550.0 | 1,519,617.0 | 1.067736x | -1.2792% | +0.6118% | 230 | 8982372 |
| eight-literals-prefix | 1,235,808.5 | 1,233,842.5 | 1,361,806.0 | 0.906034x | -0.1591% | -2.8578% | 230 | 8982372 |
| 128-biased-dynamic | 1,334,681.0 | 1,324,986.5 | 1,367,377.5 | 0.968998x | -0.7264% | -2.6302% | 210 | 8982372 |
| 32-high-entropy-stored | 1,422,700.0 | 1,419,052.0 | 1,379,518.0 | 1.028658x | -0.2564% | -0.5295% | 226 | 8982372 |
| 16384-distribution-change | 33,991,462.5 | 32,282,833.0 | 9,262,448.0 | 3.485346x | -5.0266% | -1.1869% | 522 | 8998776 |
| 33027-periodic-window | 39,327,561.0 | 38,491,953.0 | 19,394,748.0 | 1.984659x | -2.1247% | +3.1501% | 208 | 9500860 |
| random-64 | 1,553,213.5 | 1,547,205.5 | 1,401,382.0 | 1.104057x | -0.3868% | -0.2152% | 226 | 8982372 |
| random-256 | 2,151,749.0 | 2,132,047.5 | 1,507,510.0 | 1.414284x | -0.9156% | -1.3251% | 226 | 8982372 |
| random-1024 | 2,966,125.0 | 2,907,901.5 | 1,682,335.0 | 1.728491x | -1.9629% | -2.1034% | 208 | 8982372 |
| repeated-1024 | 2,610,412.5 | 2,546,171.5 | 1,964,611.0 | 1.296018x | -2.4610% | -0.9320% | 232 | 8982372 |
| random-32769 | 47,304,651.0 | 45,458,973.5 | 11,553,920.0 | 3.934507x | -3.9017% | +2.0664% | 276 | 9500860 |
| repeated-32769 | 38,920,891.0 | 38,092,492.0 | 19,250,997.0 | 1.978728x | -2.1284% | +3.9515% | 208 | 9500860 |
| reymont-prefix-1024 | 4,102,873.0 | 3,975,915.5 | 2,233,910.0 | 1.779801x | -3.0944% | -3.8509% | 266 | 8982372 |
| reymont-prefix-16384 | 65,036,948.0 | 61,210,877.0 | 24,840,637.0 | 2.464143x | -5.8829% | -4.4944% | 650 | 8998776 |
| reymont-prefix-32769 | 130,007,655.5 | 122,721,090.0 | 47,714,202.0 | 2.572003x | -5.6047% | -4.1355% | 608 | 9533536 |
| heldout-window-32767 | 129,760,549.0 | 122,474,048.5 | 47,613,743.0 | 2.572242x | -5.6153% | -4.1211% | 606 | 9015174 |
| heldout-window-32768 | 129,761,764.5 | 122,475,341.0 | 47,614,053.0 | 2.572252x | -5.6152% | -4.0990% | 606 | 9015177 |
| heldout-slides-65537 | 266,775,780.0 | 253,003,446.0 | 95,124,401.0 | 2.659711x | -5.1625% | -3.7090% | 864 | 9566339 |
| heldout-random-65537 | 93,593,019.5 | 89,896,293.5 | 21,044,070.0 | 4.271811x | -3.9498% | +2.3596% | 256 | 9500860 |
| heldout-mixed-131073 | 317,698,411.5 | 299,657,751.5 | 101,042,143.0 | 2.965671x | -5.6785% | +0.0544% | 950 | 9631947 |

## Fresh exact-executable profiles

- 33027-periodic-window: _mech_compact_closed_v2 2338/5542, _mech_compact_numeric_2624_vector 1603/5542, _mech_compact_numeric_15296 847/5542, _mech_compact_numeric_5538_vector 154/5542.
- eight-literals: _mech_compact_closed_v2 1702/5507, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 73/5507, _mech_compact_numeric_17556 51/5507, _mech_compact_numeric_16010 38/5507.
- reymont-prefix-32769: _mech_compact_numeric_15296 2472/5309, _mech_compact_numeric_1279_vector 1271/5309, _mech_compact_closed_v2 1219/5309, _mech_compact_numeric_15737 68/5309.

Actual current Timer Fired/Running PCs use this worker's own symbol/Object addresses. Samples include harness/validation and are CPU-time weights, not retired-instruction partitions. No old map or percentage-times-instruction multiplication is used. Both the native root and changed path scan remain significant. Focused current assembly windows are retained.

## Qualification and exact scope

710 maintained tests and the seven threading proof tests pass with the same seven inherited exclusions, six native features, compileropt3/test-rootopt1, -Dwarnings and nightly2026-03-03/aarch64. Flags are MECH_PRIVATE_INDEXED_CHAINS=1, MECH_PHASE_OWNER_TRANSFER=1, MECH_PHASE_VECTOR_FAST=1 and MECH_THREAD_LOOP_EDGES=1. Proof tests retain integer/raw-F64 transport, fresh visits, old-predicate snapshots, duplicate/nested edges, checked/assertion precedence, overflow, unsupported/effectful/shared-merge refusal, disable and idempotence controls.

Fresh native source execution:4640 nested-dispatch cases in parent/candidate normal/observed original/decoded forms,320 actual-path cases in the same forms,1700 candidate matrix cases and416 boundary cases, totaling7076. Dispatch has1160 fresh resident calls; matrix has1700. The1700 parent matrix rows are reused from hash-pinned published R245, not relabeled fresh. The40 old path resident rows are reused by hash, including34 inherited default-work denials; independent path expected values/native checks remain. Every source expectation and allowance is unchanged. All33 newly emitted small original/decoded Object/metadata pairs match. The native root witness is deliberately not an outlined phase; the actual path witness separately selects a phase and reports three removed edges in both modes.

50 immediate complete smokes and every timed/counter/profile observation retain exact bytes, independent inflate, input/output aliases, capacity/clear behavior and final zero private storage/owners. The full artifact starts from the same pinned revision14 decoded source; a second full re-encoding/rebuild is not repeated. The unchanged runtime's seven ABI tests are inherited from R245 by hash; fresh matrix/growth/alias/error execution qualifies the new CFG combination.

Exact expansions are in JSON receipts. Main commands:

```text
python3 run-spec.py candidate-gpu candidate-producer candidate-tests
python3 qualify-dispatch.py before observed
python3 qualify-dispatch.py before normal
python3 qualify-dispatch.py after observed
python3 qualify-dispatch.py after normal
python3 qualify-path.py {before,after} {observed,normal} # four actual receipt commands
python3 qualify-writes.py
python3 qualify-exits.py
python3 run-spec.py mech-worker after-cpu after-memory
python3 run-measurements.py
python3 replay.py
```

## Failures, exclusions and next controlled step

The predeclared performance criterion fails; this is an experimental result, not a compiler correctness failure. Preserve all contrary pairs and the confirmation eight-byte gate failures. Initial read-only inspection guessed cfg_cleanup.rs instead of cfg_cleanup/mod.rs and used unmatched globs. An overstrict parent fixture check incorrectly required the nested root-only checked-child program to have an outlined phase; it was corrected before compiler edits, without changing the source/oracle. The candidate must positively report root threading, while the separate actual path source must report phase threading. No unresolved introduced build/semantic/program resource failure remains.

The same seven maintained exclusions, unsupported native fanout class, inherited path/fanout resident default-cap denials and owner_flow file/directory layout debt remain. Full Cargo/math rebuilds, fresh5788 matching-source controls, multi-MB inputs, the ten-workload suite and a fresh unchanged-runtime ABI run are not claimed. Hosted CI is cancelled by standing instruction; the existing PR remains draft pending green CI and clean exact-head review. Ordinary exact resource/admission/accounting/canonical publication/recovery contracts remain omitted by the standalone; ordinaryIR215 is unchanged.

Next reuse the identical pass only in outlined numeric phases, with root threading disabled. Require the R245 root machine body to match and the same four shortened path bodies before timing. Then repeat all25 cases and counters/profiles to test the root-vs-phase cause. Preserve R245 and all11 wins until that combination qualifies; no phase-only implementation or speedup is claimed here.

## Pins and replay

- object: `82cef902cc3e54439a540cade2db2c4b083df0eebca1f8a38cb610b0c06c58fd`.
- worker: `46cf234db78f18f8265ecd55a2d758f8b512e0d383b725675fbec21d4b8fcf20`.
- compiler_patch: `eb1d02080a05e7dfaf73c11c4f845afb6de497fc8b6c9a5afad5ef8aa26091cc`.
- compiler_source_manifest: `10cdc196c2188890b350d21461474d129130cc6271b78a3f7bb03268063a70d9`.
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- qualification_runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`.
- reference_assembly: `acc956027028d0d4a6deb4baf69635d2d1b84c20ef45d9d508e427b4d50442b1`.

The archive supplies the full tested820-file compiler closure/nine-file patch, source witnesses, exact configurations/commands, all25 observations, portable fixtures, aggregate profiles/counters and focused assembly. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/raw stack samples and private environment exports are excluded. replay.py recomputes five byte-identical numerical/report outputs without compiling or compressing; verify-packet.py verifies every archived file hash first.

## Published evidence

[Complete source/patch, all25 observations, counters, profiles, controls and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r246-thread-combination-fb363e6d81ff.tar.gz). SHA256 `fb363e6d81ff177c89a49e5a0d327fa304a3c42cb64ec1b57704cf26366ac9b2`; 5593077B. The fresh public download verifies all2069 file hashes and five byte-identical numerical reports. Applying the patch to its declared R245 source base reconstructs all820 tested source files.
