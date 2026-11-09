# R241: group certified immutable equality into64-byte iterations

Primary target: periodic33,027B, toward<=1x reference before moving to the other inputs. The user explicitly permits eight-byte latency to rise from about0.5x up to1x. This prospective rule was declared before timing; historical1.05-relative gates remain diagnostic and every failing row stays visible.

Retain this R240-based candidate as the next periodic-case experimental control. Periodic improves 1.5038%: 1.680619x to 1.655346x reference. Both fixed cohorts improve. Eight is 0.505430x, within the user's ceiling in both cohorts. Parity is unfinished: another 39.59% of current periodic time must disappear.

Git parent367df475b7d2b721b46c9898294338959c068199; compiler control is its R240 private-write variant over R234. Retained R237 f92415a31dbf0398ae641e66c419f435a3e3cb7c remains separately measured and preserved, including its faster natural-text result. Ordinary compiler R215 42c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14, remains unchanged.

## Whole-computation elapsed and independent counters

Two fixed24-balanced-block cohorts, all four executable permutations once each, equal200ms warmup, fixed level12/runtime inputs, matched live output ownership, pinned allocator and shared Rust clock. Full compression, scratch creation, output transfer and cleanup remain timed; preparation/validation/logging/old-output drop remain outside. No builds/tests/profiling overlap the measurements.

| Input | R240/reference | R241/reference | R237/reference | Block elapsed change | Instruction change | Cycle change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 1.976190x | 1.976190x | 1.976190x | +1.1811% | +0.0000% | +9.8589% |
| one-stored | 1.976190x | 2.000000x | 1.976190x | +5.8496% | +0.0000% | -3.6088% |
| seven-stored-prefix | 2.000000x | 2.000000x | 2.000000x | +3.5944% | +0.0000% | -1.1319% |
| eight-literals | 0.499070x | 0.505430x | 0.484993x | +1.9926% | +0.0000% | -1.2552% |
| eight-overlap | 0.484621x | 0.505839x | 0.484334x | +4.7312% | -0.0003% | +1.9938% |
| 320-overlap | 0.555525x | 0.572319x | 0.556634x | +2.6520% | -0.3046% | -1.2230% |
| eight-literals-prefix | 0.494384x | 0.509358x | 0.488843x | +2.8537% | -0.0002% | +0.0621% |
| 128-biased-dynamic | 0.517776x | 0.526948x | 0.506883x | +0.4959% | +0.0004% | +2.9907% |
| 32-high-entropy-stored | 0.559817x | 0.572526x | 0.554933x | +0.8509% | -0.0001% | -1.9192% |
| 16384-distribution-change | 2.753037x | 2.762092x | 2.777806x | +0.3289% | -0.0042% | +0.2387% |
| 33027-periodic-window | 1.680619x | 1.655346x | 1.760493x | -1.5038% | -3.2905% | -1.6140% |
| random-64 | 0.587563x | 0.594568x | 0.580844x | +1.6523% | -0.0002% | -0.0083% |
| random-256 | 0.667647x | 0.681379x | 0.662953x | +2.7432% | -0.0001% | -4.2820% |
| random-1024 | 0.746037x | 0.754671x | 0.749275x | +1.4776% | +0.0015% | -2.4897% |
| repeated-1024 | 0.654215x | 0.662040x | 0.663426x | +1.8053% | -1.2886% | +0.1308% |
| random-32769 | 2.772848x | 2.770757x | 2.882429x | -0.0754% | -0.0175% | -0.4617% |
| repeated-32769 | 1.578679x | 1.553026x | 1.645974x | -1.6250% | -3.3020% | -1.7400% |
| reymont-prefix-1024 | 0.929477x | 0.941312x | 0.912494x | +1.5411% | +0.0775% | -1.2934% |
| reymont-prefix-16384 | 2.207578x | 2.199158x | 2.092967x | -0.3814% | +0.0577% | -0.1210% |
| reymont-prefix-32769 | 2.252545x | 2.236146x | 2.141133x | -0.7280% | +0.0349% | -0.2742% |
| heldout-window-32767 | 2.252539x | 2.240103x | 2.151290x | -0.5521% | +0.0314% | -0.3104% |
| heldout-window-32768 | 2.250008x | 2.235645x | 2.146163x | -0.6384% | +0.0362% | -0.1842% |
| heldout-slides-65537 | 2.326540x | 2.303395x | 2.226156x | -0.9948% | +0.0315% | -0.4701% |
| heldout-random-65537 | 3.022081x | 3.017731x | 3.187144x | -0.1439% | -0.0287% | -0.2065% |
| heldout-mixed-131073 | 2.488611x | 2.475491x | 2.534171x | -0.5272% | -0.4152% | -0.3522% |

Periodic absolute medians: R240 2.1154790ms; R241 2.0836665ms; reference 1.2587500ms. Instructions 41,856,081.5 -> 40,478,799.5, reference 19,394,852.0. Independent cycles change -1.6140%. Eight instructions are exactly unchanged; its contrary elapsed and cycle cohorts remain separate.

Historical gates are not all green: initial25/25 point and24/25 block; confirmation24/25 point and22/25 block; pooled25/25 point and24/25 block. The pooled one-stored block mean rises5.8496%; distribution16 rises0.3289%. All25 equal-case geometric elapsed rises1.0458%, while large11 falls0.6235%. These averages do not determine the new one-case objective. Natural32 still favors preserved R237,2.141133x versus candidate2.236146x; no combined-ideas speedup is inferred.

The separate5,200-observation counter cohort retains4,800 calls:13 instruction medians fall,8 rise,4 tie;19 cycle medians fall,6 rise. All25 allocation/requested-byte/reallocation/usable-peak and payload-copy/zero/private-peak fields match exactly. Periodic has208 heap requests versus84 reference; private payload peak9,500,860B. No allocation, streaming or physical-copy reduction is claimed.

