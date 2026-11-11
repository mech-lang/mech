# R277 — Measure a dense offset lookup table

**Keep the R276 compiler, R272 runtime and original benchmark source.** The source experiment changes paired elapsed by -0.320% across the 81 level-12 cases. Both timing gates pass 81/81 and 81/81. Both variants beat Roc on 38/81 cases (46.91%) and the dense variant beats libdeflate on 3/81 in its separate cohort.

The two 81-case cohorts change elapsed by -0.392% and -0.247%. The largest paired slowdown is +1.769% on 320-overlap. Cases crossing 1× Roc include none.

## Source and linked evidence

The variant replaces arithmetic and a 256-byte table with a 32,769-byte U8 distance-to-slot map for distances through 32,768. The original out-of-range rule and the separate U32 slot-cost load remain. The table agrees with libdeflate 1.26 for every supported nonzero distance. The logical compression decisions, allocation sites, capacities, input and output aliases and full-call timing boundary are preserved.

Measured and published parent 5bf88bd67a20eb9b3bf98d05e38892e8758cf1a0. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source 8830db78789aee692d89583d25458bcc32f300b3 with the retained R276 overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The recompiled source control reproduces the retained Object and linked worker byte-for-byte.

The large literal hit the default driver's syntax/recovery-limit diagnostic. The source was then compiled by a benchmark driver with explicit maximum parser limits. That driver changes only input admission for the experiment. The same original and decoded Object contains the literal as contiguous image bytes. The native scalar witness initially used the heap-call signature and failed to link. The corrected witness uses the emitted two-argument entry.

The Object grows from 1,481,616 to 1,516,192 bytes and its root frame from 57,072 to 57,216 bytes. Both variants have 170 numeric phases and 4,548 root calls.

| Linked route from packed offset extraction through slot cost | Instructions | Stack accesses |
|---|---:|---:|
| R276 | 22 | 4 |
| Dense map | 18 | 4 |
| libdeflate | 3 | 0 |

The dense route still loads descriptors and checks the one-based addition, the current table view and the slot-cost index. The three-instruction C sequence performs the corresponding packed offset extraction and two loads. The code excerpts identify this specific route. CPU sample weights and whole-call counter measurements have separate collection boundaries.

| 23-case screen | All-case elapsed change | Natural 32 KiB | Natural 1 MiB | Pooled and block misses |
|---|---:|---:|---:|---|
| dense | -0.162% | -0.445% | -0.465% | none / none |
| dense-confirmation | -0.109% | -0.409% | -0.480% | none / none |

## Matched frontier

Reference ratios use the fresh balanced four-actor comparison. The elapsed-change column uses the two primary cohorts. The complete compression API call includes scratch construction, output transfer and cleanup. Validation is outside the timer.

| Input | R276 / Roc | Dense / Roc | R276 / libdeflate | Dense / libdeflate | Elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.205395× | 0.202999× | 2.611415× | 2.580950× | -0.695% |
| prefix-random-8192 | 1.038063× | 1.042410× | 3.693084× | 3.708548× | -0.014% |
| prefix-periodic-65536 | 0.987206× | 0.987397× | 1.903066× | 1.903433× | -0.440% |
| prefix-repeated-131072 | 1.010554× | 1.013244× | 1.694033× | 1.698543× | -0.188% |
| prefix-repeated-1048576 | 1.035346× | 1.035726× | 1.627327× | 1.627924× | +0.022% |
| prefix-periodic-1048576 | 1.062828× | 1.061825× | 1.800334× | 1.798635× | -0.078% |
| prefix-natural-32768 | 1.473302× | 1.467235× | 1.344246× | 1.338710× | -0.412% |
| prefix-natural-1048576 | 1.501720× | 1.497194× | 1.210516× | 1.206867× | -0.557% |
| prefix-random-1048576 | 2.132166× | 2.101199× | 3.092903× | 3.047982× | -0.243% |

## Correctness, counters and profiles

- All 486 full original and decoded compression calls produced exact reference bytes, passed independent inflation and preserved input/output aliases and capacities. Original and decoded variant Objects and metadata are identical.
- Both linked scalar witness forms pass 65,545 independently checked offset values each, including zero, the last valid distance and large U64 inputs.
- The 81-case four-actor comparison includes 245,664 timed calls. Separate balanced counter cohorts preserve the same measured payload request count, requested payload bytes, peak payload, copied bytes, logical zeroed bytes and residual ownership in every case.
- Natural 1 MiB retires 45,996,016 fewer instructions (-1.334%). The separate cycles and time measurements below show the difference between removing instructions and improving elapsed time.

