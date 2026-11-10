# R264: branch directly on vector truth

**Retain this bounded experimental compiler improvement and continue tracking all
81 cases.** Its combined paired geometric elapsed improvement is **0.336093%**,
with both unchanged 1.05 timing gates passing 81/81. This is a small, noisy gain:
cohort A is +0.094530% and cohort B
-0.764863%. Every contrary result remains.
No new configuration crosses 1x: 22/81 are below Roc before and after.

Published parent: `f299af56c78319dd3b820dae6fd56697d12c6a61`.
PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
The parent has a clean exact-head review in comment 6776512634.
IR/work/ABI/MBC remain **215/107/39/14**. Two existing Mech files change in the
825-file source closure: the equality emitter and its tests. One existing ISLE
file changes in the 200-file retained R255 backend closure. Only the AArch64
selector is regenerated; all other generated definitions and dependency sources
stay pinned. Dependent crates are rebuilt against this selector. No source program, algorithm or table changes.

## Cases nearest parity

The main priority remains repeated and periodic cases near 32 KiB. They do not
improve uniformly here. Additional cut means the time reduction needed from this
candidate's measured ratio to reach 1x; it is not a forecast.

| Case | Bytes | R263 / Roc | R264 / Roc | Time change | Additional cut to 1x |
|---|---:|---:|---:|---:|---:|
| repeated | 32767 | 1.021118x | 1.022805x | +0.1652% | 2.230% |
| repeated | 32769 | 1.026890x | 1.026793x | -0.0094% | 2.609% |
| repeated | 32768 | 1.041336x | 1.034897x | -0.6183% | 3.372% |
| repeated | 33027 | 1.038928x | 1.040467x | +0.1481% | 3.889% |
| periodic | 32768 | 1.061908x | 1.057479x | -0.4170% | 5.435% |
| periodic | 32769 | 1.063255x | 1.061318x | -0.1822% | 5.777% |
| periodic | 32767 | 1.061943x | 1.066311x | +0.4113% | 6.219% |
| periodic | 33027 | 1.071861x | 1.068822x | -0.2835% | 6.439% |

The nearest 32767-byte repeated case worsens slightly. Repeated 32768 and periodic
32768 improve, while several adjacent sizes are nearly flat. Do not infer a
new parity crossing or a uniform matching-loop speedup from the aggregate.

## Current binary evidence and compiler rule

Fresh pre-change CPU captures on the exact R263 image place matching at
3178/5551 samples for repeated 32 KiB (57.25%) and 3207/5361 for periodic (59.82%).
Their path scan contributes 20.23% and 20.95%. The second vector-to-scalar lane
transfer is a sampled hot instruction. The pinned reference's actual matching
assembly still uses eight-byte word comparisons. Mech already has certified
16/64-byte equality loops; this experiment changes their predicate lowering.

R231 had already replaced a costly materialized vector Boolean with two lane
extracts plus OR. Simply undoing it is not a new optimization. The missing piece
is a backend branch rule: vector truth observed as a branch can use a raw
nonzero condition while ordinary Boolean storage keeps its normalized value.

The emitter expresses the already-loaded XOR/OR difference as `vany_true` under
`MECH_WIDE_ANY_BITS=1`. AArch64 lowers a branch on a 128-bit `vany_true`, with an
optional unsigned extension, to pairwise unsigned maxima of its four U32 lanes,
one U64 transfer, and a nonzero branch. The transferred lower U64 contains both
pairwise maxima; it is zero exactly when all 128 input bits are zero. This
removes Boolean materialization from the branch. The separate value-producing
rule remains intact for Boolean observers. No source/canonical Bool rule changes.

| Actual private predicate | Instructions | Vector-to-scalar transfers |
|---|---:|---:|
| Two lane transfers, scalar OR, CBNZ | 4 | 2 |
| Pairwise vector maximum, one transfer, CBNZ | 3 | 1 |

All nine certified equality loops in each matching body keep their current
readable-window proof, pure two-load cycle, induction, mismatch position and
scalar tail. There are 18 static reduction probes per body, because each loop
has 16-byte and 64-byte paths. Static probes are not measured dynamic visits.
No source ID, compressor name, field, input size or compression path selects
the rule. Loads, work debits, source assertions and their error order remain.

