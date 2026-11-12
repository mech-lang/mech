# R339 — Inline the second scalar candidate

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +1.489% in the matched comparison and +1.641% in the primary screen. Reymont changes +1.616% and +1.201% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent e1f1e04d129cd941ace82305faa964d64e81c0ba. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Complete a second admitted visit with the current minimum

The compiler keeps the first visit and its singleton exit in their original control-flow order. A short interval with a second visit evaluates that cost and compares it with the first visit’s result, so the old incoming minimum and item can stop occupying registers. Each strict comparison supplies both selects. A tie keeps the incumbent or earliest candidate. Longer intervals continue at the third index with the current reverse cursor. The complete-child proof, source work, long vector route and ordinary fallback are unchanged.

| Linked route | R333 instructions | R338 instructions | R339 instructions |
|---|---:|---:|---:|
| Valid record and cost setup | 28 | 32 | 28 |
| Singleton through next record | 52 | 58 | 52 |
| Two candidates through next record | 70 | 70 | 70 |
| Continuing scalar visit | 16 | 16 | 16 |

The R338 allocator reloaded the current cache length twice and its base once. It also saved and restored a register value around the record. R339 restores the original register residency on this route. Its second visit uses one fewer move and an extra reverse-cursor subtraction. The retained loop updates the cursor in the load. The retained reference scalar loop and the Rust control share the same condition for cost and item selection.

The composed phase changes from 1,520 to 1,584 code bytes and from 2,272 to 2,272 stack bytes. Eight of 351 bodies change. The root, matcher and 341 other bodies agree. The original and decoded images match. The flag-disabled Object and metadata reproduce R333.

## Matched measurements

| Input | R333 / Roc | R339 / Roc | R333 / libdeflate | R339 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.159023× | 2.067008× | 0.017069× | 0.016341× | -4.262% | +0.000% |
| one-stored | 2.022348× | 1.926194× | 0.018386× | 0.017512× | -4.755% | +0.000% |
| seven-stored-prefix | 2.297714× | 2.224424× | 0.018935× | 0.018331× | -3.190% | +0.000% |
| eight-literals | 0.181344× | 0.181662× | 2.308037× | 2.312083× | +0.175% | +0.316% |
| reymont-prefix-32769 | 1.257684× | 1.278010× | 1.145709× | 1.164226× | +1.616% | +1.201% |
| heldout-window-32768 | 1.259099× | 1.278615× | 1.148084× | 1.165879× | +1.550% | +1.306% |
| prefix-natural-32767 | 1.259338× | 1.278213× | 1.148574× | 1.165789× | +1.499% | +1.397% |
| prefix-natural-32768 | 1.258786× | 1.279266× | 1.149733× | 1.168439× | +1.627% | +1.254% |
| prefix-natural-65537 | 1.264023× | 1.283282× | 1.064636× | 1.080856× | +1.524% | +1.637% |
| prefix-natural-131072 | 1.267568× | 1.289471× | 1.043300× | 1.061327× | +1.728% | +1.643% |
| prefix-natural-262144 | 1.277179× | 1.296202× | 1.030525× | 1.045873× | +1.489% | +1.641% |
| prefix-natural-1048576 | 1.283568× | 1.305126× | 1.036687× | 1.054099× | +1.680% | +1.725% |
| prefix-random-16384 | 1.065655× | 1.069453× | 2.909807× | 2.920178× | +0.356% | +0.586% |
| prefix-random-1048576 | 1.588658× | 1.595704× | 2.244497× | 2.254451× | +0.443% | +0.026% |
| prefix-periodic-1048576 | 0.707447× | 0.707058× | 1.203375× | 1.202714× | -0.055% | -0.025% |
| prefix-repeated-1048576 | 0.709950× | 0.710728× | 1.115321× | 1.116544× | +0.110% | +0.062% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.796%. The highest matched change is +1.728% on prefix-natural-131072.

| Input | R333 instructions | R339 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 725,612.5 | 725,673.0 | +0.008% | +3.245% |
| reymont-prefix-32769 | 70,047,795.5 | 69,779,177.5 | -0.383% | +1.133% |
| prefix-natural-262144 | 693,665,519.5 | 690,448,463.5 | -0.464% | +1.130% |
| prefix-natural-1048576 | 2,488,132,048.0 | 2,477,465,517.5 | -0.429% | +1.124% |
| prefix-random-16384 | 10,197,632.5 | 10,197,598.0 | -0.000% | -0.220% |
| prefix-random-1048576 | 576,161,619.5 | 576,163,798.5 | +0.000% | -0.364% |
| prefix-periodic-1048576 | 488,914,267.0 | 488,912,309.5 | -0.000% | -0.009% |
| prefix-repeated-1048576 | 496,686,034.0 | 496,683,691.0 | -0.000% | +0.030% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,269 | 0 |
| prefix-natural-262144 | 5,348 | 3,090 |
| reymont-prefix-32769 | 4,901 | 2,534 |

## Qualification

Warnings-as-errors builds pass. The linked oracle checks 136,192 candidate positions across both image forms and 68,096 retained positions. It covers zero, one, two and longer intervals, nonzero starts, earliest ties, wrapping costs, vector tails, source rejection origin and retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r339-inline-second-reduction-89a71157a99d.tar.gz). SHA-256 89a71157a99dd7cb52661f978cd1dc56cf78277ab2828fc7cce0e4db134cfb1e.
