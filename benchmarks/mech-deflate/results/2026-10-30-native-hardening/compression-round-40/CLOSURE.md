# Reject scalar aggregate grouping after matched executable measurements

**Keep the published compiler.** Both proposed selections execute more hardware
instructions and regress all four independently prepared matched triplets. No
new compiler/test/CI change is retained. The previous round's confirmed 2.674%
time and 3.316% instruction improvement remains intact. Nothing is rejected for
being a small positive gain; these are measured slowdowns.

## Exact revisions and disposition

Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Published starting head: `39f0a3f662cc5b4608b5b2372955bf10992c6cbd`.
Frozen baseline compiler source: `552e537fcbcfc3811ce442c982010b5e1e999e6c`.
First valid candidate: `411c2e1f01d9435a1c78d28f3a0a123dcd2ba56e`.
Regrouping refinement: `97ab5892d136450fe260af8c16956f23488a7ec8`.
Restore: `f06fe15783556a7387c6575c29181dd41ab44f4a`.
The restore's compiler, maintained tests and CI workflow are byte-identical to
the starting published head. The net review diff consists of this experiment
packet. IR/work/ABI/bytecode remain **131/95/37/14**.

`CANDIDATE.md` and `REFINEMENT.md` preserve the rationale, intermediate failures
and proof boundaries. Full applyable baseline-to-candidate source/test patches
are archived in `rejected-source-patches/`, so removing the unsuccessful code
does not discard reproducibility. The exact unchanged Mech source and bytecode
are included too. Every observation cohort is committed immediately.

## Matched elapsed results

Complete level-12 compression of `abcdefgh`, exact ten raw bytes. The full
compressor is linked; the same tiny seed/clear JIT helpers stay outside the
unchanged `evaluate_into` timer. All three worker bodies normalize identically
apart from their checked same-build image/identity binding. No JIT-to-linked
gain is credited. All are ready before measurement; only the requested worker
runs. No build, test, audit, profile or instrumented diagnostic overlaps primary.

| Four prepared triplets | Median | Shared Roc 0.291 ms = 1× | Versus baseline |
| --- | ---: | ---: | ---: |
| Published baseline, retained | 4.3870830 ms | **15.07589×** | — |
| First valid candidate | 4.4197915 ms | 15.18829× | **0.74556% slower** |
| Regrouping refinement | 4.4207295 ms | 15.19151× | **0.76694% slower** |

Each candidate loses in all four triplets; only 10/64 descriptive blocks improve
for each. The refinement versus the first candidate is mixed: two improving
triplets, 29/64 improving blocks, +0.02122% pooled. There is no additional gain.
There are **5952 primary calls**, 5976 including warm-up/final, 1984 primary per
version, with one shared 20,000-call reference. Six rotating orders are used.
Blocks within a preparation are descriptive, not independent preparations.

The earlier four-pair cohort is retained separately: 4.370542→4.409583 ms,
15.22837×→15.36440× its own shared 0.287 ms reference (+0.89328%, all four pairs
regress, only 5/64 blocks improve). That is 3968 primary calls and 3984 total.
Both cohorts are preregistered in `MEASUREMENT.md` and `REFINED-MEASUREMENT.md`.
On the fixed historical 0.300 ms ruler the final baseline is 14.62361×. The new
fresh-reference multiple is not a new optimization relative to round 39.

Every call checks the same raw bytes, independent inflation, unchanged input
aliases/contents, output capacity 13 and stable per-compiler work. Output is
cleared and reused with the same boundary. Every worker ends at zero managed
bytes/nodes. The original source algorithm, tables, depth 300, nice 258,
near-EOF limits, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity,
source allocation sites and pinned mimalloc 2.1.7 are unchanged. No larger input
or throughput claim is made.

## Instructions, memory and generated code