The backend rule also serves other vector predicates. A new native test covers
1596 calls across six 128-bit kinds: all 128 single-bit positions, zero and four
mixed-bit patterns. It verifies a separately observed Boolean remains exactly
zero/one and that clearing aliased input after loading the vector cannot change
the captured predicate. The existing exhaustive equality and refusal tests
remain. The new rule is selected only in this experiment's backend build; the
old retained backend remains available as the reconstruction control.

## Complete emitted shape

The root Object body remains exact. Among 341 common root/numeric definitions,
339 are byte-identical and only the two matching bodies change. Code generation
is not just the local predicate replacement: register allocation and spills also
change, and both matching frames grow 16 bytes. These broader effects must be
measured rather than inferred from one shorter sequence.

| Definition | Code before | Code after | Frame before | Frame after |
|---|---:|---:|---:|---:|
| `_mech_compact_numeric_837` | 53500 | 51868 | 944 | 960 |
| `_mech_compact_numeric_837_vector` | 41456 | 41576 | 944 | 960 |

Combined code decreases 1512 bytes; the vector body itself grows 120 bytes.
The linked worker remains 2,084,296 bytes including padding.
The path bodies retain their 2048-byte code and 288-byte frames. Linked addresses
may move; no fixed-placement causal attribution is claimed.

## All 81 matched complete-call timings

Two fresh primary cohorts use all six actor orders repeated four times, giving
48 balanced blocks per case. Ratios are formed inside each block before
geometric aggregation. The existing pooled-median and block-median 1.05 gates
remain unchanged and both pass 81/81. No valid timing or counter observation is
discarded or repeated. All adverse observations remain. This geometric mean
weights configurations equally, rather than a representative input mix.

Both implementations run fixed level 12 with runtime input, the pinned allocator
and Rust clock, equal 200-ms warmups and borrowed output ownership. Full
compression, scratch construction, output transfer and private cleanup are
inside the timer; preparation, validation and old external output lifetime are
outside on both sides. The source, allocation sites and timing boundary remain
unchanged. Fresh reference denominators do not rewrite historical tables.
Sub-microsecond stored cases remain near clock resolution.

Negative time change means faster; below 1x beats Roc. Every case occurs once
in the five tables below.

### Original 25

| Case | Bytes | R263 / Roc | R264 / Roc | Time change |
|---|---:|---:|---:|---:|
| empty | 0 | 2.123732x | 2.140319x | +0.7810% |
| one-stored | 1 | 2.020623x | 2.007744x | -0.6374% |
| seven-stored-prefix | 7 | 2.302697x | 2.281788x | -0.9080% |
| eight-literals | 8 | 0.428201x | 0.424475x | -0.8701% |
| eight-overlap | 8 | 0.428681x | 0.426372x | -0.5386% |
| 320-overlap | 320 | 0.464042x | 0.456610x | -1.6016% |
| eight-literals-prefix | 8 | 0.438185x | 0.429634x | -1.9514% |
| 128-biased-dynamic | 128 | 0.428999x | 0.417930x | -2.5803% |
| 32-high-entropy-stored | 32 | 0.441319x | 0.437331x | -0.9037% |
| 16384-distribution-change | 16384 | 1.854206x | 1.854788x | +0.0314% |
| 33027-periodic-window | 33027 | 1.069804x | 1.067192x | -0.2441% |
| random-64 | 64 | 0.449571x | 0.440356x | -2.0497% |
| random-256 | 256 | 0.486666x | 0.480449x | -1.2775% |
| random-1024 | 1024 | 0.542796x | 0.531458x | -2.0887% |
| repeated-1024 | 1024 | 0.515071x | 0.512184x | -0.5605% |
| random-32769 | 32769 | 1.778049x | 1.781622x | +0.2010% |
| repeated-32769 | 32769 | 1.023460x | 1.024730x | +0.1240% |
| reymont-prefix-1024 | 1024 | 0.681201x | 0.674962x | -0.9159% |
| reymont-prefix-16384 | 16384 | 1.759720x | 1.746780x | -0.7354% |
| reymont-prefix-32769 | 32769 | 1.821084x | 1.817930x | -0.1732% |
| heldout-window-32767 | 32767 | 1.826481x | 1.822294x | -0.2292% |
| heldout-window-32768 | 32768 | 1.827241x | 1.819296x | -0.4348% |
| heldout-slides-65537 | 65537 | 1.890190x | 1.883459x | -0.3561% |
| heldout-random-65537 | 65537 | 1.956222x | 1.961092x | +0.2489% |
| heldout-mixed-131073 | 131073 | 1.865344x | 1.864389x | -0.0512% |

