# R263: reuse guarded index bounds in native phases

**Retain this experimental compiler improvement.** The geometric mean across
all 81 complete compression cases improves by **1.188790%** in the
within-block geometric comparison. Both existing 1.05 gates pass 81/81 cases.
This is a small computational gain, not parity: **22/81 configurations are below
Roc before and after; no new configuration crosses 1x.** Some small cases become
slower, and all contrary observations remain.

Published parent: `f62e16365f25a1e77988b73d22f9216e76183436`.
PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
The clean exact-head review of the parent is comment 6774340485.
The source starts from R261's repaired 825-file compiler closure; ten existing
files change, with no new source files. The complete reversible patch is
`compiler.patch`. IR/work/ABI/bytecode remain **215/107/39/14**. The R255 backend
and dependency pins are unchanged. This experimental source closure remains
separate from ordinary managed compilation.

## Priority: the cases closest to parity

These are the nearest controlled cases still above 1x. The next bounded
optimization should target the repeated and periodic cases near 32 KiB while
continuing to time every case and protecting the existing below-reference
configurations. Additional reduction means the time cut needed from R263 to
reach 1x; it is not a predicted gain.

| Input | Bytes | R261 / Roc | R263 / Roc | Time change | Additional cut to 1x |
|---|---:|---:|---:|---:|---:|
| repeated | 32767 | 1.051163x | 1.020506x | -2.9165% | 2.009% |
| repeated | 32769 | 1.052589x | 1.025549x | -2.5688% | 2.491% |
| repeated | 32768 | 1.055728x | 1.027648x | -2.6598% | 2.690% |
| repeated | 33027 | 1.073708x | 1.041968x | -2.9561% | 4.028% |
| periodic | 32768 | 1.089061x | 1.050873x | -3.5065% | 4.841% |
| periodic | 32767 | 1.092386x | 1.055233x | -3.4011% | 5.234% |
| periodic | 33027 | 1.096717x | 1.061571x | -3.2046% | 5.800% |
| periodic | 32769 | 1.094361x | 1.062217x | -2.9372% | 5.857% |

## What the compiler proves

The actual matchfinder masks a table index, doubles it and adds a bounded
one-based offset. Its lexical table-length assertion supplies a minimum current
length. The compiler can prove this static index range against that minimum
without repeating a bounds comparison on every access. There is no compressor
name, field name, input family, size, source site or compression-path selection
in the compiler rule.

The compact phase uses its existing entry guard to check the current view's
length and vector geometry. A refused guard resumes the original checked source;
it does not report an early assertion failure or promote a lexical assertion to
a document invariant. Direct canonical lexical assertion branches derive only
unsigned literal lower bounds, with checked strictness. Unsupported assertion
forms decline. Current table checks still precede the original wrapping base
arithmetic, checked window arithmetic and subsequent loads.

The new static index analysis follows unique scalar/copy definitions and
literals for bit-and and wrapping add/subtract/multiply. It uses an iterative
worklist and conservatively rejects cyclic, ambiguous, structural and opaque
definitions. Wrapping multiplication yields a bounded product only when its
entire unsigned interval fits; otherwise its upper bound is the full unsigned
range. A one-based access must have lower bound at least one and upper bound no
greater than the current guarded minimum. It does not carry runtime pointer,
content, alias or ownership facts across a child computation.

Length floors may coexist with a cold fallback only when the existing exact
private-phase contract permits the proof and exports every unsupported site.
That handoff returns before unsupported work; the next entry checks fresh
current views. Length-changing byte append remains excluded. Existing successful
growing append, pointer refresh, uniqueness/first-change checks and scalar
operation execution are retained. Bounds proofs apply to writes as well as reads;
comparison and store still use the shared canonical linear ordinal from R261.

The private diagnostic flags are `MECH_PHASE_ASSERTION_FLOORS=1` and
`MECH_TRAVERSAL_WRITE_BOUNDS=1`, in addition to the retained flags in `flags.json`.
Disabling the new experiment restores the R261 Object exactly. Additional
metadata proof lists are expected, so metadata identity is not claimed there.

## Actual emitted work

