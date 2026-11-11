# R284 — Preserve packed integer widths through joins

**Retain R284 with the R272 runtime.** Candidate paired elapsed changes -1.132% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R283 beats Roc on 45/81 cases and R284 beats it on 45/81 (55.56%). The independent four-actor cohort gives R284 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -1.193% and -1.070%. The largest slowdown is +1.074% on random-1024. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent e6e9a28c1f7f305c612e624410bb0e3d2417e2ba. Measured parent e6e9a28c1f7f305c612e624410bb0e3d2417e2ba. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R283 closure and a five-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The candidate uses the pinned input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary.

## Compiler rule and emitted code

The compiler derives an unsigned bit bound for current scalar SSA values in private vector phases. Typed loads, unsigned extension, truncation, masks and constant shifts establish bounds. At each merge the bound includes every incoming argument. A monotone worklist includes values produced by the loop backedge. Unknown values use their full type widths. The pass folds a conditional branch only when that width implies its original outcome.

The packed matcher merges a U32 load with a word assembled from four U8 values. Both paths produce at most 32 bits. The narrowed check disappears after that merge. The source length and window tests still run. The pinned Roc and libdeflate ARM64 matcher excerpts use W-register hash arithmetic on their packed values.

The selected matcher loses five unsigned high branches and shrinks from 28,500 to 28,040 code bytes. Its stack frame is 928 bytes. The audit checks 347 machine bodies. All other bodies are byte-identical, including the root and path parser. The existing owner and view guards select the same set of vector variants.

The initial 27-case screen changes paired elapsed by -0.777%. The screen records 0 pooled and 0 block misses.

## Headline matched ratios

| Input | R283 / Roc | R284 / Roc | R283 / libdeflate | R284 / libdeflate | R284 / R283 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.198133× | 0.196635× | 2.532331× | 2.513178× | -1.019% |
| prefix-random-8192 | 0.904537× | 0.896195× | 3.235427× | 3.205589× | -1.599% |
| prefix-natural-4096 | 1.082819× | 1.072412× | 1.863732× | 1.845820× | -0.824% |
| prefix-natural-1048576 | 1.510127× | 1.494275× | 1.217776× | 1.204992× | -0.671% |
| prefix-random-1048576 | 1.866443× | 1.814843× | 2.635160× | 2.562309× | -2.777% |
| prefix-periodic-1048576 | 0.905098× | 0.899808× | 1.533406× | 1.524444× | -0.590% |
| prefix-repeated-1048576 | 0.888924× | 0.884224× | 1.398782× | 1.391387× | -0.502% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 0.9719× R283.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- Two warnings-as-errors JIT tests pass. They compare 36 branch-join and 60 backedge input pairs, including a wide incoming value and a width-growing recurrence whose checks must stay. The first test run used the pre-fold control-flow graph for post-fold verification. The corrected harness refreshes that graph. The failure and corrected test receipts are archived.
- All 42 selected control-flow cleanup tests pass, including the two new JIT tests.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R283 instructions | R284 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 813,217.0 | 813,192.0 | -0.003% | -0.904% |
| prefix-random-8192 | 6,594,530.0 | 6,534,828.5 | -0.905% | -1.366% |
| prefix-natural-4096 | 10,979,601.0 | 10,896,101.0 | -0.761% | -0.831% |
| prefix-natural-1048576 | 3,393,708,616.0 | 3,340,890,146.5 | -1.556% | -0.828% |
| prefix-random-1048576 | 762,300,337.5 | 748,033,528.0 | -1.872% | -3.061% |
| prefix-periodic-1048576 | 641,836,584.5 | 642,828,773.5 | +0.155% | -0.643% |
| prefix-repeated-1048576 | 649,475,300.0 | 650,468,809.0 | +0.153% | -0.558% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,559,680 bytes before and 54,559,680 bytes after. These process-wide samples include the harness.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser samples | Vector matcher samples | Matcher stack-transfer PCs |
|---|---:|---:|---:|---:|
| eight-literals | 5488 | 9 | 655 | 18 |
| prefix-natural-1048576 | 5358 | 2638 | 2325 | 1346 |
| prefix-natural-32768 | 5359 | 2626 | 2118 | 1186 |
| prefix-natural-4096 | 5580 | 1529 | 1744 | 760 |
| prefix-periodic-1048576 | 5355 | 675 | 3815 | 2346 |
| prefix-random-1048576 | 5360 | 430 | 3467 | 1967 |
| prefix-repeated-1048576 | 5362 | 630 | 3552 | 2195 |