| Input | R276 instructions | Dense instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 818,948.5 | 817,074.5 | -0.229% | -3.106% |
| prefix-random-8192 | 7,432,095.0 | 7,425,749.5 | -0.085% | +0.594% |
| prefix-periodic-65536 | 44,514,312.0 | 44,505,208.5 | -0.020% | +0.055% |
| prefix-repeated-131072 | 89,523,829.5 | 89,508,117.5 | -0.018% | -0.010% |
| prefix-repeated-1048576 | 713,751,716.5 | 713,639,771.5 | -0.016% | -0.023% |
| prefix-periodic-1048576 | 706,188,867.5 | 706,079,413.5 | -0.015% | -0.015% |
| prefix-natural-32768 | 94,509,535.5 | 93,269,829.5 | -1.312% | +0.197% |
| prefix-natural-1048576 | 3,447,230,804.5 | 3,401,234,789.0 | -1.334% | +0.024% |
| prefix-random-1048576 | 874,086,062.5 | 874,065,459.5 | -0.002% | +0.022% |

Independent CPU sampling pins the exact dense linked executable and Object. The tables below include the profile harness and validation, and represent sample locations.

| Input | Running samples | Path-parser body | Matcher body |
|---|---:|---:|---:|
| eight-literals | 5343 | 0 | 726 |
| prefix-natural-1048576 | 5366 | 2550 | 2432 |
| prefix-natural-32768 | 5612 | 2639 | 2284 |
| prefix-periodic-1048576 | 5362 | 1210 | 3416 |
| prefix-random-1048576 | 5360 | 882 | 3223 |

The next compiler experiment starts from R276 with the unchanged benchmark source. It targets the full recurring empty path under the existing no-growth contract. The dense lookup result supports keeping constant data resident in the image, with range and current-view proof reuse still needed to shorten this route further.

## All primary timings