Of 341 common root/numeric definitions, 338 retain identical Object body bytes.
Only the root and two normal/vector matchfinder definitions change. Each matching
body gains **20 read bounds proofs and 15 write bounds proofs** (reads 29 to 49;
writes zero to 15). The path phases retain their 2048-byte bodies and 288-byte
frames; the natural-text path bottleneck is not claimed solved.

| Definition | Code before | Code after | Frame before | Frame after |
|---|---:|---:|---:|---:|
| `_mech_compact_closed_v2` | 1040856 | 1040404 | 57088 | 57056 |
| `_mech_compact_numeric_837` | 54748 | 53500 | 928 | 944 |
| `_mech_compact_numeric_837_vector` | 42800 | 41456 | 928 | 944 |

The three code bodies shrink by 3,044 bytes in total. Object size changes
1,478,952 to 1,475,808 bytes;
linked worker size changes 2,100,808 to
2,084,296 bytes. The worker's 16,512-byte difference also
includes alignment; it is not all saved instructions. Matchfinder stack frames
increase 16 bytes; the root frame decreases 32 bytes. Other linked addresses may
move; there is no fixed-placement causal attribution.

## Matched complete-call timing

Both sides use fixed level 12, runtime input, pinned allocator and clock,
matched 200-ms warmups, borrowed output ownership and the same validation
placement. Full compression, scratch construction, output transfer and private
cleanup are inside the timer. Input preparation, validation and external output
slot lifetime are outside on both sides. Two fresh primary cohorts contain all
six actor orders repeated four times: **48 balanced blocks per case**. Ratios
are formed inside each block before geometric aggregation. The old pooled
median and block-median 1.05 gates remain unchanged. No valid observation is
discarded or rerun, and historical denominators remain in prior reports.

The 81-case geometric time change is **-1.188790%**.
This gives each configuration equal weight; it is not aggregate time for a
representative workload mix. Sub-microsecond stored cases remain close to clock
resolution. Negative changes below mean faster; a ratio below 1x means faster
than Roc. Every configuration appears exactly once in the following five tables.

### Original 25 cases

| Input | Bytes | R261 / Roc | R263 / Roc | Time change |
|---|---:|---:|---:|---:|
| empty | 0 | 2.100928x | 2.121918x | +0.9991% |
| one-stored | 1 | 2.009844x | 2.029427x | +0.9744% |
| seven-stored-prefix | 7 | 2.220968x | 2.270008x | +2.2081% |
| eight-literals | 8 | 0.414859x | 0.420113x | +1.2663% |
| eight-overlap | 8 | 0.414062x | 0.421616x | +1.8244% |
| 320-overlap | 320 | 0.450602x | 0.456557x | +1.3215% |
| eight-literals-prefix | 8 | 0.410375x | 0.424771x | +3.5080% |
| 128-biased-dynamic | 128 | 0.419347x | 0.423464x | +0.9818% |
| 32-high-entropy-stored | 32 | 0.418766x | 0.432079x | +3.1792% |
| 16384-distribution-change | 16384 | 1.880897x | 1.855676x | -1.3409% |
| 33027-periodic-window | 33027 | 1.096791x | 1.064758x | -2.9206% |
| random-64 | 64 | 0.433115x | 0.450265x | +3.9597% |
| random-256 | 256 | 0.473083x | 0.485760x | +2.6795% |
| random-1024 | 1024 | 0.534467x | 0.543126x | +1.6200% |
| repeated-1024 | 1024 | 0.522752x | 0.526221x | +0.6637% |
| random-32769 | 32769 | 1.808720x | 1.787068x | -1.1971% |
| repeated-32769 | 32769 | 1.049401x | 1.025701x | -2.2584% |
| reymont-prefix-1024 | 1024 | 0.674648x | 0.680167x | +0.8181% |
| reymont-prefix-16384 | 16384 | 1.765541x | 1.760433x | -0.2893% |
| reymont-prefix-32769 | 32769 | 1.831180x | 1.824221x | -0.3800% |
| heldout-window-32767 | 32767 | 1.835187x | 1.829969x | -0.2843% |
| heldout-window-32768 | 32768 | 1.833248x | 1.825499x | -0.4227% |
| heldout-slides-65537 | 65537 | 1.900209x | 1.889706x | -0.5527% |
| heldout-random-65537 | 65537 | 1.980045x | 1.952101x | -1.4113% |
| heldout-mixed-131073 | 131073 | 1.914986x | 1.862992x | -2.7151% |

