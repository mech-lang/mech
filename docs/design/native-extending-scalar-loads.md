# R266: direct extending loads, instruction reduction without a latency win

**Keep R264 as the compiler baseline.** The evaluated candidate is
**1.747846% slower** in the all-81 paired geometric elapsed result.
Both fresh primary cohorts are adverse: A
+1.702110%, B
+1.793602%.
The original pooled and block-median 1.05 gates both pass 81/81; those ceilings
are not a speed-win criterion. The candidate is rejected globally. Preserve its
qualified instruction savings and small-input win as an alternative for later
isolated/composed measurements, rather than discarding them because they are small.

## Headline metric and frontier

**Cases faster than Roc: 22/81 = 27.1605%; goal 100%.**
The candidate also reaches 22/81 = 27.1605%:
+0.0000 percentage points. The fixed 81-case inventory is unchanged.
A case counts only when its paired geometric elapsed ratio in the same fresh
balanced blocks is strictly below 1.0. Ties do not count. This is a configuration
count, not byte-weighted throughput or a confidence/significance threshold.
Future chat updates should show the percentage/count, percentage-point movement,
100% goal and compact frontier; the full matrix remains recorded below.

| Case | Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---|---:|---:|---:|---:|
| prefix-repeated-32767 | 32767 | 1.018886x | 1.044322x | +2.4965% |
| prefix-repeated-32768 | 32768 | 1.023706x | 1.049353x | +2.5053% |
| prefix-repeated-32769 | 32769 | 1.028317x | 1.058131x | +2.8993% |
| prefix-repeated-33027 | 33027 | 1.041527x | 1.067907x | +2.5329% |
| prefix-periodic-32768 | 32768 | 1.057612x | 1.086396x | +2.7215% |
| prefix-periodic-32767 | 32767 | 1.063286x | 1.095951x | +3.0721% |
| prefix-periodic-32769 | 32769 | 1.066628x | 1.094666x | +2.6287% |
| prefix-periodic-33027 | 33027 | 1.067765x | 1.102438x | +3.2472% |

Eight literals improves 2.171652% in primary elapsed, from 0.431513x to
0.422142x, and retires 1.789593% fewer instructions in its separate CPU cohort.
That is a real retained observation, not an adopted compiler result. On the
32-KiB repeated/periodic frontier instructions decline about 1.88% but elapsed
worsens about 2.5–2.7%. Natural 1 MiB instructions fall about 3.00%, yet primary
elapsed worsens 0.84%. A local instruction reduction is not a latency proof.

## Exact checkpoint

