# R349 — Reuse current array length floors

**Retain R349 with the R272 runtime.** The 81-case primary geometric mean changes -1.931%. The fixed 17-case priority group changes -0.521% in the four-actor comparison.

In the matched run, the candidate wins 46/81 against Roc (56.79%), 4/81 against libdeflate and 0/81 against both.

## Full qualification

The two Mech executables and their complete-call boundaries match the prior campaign. Their launches use one byte-identical retained metadata file after verification of the complete input and output layouts.

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 81/81. All six heap distributions match on 81/81 cases. The highest primary movement is +0.758% on eight-literals-prefix. The highest matched movement is +0.286% on prefix-repeated-4096.

| Input | R343 / Roc | Candidate / Roc | R343 / libdeflate | Candidate / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.223099× | 1.218744× | 1.111344× | 1.107388× | -0.561% | -0.316% | -0.356% |
| prefix-natural-262144 | 1.234200× | 1.233386× | 0.998245× | 0.997587× | -0.383% | -0.201% | -0.066% |
| prefix-natural-131072 | 1.234496× | 1.226914× | 1.016277× | 1.010035× | -0.461% | -0.077% | -0.614% |
| prefix-natural-1048576 | 1.250452× | 1.247016× | 1.007804× | 1.005035× | -0.355% | -0.320% | -0.275% |
| eight-literals | 0.185115× | 0.184827× | 2.327744× | 2.324117× | +0.366% | +0.109% | -0.156% |
| prefix-random-16384 | 1.060554× | 1.016502× | 2.866315× | 2.747258× | -5.320% | -3.668% | -4.154% |
| prefix-natural-65537 | 1.226916× | 1.222624× | 1.029471× | 1.025870× | -0.332% | -0.203% | -0.350% |
| heldout-window-32768 | 1.225733× | 1.220000× | 1.115587× | 1.110369× | -0.485% | -0.269% | -0.468% |
| eight-literals-prefix | 0.185811× | 0.185534× | 2.311712× | 2.308275× | +0.328% | +1.190% | -0.149% |
| prefix-repeated-4096 | 0.398699× | 0.399841× | 1.480015× | 1.484255× | -3.128% | -2.375% | +0.286% |

Both builds win the same cases in this matched cohort. R343 also comes in below libdeflate on natural 256 KiB, at 0.998245×.

## Empty input with shared metadata

| Empty call cohort | R343 block median | R349 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 70.926 ns | 69.507 ns | -2.001% |
| primary-b | 71.493 ns | 70.080 ns | -1.977% |
| combined | 71.192 ns | 69.748 ns | -2.029% |

Both executables use the shared R344 startup descriptor. The root Object changes code and frame size with the new guards. The numeric dispatch decisions agree.


## Independent counter cohort

| Input | R343 instructions | R349 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 67,181,929 | 66,843,982 | -0.503% | -0.580% |
| prefix-natural-262144 | 660,931,226 | 658,284,756 | -0.400% | -0.176% |
| prefix-natural-1048576 | 2,381,673,423 | 2,371,717,666 | -0.418% | -0.273% |
| eight-literals | 715,935 | 716,380 | +0.062% | +1.455% |
| prefix-random-1048576 | 576,158,958 | 557,568,547 | -3.227% | -3.136% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5354 | 2274 | 2746 | 2746 | 334 |
| prefix-natural-262144 | 5589 | 2099 | 3145 | 3145 | 345 |
| prefix-random-1048576 | 5357 | 3384 | 467 | 0 | 1506 |
| prefix-random-16384 | 5138 | 2324 | 521 | 0 | 2293 |

## Screen, machine code, and correctness evidence

**The screen advanced to the full matrix.** Natural 256 KiB changes -0.357% in the matched comparison and -0.265% in the primary screen. Reymont changes -0.306% and -0.342% in those comparisons.

Measured parent a1916515a8e8adbf8497cb0f04861b77dbed1e1f. Published parent a1916515a8e8adbf8497cb0f04861b77dbed1e1f. PR base 92f75db84e2523693648084bea16cd52ad60777a. The two workers share the exact benchmark descriptor used in R344. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R343 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, tables, capacities and complete-call boundary are pinned.

## Check current lengths once before a fixed phase

The compiler follows a uniform constructor through complete view copies and shape-preserving indexed writes. Unknown or conflicting origins decline. It treats those lengths as candidates and checks the required floors on the actual current writable views before calling the phase. The original root path runs when the current view is too short, shared, or otherwise ineligible.

After packet phis are resolved, bounded SSA arithmetic joins every incoming arm and proves the maximum index, including the zero default and the one-based index adjustment. Only comparisons against the same verified immutable packet lane are folded. The phase returns the actual view geometry. The entry checks only the minimum required by a folded use. Unknown arithmetic, wrap, a changed view and narrower offers keep the original checks.

| Linked route | R343 instructions | R349 instructions | R343 branches | R349 branches |
|---|---:|---:|---:|---:|
| Literal tally backedge | 25 | 22 | 5 | 4 |
| Matched-length tally backedge | 47 | 43 | 8 | 6 |

The linked tally keeps the source length and offset table defaults. The linked reference literal paths use 16 instructions in Roc and 14 in libdeflate. The Rust literal-only control checks its histogram at entry and uses UBFX for the eight-bit field.