### Controlled natural prefixes

| Bytes | R261 / Roc | R263 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.673115x | 0.682669x | +1.4193% |
| 4096 | 1.323234x | 1.300824x | -1.6935% |
| 8192 | 1.590129x | 1.576312x | -0.8690% |
| 16384 | 1.761683x | 1.753969x | -0.4379% |
| 32767 | 1.829912x | 1.825949x | -0.2166% |
| 32768 | 1.836182x | 1.832518x | -0.1995% |
| 32769 | 1.829457x | 1.825492x | -0.2167% |
| 33027 | 1.828243x | 1.822340x | -0.3229% |
| 65535 | 1.905066x | 1.873334x | -1.6657% |
| 65536 | 1.907484x | 1.875407x | -1.6817% |
| 65537 | 1.911922x | 1.903207x | -0.4559% |
| 131072 | 1.898406x | 1.880313x | -0.9531% |
| 262144 | 1.903724x | 1.893498x | -0.5372% |
| 1048576 | 1.902061x | 1.896635x | -0.2852% |

### Controlled random prefixes

| Bytes | R261 / Roc | R263 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.534120x | 0.542053x | +1.4853% |
| 4096 | 0.963821x | 0.912740x | -5.2999% |
| 8192 | 1.211215x | 1.165534x | -3.7715% |
| 16384 | 1.550805x | 1.520843x | -1.9321% |
| 32767 | 1.822619x | 1.794675x | -1.5332% |
| 32768 | 1.824029x | 1.786578x | -2.0532% |
| 32769 | 1.815876x | 1.791686x | -1.3321% |
| 33027 | 1.820359x | 1.790438x | -1.6437% |
| 65535 | 1.991200x | 1.968147x | -1.1577% |
| 65536 | 1.991144x | 1.969138x | -1.1052% |
| 65537 | 1.984390x | 1.959851x | -1.2366% |
| 131072 | 2.080198x | 2.064136x | -0.7721% |
| 262144 | 2.123520x | 2.108392x | -0.7124% |
| 1048576 | 2.167748x | 2.158164x | -0.4421% |

### Controlled periodic prefixes

| Bytes | R261 / Roc | R263 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.519691x | 0.528804x | +1.7535% |
| 4096 | 0.768891x | 0.744802x | -3.1330% |
| 8192 | 0.876431x | 0.855406x | -2.3989% |
| 16384 | 0.981293x | 0.953578x | -2.8243% |
| 32767 | 1.092386x | 1.055233x | -3.4011% |
| 32768 | 1.089061x | 1.050873x | -3.5065% |
| 32769 | 1.094361x | 1.062217x | -2.9372% |
| 33027 | 1.096717x | 1.061571x | -3.2046% |
| 65535 | 1.170246x | 1.137611x | -2.7887% |
| 65536 | 1.172305x | 1.133342x | -3.3236% |
| 65537 | 1.175149x | 1.136399x | -3.2975% |
| 131072 | 1.216380x | 1.177160x | -3.2243% |
| 262144 | 1.239611x | 1.205279x | -2.7696% |
| 1048576 | 1.267936x | 1.221557x | -3.6578% |

### Controlled repeated prefixes

| Bytes | R261 / Roc | R263 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.515927x | 0.514772x | -0.2239% |
| 4096 | 0.731004x | 0.712504x | -2.5308% |
| 8192 | 0.857041x | 0.837247x | -2.3096% |
| 16384 | 0.949188x | 0.922509x | -2.8107% |
| 32767 | 1.051163x | 1.020506x | -2.9165% |
| 32768 | 1.055728x | 1.027648x | -2.6598% |
| 32769 | 1.052589x | 1.025549x | -2.5688% |
| 33027 | 1.073708x | 1.041968x | -2.9561% |
| 65535 | 1.139199x | 1.102919x | -3.1847% |
| 65536 | 1.132094x | 1.099858x | -2.8475% |
| 65537 | 1.133439x | 1.101124x | -2.8511% |
| 131072 | 1.182683x | 1.150376x | -2.7317% |
| 262144 | 1.224568x | 1.168108x | -4.6106% |
| 1048576 | 1.222298x | 1.180133x | -3.4497% |