### Controlled repeated prefixes

| Bytes | R263 / Roc | R264 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.515929x | 0.520580x | +0.9014% |
| 4096 | 0.730315x | 0.733089x | +0.3799% |
| 8192 | 0.838740x | 0.835267x | -0.4141% |
| 16384 | 0.925866x | 0.928528x | +0.2875% |
| 32767 | 1.021118x | 1.022805x | +0.1652% |
| 32768 | 1.041336x | 1.034897x | -0.6183% |
| 32769 | 1.026890x | 1.026793x | -0.0094% |
| 33027 | 1.038928x | 1.040467x | +0.1481% |
| 65535 | 1.106065x | 1.105040x | -0.0927% |
| 65536 | 1.104200x | 1.102111x | -0.1892% |
| 65537 | 1.101996x | 1.100254x | -0.1580% |
| 131072 | 1.150094x | 1.148042x | -0.1784% |
| 262144 | 1.172970x | 1.172583x | -0.0330% |
| 1048576 | 1.180242x | 1.180345x | +0.0087% |

### Controlled periodic prefixes

| Bytes | R263 / Roc | R264 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.525986x | 0.516232x | -1.8544% |
| 4096 | 0.749251x | 0.740397x | -1.1818% |
| 8192 | 0.852374x | 0.867772x | +1.8065% |
| 16384 | 0.944786x | 0.944939x | +0.0162% |
| 32767 | 1.061943x | 1.066311x | +0.4113% |
| 32768 | 1.061908x | 1.057479x | -0.4170% |
| 32769 | 1.063255x | 1.061318x | -0.1822% |
| 33027 | 1.071861x | 1.068822x | -0.2835% |
| 65535 | 1.139407x | 1.140264x | +0.0752% |
| 65536 | 1.135275x | 1.137919x | +0.2329% |
| 65537 | 1.136212x | 1.135953x | -0.0228% |
| 131072 | 1.181943x | 1.178005x | -0.3332% |
| 262144 | 1.206466x | 1.204222x | -0.1860% |
| 1048576 | 1.220897x | 1.222641x | +0.1428% |

### Controlled natural prefixes

| Bytes | R263 / Roc | R264 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.684440x | 0.683689x | -0.1097% |
| 4096 | 1.308476x | 1.306907x | -0.1199% |
| 8192 | 1.578405x | 1.575333x | -0.1947% |
| 16384 | 1.758775x | 1.749678x | -0.5173% |
| 32767 | 1.822032x | 1.815035x | -0.3840% |
| 32768 | 1.820685x | 1.813273x | -0.4071% |
| 32769 | 1.825668x | 1.817983x | -0.4209% |
| 33027 | 1.824194x | 1.819294x | -0.2687% |
| 65535 | 1.878519x | 1.868723x | -0.5214% |
| 65536 | 1.873888x | 1.865129x | -0.4674% |
| 65537 | 1.874895x | 1.870493x | -0.2348% |
| 131072 | 1.875195x | 1.877262x | +0.1102% |
| 262144 | 1.883912x | 1.888962x | +0.2681% |
| 1048576 | 1.889632x | 1.890300x | +0.0353% |

### Controlled random prefixes

