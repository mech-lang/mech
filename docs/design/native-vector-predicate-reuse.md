# R283 — Reuse selected-edge predicates in private vector phases

**Retain R283 with the R272 runtime.** Candidate paired elapsed changes -1.947% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R280 beats Roc on 45/81 cases and R283 beats it on 45/81 (55.56%). The independent four-actor cohort gives R283 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -1.993% and -1.901%. The largest slowdown is +1.561% on one-stored. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent 8521ae943b38d07f8b042e4643b0a33630c02c6a. Measured parent 74205739c757a23c01994b902649a765589cf364. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R280 closure and a one-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary stay fixed.

## Compiler rule and emitted code

The existing selected-edge pass propagates range facts through the current SSA values of a uniquely reached branch. The compact compiler now runs it in private vector phases after ordinary lowering. The pass replaces a conditional branch only when its condition is fixed on the current route. It leaves the dominating check, computed result, current view admission and rejection on other routes in place.

In the measured matcher, the source four-byte test of the current input length remains. Its selected route already proves the following subtraction safe, so the second comparison and its impossible failure branch disappear. The checked window comparison before the load remains. All general numeric bodies are byte-identical.

The pass removes 220 static branches in 43 private vector bodies. The main vector matcher loses 62 branches and shrinks from 42,052 to 28,500 code bytes and from 992 to 928 stack bytes. Forty-two existing vector bodies now differ from their general counterparts, which makes the caller select those bodies through the existing guards. The root grows from 1,039,432 to 1,045,360 code bytes and keeps its 57,072-byte frame. The generated Object audit identifies 44 changed bodies among 347. The remaining bodies, including the main path parser, match exactly.

The initial 27-case screen changes paired elapsed by -2.332%. The screen records 0 pooled and 0 block misses. The screen includes a slower eight-literal observation.

## Headline matched ratios

| Input | R280 / Roc | R283 / Roc | R280 / libdeflate | R283 / libdeflate | R283 / R280 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.197765× | 0.199712× | 2.546495× | 2.571557× | +1.182% |
| prefix-random-8192 | 0.934169× | 0.902508× | 3.336731× | 3.223645× | -3.945% |
| prefix-natural-4096 | 1.110450× | 1.088399× | 1.904977× | 1.867148× | -0.873% |
| prefix-natural-1048576 | 1.526360× | 1.509112× | 1.231512× | 1.217595× | -1.023% |
| prefix-random-1048576 | 1.919229× | 1.866758× | 2.715268× | 2.641033× | -2.788% |
| prefix-periodic-1048576 | 0.933312× | 0.903041× | 1.580701× | 1.529433× | -3.359% |
| prefix-repeated-1048576 | 0.916928× | 0.889635× | 1.442804× | 1.399858× | -3.093% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 1.0260× R280.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- Five existing selected-edge tests pass with warnings-as-errors on the current compiler. They cover current value transport, failure prefixes, changed loads, join argument ordering, and refusal across unsupported reentry. The complete linked calls exercise the selected dispatch through the existing owner and vector guards.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R280 instructions | R283 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,073.0 | 813,266.5 | +0.147% | -3.262% |
| prefix-random-8192 | 6,940,097.0 | 6,594,998.0 | -4.973% | -2.812% |
| prefix-natural-4096 | 11,231,472.0 | 10,981,236.5 | -2.228% | -0.417% |
| prefix-natural-1048576 | 3,489,491,151.5 | 3,393,725,750.5 | -2.744% | -0.265% |
| prefix-random-1048576 | 814,557,959.0 | 762,299,748.0 | -6.416% | -2.590% |
| prefix-periodic-1048576 | 664,962,996.5 | 641,835,508.0 | -3.478% | -3.430% |
| prefix-repeated-1048576 | 672,520,892.5 | 649,484,026.5 | -3.425% | -3.094% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,559,680 bytes before and 54,592,448 bytes after. These process-wide samples include the harness.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser samples | Vector matcher samples | Matcher stack-transfer PCs |
|---|---:|---:|---:|---:|
| eight-literals | 5350 | 4 | 625 | 31 |
| prefix-natural-1048576 | 5350 | 2615 | 2361 | 1621 |
| prefix-natural-32768 | 5357 | 2589 | 2148 | 1402 |
| prefix-natural-4096 | 5585 | 1529 | 1745 | 919 |
| prefix-periodic-1048576 | 5359 | 657 | 3820 | 2287 |
| prefix-random-1048576 | 5361 | 435 | 3506 | 2560 |
| prefix-repeated-1048576 | 5360 | 623 | 3561 | 2059 |