These are separate diagnostic cohorts, excluded from primary elapsed values.
CPU medians use 32 steady calls per executable; allocation uses eight steady
Mech calls and 32 reference calls. Hardware `proc_pid_rusage(RUSAGE_INFO_V4)`
instruction/cycle differences include the recorded empty-getter calibration
without subtraction. Every retained CPU observation has zero page-ins. The
initial-candidate CPU/profile capture precedes the refinement captures; cycles
remain diagnostic, not a substitute for matched elapsed timings. MB is decimal.

| Per complete compression | Roc | Baseline | First candidate | Refinement |
| --- | ---: | ---: | ---: | ---: |
| Actual CPU instructions | 1,364,372 | **49,008,490.5** | 49,214,104.5 | 49,203,529.5 |
| Instructions versus baseline | — | — | +0.41955% | +0.39797% |
| CPU cycles | 935,346.5 | 13,808,872.5 | 13,892,111 | 13,887,614.5 |
| Allocation requests | 95 | 13,994 | 13,994 | 13,994 |
| Requested bytes | 9,021,923 | 98,586,843 | 98,586,843 | 98,586,843 |
| Additional peak tracked heap | 9,257,328 | 20,575,432 | 20,575,432 | 20,575,432 |
| Generated compressor text bytes | whole app: 195,456 | 13,885,696 | 13,885,352 | 13,883,072 |
| Checked generated stack bytes | not function-matched | 108,400 | 108,448 | 108,528 |

The retained baseline executes about 35.92× the reference instructions. It makes
147.31× the requests, requests 10.93× the bytes and adds 2.22× the tracked peak
heap. All Mech large-backing histograms stay **35 path / five tree / one cache**.
No compression-memory win is established. Prepared tracked heap is 141,595,920,
141,592,160 and 141,592,256 bytes respectively; these tiny fixed differences do
not justify a time regression. Requested capacity is not payload copied.

Baseline whole prepared footprint is 227,755,776 bytes and RSS 318,717,952 bytes.
This retains setup/compiler/catalog state; it is not a minimal runtime-only
application or incremental compression heap. Tracked heap omits direct VM,
allocator-private metadata and transient internal realloc overlap. Both receive
already loaded whole input and neither streams files during compression. No
total-copy-byte, bandwidth, branch/cache-miss, energy or scaling result is claimed.
Generated entry text versus reference whole-app text has different scopes and
includes cold/error paths; static slots are not executed instructions.

The baseline work receipt is 60,260,219,686; candidates are 60,260,484,982 and
60,260,490,614. These conservative logical units are not retired instructions.
Prices and source allowances were not changed.

## What the experiment established

The original executed census reaches 917 already-native scalar aggregate steps
at 99 sites. Recognizing their proof alone is not enough: ordinary groups and
numeric pieces need one entry authority. Deleting complete intersecting groups
lost useful grouping; rederiving groups around selected pieces restores the
exact tuple/enum state packs after the path-scan prefix. It still does not beat
the baseline. Static grouped sites are 8642 / 8406 / 8414, with 67 native pieces
in each version. The instruction counters and both elapsed cohorts reject this
as a useful optimization at this checkpoint.

Fresh linked profiles retain 1301 / 1183 / 1014 exclusive ancestry weights.
Buffer/frame actions account for 340 / 336 / 234 (baseline about 26.1%). These
695-call captures include surrounding harness work and are not precise timer
percentages or paired samples. `analyze.py` partitions the raw call graphs again.
The unchanged reference's direct tree/cache initialization stores remain
documented in `../deep-profile/REPORT.md`.

The next bounded target is ordinary managed-array projection transport. A fresh
analysis of the SAME previously executed baseline census distinguishes **2173
managed unsigned-matrix projections, 1829 scalar projections and 13 other
projections**. The managed counts are U8 460, U16 50, U32 1110 and U64 553.
There are 67 adjacent same-source structural managed-projection runs that could
combine at most 706 per-field admissions and their 706 helper calls. This is a
static opportunity ceiling, not an accepted selector or promised speedup.

