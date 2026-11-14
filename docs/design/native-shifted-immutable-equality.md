# R275 — Reuse proved equality across shifted immutable windows

**Retain R275 with the R272 runtime.** Across all 81 cases, paired elapsed changes -2.546%. The fresh score is 32/81 before and 38/81 after (46.91%) faster than Roc. The pooled and block gates pass 81/81 and 81/81. In the separate libdeflate comparison, 3/81 cases are faster than libdeflate.

Paired geomean changes by -2.477% and -2.616% in the two cohorts. Cases that cross below parity are prefix-periodic-65535, prefix-periodic-65536, prefix-periodic-65537, prefix-repeated-65535, prefix-repeated-65536, prefix-repeated-65537. Cases that cross above parity are none. The largest paired elapsed increase is +3.498% on seven-stored-prefix.

Pooled gate misses include none. Block gate misses include none.

Measured and published parent 7708d533c8875a39bb9ca97e3e665dc15a53e341. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R274 source closure. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The compressor source, tables, search limits, scratch and output capacities, R272 runtime and complete-call timer stay pinned.

## Proof and generated code

The existing compact equality certificate admits two flat read-only U8 windows and a pure packed comparison cycle. The reuse proof additionally requires both windows to come from the same immutable captured array. It derives a nested fixed-update scope from the canonical loop ancestry and effects. For the full compressor the certificate selects exactly the two longer scan sites 2733 and 3122 inside skip recurrence 2398. The short known-width scan and both search recurrences keep their existing comparisons.

Each admitted scan keeps one interval in the current native activation. A current left start inside the previous interval and the same displacement between the two windows prove the overlapping bytes equal. The compiler intersects that interval with the current limit, rounds down to complete eight-byte groups and starts the same compare cycle at the first unproved group. A new record starts empty. Only the existing bound-exit edge can extend it, after the compare cycle reaches that edge. Mismatches continue through the original scalar continuation. Fresh scope entries clear the record. The first eligible skip comparison establishes the interval for later visits in that activation. The earlier four-word prefix, entry assertions, tail reads, match selection, table writes and publication retain their source order on every reached path.

The actual ARM64 vector body has a 28-instruction cycle for each 64-byte equal group, including eight NEON loads and no stack transfers. That cycle remains in the candidate. The new proof entries span 28 and 25 instructions, including address setup and eight and seven stack accesses. Successful reuse reduces visits to the comparison cycle. Those static spans do not count dynamic instructions.

Object comparison verifies 347 bodies. Only the two collector bodies change. The general body moves from 52,028 to 53,248 code bytes and 944 to 1,056 frame bytes. The vector body moves from 41,780 to 42,052 code bytes and 928 to 992 frame bytes. The root body and 57,072-byte frame are identical in the Object. Its linked entry moves from 0x100092080 to 0x100092600.

## Timing frontier

Each ratio below uses the same separate four-actor cohort with 24 balanced orders per case. Both Mech images and Roc use the matching borrowed output contract. The libdeflate 1.26 comparison includes compressor creation and destruction and uses its pinned output bound. Validation is outside timing.

| Case | R274 / Roc | R275 / Roc | R274 / libdeflate | R275 / libdeflate | Paired elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.199350× | 0.200462× | 2.536363× | 2.550512× | +0.406% |
| prefix-periodic-32768 | 0.985956× | 0.908698× | 2.109599× | 1.944295× | -7.513% |
| prefix-periodic-65536 | 1.076877× | 0.991375× | 2.070482× | 1.906090× | -7.624% |
| prefix-periodic-1048576 | 1.159278× | 1.063635× | 1.963402× | 1.801417× | -8.172% |
| prefix-repeated-65536 | 1.044666× | 0.972245× | 1.843924× | 1.716094× | -7.256% |
| prefix-repeated-131072 | 1.095238× | 1.015547× | 1.831637× | 1.698365× | -7.640% |
| prefix-repeated-262144 | 1.115580× | 1.032471× | 1.809245× | 1.674460× | -7.453% |
| prefix-repeated-1048576 | 1.121432× | 1.035256× | 1.761369× | 1.626018× | -7.629% |
| prefix-natural-1048576 | 1.508668× | 1.512215× | 1.215593× | 1.218451× | +0.314% |
| prefix-random-1048576 | 2.111450× | 2.121498× | 2.982260× | 2.996453× | +0.765% |

The elapsed-change column uses the two primary cohorts. The reference multiples use the four-actor cohort.

## Qualification and adverse observations