The matcher retains an unsigned 32-bit narrowing check after merging the packed load with checked byte assembly. The corresponding pinned libdeflate code loads a 32-bit word and uses W-register operations through the hash. Both Mech paths need an explicit current width proof at their join before the check can be removed.

The closest candidate case above parity is prefix-natural-4096 at 1.107337× Roc. Carry integer width through packed-read joins. The exact matcher still checks the merged result against 0xffffffff after a 32-bit load or byte assembly. Prove the range on every incoming edge before removing the check.

## All primary timings

| Input | Bytes | R280 / Roc | R283 / Roc | R283 / R280 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.112947× | 2.134627× | 1.010261× | pass/pass |
| one-stored | 1 | 1.992436× | 2.023540× | 1.015611× | pass/pass |
| seven-stored-prefix | 7 | 2.268758× | 2.239406× | 0.987063× | pass/pass |
| eight-literals | 8 | 0.200266× | 0.202633× | 1.011817× | pass/pass |
| eight-overlap | 8 | 0.187586× | 0.190250× | 1.014202× | pass/pass |
| 320-overlap | 320 | 0.219512× | 0.220603× | 1.004969× | pass/pass |
| eight-literals-prefix | 8 | 0.198918× | 0.199762× | 1.004248× | pass/pass |
| 128-biased-dynamic | 128 | 0.268359× | 0.271653× | 1.012275× | pass/pass |
| 32-high-entropy-stored | 32 | 0.266665× | 0.267109× | 1.001665× | pass/pass |
| 16384-distribution-change | 16384 | 1.623521× | 1.584210× | 0.975786× | pass/pass |
| 33027-periodic-window | 33027 | 0.816414× | 0.794307× | 0.972922× | pass/pass |
| random-64 | 64 | 0.278365× | 0.279541× | 1.004226× | pass/pass |
| random-256 | 256 | 0.340039× | 0.339793× | 0.999275× | pass/pass |
| random-1024 | 1024 | 0.415664× | 0.412557× | 0.992524× | pass/pass |
| repeated-1024 | 1024 | 0.274163× | 0.272435× | 0.993696× | pass/pass |
| random-32769 | 32769 | 1.560946× | 1.500680× | 0.961391× | pass/pass |
| repeated-32769 | 32769 | 0.802903× | 0.780315× | 0.971867× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.534954× | 0.531531× | 0.993601× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.446625× | 1.431719× | 0.989696× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.498849× | 1.484192× | 0.990222× | pass/pass |
| heldout-window-32767 | 32767 | 1.500897× | 1.491338× | 0.993631× | pass/pass |
| heldout-window-32768 | 32768 | 1.499804× | 1.485415× | 0.990406× | pass/pass |
| heldout-slides-65537 | 65537 | 1.536788× | 1.518949× | 0.988392× | pass/pass |
| heldout-random-65537 | 65537 | 1.729959× | 1.672366× | 0.966708× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.515353× | 1.489489× | 0.982932× | pass/pass |
| prefix-natural-1024 | 1024 | 0.539856× | 0.536219× | 0.993264× | pass/pass |
| prefix-natural-4096 | 4096 | 1.117083× | 1.107337× | 0.991275× | pass/pass |
| prefix-natural-8192 | 8192 | 1.330507× | 1.319069× | 0.991403× | pass/pass |
| prefix-natural-16384 | 16384 | 1.444669× | 1.431390× | 0.990809× | pass/pass |
| prefix-natural-32767 | 32767 | 1.498939× | 1.483940× | 0.989994× | pass/pass |
| prefix-natural-32768 | 32768 | 1.498351× | 1.482378× | 0.989339× | pass/pass |
| prefix-natural-32769 | 32769 | 1.503095× | 1.486115× | 0.988703× | pass/pass |
| prefix-natural-33027 | 33027 | 1.502738× | 1.485868× | 0.988774× | pass/pass |
| prefix-natural-65535 | 65535 | 1.497237× | 1.485488× | 0.992153× | pass/pass |
| prefix-natural-65536 | 65536 | 1.493070× | 1.487925× | 0.996554× | pass/pass |
| prefix-natural-65537 | 65537 | 1.492962× | 1.481383× | 0.992245× | pass/pass |
| prefix-natural-131072 | 131072 | 1.499269× | 1.486676× | 0.991600× | pass/pass |
| prefix-natural-262144 | 262144 | 1.501894× | 1.485511× | 0.989092× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.525344× | 1.509733× | 0.989766× | pass/pass |
| prefix-random-1024 | 1024 | 0.420565× | 0.416898× | 0.991281× | pass/pass |
| prefix-random-4096 | 4096 | 0.734092× | 0.709029× | 0.965859× | pass/pass |
| prefix-random-8192 | 8192 | 0.944416× | 0.907156× | 0.960547× | pass/pass |
| prefix-random-16384 | 16384 | 1.289546× | 1.245293× | 0.965683× | pass/pass |
| prefix-random-32767 | 32767 | 1.566579× | 1.512810× | 0.965677× | pass/pass |
| prefix-random-32768 | 32768 | 1.566428× | 1.513519× | 0.966223× | pass/pass |
| prefix-random-32769 | 32769 | 1.564287× | 1.509528× | 0.964994× | pass/pass |
| prefix-random-33027 | 33027 | 1.570718× | 1.512983× | 0.963243× | pass/pass |
| prefix-random-65535 | 65535 | 1.755195× | 1.696095× | 0.966329× | pass/pass |
| prefix-random-65536 | 65536 | 1.755276× | 1.702534× | 0.969953× | pass/pass |
| prefix-random-65537 | 65537 | 1.747671× | 1.686553× | 0.965029× | pass/pass |
| prefix-random-131072 | 131072 | 1.849420× | 1.794217× | 0.970151× | pass/pass |
| prefix-random-262144 | 262144 | 1.894886× | 1.843453× | 0.972857× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.921500× | 1.867932× | 0.972122× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.341752× | 0.340305× | 0.995767× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.497269× | 0.484806× | 0.974937× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.587441× | 0.569374× | 0.969245× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.711555× | 0.688621× | 0.967769× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.802969× | 0.776097× | 0.966534× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.803931× | 0.777588× | 0.967233× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.815320× | 0.789891× | 0.968812× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.816942× | 0.790494× | 0.967626× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.872229× | 0.843187× | 0.966704× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.874763× | 0.842778× | 0.963435× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.877277× | 0.845220× | 0.963459× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.907946× | 0.874665× | 0.963344× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.924373× | 0.894102× | 0.967252× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.934781× | 0.903386× | 0.966415× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.274823× | 0.273488× | 0.995141× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.449747× | 0.438811× | 0.975685× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.548256× | 0.534061× | 0.974110× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.672450× | 0.653193× | 0.971363× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.769432× | 0.745561× | 0.968976× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.775897× | 0.748539× | 0.964741× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.794421× | 0.767805× | 0.966495× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.812360× | 0.788620× | 0.970777× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.859508× | 0.833211× | 0.969405× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.862785× | 0.834447× | 0.967155× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.861857× | 0.833473× | 0.967066× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.899657× | 0.872060× | 0.969326× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.914160× | 0.887081× | 0.970378× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.917723× | 0.889339× | 0.969072× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r283-vector-predicate-reuse-4ecc4bfbd4bf.tar.gz). SHA-256 4ecc4bfbd4bf4d719d60587523877a2ada0007f22ebd11d5c36ef9528470100a.