The tally body changes from 640 to 552 code bytes and keeps its 192-byte frame. Seven phases use the proof in both variants. The 4,600 generated calls and all 170 vector decisions agree. Fourteen phase bodies and the root change out of 351. The new root has 20 more CLIF instructions and the same block count. Its linked code shrinks from 1,000,052 to 981,288 bytes and its frame from 57,472 to 57,440 bytes. Root code placement and allocation may contribute to whole-call changes. Original and decoded Objects and metadata agree. The disabled trial reproduces the retained Object and metadata byte for byte.

## Matched measurements

| Input | R343 / Roc | R349 / Roc | R343 / libdeflate | R349 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.144833× | 2.141391× | 0.016784× | 0.016757× | -0.160% | +0.000% |
| one-stored | 1.995125× | 2.010278× | 0.017938× | 0.018074× | +0.759% | +0.000% |
| seven-stored-prefix | 2.274911× | 2.266748× | 0.018840× | 0.018772× | -0.359% | +0.000% |
| eight-literals | 0.182899× | 0.183723× | 2.327923× | 2.338403× | +0.450% | +0.002% |
| reymont-prefix-32769 | 1.224788× | 1.221043× | 1.113143× | 1.109740× | -0.306% | -0.342% |
| heldout-window-32768 | 1.226474× | 1.219688× | 1.116332× | 1.110156× | -0.553% | -0.461% |
| prefix-natural-32767 | 1.222407× | 1.217280× | 1.113032× | 1.108364× | -0.419% | -0.263% |
| prefix-natural-32768 | 1.225997× | 1.220967× | 1.116230× | 1.111650× | -0.410% | -0.561% |
| prefix-natural-65537 | 1.223936× | 1.220986× | 1.031551× | 1.029065× | -0.241% | -0.251% |
| prefix-natural-131072 | 1.230490× | 1.230043× | 1.012951× | 1.012584× | -0.036% | -0.515% |
| prefix-natural-262144 | 1.238872× | 1.234450× | 0.999535× | 0.995967× | -0.357% | -0.265% |
| prefix-natural-1048576 | 1.250699× | 1.245627× | 1.009759× | 1.005665× | -0.405% | -0.280% |
| prefix-random-16384 | 1.069727× | 1.028752× | 2.867765× | 2.757919× | -3.830% | -5.334% |
| prefix-random-1048576 | 1.590413× | 1.538745× | 2.248027× | 2.174995× | -3.249% | -3.350% |
| prefix-periodic-1048576 | 0.709765× | 0.688826× | 1.204533× | 1.168997× | -2.950% | -2.860% |
| prefix-repeated-1048576 | 0.713447× | 0.693147× | 1.114976× | 1.083252× | -2.845% | -2.620% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -1.081%. The priority-subset matched mean changes -0.227%. The highest matched change is +0.759% on one-stored.

The matched screen counts 3 to 3 cases faster than Roc, 4 to 4 faster than libdeflate, and 0 to 0 faster than both.

| Input | R343 instructions | R349 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,809.0 | 716,276.0 | +0.065% | -0.857% |
| reymont-prefix-32769 | 67,174,298.0 | 66,841,383.0 | -0.496% | -0.405% |
| prefix-natural-262144 | 661,261,933.5 | 658,582,650.0 | -0.405% | -0.199% |
| prefix-natural-1048576 | 2,381,241,132.5 | 2,371,213,221.5 | -0.421% | -0.307% |
| prefix-random-16384 | 10,198,284.5 | 9,848,101.0 | -3.434% | -4.569% |
| prefix-random-1048576 | 576,158,097.5 | 557,571,784.0 | -3.226% | -3.470% |
| prefix-periodic-1048576 | 488,782,132.0 | 480,618,543.5 | -1.670% | -2.848% |
| prefix-repeated-1048576 | 496,554,005.5 | 488,719,974.0 | -1.578% | -2.552% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running PCs | Tally PCs | Matcher PCs |
|---|---:|---:|---:|
| eight-literals | 5,258 | 0 | 649 |
| prefix-natural-262144 | 5,358 | 104 | 2,010 |
| prefix-random-1048576 | 5,363 | 1,014 | 3,381 |
| prefix-random-16384 | 5,296 | 1,153 | 2,444 |
| reymont-prefix-32769 | 4,414 | 120 | 1,695 |

## Qualification

The compiler, producer, 54 CFG tests and six view tests build with warnings as errors. The focused test executes 30,720 native controls across four offered lengths, all byte values, zero defaults, wrapping indices and an unrestricted operand. The 324 complete-stream checks retain exact bytes, independent inflation, capacities, aliases and cleanup across both artifact forms. The benchmark uses the compact diagnostic entry.

The initial constant-substitution build retained histogram write checks. The first range pass ran before the packet phis resolved and emitted the exact retained Object. The final placement runs after shared cleanup and uses only comparisons with an actual discharged bound. Neither initial build entered timing.

The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r349-current-array-length-floors-d10c1fe04c89.tar.gz). SHA-256 d10c1fe04c899b96ad1e350691b8b5c8e9dceff80d9463bd44a5079c2b184a61.