- The original and decoded full Objects and metadata match exactly. Across both forms 324 complete calls preserve all reference streams, independent decompression, output capacity, aliases and zero final private ownership.
- The compiled recurrence witness checks 50,688 lengths against an independent word oracle, varying alignment, mismatch, window distance, start, bound and fresh invocation. The Rust build uses warnings as errors. All 28 selected tests pass, including the wide equality, nested phase, lexical failure and compact range controls. The tests use a separate source overlay.
- Both 23-case screens reproduce the periodic and repeated improvements. Every pooled gate passes. The first block gate misses empty, and the second misses seven-stored-prefix. Both cases exit before the matcher and the complete-call retired instruction counts are unchanged on those stored cases. The timing misses remain in the packet with the raw blocks. Full acceptance keeps the same timing gates and adds both complete cohorts without replacing any pilot observations.
- The formatter-only source overlay passes Rust formatting. Its independent compiler rebuild emits the same Object and metadata, and the measured source is preserved in the archive.
- The CPU and memory cohorts cover all 81 cases separately from elapsed. The six logical heap families match across the 81 cases. The full library comparison includes 245,664 timed calls.

| Case | Before instructions | Candidate instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,343.0 | 812,386.0 | +0.005% | -2.828% |
| prefix-periodic-32768 | 24,867,060.5 | 22,613,147.0 | -9.064% | -7.559% |
| prefix-periodic-65536 | 49,038,748.0 | 44,516,602.5 | -9.222% | -7.835% |
| prefix-periodic-1048576 | 778,679,680.0 | 706,236,934.0 | -9.303% | -8.187% |
| prefix-repeated-65536 | 49,395,534.5 | 44,885,129.0 | -9.131% | -7.360% |
| prefix-repeated-131072 | 98,548,795.0 | 89,528,590.0 | -9.153% | -7.421% |
| prefix-repeated-262144 | 196,800,612.0 | 178,767,764.0 | -9.163% | -7.530% |
| prefix-repeated-1048576 | 785,960,088.0 | 713,791,418.0 | -9.182% | -7.566% |
| prefix-natural-1048576 | 3,454,112,492.5 | 3,470,795,447.0 | +0.483% | +0.117% |
| prefix-random-1048576 | 858,595,540.5 | 874,130,668.0 | +1.809% | +0.742% |

The seven CPU profiles use the exact linked candidate. Samples include harness validation and are independent of the whole-call retired-instruction and cycle counters.

| Profile input | Running samples | Collector samples | In proof entry | In the two admitted wide spans |
|---|---:|---:|---:|---:|
| eight-literals | 5339 | 618 | 0 | 0 |
| prefix-natural-1048576 | 5361 | 2433 | 0 | 0 |
| prefix-periodic-1048576 | 5365 | 3411 | 194 | 1 |
| prefix-periodic-32768 | 5363 | 3431 | 182 | 3 |
| prefix-random-1048576 | 5370 | 3232 | 0 | 0 |
| prefix-repeated-1048576 | 5606 | 3341 | 186 | 3 |
| prefix-repeated-33027 | 5358 | 3041 | 191 | 4 |

The matching retained profiles give the following static-span comparison.

| Input | R274 samples in the retained long skip cycle | R275 samples in the two admitted long cycles | R275 proof-entry samples |
|---|---:|---:|---:|
| prefix-periodic-32768 | 411 | 3 | 182 |
| prefix-repeated-33027 | 400 | 4 | 191 |

## All primary timings