| Bytes | R263 / Roc | R264 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.543084x | 0.533569x | -1.7520% |
| 4096 | 0.936537x | 0.927631x | -0.9510% |
| 8192 | 1.173711x | 1.159706x | -1.1932% |
| 16384 | 1.528240x | 1.522006x | -0.4079% |
| 32767 | 1.797301x | 1.796958x | -0.0191% |
| 32768 | 1.803677x | 1.798011x | -0.3141% |
| 32769 | 1.791391x | 1.781135x | -0.5725% |
| 33027 | 1.800849x | 1.789857x | -0.6104% |
| 65535 | 1.975556x | 1.983256x | +0.3898% |
| 65536 | 1.972537x | 1.978012x | +0.2776% |
| 65537 | 1.968477x | 1.970469x | +0.1012% |
| 131072 | 2.064735x | 2.068813x | +0.1975% |
| 262144 | 2.112934x | 2.117090x | +0.1967% |
| 1048576 | 2.145590x | 2.149057x | +0.1616% |

## Independent instructions, cycles and memory

Hardware counters and allocation counters are separate balanced cohorts; CPU
samples are not an instruction partition. All 81 payload-allocation,
requested-byte, initialized-copy, zeroing, private-peak and final nonreturned
payload families remain identical. Final nonreturned private payload is zero.
No allocation, streaming or whole-worker RSS saving is claimed.

| Case | Instructions before | Instructions after | Change | Cycles before | Cycles after | Change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 1,134,690.0 | 1,134,711.5 | +0.0019% | 500,019.5 | 506,156.0 | +1.2273% |
| prefix-repeated-32768 | 27,685,093.0 | 27,587,671.0 | -0.3519% | 4,371,518.0 | 4,415,310.0 | +1.0018% |
| prefix-periodic-32768 | 27,741,807.0 | 27,644,489.0 | -0.3508% | 4,254,658.5 | 4,262,143.5 | +0.1759% |
| prefix-natural-1048576 | 3,783,759,596.0 | 3,793,894,873.5 | +0.2679% | 972,208,920.5 | 974,766,495.5 | +0.2631% |
| prefix-random-1048576 | 950,846,292.5 | 957,980,655.5 | +0.7503% | 197,546,860.0 | 198,521,990.5 | +0.4936% |
| prefix-periodic-1048576 | 866,566,035.0 | 863,430,852.0 | -0.3618% | 122,977,553.0 | 122,746,309.0 | -0.1880% |
| prefix-repeated-1048576 | 873,786,475.0 | 870,671,114.5 | -0.3565% | 129,603,695.5 | 129,654,674.0 | +0.0393% |

Natural and random 1 MiB execute more instructions despite nearly flat elapsed
measurements. Repetitive instructions decline, but several cycle measurements
are adverse. These remain explicit costs; the small overall elapsed signal is
not proof that every shorter predicate or instruction reduction saves time.
Eight's instruction count is essentially unchanged, so its elapsed movement is
not claimed as a new eight-byte algorithmic saving.

## Fresh profiles and next bounded target

Twelve new captures include baseline/reference on both border inputs and eight
candidate cases: those border inputs, eight literals, natural/random 32 KiB and
natural/random/periodic 1 MiB. Each uses its exact current image's PCs and symbols.
All four original measurement cohorts finish before post-change profiling starts;
no build, test or profile overlaps primary or counter collection. The recorded
isolation barrier and per-capture timestamps remain in the packet.

Candidate matching contributes 3201/5569 repeated-32 samples (57.48%) and
3223/5358 periodic-32 samples (60.15%). Their path phase contributes 19.82% and
21.13%. Natural 1 MiB still has 3130/5353 path samples (58.47%) and 1894/5353
matching samples (35.38%). These are sampled CPU-time shares including harness/
validation, not removable-time budgets. External/unmapped samples are not
established runtime overhead; root source attribution remains unknown.

The next bounded experiment should revisit R235's checked nontrapping
argument-packet reads on the current outlined architecture. The current path
prologue still loads unused descriptor words; its unchanged assembly provides
a current witness. R235 previously removed ten loads per path routine but made
repeated/periodic slower, so its old result is retained and a new win is not
assumed. Current outlined frames, selected-path bounds and private composition
are materially different. Reconstruct the original span proof and negative
controls first; keep source-array loads trapping and avoid readonly/aligned or
movable-load assumptions. Then measure all 81 cases, especially those nearest 1x.
Do not combine another semantic compiler change into this checkpoint.