## Generic compiler rule and machine evidence

Only wide_equal.rs and its regression test differ from R240. Reuse the existing private certificate: paired immutable U8 windows, one changing induction value, confined pure8-byte equality cycle, exact current geometry and no debits/stores/calls/extra state. A current limit-induction>=64 guard precedes four16-byte loads from each input. OR the four XOR results, test both U64 lanes, and advance64 only if every compared bit agrees. Mismatch resumes the unchanged scalar8-byte header at the original position; short remainders retain the16-byte path. limit-at>=64 proves the advance cannot wrap. Source assertions, scalar mismatch/tail/error order, current aliases and existing fallback remain.

Nine static loops in three existing matching bodies change; each adds348B,1,044B total. All133 other numeric/map bodies are byte-identical. Complete root text5,654,028B and frame55,152B stay byte-identical; the matching frames stay384/384/304B. All134 numeric proofs/two map selections and12 private-write intervals remain. Static eligibility is not a dynamic probe count. The pinned reference's archived real matching assembly still uses8-byte comparisons; this transformation changes no compressor algorithm.

## Current executable profile and next periodic target

33027-periodic-window: 5532 actual current-PC samples; _mech_compact_closed_v2 41.05%, _mech_compact_numeric_2624 32.47%, _mech_compact_numeric_15296 12.82%, _mech_compact_numeric_5538 2.91%.

eight-literals: 5391 actual current-PC samples; _mech_compact_closed_v2 29.85%, _mech_compact_numeric_17556 0.96%, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 0.93%, _mech_compact_numeric_16152 0.87%.

reymont-prefix-32769: 5400 actual current-PC samples; _mech_compact_numeric_15296 47.07%, _mech_compact_numeric_1279 24.19%, _mech_compact_closed_v2 21.56%, _mech_compact_numeric_15737 1.48%.

Profiles use the current executable's own symbol map. They are sampled time, not instruction partitions or removable-overhead percentages. Root state/owner transport remains the next periodic target. In current assembly, a phase return copies six-word views and can retain an owner immediately before releasing that same owner. Count reached equal-owner replacements first, then test a generic same-owner settlement path with original different-owner behavior and failure order. Do not resume the deferred natural-text terminal-return patch first.

## Correctness, provenance and failures

700 maintained tests pass under the same six native features and-Dwarnings; seven inherited exclusions remain. Two focused equality tests execute5,346,432 native value checks each, once separately and once inside the suite.5,788 compiled native source controls pass across current parent/candidate and original/decoded forms: exact errors/origins, current matrix views, aliases, previous publication, cleanup and successful retry. Eight small original/decoded image pairs match byte-for-byte; fallible and changed-stride sources decline widening. Frozen R230 resident observations are reused with exact source/library provenance and an independent source-operation oracle; no fresh resident-call or allowance-change claim. All25 full streams match and independently inflate, preserve input/output aliases and end with zero nonreturned private payload/owner records.

Preflight caught an introduced staging mismatch: the initial emission omitted MECH_PRIVATE_INDEXED_CHAINS=1, giving0 intervals and a different root. Preserve that image, receipts and50 excluded smokes. The corrected producer command explicitly enables the same12 intervals as R240, relinks all workers and repeats50 smokes before timing. There are no new semantic build/test failures or resource denials. Read-only lookup mistakes remain listed separately.

Inherited experimental owner_flow.rs plus owner_flow/ layout debt remains assigned before ordinary integration. Hosted CI is cancelled by standing instruction; no green-CI or clean exact-head review claim. Full Cargo/math/dependency rebuilding, fresh resident source controls, the multi-MB corpus and separate ten-workload suite are skipped explicitly. Complete source-vs-MBC Object equivalence is reused from R240; this round executes strict MBC14 and the changed class's original/decoded small images.

The standalone experiment still omits ordinary exact resource/admission/accounting/canonical publication/recovery contracts; those are not silently restored or counted as optimized away. Preserve fixed source/bytecode, algorithm/tables/depth300/nice258/search/near-EOF rules,262144U16 tree,1500513U32 cache,305257U64 path capacity and eight-byte output13/clear/reuse, allocation sites, aliases and timing boundaries. No new branch/PR or LLVM change.

## Pins and reproduction

- object: `b2be9f8f76f3f12c38fb1ccd596d3d1d969fff5b2e642113a33a24b2f92e240a`
- worker: `ddf10ee8f0d4c0d6cf7e2ad36a020ed7ca8179f88e140c039981f598e07c9034`
- compiler_patch: `22468e8bce608cdb7be8432371eb5f8542d39f1c587a59ef7e7f0f283c12b684`
- compiler_source_manifest: `6a2195b11847169671bd378c198bc9a1c2a69026aa4fd4aaa479b8d2d5a9a905`
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

Exact command/build specifications, complete cohort observations, source patch/closure, portable25-input fixtures, focused assembly, counter/profile summaries and receipts accompany this report. Replay analyze-primary.py, analyze-counters.py and make-report.py to regenerate the numerical reports without compilation or compression. replay.py asserts byte-identical output; verify-packet.py verifies every file hash. Native binaries/Objects, MBC, full executable metadata/descriptors, raw traces/stack samples and private environment exports are excluded.

[Complete source/patch and all25 observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r241-wide-group-2f9a06031ee5.tar.gz). SHA-256`2f9a06031ee5ffc594820f922004f352b4e7175bb7b7f722214750da12bff162`;5263818B,1480 file hashes and five numerical replays verified after public retrieval.
