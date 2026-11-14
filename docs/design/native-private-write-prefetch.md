# R282 — Prefetch a proved future private write

**Retain R280.** The 27-case screen changes paired elapsed +0.208% overall. The pooled gate passes 27/27 and the block gate passes 26/27. The block miss is seven-stored-prefix. The candidate and retained compiler each beat Roc on 18/27 screened cases. The retained full-suite score remains 45/81 below Roc and 3/81 below libdeflate.

1 MiB random improves -3.474%. Its hint-enabled executable is 0.961116× the elapsed time of the fixed-placement no-hint copy. Several repetitive cases slow about 1%. The matching control keeps the changed address calculations, register assignments and layout, so the comparison isolates the hint instructions with those other changes held fixed.

Published parent 1f388ba572d4d0e767e50e54aebd92090468ac16. Measured parent 74205739c757a23c01994b902649a765589cf364. PR base 92f75db84e2523693648084bea16cd52ad60777a. Pinned Mech source, MBC14, level 12, capacities, tables, limits, allocation sites, R272 runtime, exact streams and timing boundaries stay fixed. Apple ARM64, Cranelift 0.131.3 with the inherited R264 patch and five prefetch backend files, regalloc2 0.15.2.

## Proof and machine code

The compiler collects exact typed writes from a compact private phase with an existing current flat-view length floor. It follows a masked affine one-based selector to its loop parameter and walks the incoming SSA arguments dominated by that header. A fresh bounded definition receives a hint only when the same fixed base dominates the loop and the entire byte address range fits the current initialized view. A fallible operation, changed base or unbounded update supplies no such certificate. The emitted hint is an ARM64 cache operation. Existing loads, stores, checks, short-window defaults and failure branches keep their source meaning.

Five static hints appear in one vector matcher among 347 compared bodies. The matcher changes 42,052 to 42,020 code bytes and 992 to 1,024 frame bytes. The root, general matrix body, parser and remaining generated bodies are byte-identical. The first search copy has a hint for one of its bucket indices. The second reaches its write through additional forwarding merges and the first rule declines it. Two subsequent copies get both hints.

The initial build required the additional vector contract to list the destination. The length floor already checks its flat geometry, and the caller excludes such arrays from the separate list. That build selected zero hints and reproduced R280 Object and metadata exactly. The measured build consumes the existing guarded length floor and canonical vector address instead.

## Fresh matched peer timings

| Input | R280 / Roc | Prefetch / Roc | R280 / libdeflate | Prefetch / libdeflate | Screen elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.199537× | 0.199446× | 2.540260× | 2.539101× | -0.415% |
| prefix-natural-4096 | 1.101704× | 1.093583× | 1.881540× | 1.867669× | -0.624% |
| prefix-natural-1048576 | 1.523482× | 1.528735× | 1.229464× | 1.233703× | +0.358% |
| prefix-random-8192 | 0.940963× | 0.922097× | 3.357569× | 3.290249× | -3.090% |
| prefix-random-32768 | 1.564855× | 1.520039× | 3.191870× | 3.100458× | -2.183% |
| prefix-random-1048576 | 1.919873× | 1.858858× | 2.709522× | 2.623412× | -3.474% |
| prefix-periodic-1048576 | 0.934363× | 0.944784× | 1.584655× | 1.602329× | +1.153% |
| prefix-repeated-1048576 | 0.919031× | 0.927715× | 1.442503× | 1.456134× | +1.076% |

The 27-case screen includes 151,968 timed calls across all 24 orders of the retained, hint-enabled, fixed-placement control and Roc actors. The separate eight-case library cohort includes 6,816 timed calls with Mech control, Mech candidate, Roc and libdeflate. Timers include complete compression and the established allocation and cleanup boundaries. Both comparisons use the existing warmup, live output aliases and validation outside timing. Full 81-case adoption timing was skipped after the screen.

## Independent hardware counters and profile

| Input | R280 instructions | Prefetch instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,121.0 | 811,934.5 | -0.023% | +4.474% |
| prefix-natural-1048576 | 3,489,528,832.5 | 3,486,351,742.5 | -0.091% | +0.266% |
| prefix-natural-4096 | 11,238,360.0 | 11,199,882.0 | -0.342% | -0.435% |
| prefix-periodic-1048576 | 664,964,363.5 | 673,227,573.5 | +1.243% | +1.016% |
| prefix-random-1048576 | 814,560,496.5 | 808,189,175.5 | -0.782% | -3.466% |
| prefix-random-32768 | 26,142,985.0 | 25,959,444.5 | -0.702% | -2.454% |
| prefix-random-8192 | 6,939,558.0 | 6,898,357.0 | -0.594% | -1.025% |
| prefix-repeated-1048576 | 672,525,547.5 | 680,760,544.5 | +1.224% | +1.032% |

The tracked payload allocation, copy, zero and peak fields agree on 8/8 diagnostic cases. Process lifetime peak physical memory is reported separately in the counter receipt.

