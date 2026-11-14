# R200: short ordered reduction experiment

**Reject the four-candidate SIMD cutoff and retain the expanded independent-oracle tests.** Production stays at the reviewed R199 compiler, IR/work/ABI/bytecode **214/107/39/14**. The experimental image used **215/107/39/14** and is not selected. Parity is incomplete.

Base: `7748204bd12e57a20369ce7970f31f8d15c3901f`, clean review in [comment 6715305806](https://github.com/mech-lang/mech/pull/973#issuecomment-6715305806). The final patch changes two existing test tables only. Both experimental compiler files are restored byte-for-byte to the reviewed parent; their candidate snapshots and patch remain in evidence. The existing faster source stays retained; no state-machine source is adopted.

## Complete compression timings

All twenty original 1.05 gates pass, but **nine elapsed medians improve and eleven worsen**. Geometric elapsed changes **+0.066724%**. Natural 32/16 KB, repeated 32 KB, periodic 33 KB and distribution change all have higher elapsed medians. This fails the intended complete-program improvement. Sixteen lower instruction medians remain useful independent evidence; they do not establish a timing win. No unchanged confirmation run selects a more favorable result.

| Input | Retained / Roc | Candidate / Roc | Elapsed change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 3.347987× | 3.373958× | +0.775705% |
| random-32769 | 4.729088× | 4.727620× | -0.031024% |
| 16384-distribution-change | 6.184058× | 6.190140× | +0.098349% |
| repeated-32769 | 2.870054× | 2.886755× | +0.581905% |
| 33027-periodic-window | 3.359533× | 3.373970× | +0.429732% |
| eight-literals | 4.435408× | 4.439590× | +0.094279% |
| reymont-prefix-16384 | 4.047769× | 4.081688× | +0.837971% |
| empty | unresolved | unresolved | -0.303267% |
| one-stored | unresolved | unresolved | -0.518369% |
| seven-stored-prefix | unresolved | unresolved | -0.086682% |
| eight-overlap | 4.264458× | 4.267521× | +0.071832% |
| 320-overlap | 4.113055× | 4.104583× | -0.205970% |
| eight-literals-prefix | 4.455383× | 4.403499× | -1.164530% |
| 128-biased-dynamic | 3.781177× | 3.756149× | -0.661904% |
| 32-high-entropy-stored | 4.063729× | 4.127819× | +1.577130% |
| random-64 | 4.136678× | 4.149078× | +0.299749% |
| random-256 | 4.255156× | 4.236040× | -0.449263% |
| random-1024 | 3.823086× | 3.799269× | -0.622994% |
| repeated-1024 | 3.982596× | 3.987869× | +0.132398% |
| reymont-prefix-1024 | 4.434220× | 4.457120× | +0.516425% |

Eight balanced blocks place each side first four times: 3,968 primary observations and 4,048 complete calls, with exact reference bytes, independent inflate, logical work, aliases, capacities and cleanup agreement. Each pair uses its own fresh shared reference denominator; independent rounds are not compounded. Three zero reference medians remain unresolved. The retained binary is exactly the R199 executable, so between-cohort shifts are not attributed to a new compiler change.

## Mechanism and measured limits

The existing complete-child proof admits immutable cost reads, source prices/visits and strict earliest-index selection. The candidate changes the generic vector cutoff from sixteen candidates to four. Its scalar tails, current window checks, finite funding, source arithmetic, failure order, fallbacks and publication remain. It does not add readable capacity or an authority cache.

Actual complete Object comparison finds **exactly four changed instruction words**, the four threshold comparisons. Accounted code remains 8,152,436 bytes, the text section including its existing padding 8,152,756 bytes, and reported stack 16,480 bytes. Metadata changes only in identity/IR fields. Source and bytecode remain identical. The backend did execute different reduction choices; there is no code-size gain.

| Input | Instructions retained | Candidate | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 208,197,345.5 | 206,814,387.0 | -0.664254% | +1.035895% |
| reymont-prefix-16384 | 118,861,897.5 | 118,063,033.0 | -0.672095% | -0.263226% |
| 16384-distribution-change | 64,686,590.5 | 64,687,877.0 | +0.001989% | -0.262159% |
| repeated-32769 | 66,842,946.5 | 66,842,865.5 | -0.000121% | +0.439269% |
| 33027-periodic-window | 72,490,042.5 | 72,489,838.5 | -0.000281% | -1.429055% |
| eight-literals | 12,432,490.0 | 12,432,357.5 | -0.001066% | +1.639479% |

Separate sixteen-observation-per-side CPU/heap cohorts cover all twenty inputs. Sixteen instruction medians fall and four rise; four cycle medians rise. Natural 32 KB executes about 0.66% fewer instructions while taking about 0.78% more primary elapsed time and 1.04% more cycles in their separate cohorts. No microarchitectural cause is established. All twenty request/byte/reallocation/incremental usable-peak medians are equal; no allocation, copy-volume or streaming improvement is claimed. Capacity-level output clear/preparation reuse remains; physical zero-allocation reuse is not claimed.

A fresh exact R199-worker natural-text PC profile completes 832 correct calls. Only its independently byte-matched 4,016-byte phase uses retained source markers. There are 5,567 Running/Timer-Fired samples, with 2,343 in that phase. Counts include scalar body 195, scalar backedge 15, scalar finish 377, vector body 13, vector finish 6, entry 167 and setup 57. Other marked phase work and unmarked/native/host work remain separately retained. These are PC samples, not candidate visit counts, retired instructions or additive category medians.

The candidate has **6,673** correct calls in four fresh wall-stack profiles, ending at zero managed bytes/nodes. Generated-exclusive shares are 64.73% periodic, 69.30% repeated, 77.67% natural and 9.17% eight-byte. The exact R199 baseline stack profile is explicitly reused; primary and counter before/after cohorts are fresh. Emitted word helpers count as native. Host frames include useful work. Stack shares are not multiplied into instruction totals.

## Retained tests and qualification

The existing independent scalar oracle now covers every short length 0–20 (plus retained larger lengths), all four winning lanes, incoming/cross-group ties, ignored high halves and U32 wrapping. Current-geometry controls add nonzero starts around four/eight candidates and a complete reverse group ending at the physical allocation boundary, for all forward/reverse U32/U64 width pairs and changing current lengths. The two expanded controls execute 6,384 and 2,688 oracle comparisons respectively across ordinary/optimized, original/decoded and direct/restored execution. No expected value or allowance increases.

The candidate passes 675 native tests with the same seven inherited exclusions, all 258 actual CI selections, fourteen ordered/child/exhaustion controls inside that suite, unchanged 236 component / eight composition / one real-capacity qualifications, generic original/decoded state failure/recovery, fifteen architecture/catalog checks and 132 workflow tests. Runtime-only, formatting and warnings-as-errors pass. After restoring production, a freshly compiled final test root passes all fourteen focused controls. The wider final-production suite is the unchanged reviewed R199 coverage, not a newly rerun 675-test receipt. The 1,724 native match controls are retained baseline coverage, not fresh candidate execution.

Profiles use the same aot/jit/canonical-native/program/runtime feature profile, opt3 libraries / opt1 test roots, rustc nightly 2026-03-03, Apple M1 / 8 GiB, frozen R145/R196 dependencies and pinned mimalloc 2.1.7. Exact command arrays and source hashes remain. No build, qualification, archive or profiler competes with primary or CPU/heap measurement. A failed formatting check is corrected before compilation; a premature analyzer call before the final focused receipt is corrected after that handle completes. No unresolved introduced functional failure remains. Wider dependency/all-features/registry-project, multi-megabyte, separate-workload, standalone scalar-latency and new full state-source timing remain skipped.

Source SHA-256 stays `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The complete level 12 algorithm, tables, depth 300/nice 258, original EOF/search rules, tree 262,144 U16 / cache 1,500,513 U32 / path 305,257 U64 capacity, source allocation sites, aliases, output clear/preparation and complete evaluate_into boundary stay pinned. Preparation, image restoration, validation/drop and output-slot handling stay outside that boundary as before.

## Next experiment and replay

The user requested a matched optional LLVM backend comparison using #990. Its published related-post measurements on M3 are not DEFLATE numbers. That comparison uses a separate checkout of the exact LLVM implementation, an explicitly bound M1 target and matched backend/runtime controls. No LLVM source or dependency is adopted by this patch, and no LLVM speedup is claimed here.

Run heap-summary.py then analyze.py after unpacking. They reproduce the numerical result from retained observations without running a compiler or compressor. Native binaries, bytecode, executable descriptors and raw trace stores are excluded from public evidence. Raw traces remain local. Existing draft #973 only; ordinary exact-head CI is cancelled and review requested after publication.

[Immutable evidence and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r200-short-reduction-cutoff-014ddff45193.tar.gz) — SHA-256 `014ddff4519347f727664c6b5b6f2aece97be07fc3d35ca2fc3a9767c5ec5c03`, 3,501,725 bytes / 387 hash-listed files. A fresh public download verified every file and reproduced results.json byte for byte without compiler or compressor execution.