The exact linked parser emits two strict selection spans with a duplicate comparison. The item calculation between comparisons consists of flag-preserving register writes and leaves the compared registers live. On 1 MiB natural text the parser accounts for 2,638 of 5,358 samples, with 558 in the two spans. The compact timing worker omits the ordinary path’s additional managed admission, accounting and recovery.

The closest candidate case above parity is prefix-natural-4096 at 1.096148× Roc. Extend the existing comparison tracker through the exact non-flag-setting add and shifted OR used to construct the selected item. Require the current compare operands to survive. Audit the complete parser, including empty visits, and then run the same paired tests.

## All primary timings

| Input | Bytes | R283 / Roc | R284 / Roc | R284 / R283 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.172197× | 2.121203× | 0.976524× | pass/pass |
| one-stored | 1 | 2.083350× | 2.030466× | 0.974616× | pass/pass |
| seven-stored-prefix | 7 | 2.255011× | 2.261118× | 1.002708× | pass/pass |
| eight-literals | 8 | 0.198181× | 0.196163× | 0.989813× | pass/pass |
| eight-overlap | 8 | 0.187814× | 0.185885× | 0.989731× | pass/pass |
| 320-overlap | 320 | 0.219186× | 0.217325× | 0.991508× | pass/pass |
| eight-literals-prefix | 8 | 0.199139× | 0.197032× | 0.989418× | pass/pass |
| 128-biased-dynamic | 128 | 0.266397× | 0.267612× | 1.004560× | pass/pass |
| 32-high-entropy-stored | 32 | 0.268699× | 0.266028× | 0.990061× | pass/pass |
| 16384-distribution-change | 16384 | 1.581288× | 1.560930× | 0.987126× | pass/pass |
| 33027-periodic-window | 33027 | 0.793601× | 0.785330× | 0.989578× | pass/pass |
| random-64 | 64 | 0.282643× | 0.281526× | 0.996049× | pass/pass |
| random-256 | 256 | 0.340436× | 0.342988× | 1.007494× | pass/pass |
| random-1024 | 1024 | 0.412495× | 0.416927× | 1.010743× | pass/pass |
| repeated-1024 | 1024 | 0.272355× | 0.269895× | 0.990968× | pass/pass |
| random-32769 | 32769 | 1.512179× | 1.479407× | 0.978328× | pass/pass |
| repeated-32769 | 32769 | 0.771324× | 0.764046× | 0.990563× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.532387× | 0.533282× | 1.001681× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.430867× | 1.414276× | 0.988405× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.480430× | 1.464001× | 0.988903× | pass/pass |
| heldout-window-32767 | 32767 | 1.485341× | 1.467611× | 0.988063× | pass/pass |
| heldout-window-32768 | 32768 | 1.484564× | 1.467292× | 0.988365× | pass/pass |
| heldout-slides-65537 | 65537 | 1.517484× | 1.497540× | 0.986857× | pass/pass |
| heldout-random-65537 | 65537 | 1.668819× | 1.628330× | 0.975738× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.492074× | 1.473375× | 0.987467× | pass/pass |
| prefix-natural-1024 | 1024 | 0.533636× | 0.534027× | 1.000733× | pass/pass |
| prefix-natural-4096 | 4096 | 1.105258× | 1.096148× | 0.991758× | pass/pass |
| prefix-natural-8192 | 8192 | 1.315209× | 1.301706× | 0.989733× | pass/pass |
| prefix-natural-16384 | 16384 | 1.432153× | 1.416727× | 0.989229× | pass/pass |
| prefix-natural-32767 | 32767 | 1.486725× | 1.469526× | 0.988432× | pass/pass |
| prefix-natural-32768 | 32768 | 1.485943× | 1.466459× | 0.986888× | pass/pass |
| prefix-natural-32769 | 32769 | 1.485313× | 1.465574× | 0.986710× | pass/pass |
| prefix-natural-33027 | 33027 | 1.489578× | 1.470643× | 0.987289× | pass/pass |
| prefix-natural-65535 | 65535 | 1.485912× | 1.471893× | 0.990565× | pass/pass |
| prefix-natural-65536 | 65536 | 1.485349× | 1.471460× | 0.990649× | pass/pass |
| prefix-natural-65537 | 65537 | 1.480828× | 1.465945× | 0.989950× | pass/pass |
| prefix-natural-131072 | 131072 | 1.485032× | 1.472215× | 0.991369× | pass/pass |
| prefix-natural-262144 | 262144 | 1.488814× | 1.475781× | 0.991246× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.508491× | 1.498365× | 0.993287× | pass/pass |
| prefix-random-1024 | 1024 | 0.417040× | 0.420763× | 1.008926× | pass/pass |
| prefix-random-4096 | 4096 | 0.710301× | 0.699629× | 0.984975× | pass/pass |
| prefix-random-8192 | 8192 | 0.913306× | 0.898703× | 0.984011× | pass/pass |
| prefix-random-16384 | 16384 | 1.256664× | 1.234157× | 0.982090× | pass/pass |
| prefix-random-32767 | 32767 | 1.517834× | 1.483731× | 0.977532× | pass/pass |
| prefix-random-32768 | 32768 | 1.512560× | 1.491106× | 0.985816× | pass/pass |
| prefix-random-32769 | 32769 | 1.511693× | 1.477107× | 0.977121× | pass/pass |
| prefix-random-33027 | 33027 | 1.511958× | 1.482939× | 0.980807× | pass/pass |
| prefix-random-65535 | 65535 | 1.704488× | 1.662244× | 0.975216× | pass/pass |
| prefix-random-65536 | 65536 | 1.703483× | 1.657441× | 0.972972× | pass/pass |
| prefix-random-65537 | 65537 | 1.694251× | 1.655922× | 0.977377× | pass/pass |
| prefix-random-131072 | 131072 | 1.798753× | 1.751721× | 0.973854× | pass/pass |
| prefix-random-262144 | 262144 | 1.842946× | 1.795855× | 0.974448× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.864879× | 1.813085× | 0.972226× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.333858× | 0.332177× | 0.994965× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.482178× | 0.477360× | 0.990006× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.575303× | 0.569793× | 0.990422× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.691818× | 0.681961× | 0.985753× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.775737× | 0.769105× | 0.991451× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.779300× | 0.771697× | 0.990244× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.783983× | 0.776959× | 0.991040× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.795258× | 0.782294× | 0.983699× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.844674× | 0.839417× | 0.993777× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.848895× | 0.841533× | 0.991327× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.849721× | 0.841778× | 0.990653× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.878959× | 0.873166× | 0.993410× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.895308× | 0.890018× | 0.994091× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.902029× | 0.896706× | 0.994099× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.272726× | 0.270292× | 0.991078× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.438516× | 0.427467× | 0.974803× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.536654× | 0.530130× | 0.987844× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.654605× | 0.647877× | 0.989723× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.750976× | 0.743795× | 0.990438× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.747510× | 0.737716× | 0.986897× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.769129× | 0.760954× | 0.989371× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.790969× | 0.782201× | 0.988915× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.835160× | 0.827867× | 0.991268× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.835294× | 0.828014× | 0.991284× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.837437× | 0.828015× | 0.988749× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.873842× | 0.865710× | 0.990694× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.887388× | 0.882347× | 0.994318× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.889882× | 0.885416× | 0.994981× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r284-packed-join-widths-8468d74a1c5e.tar.gz). SHA-256 8468d74a1c5ecb689cb26495bf550272d168be850fd58576895a9b693aeb3c16.