## Qualification and coverage limits

- 720 native tests pass, zero fail, with the same seven inherited exclusions.
  Three focused equality tests also pass. Exhaustive equality executes 5346432
  value combinations per execution; the new vector-bit/observer test executes
  1596. Both execute focused and in the full suite.
- 5788 fresh compiled native source controls pass across normal, swapped,
  fallible and stride12 programs, before/after and original/decoded. Current
  phase visits agree across those four forms. Effects and unproved strides
  decline widening. Current arguments/2D views, overflow/error order, aliases,
  previous publication, cleanup and retry retain their existing oracle results.
- The unchanged R230 source/resident rows and their library provenance are
  reused, with recorded default caps. **Zero fresh resident calls are claimed**
  for those source controls. This is separate from fresh native execution.
- 324 final full smokes cover all 81 inputs in original and decoded forms:
  exact reference raw streams, independent inflate, input/output aliases and
  zero final private ownership. All nine small/full Object/metadata pairs match.
- Native tests use opt-level 1; the performance compiler library uses opt-level
  3, with `-D warnings` and the same six features:
  `aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime`.
  The retained backend comparison-reuse configuration remains enabled. Exact
  compilation/link and invocation commands are in the packet.
- Changed-module formatting and three tracked architecture/catalog checks pass.
  Expected results, source prices and allowances remain unchanged.

```sh
python3 run-spec.py codegen-on cranelift_module-on cranelift_native-on \
  cranelift_object-on cranelift_jit-on cranelift_frontend-on \
  candidate-gpu candidate-producer candidate-tests
python3 qualify-matching.py
python3 run-measurements.py
python3 analyze-primary.py
python3 analyze-counters.py
```

The unchanged maintained 680-test result is inherited from R261. Full hosted CI,
a fresh full linked/AOT integration suite, full 6.6-MB input and a fresh full
pipeline-disabled emission are not claimed. The compact timing entry retains
its additional ordinary managed admission/accounting/publication/recovery
omissions; production contracts are not weakened or priced from these timings.
No new compiler operation class or source-level compression choice is added.

## Development failures and source fidelity

Two selector-generation failures (side-effect typing and overlap priority) are
corrected before the final backend build. A dependent compile from unchanged
generated definitions is not used for any image or measurement. A missing copied
producer source and a smoke-receipt naming preflight are corrected. Stale copied
actor paths/duration estimates are fixed before timing; every actual measured
candidate hash differs from the baseline and matches its independent link receipt.
No invalid/stale-binary observation is accepted, discarded or rerun. Correctness
smokes are repeated during preflight, with the final 324 counted once. There is
no introduced functional failure remaining in tested profiles or new resource
denial; seven inherited exclusions remain assigned. Tiny read-only path guesses
and their corrections stay in the local command history.

Benchmark source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
MBC14 SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Preserve the complete level-12 algorithm, tables, depth 300, nice length 258,
original EOF rules, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity,
output capacity 13 and clear/reuse, input aliases, source allocation sites,
pinned allocator and timing boundaries. Final private payload/owners remain zero.

The archive contains all 825 Mech compiler files, 200 backend sources, generated
selector inputs and their reversible two-file/one-file patches, qualified small
metadata, commands, observations, counters and aggregate profile PCs. Native
binaries/libraries/Objects, bytecode, full compressor metadata/IR/CFG/maps and raw
trace/XML/stacks are excluded. `verify-source.py` and `verify-backend.py` restore
and reapply both exact source closures without fuzz. Five offline reports replay
without compiling or compressing. Large-input parity remains open.

## Public evidence and replay

[Compiler/backend patches, all81 comparisons, commands and validation](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r264-vector-truth-branch-f40f9602e8dc.tar.gz).
SHA-256: `f40f9602e8dc8bc5e0142e57cb9c747e8a4bb249001d9f58be4e0c51dbb0c3e5`.
Fresh public retrieval verifies 2,252 file hashes, both reversible source
closures and five deterministic reports. Native binaries, bytecode and raw
traces are excluded.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 264 \
  --output /tmp/mech-r264-review
```