## Independent instruction, cycle and allocation cohorts

These are separate balanced cohorts, not partitions of elapsed time or CPU-time
samples. Payload allocation counts are not total allocator request counts. The
entire 81-case payload-allocation/requested-byte/copy/zero/private-peak families
are unchanged; final nonreturned private payload remains zero. This compiler
change removes checks rather than allocations.

| Input | Instructions before | Instructions after | Change | Cycles before | Cycles after | Change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 1,137,872.5 | 1,134,680.0 | -0.2806% | 493,153.0 | 497,269.5 | +0.8347% |
| prefix-natural-1048576 | 3,846,926,363.0 | 3,783,730,110.5 | -1.6428% | 979,231,183.5 | 971,771,151.5 | -0.7618% |
| prefix-random-1048576 | 987,887,090.5 | 952,116,793.5 | -3.6209% | 199,829,379.0 | 199,116,693.5 | -0.3566% |
| prefix-periodic-1048576 | 896,573,225.5 | 866,566,952.5 | -3.3468% | 127,042,473.0 | 122,587,082.0 | -3.5070% |
| prefix-repeated-1048576 | 903,749,424.5 | 873,789,340.5 | -3.3151% | 134,035,136.5 | 129,383,548.5 | -3.4704% |

Eight-literal primary time worsens 1.2663%, despite its independent instruction
count declining 0.2806%; its cycles increase 0.8347%. Random-64 worsens 3.9597%
and the eight-literal-prefix case worsens 3.5080%. These do not become gains by
averaging. All 81 existing gates still pass, and all raw observations remain.

## Fresh profiles

Eighteen fresh diagnostic captures cover the baseline, candidate and reference
on eight literals, natural/random 32 KiB, and natural/random/periodic 1 MiB.
Aggregation uses each exact current executable's symbols and PCs. The candidate's
natural 1-MiB path phase accounts for 3156/5365 running samples (58.83%), with
matching at 1868/5365 (34.82%); baseline shares are 57.70% and 35.69%. Candidate
random 1 MiB has 3270/5613 matching samples (58.26%) and 1029/5613 path samples
(18.33%). These samples include harness/validation and are CPU-time shares, not
retired-instruction partitions. Root source attribution remains unknown.

The eight-literal candidate has 3443/5346 external/unmapped samples. That category
is not established host-runtime overhead. Reference hashed native symbols retain
their exact names without invented source attribution. Aggregate PC inputs and
analysis are included; raw trace/XML/stacks are excluded.

Primary A/B and the CPU instruction/cycle cohort finish before profiles begin.
The first diagnostic profile overlaps the tail of the memory-only cohort:
ten packets finish after profile launch. Per-worker allocation/copy/private-peak
counters remain valid, but no isolated RSS or wall-time improvement is claimed
from that overlapping tail. `measurement-overlap.json` preserves the exact cases
and time. No valid sample is discarded or repeated.

## Qualification, commands and limits

- **719 native tests pass, zero fail, seven inherited exclusions remain.** Four
  new compiler unit tests cover lower-bound extraction, checked strictness,
  current-array bounds, zero/outside/wrapped indices and conservative products.
- **3936 linked native calls agree with 3936 fresh resident calls** across seven
  real programs, before/after and original/decoded. Basic masked indices,
  masked products, unmasked/dynamic-mask refusal, upper-bound/no-assertion
  refusal and actual private cold handoff are covered. Current arguments,
  row/column/2D views, fresh recursive visits, unselected paths, overflow,
  assertion/arithmetic/bounds precedence, live aliases, prior publication,
  failure cleanup and successful retry are checked.
