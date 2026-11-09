# Existing single-loop numeric proofs in compact native entry (R222)

**The complete vector-input experiment improves balanced block means by 17.150% over R221. All 20 unchanged elapsed gates pass. One inherited multidimensional fallback correctness defect is newly reproduced and remains open at this checkpoint. Normal compilation remains R215/IR215.**

## Revisions and contract

Base `52131f04f546ad25875d457d583a69402952a2c1`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. The measured compiler is archived R221 plus the exact two-file `compiler.patch` in this packet. Existing draft PR #973 and branch only. No LLVM, runtime, allocator or compressor-source change. Normal IR/work/ABI/MBC remains 215/107/39/14.

Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Same nightly 2026-03-03, Cranelift 0.131.3 and mimalloc 2.1.7. Algorithm, tables, depth 300, nice length 258, original near-EOF limits, allocation sites, real 262144-U16 tree, 1500513-U32 cache, 305257-U64 path capacity, eight-byte output capacity 13, clear/reuse, genuine input/prepared-output aliases and timer endpoints stay pinned. Largest completed input is 33,027 bytes.

**These are experimental whole-native results. Ordinary exact admission/accounting/canonical publication/recovery is omitted. Normal R215 is measured separately; no normal-entry speedup, general qualification or parity is claimed.**

## Bounded compiler change

The existing selector and compact boundary required a nested numeric phase. Two real hot operations, update 12928/header 12868 and update 15815/header 15737, already have complete numeric proofs without nested schedules. Reuse those proofs and the existing shared numeric emitter.

The additional selector class requires no cold exits, no appends, at least one proved write, and an array or private-frame result. Existing checked-child/cold-resume restrictions stay. The compact boundary accepts an absent nested schedule only with concrete storage; standalone diagnostics keep their previous contract. Current writable owner uniqueness, fixed-length/capacity gates, checked arithmetic/bounds/source origins, genuine aliases and the original source fallback remain.

Two compiler files change: `compact/phases.rs` and `codegen/numeric_minimal.rs`. No second proof engine, source-ID/name specialization, weaker mutation permission, new host dispatcher or unchecked operation implementation is added. The normal compiler source stays unchanged.

Selection grows from 7 nested phases to 134 disjoint existing proofs: 7 nested and 127 single loops. A separate real-buffer observer reaches and selects 92 of those 127 new headers across the matrix. These are phase activations, not loop-body iterations or instruction counts. Both target headers actually select. All seven earlier phase bodies AND both total-map kernels retain byte-identical actual code and metadata; root packed-read selection is unchanged.

The actual header-12868 ARM64 excerpt contains the loop's typed loads/stores and original checked errors. Ownership reference changes are outside that loop. Generic matrix-geometry branches and checked scalar operations still survive; their removal is not claimed.

| Input | New phase selections per compression | New phase fallbacks |
|---|---:|---:|
| reymont-prefix-32769 | 267 | 548 |
| 33027-periodic-window | 68 | 3 |
| repeated-32769 | 68 | 3 |
| random-32769 | 195 | 790 |
| 16384-distribution-change | 227 | 819 |
| eight-literals | 83 | 3 |

## All 20 matched elapsed results

The normal compiler, frozen R221 and R222 run sequentially in 24 balanced order blocks. Each block sends 512 calls for clock-limited stored cases, 64 for other inputs at most 1024 bytes and one for larger inputs. Request batching remains outside the unchanged per-call timer. Every call verifies exact raw reference bytes, independent inflate and held aliases. No allowance or expected value increases.