| Input | Bytes | R274 / Roc | R275 / Roc | R275 / R274 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.090923× | 2.113220× | 1.010664× | pass/pass |
| one-stored | 1 | 2.029517× | 2.041455× | 1.005883× | pass/pass |
| seven-stored-prefix | 7 | 2.234837× | 2.313003× | 1.034976× | pass/pass |
| eight-literals | 8 | 0.201260× | 0.202077× | 1.004057× | pass/pass |
| eight-overlap | 8 | 0.189166× | 0.189226× | 1.000318× | pass/pass |
| 320-overlap | 320 | 0.232157× | 0.227948× | 0.981871× | pass/pass |
| eight-literals-prefix | 8 | 0.199083× | 0.198976× | 0.999459× | pass/pass |
| 128-biased-dynamic | 128 | 0.270532× | 0.269901× | 0.997668× | pass/pass |
| 32-high-entropy-stored | 32 | 0.272779× | 0.270923× | 0.993198× | pass/pass |
| 16384-distribution-change | 16384 | 1.669763× | 1.671778× | 1.001207× | pass/pass |
| 33027-periodic-window | 33027 | 0.994963× | 0.913285× | 0.917908× | pass/pass |
| random-64 | 64 | 0.283040× | 0.282534× | 0.998212× | pass/pass |
| random-256 | 256 | 0.350688× | 0.346864× | 0.989096× | pass/pass |
| random-1024 | 1024 | 0.442104× | 0.434761× | 0.983390× | pass/pass |
| repeated-1024 | 1024 | 0.305282× | 0.297452× | 0.974353× | pass/pass |
| random-32769 | 32769 | 1.730966× | 1.743077× | 1.006996× | pass/pass |
| repeated-32769 | 32769 | 0.954485× | 0.883019× | 0.925126× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.548469× | 0.544384× | 0.992552× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.421729× | 1.426480× | 1.003342× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.472535× | 1.476364× | 1.002601× | pass/pass |
| heldout-window-32767 | 32767 | 1.475032× | 1.476828× | 1.001218× | pass/pass |
| heldout-window-32768 | 32768 | 1.472006× | 1.479051× | 1.004786× | pass/pass |
| heldout-slides-65537 | 65537 | 1.514821× | 1.517416× | 1.001713× | pass/pass |
| heldout-random-65537 | 65537 | 1.913396× | 1.922166× | 1.004583× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.644829× | 1.639235× | 0.996599× | pass/pass |
| prefix-natural-1024 | 1024 | 0.547886× | 0.543627× | 0.992226× | pass/pass |
| prefix-natural-4096 | 4096 | 1.113259× | 1.113362× | 1.000093× | pass/pass |
| prefix-natural-8192 | 8192 | 1.312251× | 1.315379× | 1.002384× | pass/pass |
| prefix-natural-16384 | 16384 | 1.423836× | 1.429966× | 1.004306× | pass/pass |
| prefix-natural-32767 | 32767 | 1.472070× | 1.475220× | 1.002140× | pass/pass |
| prefix-natural-32768 | 32768 | 1.473154× | 1.475628× | 1.001680× | pass/pass |
| prefix-natural-32769 | 32769 | 1.472936× | 1.475875× | 1.001995× | pass/pass |
| prefix-natural-33027 | 33027 | 1.473734× | 1.478339× | 1.003125× | pass/pass |
| prefix-natural-65535 | 65535 | 1.473978× | 1.477730× | 1.002546× | pass/pass |
| prefix-natural-65536 | 65536 | 1.474457× | 1.478307× | 1.002611× | pass/pass |
| prefix-natural-65537 | 65537 | 1.464242× | 1.469351× | 1.003490× | pass/pass |
| prefix-natural-131072 | 131072 | 1.478511× | 1.482313× | 1.002571× | pass/pass |
| prefix-natural-262144 | 262144 | 1.475193× | 1.484234× | 1.006129× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.505158× | 1.509884× | 1.003140× | pass/pass |
| prefix-random-1024 | 1024 | 0.443951× | 0.439309× | 0.989543× | pass/pass |
| prefix-random-4096 | 4096 | 0.794909× | 0.789788× | 0.993557× | pass/pass |
| prefix-random-8192 | 8192 | 1.037455× | 1.042150× | 1.004526× | pass/pass |
| prefix-random-16384 | 16384 | 1.431528× | 1.438371× | 1.004780× | pass/pass |
| prefix-random-32767 | 32767 | 1.738976× | 1.742402× | 1.001970× | pass/pass |
| prefix-random-32768 | 32768 | 1.732437× | 1.746923× | 1.008361× | pass/pass |
| prefix-random-32769 | 32769 | 1.727002× | 1.738711× | 1.006780× | pass/pass |
| prefix-random-33027 | 33027 | 1.728094× | 1.740657× | 1.007270× | pass/pass |
| prefix-random-65535 | 65535 | 1.934530× | 1.949209× | 1.007588× | pass/pass |
| prefix-random-65536 | 65536 | 1.931989× | 1.941416× | 1.004880× | pass/pass |
| prefix-random-65537 | 65537 | 1.925818× | 1.942732× | 1.008783× | pass/pass |
| prefix-random-131072 | 131072 | 2.032747× | 2.044917× | 1.005987× | pass/pass |
| prefix-random-262144 | 262144 | 2.083022× | 2.097116× | 1.006766× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.105410× | 2.121523× | 1.007653× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.370051× | 0.361443× | 0.976738× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.578677× | 0.550253× | 0.950881× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.707256× | 0.669910× | 0.947196× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.846446× | 0.784151× | 0.926404× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.970173× | 0.901353× | 0.929065× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.976456× | 0.903098× | 0.924874× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.980580× | 0.909190× | 0.927196× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.991922× | 0.911926× | 0.919352× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.070589× | 0.984213× | 0.919319× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.070827× | 0.989183× | 0.923756× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.070597× | 0.985995× | 0.920978× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.122034× | 1.030340× | 0.918279× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.143197× | 1.051794× | 0.920046× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.156790× | 1.062261× | 0.918284× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.305689× | 0.297733× | 0.973972× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.517355× | 0.489148× | 0.945479× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.661972× | 0.623992× | 0.942626× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.800782× | 0.746189× | 0.931826× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.929984× | 0.862643× | 0.927589× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.929846× | 0.859885× | 0.924760× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.949213× | 0.879753× | 0.926823× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.974899× | 0.903846× | 0.927118× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.039381× | 0.961846× | 0.925403× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.041121× | 0.965575× | 0.927438× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.041418× | 0.963526× | 0.925206× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.093004× | 1.009502× | 0.923603× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.115727× | 1.032575× | 0.925473× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.120302× | 1.034839× | 0.923714× | pass/pass |

[Verified compiler patch, linked controls and all measurement receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r275-shifted-immutable-equality-0bff8acf380b.tar.gz). SHA-256 `0bff8acf380b64fcc23250beb25995512c74f28eb7358e7e679d76651dbd90d0`.