Before implementation, use the real function/header binding operation as the
reproducer; prove exact entry/frontiers and retirement, U8/U16/U32/U64 values,
aliases, finite-denial fallback, error precedence, publication/cleanup/retry and
original/decoded/linked agreement. Use the existing owner/value bank and ordered
storage adapter, preserving direct scalar emission. Do not add an evaluator,
change the compressor, or substitute matrices/concatenation/fresh output buffers.
`targets.py` reproduces the count from the archived census and same-build image.

## Validation and every failure's disposition

Both valid experimental sources pass **589 selected qualification cases**:
314 runtime, 31 work/memory/value, 234 existing component cases, eight connected
composition cases and two real path mechanism cases. Original/decoded and
checked-image cases, changing Bool/U64/I64/F64 arguments (including NaN and
negative infinity), interior arithmetic origin, assertion-before-overflow/
division, one-short group funding, unchanged outputs/aliases and retry pass.
The old Huffman work receipt stays 91,452,918; no old golden or allowance changes.

The temporary CI selection lists and runs all 19 cases; 132 configured Python
workflow/inventory checks pass. Its separate commit is `d00d56c0b`. It is restored
with the unsuccessful feature, leaving the published strict inventory of 15.
Final restored-source equality, actual 15-case inventory/execution, format and
diff checks pass against the frozen published test root. No new production build
or broad rerun is needed to claim a docs-only final diff. The prior round's full
baseline qualification/control/architecture receipts remain authoritative.

Release builds take 75.182 s / 74.382 s, compiling only the GPU host and native
profile harness. All seven non-host library hashes/features/profiles are
identical to baseline, including Engine/math. Rust nightly 2026-03-03
(`ec818fda361ca216eb186f5cf45131bd9c776bb4`), LLVM 22.1.0,
aarch64-apple-darwin; release opt 3, maintained unit/component roots opt 1,
warnings as errors, incremental off, locked/offline, one build job. Exact
commands/configurations/hashes/stdout/stderr/counts are archived. Hardware is
Apple M1 Mac mini, 8 GiB RAM, macOS 15.6.1 (24G90).

Introduced failures are preserved: the first new test adapter used f64 instead
of canonical F64Bits; the first selector bypassed a grouped entry (two old tests
failed); cutting every entry disabled old Huffman pieces (work 229,673,588 versus
91,452,918); declining every partial overlap selected no piece in the old Boolean
witness. Each was corrected before the valid measured variants. The strengthened
real state-pack grouping test fails on the first valid candidate and passes on
the refinement. All experimental source/tests are ultimately removed because of
the measured regressions, not hidden by changing expectations.

The linker failed with errno 28 at 193 MiB free: environmental. Thirty obsolete
executables were deleted, releasing 3,638,123,584 file bytes; completed diagnostic
payloads were later removed after capture, while sources/raw results/hashes and
matched workers/libraries remained. The unchanged build then succeeds. A
dependent launcher also reported the absent executable after the failed link;
it produced no benchmark observation. A first restoration archive preceded the
format check's completion; `restored-final/` captures the full completed receipt.
No unresolved introduced correctness failure remains in the restored source.

Scalar/frame latency controls and the ten architecture audits are not newly
claimed for the rejected variants. No Core/Engine/all-feature broad suite,
unrelated target or larger full-compressor input is newly claimed. The inherited
reduced-catalog knucleotide case remains excluded. These omissions are explicit;
passing targeted tests is not reported as green hosted CI or achieved parity.
PR #973 remains draft; hosted CI is cancelled by user direction and a new review
is requested after publication.

`scorecard.json`, `scorecard.csv` and `targets.json` retain all candidates and
scopes. Run `python3 reconstruct.py EMPTY_SCRATCH_DIRECTORY` to verify every
archive's individual paths/lengths/hashes and regenerate all three byte-for-byte,
without a compiler, network or benchmark execution. `reconstruction.json`
records the observed verified inventory. Parity remains outstanding at 1×.
The independent run verifies **68 archives, 475 file occurrences and 368 unique
paths**, and reproduces both scorecards and the projection analysis byte-for-byte.