| Input | Normal R215 / Roc | R221 / Roc | R222 / Roc | Block-mean change vs R221 |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.445842× | 2.657029× | 2.456952× | -7.530% |
| 33027-periodic-window | 3.642628× | 2.418937× | 2.189403× | -9.489% |
| repeated-32769 | 3.129635× | 2.236612× | 2.045910× | -8.526% |
| random-32769 | 5.061611× | 3.977705× | 3.073894× | -22.722% |
| 16384-distribution-change | 6.554198× | 3.732273× | 3.191886× | -14.479% |
| eight-literals | 4.448211× | 0.760708× | 0.538732× | -28.578% |
| reymont-prefix-16384 | 4.185662× | 2.648160× | 2.462527× | -7.010% |
| empty | unresolved | unresolved | unresolved | +1.117% |
| one-stored | unresolved | unresolved | unresolved | +4.683% |
| seven-stored-prefix | unresolved | unresolved | unresolved | +4.295% |
| eight-overlap | 4.354167× | 0.763963× | 0.540484× | -29.330% |
| 320-overlap | 4.024028× | 0.810277× | 0.605833× | -24.848% |
| eight-literals-prefix | 4.345329× | 0.749569× | 0.533621× | -29.007% |
| 128-biased-dynamic | 3.678293× | 0.700031× | 0.548310× | -21.854% |
| 32-high-entropy-stored | 3.977728× | 0.779526× | 0.575145× | -26.258% |
| random-64 | 4.020726× | 0.823756× | 0.611563× | -24.553% |
| random-256 | 4.099832× | 0.969099× | 0.721128× | -25.254% |
| random-1024 | 3.710418× | 1.014985× | 0.813442× | -20.179% |
| repeated-1024 | 4.018112× | 0.973278× | 0.757725× | -22.076% |
| reymont-prefix-1024 | 4.628002× | 1.280342× | 1.042167× | -17.877% |

161832 complete Mech calls / 161712 retained observations and 5120 fresh reference compressions. All 20 original per-call-median 1.05 gates and all 20 supplementary block-mean gates pass. 17 medians fall, 3 equal and none rise. Equal-case geometric block-mean ratio is 0.828500573; raw-median geometric ratio 0.823044392 is separately retained. Three stored-only reference medians round to zero, so their multiples remain unresolved.

## Independent counters and memory

Separate 3120 Mech CPU/heap calls and 1280 fresh reference windows covering 10240 verified compressions. No counter-overhead subtraction or multiplication of independent cohorts.

| Input | Instructions R221 → R222 | Instruction change | R222 / Roc instructions | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 159,753,948.0 → 164,122,373.5 | +2.734% | 3.449343× | -7.246% |
| 33027-periodic-window | 53,102,035.5 → 54,807,676.5 | +3.212% | 2.844951× | -9.443% |
| repeated-32769 | 52,574,616.0 → 54,251,102.0 | +3.189% | 2.837148× | -8.699% |
| random-32769 | 68,595,976.5 → 62,784,103.0 | -8.473% | 5.342064× | -22.469% |
| 16384-distribution-change | 46,512,483.0 → 43,785,689.0 | -5.862% | 4.673762× | -13.805% |
| eight-literals | 1,946,016.0 → 1,410,419.5 | -27.523% | 1.033750× | -31.134% |
| reymont-prefix-16384 | 80,874,843.0 → 82,082,916.0 | +1.494% | 3.307243× | -7.125% |

Instructions fall on 13 cases and rise on 7. Cycles fall on 18 and rise on 2 (empty, one-stored). Natural32, periodic and repeated execute roughly 2.7–3.2% more instructions despite lower elapsed. These adverse counters remain part of the retained experiment.

All 20 global allocation-request/requested-byte/reallocation/incremental-usable-peak measurements, and all payload/copy/zero/peak counters match R221. Natural32 remains 608 requests, 9687324 requested bytes, 9750840 incremental usable-peak bytes and 14932 tracked writable-copy bytes. Eight literals remains 230 requests. Requested capacity, initialized copying, usable live peak, RSS and stack are distinct; writable-helper copy counters exclude generated stores/direct memmove. Physical final-output backing reuse remains unestablished.

Root code grows 5611688 → 6762272 bytes (+1150584); root stack 61552 → 86784 bytes (+25232). All 127 new proofs have source fallback bodies retained. The latency win carries a substantial static code/stack cost; no claim of universally fewer instructions or lower total memory is made.

## Qualification and current profiles