Git parent: `3a04fd479caf4f2df37b749c7c2b2a0744cadf4e` (R265's measured rejection).
PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
The retained computational source is **R264**, published at
`9676a9cbad760dfc44f9b69aee746e1ca9080e13`. The R266 candidate is built from
that exact 825-file source, not from R265's rejected input-packet proof.
The immediate parent's exact-head review is clean:
https://github.com/mech-lang/mech/pull/973#issuecomment-6779140146 .
IR/work/ABI/MBC remain **215/107/39/14**. Six existing compiler/test files change
and two are added, for 827 files. No-fuzz reverse/forward patch reconstruction
restores 825 exact R264 files and reconstructs all 827 candidate files. The
R264 backend, generated selector inputs and dependent libraries remain exact.
Compiler controls use rustc1.96.0-nightly `ec818fda361ca216eb186f5cf45131bd9c776bb4`
(LLVM22.1.0) and Apple Clang17.0.0 `clang-1700.3.19.1`, targeting aarch64/arm64
Apple Darwin. An optional CPU-brand query was denied with `Operation not permitted`;
no machine-setting change or successful fresh CPU-brand verification is claimed.
The included 200-file backend patch is inherited reconstruction evidence,
not a new backend optimization.

## Real operation, reference evidence and compiler change

The compressor's U32 cache/cost reads, U16 tree reads and U8 slot/input reads
must become I64 semantic words. The previous emitters construct a narrow load
and a separate sign/zero extension. The current linked path routine actually
contains `ldr w0; mov w1,w0`, `ldrb w0; uxtb w0,w0` and analogous pairs.
The existing AArch64 selector can fold a load/extension only when the load can
be sunk into that consumer. The observed pairs establish a current missed fold;
the source spelling alone would not establish one.

The pinned reference path routine has direct narrow loads consumed as word
values; other extensions remain, so no universally minimal reference code is
claimed. Separate unmeasured C and Rust controls return unaligned unsigned and
signed 8/16/32-bit values as 64-bit words. Both optimized compilers emit one
extending load and a return for all six controls. These are instruction-selection
witnesses, not C/Rust complete-compression timings or source substitutions.
Exact control sources, assembly and build receipts are in the packet.

The candidate adds a small shared `codegen/common/scalar_memory.rs` helper.
It selects `uload8/16/32` or `sload8/16/32` directly, and retains ordinary I64
loads for eight-byte storage. Result words remain I64. It preserves the exact
physical access width and caller-supplied flags; it grants no nontrapping,
readonly, alignment, movable-load or alias permission. Packed scalar/product
loads, numeric source reads, shared first-change old-value comparisons and
little-endian packed-word reads use that helper. The original current-view
bounds check and canonical linear-ordinal translation still precede the load.
Packed-record offsets and strides, stores and snapshots remain in their existing
places. Floating scalar storage remains its original integer bit pattern.

Changing the IR shape initially lost the unsigned width fact previously supplied
by `uextend` of an I8/I16/I32 load. A focused failing reproducer observes U64::MAX
instead of 255 as a byte load's conservative upper bound. The same change
therefore teaches the existing immutable-SSA range analysis the encoded bounds
of unsigned extending loads. Signed/full-word loads stay conservative; a fresh
load after a store does not become equal to an earlier value. This is a value
range fact, not source type trust, access permission, load motion or memory reuse.

There is no new backend flag, program-name/input-size selection, compression
path choice, source algorithm edit, ABI revision, callback or runtime dispatch.
The accepted source/bytecode operations and allocation sites remain unchanged.

## Actual emitted result

Six adjacent narrow-load/extension-or-copy pairs in the linked path body become
zero. This is a static count in the exact image, not six saved instructions per
complete call. Most code bodies shrink, but normal matching grows; record that
contrary code-generation outcome as well as smaller selected-vector code.

| Root/numeric Object definitions | Count |
|---|---:|
| Common definitions | 341 |
| Shrinking bodies | 308 |
| Growing bodies | 1 |
| Exact unchanged bodies | 32 |
| Growing stack frames | 0 |

| Definition | Code before | Code candidate | Stack before | Stack candidate |
|---|---:|---:|---:|---:|
| `_mech_compact_closed_v2` | 1040404 | 1034592 | 57056 | 56992 |
| `_mech_compact_numeric_837` | 51868 | 52664 | 960 | 960 |
| `_mech_compact_numeric_837_vector` | 41576 | 40544 | 960 | 944 |
| `_mech_compact_numeric_15296` | 2048 | 2016 | 288 | 288 |
| `_mech_compact_numeric_15737` | 704 | 684 | 192 | 192 |

Total recorded Object root/numeric code decreases 1,359,288 to
1,349,168 bytes. The linked worker stays **2,084,296 bytes** including
padding. Register allocation, instruction selection and linked placement all
change. A smaller selected matching frame does not establish lower latency.
No exact causal split of the measured regression is established.

All full-compressor metadata except cleanup statistics, code bytes and frames
is identical. Source proof sites, phase geometry/layouts, source errors, owner
transfers, input/output words and widened-loop choices remain exact. The physical
algorithm and current invocation behavior are unchanged.

## Separate instructions, cycles and memory

The CPU cohort is independent of primary elapsed and the heap cohort. Counter
reductions and primary slowdowns can coexist; do not multiply cohorts or infer
per-function instructions from CPU-time samples.

| Case | Reference instructions | Retained instructions | Candidate instructions | Change | Candidate / reference |
|---|---:|---:|---:|---:|---:|
| eight-literals | 1,361,942.0 | 1,134,699.5 | 1,114,393.0 | -1.7896% | 0.818238x |
| prefix-repeated-32768 | 19,152,129.5 | 27,588,139.5 | 27,069,994.5 | -1.8781% | 1.413420x |
| prefix-periodic-32768 | 19,155,530.0 | 27,644,594.5 | 27,125,391.0 | -1.8781% | 1.416061x |
| prefix-natural-32768 | 47,614,243.0 | 104,772,752.5 | 101,660,023.5 | -2.9709% | 2.135076x |
| prefix-random-32768 | 11,453,635.0 | 31,018,443.0 | 30,348,638.5 | -2.1594% | 2.649695x |
| prefix-natural-1048576 | 1,670,741,520.5 | 3,793,895,738.5 | 3,680,128,560.5 | -2.9987% | 2.202692x |
| prefix-random-1048576 | 320,159,256.5 | 959,250,239.0 | 938,802,322.0 | -2.1317% | 2.932298x |
| prefix-periodic-1048576 | 575,955,277.0 | 863,433,988.0 | 847,461,018.5 | -1.8499% | 1.471401x |
| prefix-repeated-1048576 | 576,015,293.5 | 870,673,402.5 | 854,712,542.0 | -1.8332% | 1.483837x |

| Case | Cycles before | Cycles candidate | Cycle change |
|---|---:|---:|---:|
| eight-literals | 481,657.5 | 472,816.0 | -1.8356% |
| prefix-repeated-32768 | 4,374,551.5 | 4,450,907.0 | +1.7454% |
| prefix-periodic-32768 | 4,211,320.0 | 4,279,202.0 | +1.6119% |
| prefix-natural-32768 | 25,462,797.5 | 25,006,994.0 | -1.7901% |
| prefix-random-32768 | 6,102,962.5 | 6,078,700.5 | -0.3975% |
| prefix-natural-1048576 | 975,175,371.0 | 957,313,759.5 | -1.8316% |
| prefix-random-1048576 | 199,866,844.5 | 199,038,834.0 | -0.4143% |
| prefix-periodic-1048576 | 122,901,137.0 | 125,520,290.0 | +2.1311% |
| prefix-repeated-1048576 | 129,839,735.5 | 131,849,385.0 | +1.5478% |

All adverse elapsed/cycle results remain. Repeated/periodic cycles increase,
while the separate natural/random 1-MiB CPU cohorts have small cycle reductions.
Those reductions are not substituted for the adverse primary measurements.

Mech's payload requests, requested bytes, initialized copies, zeroing, private
payload peaks and final nonreturned private bytes are equal on every case.
Final private payload and owner records are zero. No allocation, copying,
streaming or live-payload improvement is claimed. For eight literals the retained
and candidate each request 111 payload allocations / 9,024,420 bytes, with a
private payload peak 8,982,372 bytes and 2,188 initialized bytes copied. These are
Mech payload counters, not an all-allocator request census. No fresh reference
allocator census is collected, so historical 95-request data is not mixed in.

Whole-worker physical/RSS readings remain in the CPU receipts. They include
unequal harness/metadata/library state and are not compressor-only peaks. For
eight, lifetime physical peaks are 49,808,320 retained, 49,906,624 candidate and
8,356,736 reference bytes. This does not establish a per-compression memory
ratio or a memory saving. Requested/zeroed bytes are not measured cache traffic.

## All 81 fresh matched complete-call timings

Two fresh primary cohorts contain all six actor orders repeated four times,
48 balanced blocks per case. Ratios are formed within each block before
geometric aggregation. The existing pooled and block-median 1.05 gates remain
unchanged and pass 81/81. Equal case weighting is not a representative input mix.
No valid observation is discarded, repeated or substituted.

Both sides use fixed level 12, runtime input, equal 200-ms warmups, the pinned
allocator and Rust clock, and borrowed output ownership. Full compression,
scratch construction, output transfer and private cleanup remain timed.
Preparation, validation and old external output lifetime remain outside on both
sides. Reference denominators are fresh and do not rewrite historical tables.
Stored sub-microsecond cases remain close to timer resolution.

The candidate is rejected; the left column remains the retained compiler.
Negative change means faster. Each configured case appears exactly once below.

### Original 25

| Case | Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---|---:|---:|---:|---:|
| empty | 0 | 2.116208x | 2.189312x | +3.4545% |
| one-stored | 1 | 2.030806x | 2.080364x | +2.4404% |
| seven-stored-prefix | 7 | 2.236055x | 2.266632x | +1.3675% |
| eight-literals | 8 | 0.431513x | 0.422142x | -2.1717% |
| eight-overlap | 8 | 0.444169x | 0.442231x | -0.4365% |
| 320-overlap | 320 | 0.475537x | 0.473278x | -0.4750% |
| eight-literals-prefix | 8 | 0.422304x | 0.430187x | +1.8666% |
| 128-biased-dynamic | 128 | 0.421944x | 0.430251x | +1.9689% |
| 32-high-entropy-stored | 32 | 0.438096x | 0.439076x | +0.2239% |
| 16384-distribution-change | 16384 | 1.863635x | 1.927142x | +3.4077% |
| 33027-periodic-window | 33027 | 1.077697x | 1.112197x | +3.2013% |
| random-64 | 64 | 0.460312x | 0.453283x | -1.5272% |
| random-256 | 256 | 0.496686x | 0.488591x | -1.6297% |
| random-1024 | 1024 | 0.556062x | 0.558473x | +0.4336% |
| repeated-1024 | 1024 | 0.538202x | 0.545862x | +1.4233% |
| random-32769 | 32769 | 1.791110x | 1.812939x | +1.2187% |
| repeated-32769 | 32769 | 1.032889x | 1.053118x | +1.9585% |
| reymont-prefix-1024 | 1024 | 0.694231x | 0.694797x | +0.0815% |
| reymont-prefix-16384 | 16384 | 1.753522x | 1.789480x | +2.0506% |
| reymont-prefix-32769 | 32769 | 1.821585x | 1.849235x | +1.5179% |
| heldout-window-32767 | 32767 | 1.828361x | 1.856167x | +1.5208% |
| heldout-window-32768 | 32768 | 1.830533x | 1.857697x | +1.4839% |
| heldout-slides-65537 | 65537 | 1.893161x | 1.913161x | +1.0564% |
| heldout-random-65537 | 65537 | 1.967974x | 1.992819x | +1.2625% |
| heldout-mixed-131073 | 131073 | 1.865276x | 1.921254x | +3.0010% |

### Controlled repeated prefixes

| Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.518691x | 0.513948x | -0.9144% |
| 4096 | 0.721662x | 0.731171x | +1.3177% |
| 8192 | 0.851519x | 0.866204x | +1.7245% |
| 16384 | 0.928869x | 0.948326x | +2.0947% |
| 32767 | 1.018886x | 1.044322x | +2.4965% |
| 32768 | 1.023706x | 1.049353x | +2.5053% |
| 32769 | 1.028317x | 1.058131x | +2.8993% |
| 33027 | 1.041527x | 1.067907x | +2.5329% |
| 65535 | 1.103070x | 1.138498x | +3.2117% |
| 65536 | 1.102019x | 1.133365x | +2.8444% |
| 65537 | 1.102792x | 1.133438x | +2.7789% |
| 131072 | 1.148633x | 1.182235x | +2.9253% |
| 262144 | 1.170460x | 1.207432x | +3.1587% |
| 1048576 | 1.180565x | 1.216783x | +3.0678% |

### Controlled periodic prefixes

| Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.520605x | 0.525238x | +0.8898% |
| 4096 | 0.742506x | 0.754089x | +1.5599% |
| 8192 | 0.853120x | 0.888103x | +4.1007% |
| 16384 | 0.948777x | 0.976819x | +2.9555% |
| 32767 | 1.063286x | 1.095951x | +3.0721% |
| 32768 | 1.057612x | 1.086396x | +2.7215% |
| 32769 | 1.066628x | 1.094666x | +2.6287% |
| 33027 | 1.067765x | 1.102438x | +3.2472% |
| 65535 | 1.137987x | 1.177812x | +3.4996% |
| 65536 | 1.135926x | 1.174909x | +3.4319% |
| 65537 | 1.137717x | 1.177117x | +3.4631% |
| 131072 | 1.178117x | 1.223320x | +3.8369% |
| 262144 | 1.205490x | 1.250460x | +3.7304% |
| 1048576 | 1.221513x | 1.267874x | +3.7953% |

### Controlled natural prefixes

| Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.707562x | 0.719085x | +1.6286% |
| 4096 | 1.325776x | 1.349622x | +1.7987% |
| 8192 | 1.585077x | 1.617966x | +2.0750% |
| 16384 | 1.761372x | 1.799941x | +2.1897% |
| 32767 | 1.827124x | 1.855928x | +1.5765% |
| 32768 | 1.828950x | 1.858158x | +1.5970% |
| 32769 | 1.822007x | 1.849531x | +1.5107% |
| 33027 | 1.825688x | 1.851873x | +1.4342% |
| 65535 | 1.864338x | 1.889291x | +1.3385% |
| 65536 | 1.863133x | 1.890341x | +1.4603% |
| 65537 | 1.897184x | 1.923582x | +1.3914% |
| 131072 | 1.868051x | 1.888653x | +1.1028% |
| 262144 | 1.887260x | 1.908558x | +1.1285% |
| 1048576 | 1.889605x | 1.905452x | +0.8386% |

### Controlled random prefixes

| Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.535722x | 0.539031x | +0.6176% |
| 4096 | 0.933811x | 0.944069x | +1.0986% |
| 8192 | 1.153480x | 1.160862x | +0.6399% |
| 16384 | 1.522154x | 1.543990x | +1.4345% |
| 32767 | 1.803082x | 1.828493x | +1.4093% |
| 32768 | 1.801968x | 1.823499x | +1.1948% |
| 32769 | 1.800277x | 1.831451x | +1.7316% |
| 33027 | 1.807315x | 1.832523x | +1.3948% |
| 65535 | 1.981064x | 2.006527x | +1.2853% |
| 65536 | 1.974093x | 2.000351x | +1.3301% |
| 65537 | 1.974615x | 1.995819x | +1.0738% |
| 131072 | 2.066455x | 2.095707x | +1.4156% |
| 262144 | 2.117657x | 2.144047x | +1.2462% |
| 1048576 | 2.168521x | 2.190001x | +0.9905% |

## Current profiles and next bounded compiler target

Eight fresh captures use the exact candidate image: eight literals, four 32-KiB
families and natural/random/periodic 1 MiB. All four primary/counter cohorts finish
before profiling starts; no build/test/profile overlaps those cohorts. The
serialized isolation receipt is checked in replay. R264 baseline/reference
captures are explicitly inherited and hash-matched; zero new pre-change or
reference captures are claimed. Only each image's own PCs/symbols are used.
Root source attribution remains unknown.

| Candidate case | Running samples | Matching phase | Path phase |
|---|---:|---:|---:|
| eight-literals | 5503 | 0 (0.00%) | 0 (0.00%) |
| prefix-natural-1048576 | 5349 | 1828 (34.17%) | 3183 (59.51%) |
| prefix-natural-32768 | 5518 | 1723 (31.23%) | 3258 (59.04%) |
| prefix-periodic-1048576 | 5375 | 3339 (62.12%) | 1444 (26.87%) |
| prefix-periodic-32768 | 5364 | 3042 (56.71%) | 1346 (25.09%) |
| prefix-random-1048576 | 5373 | 3009 (56.00%) | 1224 (22.78%) |
| prefix-random-32768 | 5510 | 2451 (44.48%) | 1339 (24.30%) |
| prefix-repeated-32768 | 5548 | 3083 (55.57%) | 1327 (23.92%) |

These are CPU-time samples including harness and validation, not a removable-time
budget or a retired-instruction partition. Larger cases still run predominantly
inside native numeric phases. The static load improvement is established; its
unfavorable scheduling/allocation/placement effects are not causally separated.

The next bounded target should start from retained R264 and simplify the
**unit append spare-capacity predicate**. The source operation is the
`sequence/append(nodes,node)` in `optimal-path-position`, source line 724. The
shared numeric emitter currently checks:

`length <= capacity && wrapping_sub(capacity,length) >= maximum`

For `maximum == 1` this is exactly unsigned `length < capacity`, including
length0, invalid length>capacity, equality and U64 extremes. No source-array or
ownership proof is added. Keep the current expression for other maxima and the
same cold growth/rejection path. The actual retained path binary uses eight
instructions: CMP,CSET,SUB,CMP,CSET,AND,UXTB,CBZ. The narrow window is archived.
The proposed simplification is not yet implemented or measured in this round.

Acceptance must cover exhaustive small values and U64 boundaries, maximum 0/
general cases, native true/false decisions, growth/admission failure, aliases,
error order, prior publication, cleanup/retry and original/decoded behavior.
Measure the actual emitted sequence and all 81 complete cases before adoption.
Do not predict elapsed from static instruction savings or raise allowances.
Keep the qualified R266 load alternative available for a later measured
combination rather than adopting it over the current frontier regression.

## Qualification and coverage

- **725 native tests pass; zero fail; seven unchanged inherited exclusions.**
  Five focused tests also pass: four shared-memory tests and one unsigned-range
  test, and all execute again in the full suite. The native alias/misalignment/
  sign/native-or-explicit-native-endian matrix executes 2048 calls; protected
  page-end reads execute 64. Source flags and physical access widths are checked.
  Eight old/new explicit-big-endian support comparisons agree. On this AArch64
  backend explicit Big loads remain unsupported in both representations;
  no working Big-endian runtime coverage or backend repair is claimed.
- **7384 fresh compiled native source calls and 759 fresh resident calls pass.**
  Counts are 1116 checked array-read controls +480 private cold-handoff controls
  +5788 matching controls; resident calls are 279+480. Matching reuses frozen
  R230 resident rows, with zero fresh matching-resident calls claimed. Current
  arguments, two-dimensional views, unselected branches, overflow/assertion/
  bounds precedence, aliases, prior publication, cleanup and retry retain their
  existing oracle results. Before/after and original/decoded observations agree.
- **324 full smokes** cover 81 complete cases in both forms, with exact reference
  bytes, independent inflate, input/borrowed-output aliases and zero final private
  ownership. All 17 full/small Object and metadata pairs are exact.
- Performance compiler library opt-level 3; native tests opt-level 1; the same six
  features and warnings-as-errors:
  `aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime`.
  Changed-module formatting and three tracked architecture/catalog checks pass.
  Expected values, source prices, allowances and timing ceilings are unchanged.

```sh
python3 run-spec.py candidate-gpu candidate-producer candidate-tests
python3 qualify-matching.py
python3 qualify-packet-source.py
python3 qualify-cold.py
python3 run-spec.py mech-worker decoded-worker after-cpu after-memory
python3 run-measurements.py
python3 profile-border-pcs.py after
python3 replay.py
```

Exact commands, flags, hashes and results are in the packet. The unchanged
maintained 680-test result is inherited from R261. Full hosted CI, a fresh full
linked/AOT integration suite, full 6.6 MB input and a full disabled-pipeline emission
are not claimed. The compact entry's additional ordinary managed admission/
accounting/publication/recovery omissions remain explicit. This experiment
neither restores nor weakens the ordinary contract.

## Development findings and disposition

The first helper build used the wrong local builder arity for 32-bit extending
loads; it was corrected before final builds. Initial staging mistakes about a
Python replacement argument, flags-file shape, source-search paths and assembly
whitespace changed no accepted measurement. Preserved preliminary build receipts
are not final-binary provenance. The intentional range reproducer fails before
its fact integration and passes afterward: this was a lost optimization fact
introduced by the representation change, not a wrong source result. The first
native matrix assumed explicit Big support; an old/new control proves that
limitation inherited, and the final test checks matching support rather than
claiming unsupported byte-order execution. The normal/native byte-order matrix
and protected-page controls remain mandatory. No compression oracle is changed.

No introduced correctness failure or new resource denial remains in the tested
profiles. The measured primary regression is introduced by this candidate and
is the reason it stays rejected. Seven inherited native exclusions remain
assigned. Every valid primary/counter observation and the isolated eight-byte/
instruction wins are preserved. R264 remains the active computational source.

## Review contract and fidelity

Review against Git parent `3a04fd479caf4f2df37b749c7c2b2a0744cadf4e` and its
retained R264 source:

1. Verify the evaluated eight-file patch reconstructs 825/827 source files and
   remains rejected, with R264 still retained. Do not confuse this candidate with
   R265's separate rejected input-proof experiment.
2. Verify memory width, sign, result-word bits and flags. Bounds/ordinal/error
   order precede the same source access; source alias/snapshot/store behavior,
   cleanup/retry and original/decoded agreement remain. Range facts must be
   unsigned width facts only, with conservative signed/fresh-load controls.
3. Recompute 22/81 = 27.160494%, the 100% goal and 0.0000 percentage-point change on
   the fixed inventory. Recompute 48 matched blocks/case and the adverse elapsed
   result. Preserve the instruction and eight-byte savings without presenting
   them as a global latency win. Confirm reference/actor/source/allocator hashes,
   timer/ownership boundaries, isolation and inherited coverage labels.
4. Reproduce the public packet hashes and five deterministic reports. The next
   unit-capacity simplification is a bounded proposal, not a result or an
   unreviewed extra compiler change in this checkpoint.

Benchmark source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
MBC14 SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Keep the complete level 12 algorithm, tables, depth 300, nice 258, original EOF
rules, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, output capacity 13
with clear/reuse, input aliases, source allocation sites, allocator and timing.

The packet includes 827 compiler files, their reversible eight-file patch, the
unchanged 200-file backend reconstruction, small fixture metadata, commands,
all observations/counters and aggregate profile PCs. Native binaries/libraries/
Objects, bytecode, full compressor metadata/IR/CFG/maps and raw trace/XML/stacks
are excluded. Five offline reports replay without compiling or compressing.
PR973 remains the only active optimization PR and stays draft. Parity is open.

## Public evidence and replay

[Evaluated compiler patch, all 81 measurements, native controls and assembly](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r266-extending-scalar-loads-8bd91ced424a.tar.gz).
SHA-256: `8bd91ced424ad5b3b751660d7eb6d1696498a93a2c4c233482568f85e56863ad`.
Fresh public retrieval verifies 2,389 file hashes, exact 827/825 compiler and
200-file backend reconstruction, and five identical offline reports. R266 is a
qualified rejected alternative; R264 remains the retained source packet.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 266 \
  --output /tmp/mech-r266-review
```