- The cold witness uses the existing nested-rebase fixture with minimum-length
  assertions added. It executes actual collection fallback and native re-entry;
  selected visits are 6666/6666/8202/8202 with 66 rejections per form. Tiny toys
  without a parent native phase are not used as handoff qualification.
- **324 complete smokes pass**, covering all 81 cases in original and decoded
  forms with exact reference raw bytes, independent inflate, output/input alias
  controls and final private cleanup. Full enabled Object/metadata pairs match.
  Eight literals retain `[75,76,74,78,73,77,75,207,0,0]`.
- Performance GPU library uses `rustc -C opt-level=3 -D warnings`; native test
  roots use `-C opt-level=1 -D warnings`. Both use the same six features:
  `aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime`.
  Exact commands and environment are in the build-spec and command receipts.
- Three tracked architecture/catalog checks and formatting of all ten changed
  modules pass. No allowance or expected result changes.

```sh
python3 run-spec.py candidate-gpu
python3 run-spec.py candidate-tests
# Exact test flags and seven exclusions: candidate-qualified-tests.json.
python3 qualify-floors.py
python3 smoke-all.py
python3 smoke-decoded.py
python3 analyze-primary.py
python3 analyze-counters.py
python3 make-qualification.py
```

The complete command receipts are authoritative; helper replay can use retained
results and is not a claim to rerun compiler tests. The unchanged maintained
680-test result is inherited from R261, not fresh. Full hosted CI, a fresh full
linked/AOT integration suite and the full 6.6-MB input are not claimed. Compact
measurements retain their established omission of ordinary managed
admission/accounting/publication/recovery costs; production guarantees are not
weakened and their cost is not inferred from these timings.

## Developmental failures and disposition

No introduced functional failure remains in the tested profiles. Intermediate
no-handoff and non-static floor attempts remove no real checks and are not
performance results. A moved-value compile error, fixture frontend schema
mismatches, an unused driver method under warnings-as-errors, a missing boxed
matrix conversion, and stale local pin/image paths are corrected before final
qualification. The same frontend schema rejection is observed on the clean R261
parent; no unrelated compiler repair is added. The stale cohort pin paths reject
preflight before starting workers or recording samples. Original failure receipts
remain. A gzip reader error in offline report construction is corrected without
changing observations. The ten overlapping memory-tail packets remain explicitly
limited above. Seven inherited native exclusions remain assigned, not fixed by
this experiment. The small measured timing regressions remain recorded.

## Source fidelity and replay

Benchmark source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
MBC14 SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
No benchmark source change occurs. The complete level-12 algorithm, tables,
depth300, nice258, original EOF limits, allocation sites and timing boundary are
preserved. Real buffers remain 262144 U16 tree entries, 1500513 U32 cache words,
305257 U64 path capacity and output capacity13 with clear/reuse. The allocator
and reference executable remain pinned in `source-pins.json`.

The packet includes all 825 compiler sources and the exact ten-file patch,
qualified fixture sources and small metadata, raw timing/counter observations,
aggregate profile PCs, development/validation commands and five deterministic
offline reports. Native binaries, bytecode, full compressor metadata/IR/CFG/maps
and raw traces are excluded. `verify-source.py` verifies all hashes and reverses
then reapplies the patch without fuzz against the exact repaired R261 closure.
`replay.py` recomputes primary/counter/qualification results, this report and the
decision without executing a compiler or compressor.

The next implementation target is one measured cost on the repeated/periodic
cases just above 1x near 32 KiB. Full all-81 timing remains mandatory. Large-input
parity and integration of the additional ordinary managed contract remain open.

## Public evidence packet

[Complete compiler patch, all81 timings, profiles and validation receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r263-guarded-index-bounds-4e0c510fac04.tar.gz).
Archive SHA-256: `4e0c510fac04305843a70ca38b61febcb62382ccb12344e1e0acf2e3543179b6`.
The packet contains 2,283 hash-listed files; fresh public retrieval
verifies every file, reconstructs the source in both directions and reproduces
five offline reports. Native binaries and bytecode are excluded.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 263 \
  --output /tmp/mech-r263-review
```