| Input | Bytes | R276 / Roc | Dense / Roc | Dense / R276 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.098594× | 2.063450× | 0.983253× | pass/pass |
| one-stored | 1 | 2.009161× | 1.992011× | 0.991464× | pass/pass |
| seven-stored-prefix | 7 | 2.216339× | 2.203595× | 0.994250× | pass/pass |
| eight-literals | 8 | 0.203135× | 0.201723× | 0.993048× | pass/pass |
| eight-overlap | 8 | 0.191076× | 0.189671× | 0.992645× | pass/pass |
| 320-overlap | 320 | 0.232245× | 0.236353× | 1.017688× | pass/pass |
| eight-literals-prefix | 8 | 0.200788× | 0.199375× | 0.992966× | pass/pass |
| 128-biased-dynamic | 128 | 0.270241× | 0.269688× | 0.997957× | pass/pass |
| 32-high-entropy-stored | 32 | 0.268776× | 0.266999× | 0.993389× | pass/pass |
| 16384-distribution-change | 16384 | 1.669468× | 1.663894× | 0.996661× | pass/pass |
| 33027-periodic-window | 33027 | 0.914564× | 0.912131× | 0.997341× | pass/pass |
| random-64 | 64 | 0.281618× | 0.279666× | 0.993069× | pass/pass |
| random-256 | 256 | 0.344635× | 0.342888× | 0.994930× | pass/pass |
| random-1024 | 1024 | 0.432095× | 0.430856× | 0.997131× | pass/pass |
| repeated-1024 | 1024 | 0.298477× | 0.297123× | 0.995464× | pass/pass |
| random-32769 | 32769 | 1.741587× | 1.739342× | 0.998711× | pass/pass |
| repeated-32769 | 32769 | 0.891309× | 0.887509× | 0.995737× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.541602× | 0.539902× | 0.996861× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.418529× | 1.413296× | 0.996311× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.467417× | 1.458754× | 0.994096× | pass/pass |
| heldout-window-32767 | 32767 | 1.473549× | 1.468299× | 0.996438× | pass/pass |
| heldout-window-32768 | 32768 | 1.472437× | 1.466332× | 0.995854× | pass/pass |
| heldout-slides-65537 | 65537 | 1.514351× | 1.508103× | 0.995874× | pass/pass |
| heldout-random-65537 | 65537 | 1.923443× | 1.922343× | 0.999428× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.636931× | 1.633372× | 0.997826× | pass/pass |
| prefix-natural-1024 | 1024 | 0.542109× | 0.539634× | 0.995434× | pass/pass |
| prefix-natural-4096 | 4096 | 1.093476× | 1.088056× | 0.995044× | pass/pass |
| prefix-natural-8192 | 8192 | 1.296131× | 1.291909× | 0.996743× | pass/pass |
| prefix-natural-16384 | 16384 | 1.419946× | 1.411530× | 0.994073× | pass/pass |
| prefix-natural-32767 | 32767 | 1.474563× | 1.470150× | 0.997007× | pass/pass |
| prefix-natural-32768 | 32768 | 1.473650× | 1.467584× | 0.995884× | pass/pass |
| prefix-natural-32769 | 32769 | 1.472853× | 1.466502× | 0.995688× | pass/pass |
| prefix-natural-33027 | 33027 | 1.472810× | 1.466189× | 0.995504× | pass/pass |
| prefix-natural-65535 | 65535 | 1.473376× | 1.465073× | 0.994365× | pass/pass |
| prefix-natural-65536 | 65536 | 1.472711× | 1.462364× | 0.992975× | pass/pass |
| prefix-natural-65537 | 65537 | 1.470204× | 1.461741× | 0.994244× | pass/pass |
| prefix-natural-131072 | 131072 | 1.475626× | 1.467702× | 0.994630× | pass/pass |
| prefix-natural-262144 | 262144 | 1.477226× | 1.470137× | 0.995201× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.502419× | 1.494046× | 0.994427× | pass/pass |
| prefix-random-1024 | 1024 | 0.439015× | 0.436425× | 0.994102× | pass/pass |
| prefix-random-4096 | 4096 | 0.785974× | 0.778002× | 0.989857× | pass/pass |
| prefix-random-8192 | 8192 | 1.042386× | 1.042236× | 0.999856× | pass/pass |
| prefix-random-16384 | 16384 | 1.442114× | 1.443742× | 1.001129× | pass/pass |
| prefix-random-32767 | 32767 | 1.748233× | 1.751339× | 1.001777× | pass/pass |
| prefix-random-32768 | 32768 | 1.746071× | 1.742556× | 0.997987× | pass/pass |
| prefix-random-32769 | 32769 | 1.752022× | 1.753661× | 1.000936× | pass/pass |
| prefix-random-33027 | 33027 | 1.749162× | 1.749520× | 1.000205× | pass/pass |
| prefix-random-65535 | 65535 | 1.944776× | 1.942945× | 0.999059× | pass/pass |
| prefix-random-65536 | 65536 | 1.949460× | 1.943384× | 0.996883× | pass/pass |
| prefix-random-65537 | 65537 | 1.939329× | 1.935959× | 0.998262× | pass/pass |
| prefix-random-131072 | 131072 | 2.049632× | 2.045128× | 0.997802× | pass/pass |
| prefix-random-262144 | 262144 | 2.097162× | 2.097299× | 1.000065× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.125614× | 2.120453× | 0.997572× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.361462× | 0.359304× | 0.994032× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.551741× | 0.546994× | 0.991396× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.669382× | 0.667191× | 0.996727× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.790111× | 0.785288× | 0.993896× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.902953× | 0.898023× | 0.994540× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.903879× | 0.904485× | 1.000670× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.911804× | 0.907192× | 0.994942× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.920590× | 0.915356× | 0.994315× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.985864× | 0.984567× | 0.998684× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.987484× | 0.983139× | 0.995600× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.986661× | 0.989386× | 1.002761× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.032316× | 1.031696× | 0.999399× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.050876× | 1.051949× | 1.001021× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.062672× | 1.061843× | 0.999220× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.298197× | 0.296935× | 0.995766× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.493034× | 0.492389× | 0.998693× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.631665× | 0.632254× | 1.000932× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.746392× | 0.746996× | 1.000810× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.862956× | 0.862150× | 0.999066× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.865903× | 0.863150× | 0.996820× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.886082× | 0.881476× | 0.994803× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.904495× | 0.901431× | 0.996612× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.959563× | 0.960341× | 1.000811× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.967398× | 0.962985× | 0.995438× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.966335× | 0.967432× | 1.001135× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.011317× | 1.009420× | 0.998124× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.030930× | 1.030891× | 0.999962× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.035176× | 1.035401× | 1.000217× | pass/pass |

[Verified source, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r277-dense-offset-slot-47ba57889a90.tar.gz). SHA-256 47ba57889a90f8889e165b21f81c7b8ed9ae660ba95eab2297c892b0682c80d7.