- 640 original/decoded before/after array and escaping-record controls plus 320 private-record controls pass. Actual selected and shared-fallback entries execute. Inputs change across calls; checked errors, prior publication/backing, aliases, cleanup and retry are checked. Public record escape remains ineligible; projected private record fields select the existing proof.
- All 2560 existing ownership controls pass. The resident comparisons retain 464 inherited default-cap denials as denials, not successful resident outcomes.
- All 48 unchanged real-capacity buffer controls pass, including reserve/append/clear, 305257-U64 path capacity, U8 capacity 13, growth, live aliases and errors. Hardcoded values and allocation/copy allowances are unchanged.
- All 47 newly emitted small original/decoded Object-and-metadata pairs match. Actual Bool/U64/I64/F64 scalar-control Objects match R221 byte-for-byte, with no scalar retirement or new scalar timing claim.
- 40 complete-stream smokes and 40 separate observer calls pass. Full streams use strict decoded MBC14; original/decoded execution is on the named small controls. No multi-MB run, ten-workload suite or fresh broad dependency rebuild.
- Fresh exact-executable profiles execute 24035 wall-profile calls and 96779 PC-profile calls across six cases. Large generated-exclusive weights remain about 94–97%; the existing phases 15296/2624 and the outer function remain the main targets. Timer samples are not retired-instruction shares or wholly removable overhead. Each capture resolves its own actual load slide; no old root source map is reused.
- The final eight-byte PC capture exposed synchronous request/reply pipe deadlock as the faster case needed 78433 queued bytes. The interrupted controller/worker are identified, the failure receipt is preserved, and only the sixth capture is resumed with concurrent delivery/draining. The first five verified captures stay. The initial PC session did not exit gracefully; the resumed worker and wall/primary/counter/observer sessions do. Completed compression calls enforce private cleanup. This diagnostic-harness repair does not alter primary timers or compiler execution.
- Warnings-as-errors stay enabled. Normal R215 source remains unchanged; its historical 678 tests, 52 linked checks, 15 architecture checks, 132 workflow checks and seven inherited exclusions are not claimed freshly repeated. Fresh maintained layout/index/diff checks and exact-head CI/review disposition are in the publication receipt. Existing draft PR only.

## Newly reproduced inherited matrix failure

An additional generic-shape control stops the broad matrix qualification on the frozen R221 parent. A legal 2×3 row-major backing `[1,2,3,4,5,6]` is read in Mech column order. Starting at ordinal 1 and adding two values to seed 8 must produce 14; resident execution and the independent column-major oracle agree. R221 returns 13. The private R222 numeric entry returns 14, while a shared-alias case that first falls back to the root and then enters the native loop returns 12. Original and decoded artifacts agree in all eight controlled probes.

The underlying defect is inherited: `compact/emit.rs::Recipe::Read` directly scales the logical ordinal, without canonical matrix translation. The newly selected route changes where correct reads occur after fallback; it does not make that fallback generally correct. Inspection also finds direct ordinal scaling in `compact/arrays.rs::MatrixIndexedReplace`; that needs its own minimal executed witness before repair. The compressor's 20 inputs and internal buffers are vectors, so the verified streams and measurements above remain applicable. Normal maintained source is unchanged. General compact qualification is withheld.

The broad planned matrix suite did not complete; its clean-parent first negative receipt and subsequent eight controlled probes are retained. An initial start-zero alias probe succeeded after its first copy made the current owner private; the nonzero-start reproducer records the actual failure without assuming every visit takes fallback.

## Retention and next step

Retain this measured vector-compression experiment with its explicit inherited multidimensional limitation. All20 timing/stream gates pass, balanced block means fall 17.150%, and heap/copy counts stay equal. Preserve the code/stack increase and seven adverse instruction cases. Normal R215 and whole-native ordinary qualification remain separate; parity is unachieved.

**Next fix the inherited matrix fallback before further optimization or ordinary adoption.** Reuse/factor the existing canonical indexing helper in `codegen/numeric_words.rs`, after the original one-based bounds checks. Supply a minimal multidimensional indexed-write witness, then cover 2×3/3×2/row/column/empty shapes, changing starts, selected/native and shared fallback, error order, aliases, previous outputs and retry on original/decoded artifacts. Repeat all20 streams/timings/counters/profiles. This is the same packed read/indexed-update operation used by compressor inputs and frequency tables; it adds no source rewrite or new proof engine.

Then qualify a hot single-loop class through ordinary admission/ownership/funding, checking its current ordinary selection first. The complete diagnostic remains unqualified for those contracts.

Replay only retained observations with `python3 heap-summary.py`, `python3 analyze-phase-census.py`, `python3 analyze-samples.py`, and `python3 analyze.py`. Exact native commands, feature profiles, source pins and measured patches are in the archive. Numerical replay does not execute a compiler or compressor.

## Public evidence

[Exact measured source, all20 timings/counters, current profiles, passing vector controls and inherited matrix repro](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r222-single-loop-native-9a8c862c0a70.tar.gz) (2867639 bytes; 1536 files). SHA256 `9a8c862c0a709f7e0b7b49663cbfa318a91b26a234cf002b1c16ef6ee31c6ac6`. The final publication receipt records downloaded all-file hashes, four numerical replays, exact published head, CI/review disposition and the full source-tree pin.