| Input | Running samples | Vector matcher PCs | Matcher stack-transfer PCs | Prefetch instruction PCs |
|---|---:|---:|---:|---:|
| eight-literals | 5342 | 619 | 25 | 0 |
| prefix-natural-1048576 | 5360 | 2394 | 1526 | 0 |
| prefix-natural-32768 | 5360 | 2174 | 1318 | 0 |
| prefix-natural-4096 | 5609 | 1785 | 960 | 0 |
| prefix-periodic-1048576 | 5361 | 3898 | 2056 | 32 |
| prefix-random-1048576 | 5360 | 3508 | 2295 | 0 |
| prefix-repeated-1048576 | 5365 | 3624 | 1921 | 34 |

These are sampled instruction locations in the exact linked candidate. They include harness and validation work and cannot allocate retired instructions or latency to individual operations.

## Qualification and next experiment

- 324 linked complete calls cover all 81 original and decoded streams, independent inflate, capacities, aliases and zero final private ownership. The Object and metadata match byte-for-byte.
- 120 paired ARM64 JIT calls check live writes, loop values, zero visits, short-length rejection and identical resulting buffers. Four additional controls cover the first eligible slot, a smaller length floor, a loop-local base and an unsupported update.
- Warnings-as-errors and the baseline-reproduction build passed. Initial compile and test setup errors are retained with the corrected receipts. Hosted CI is outside the measurement gate.

The current checked vector body loads the same input length for a successful four-byte guard and its checked subtraction in the selected arm. The existing selected-edge predicate pass runs in the root only. The next standalone compiler test will apply that proof to the private vector phase and inspect the exact branch and recurring state before timing.

## All screen observations

| Input | R280 / Roc | Prefetch / Roc | Prefetch / R280 | Prefetch / no hint | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 1.976190× | 1.976190× | 1.000000× | 1.000000× | pass/pass |
| one-stored | 1.976190× | 1.976190× | 1.000000× | 1.000000× | pass/pass |
| seven-stored-prefix | 1.976190× | 1.976190× | 1.000000× | 1.000000× | pass/miss |
| eight-literals | 0.197174× | 0.196702× | 0.995845× | 0.993870× | pass/pass |
| 33027-periodic-window | 0.816376× | 0.821438× | 1.004135× | 0.993358× | pass/pass |
| prefix-natural-4096 | 1.114551× | 1.101468× | 0.993762× | 0.992027× | pass/pass |
| prefix-natural-8192 | 1.313982× | 1.314199× | 0.998924× | 0.995023× | pass/pass |
| prefix-natural-32768 | 1.493883× | 1.493150× | 1.000977× | 0.996000× | pass/pass |
| prefix-natural-1048576 | 1.524348× | 1.529114× | 1.003577× | 0.999996× | pass/pass |
| prefix-random-8192 | 0.950766× | 0.932320× | 0.969097× | 0.980584× | pass/pass |
| prefix-random-32768 | 1.573555× | 1.540289× | 0.978171× | 0.976639× | pass/pass |
| prefix-random-1048576 | 1.921459× | 1.853901× | 0.965264× | 0.961116× | pass/pass |
| prefix-periodic-32767 | 0.795613× | 0.808797× | 1.013059× | 0.998591× | pass/pass |
| prefix-periodic-32768 | 0.800779× | 0.809785× | 1.010351× | 1.000802× | pass/pass |
| prefix-periodic-32769 | 0.808847× | 0.816994× | 1.010428× | 0.998291× | pass/pass |
| prefix-periodic-33027 | 0.819802× | 0.825235× | 1.008280× | 0.999576× | pass/pass |
| prefix-periodic-65536 | 0.875938× | 0.882692× | 1.008930× | 0.998974× | pass/pass |
| prefix-periodic-131072 | 0.906548× | 0.915776× | 1.010309× | 0.996282× | pass/pass |
| prefix-periodic-1048576 | 0.933123× | 0.944758× | 1.011530× | 0.999921× | pass/pass |
| prefix-repeated-4096 | 0.444885× | 0.448686× | 1.012119× | 0.999470× | pass/pass |
| prefix-repeated-32767 | 0.771255× | 0.779952× | 1.009575× | 0.997543× | pass/pass |
| prefix-repeated-32768 | 0.769898× | 0.777925× | 1.007704× | 1.001417× | pass/pass |
| prefix-repeated-32769 | 0.809557× | 0.822011× | 1.006728× | 1.000035× | pass/pass |
| prefix-repeated-33027 | 0.812329× | 0.822102× | 1.011607× | 0.995113× | pass/pass |
| prefix-repeated-65536 | 0.860944× | 0.869710× | 1.007759× | 0.999238× | pass/pass |
| prefix-repeated-262144 | 0.915751× | 0.923581× | 1.009326× | 0.998407× | pass/pass |
| prefix-repeated-1048576 | 0.916918× | 0.925782× | 1.010762× | 1.000261× | pass/pass |

[Verified compiler patches, control and measurements](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r282-private-write-prefetch-86c770541636.tar.gz). SHA-256 86c770541636c52732288e2eb079a25b1a8f019a109c22ba5fdd6ba0dadb0e7e.
