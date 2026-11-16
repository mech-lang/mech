# R278 — Specialize the checked append run

**Retain R278 with the R272 runtime.** The R278 candidate changes paired elapsed by -4.382% across the 81 level-12 cases. Timing gates pass 81/81 and 81/81. In the primary cohorts the control beats Roc on 38/81 cases and the candidate beats it on 45/81 (55.56%). The candidate beats libdeflate on 3/81 in the separate four-actor cohort.

The two primary cohorts change elapsed by -4.510% and -4.254%. The largest paired slowdown is +2.002% on prefix-natural-131072.

Cases crossing below 1× Roc include prefix-random-8192, prefix-periodic-131072, prefix-periodic-262144, prefix-periodic-1048576, prefix-repeated-131072, prefix-repeated-262144, prefix-repeated-1048576. Cases crossing above include none.

Pooled gate misses include none. Block gate misses include none.

Measured compiler parent 5bf88bd67a20eb9b3bf98d05e38892e8758cf1a0. Published parent 3dd44874b29e65fede81ea45a498830a63cdaff8. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the verified R276 closure and the five-file experimental overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14.

## Recurrence and proof

The compact phase already admits a unique current writable owner and enough existing capacity for one append per countdown visit. Within that phase, the emitter walks the lowered SSA from the source loop header along a guarded, effect-free prefix to a single U64 store and its latch. It recognizes a modular difference equal to its starting cursor as a zero-width case. The proved address of the last-word read is the preceding append location, the backing base is invariant, and the next length is the old length plus one. The emitted loop retains countdown, cache cursor, output length and preceding stored word. Duplicate geometry lanes use the same next word.

An existing append seeds this loop with its actual stored word and next state. An unsupported guard resumes the original visit before the next store. The cache and literal bounds tests are evaluated on current views. On a nonempty header the original reduction computes the choice, including its original lexical assertion. The ordinary call path still checks alias ownership, capacity, geometry and publication. Finite managed work and recovery are outside the diagnostic closed timing entry and retain their existing path.

The first matcher could admit a loop in which the forwarded read also produced the selector. The resulting clone could omit the selector guard. A direct lowering control reproduced that admission, and the qualified matcher rejects it while admitting a distinct selector. Both complete guarded compiler artifacts match the earlier screened Object and metadata byte for byte.

## Linked machine code

Exactly four of 347 machine bodies change. Their native frames stay at 256 bytes and code grows from 1,488 to 1,744 bytes each. The root and its frame match R276. The archived libdeflate sequence performs a comparable continuing empty visit in 16 instructions with no stack transfers. [Peer binary comparison](native-libdeflate-comparison.md).

| Static linked span | R276 instructions / stack accesses | R278 instructions / stack accesses |
|---|---:|---:|
| Continued readable empty visit | 50 / 2 | 34 / 3 |
| Ordinary header through first candidate word | 41 / 2 | 44 / 4 |
| Matched append through next ordinary header | 7 / 0 | 11 / 0 |

A nonempty header reached from the short loop executes 15 instructions including 1 stack reload before the original visit. The cache base gains a stack home at offset 0x98, adding a reload at the ordinary header and another at the first match-entry load. The selected loop also reloads the literal table base and length. The normal matched completion checks the selector and moves additional state. These are directly observed routes in the linked image and plausible costs for natural text. The timing and CPU samples do not assign a causal fraction to each instruction.

The 23-case screen changes elapsed by -5.632% and passes both gates on 23/23 and 23/23 cases. The separately balanced seven-case library screen also remains in the evidence.

## Matched frontier

Ratios to each peer use the same fresh four-actor comparison. Elapsed change uses the two primary cohorts. Each times fixed level-12 runtime-input compression with the original scratch and output capacities and the same complete-call boundaries. Validation is outside the timer.

| Input | R276 / Roc | R278 / Roc | R276 / libdeflate | R278 / libdeflate | Elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.198646× | 0.198668× | 2.523829× | 2.524102× | -0.745% |
| prefix-random-8192 | 1.040506× | 0.971396× | 3.726015× | 3.478534× | -7.790% |
| prefix-periodic-131072 | 1.032247× | 0.947082× | 1.878022× | 1.723077× | -8.040% |
| prefix-periodic-262144 | 1.051552× | 0.966615× | 1.854285× | 1.704509× | -8.134% |
| prefix-periodic-1048576 | 1.062911× | 0.976064× | 1.798539× | 1.651587× | -8.304% |
| prefix-repeated-131072 | 1.012677× | 0.938080× | 1.700352× | 1.575098× | -7.627% |
| prefix-repeated-262144 | 1.034925× | 0.953692× | 1.682339× | 1.550290× | -7.714% |
| prefix-repeated-1048576 | 1.034364× | 0.954039× | 1.625450× | 1.499224× | -7.866% |
| prefix-natural-4096 | 1.083503× | 1.099890× | 1.865332× | 1.893543× | +0.812% |
| prefix-natural-32768 | 1.475150× | 1.506867× | 1.343178× | 1.372057× | +1.767% |
| prefix-natural-1048576 | 1.501933× | 1.527530× | 1.210507× | 1.231137× | +1.619% |
| prefix-random-1048576 | 2.124785× | 1.983389× | 3.000450× | 2.800781× | -6.818% |

## Correctness and independent counters

- 324 original and decoded complete calls produce exact bytes and independently inflate. Both forms preserve the source allocation capacities and live aliases.
- 140 linked program controls cover finite growth, failed admission, aliases, original assertion origins, previous publication and successful retry. The small observer fixtures route through the existing capacity gate. The new matcher declines their different branch shape and leaves the native body in place.
- 21 direct calls enter the exact production no-growth function from the baseline, qualified original and qualified decoded images. Symbol export for the test preserves the verified machine instruction words. The controls exercise consecutive empties, a following match, a following failing assertion, read defaults and countdown exit. An independent oracle and the baseline match both candidate forms. The surrounding linked controls verify publication and cleanup.
- The four-actor cohort includes 245,664 timed calls. All measured logical payload requests, requested bytes, peak private payload, copied and zeroed bytes and residual ownership agree in the separate memory cohort.

| Input | R276 instructions | R278 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,601.0 | 812,170.5 | -0.053% | -2.636% |
| prefix-random-8192 | 7,432,895.0 | 7,037,686.0 | -5.317% | -8.582% |
| prefix-periodic-131072 | 88,300,768.5 | 84,166,890.0 | -4.682% | -7.719% |
| prefix-periodic-262144 | 175,825,495.5 | 167,554,232.0 | -4.704% | -8.108% |
| prefix-periodic-1048576 | 706,194,799.0 | 673,121,469.0 | -4.683% | -8.176% |
| prefix-repeated-131072 | 89,523,593.5 | 85,389,739.5 | -4.618% | -7.405% |
| prefix-repeated-262144 | 178,758,889.5 | 170,486,781.0 | -4.628% | -7.733% |
| prefix-repeated-1048576 | 713,752,551.5 | 680,673,325.5 | -4.635% | -7.888% |
| prefix-natural-4096 | 11,131,999.0 | 11,287,637.5 | +1.398% | +1.211% |
| prefix-natural-32768 | 94,512,479.0 | 96,400,045.5 | +1.997% | +2.414% |
| prefix-natural-1048576 | 3,447,129,361.0 | 3,515,969,018.0 | +1.997% | +2.675% |
| prefix-random-1048576 | 874,104,144.0 | 826,378,236.0 | -5.460% | -6.836% |

The CPU-time profile samples the exact linked candidate. Shares include the harness and validation. The short-loop region includes its exits. The sampling counts describe machine locations.

| Input | Running samples | Path body | Short-loop region | Matcher body |
|---|---:|---:|---:|---:|
| eight-literals | 5345 | 4 | 2 | 647 |
| prefix-natural-1048576 | 5358 | 2599 | 10 | 2385 |
| prefix-natural-32768 | 5360 | 2578 | 33 | 2169 |
| prefix-natural-4096 | 5565 | 1513 | 70 | 1776 |
| prefix-periodic-1048576 | 5363 | 834 | 691 | 3736 |
| prefix-random-1048576 | 5360 | 555 | 551 | 3459 |
| prefix-repeated-1048576 | 5351 | 791 | 676 | 3445 |

The next experiment will reduce the current descriptor reloads and the cost of returning to the original match path. The remaining natural-text regression is measured explicitly. The profile and static peer comparison define the candidate scope.

## All primary timings

| Input | Bytes | R276 / Roc | R278 / Roc | R278 / R276 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.228659× | 2.124867× | 0.953429× | pass/pass |
| one-stored | 1 | 2.010830× | 1.999797× | 0.994513× | pass/pass |
| seven-stored-prefix | 7 | 2.275618× | 2.255088× | 0.990979× | pass/pass |
| eight-literals | 8 | 0.199840× | 0.198351× | 0.992551× | pass/pass |
| eight-overlap | 8 | 0.191296× | 0.190042× | 0.993443× | pass/pass |
| 320-overlap | 320 | 0.227474× | 0.221169× | 0.972281× | pass/pass |
| eight-literals-prefix | 8 | 0.198901× | 0.198302× | 0.996989× | pass/pass |
| 128-biased-dynamic | 128 | 0.265848× | 0.264048× | 0.993228× | pass/pass |
| 32-high-entropy-stored | 32 | 0.274171× | 0.270879× | 0.987994× | pass/pass |
| 16384-distribution-change | 16384 | 1.660874× | 1.629742× | 0.981255× | pass/pass |
| 33027-periodic-window | 33027 | 0.915112× | 0.845448× | 0.923873× | pass/pass |
| random-64 | 64 | 0.283700× | 0.280314× | 0.988066× | pass/pass |
| random-256 | 256 | 0.349057× | 0.342449× | 0.981068× | pass/pass |
| random-1024 | 1024 | 0.435912× | 0.420820× | 0.965378× | pass/pass |
| repeated-1024 | 1024 | 0.299893× | 0.281200× | 0.937667× | pass/pass |
| random-32769 | 32769 | 1.737341× | 1.602682× | 0.922492× | pass/pass |
| repeated-32769 | 32769 | 0.890209× | 0.821327× | 0.922623× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.540893× | 0.538416× | 0.995420× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.428100× | 1.452412× | 1.017024× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.472790× | 1.498188× | 1.017245× | pass/pass |
| heldout-window-32767 | 32767 | 1.475125× | 1.501272× | 1.017725× | pass/pass |
| heldout-window-32768 | 32768 | 1.475819× | 1.502885× | 1.018339× | pass/pass |
| heldout-slides-65537 | 65537 | 1.513922× | 1.539048× | 1.016597× | pass/pass |
| heldout-random-65537 | 65537 | 1.927019× | 1.777878× | 0.922605× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.641636× | 1.556770× | 0.948304× | pass/pass |
| prefix-natural-1024 | 1024 | 0.542488× | 0.540415× | 0.996178× | pass/pass |
| prefix-natural-4096 | 4096 | 1.106935× | 1.115921× | 1.008118× | pass/pass |
| prefix-natural-8192 | 8192 | 1.309234× | 1.334146× | 1.019028× | pass/pass |
| prefix-natural-16384 | 16384 | 1.426167× | 1.450475× | 1.017045× | pass/pass |
| prefix-natural-32767 | 32767 | 1.473885× | 1.499985× | 1.017709× | pass/pass |
| prefix-natural-32768 | 32768 | 1.472063× | 1.498077× | 1.017672× | pass/pass |
| prefix-natural-32769 | 32769 | 1.474028× | 1.499486× | 1.017271× | pass/pass |
| prefix-natural-33027 | 33027 | 1.474680× | 1.501321× | 1.018066× | pass/pass |
| prefix-natural-65535 | 65535 | 1.479877× | 1.504671× | 1.016754× | pass/pass |
| prefix-natural-65536 | 65536 | 1.476717× | 1.503599× | 1.018204× | pass/pass |
| prefix-natural-65537 | 65537 | 1.470688× | 1.498540× | 1.018938× | pass/pass |
| prefix-natural-131072 | 131072 | 1.478592× | 1.508199× | 1.020024× | pass/pass |
| prefix-natural-262144 | 262144 | 1.477084× | 1.506154× | 1.019681× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.502347× | 1.526671× | 1.016190× | pass/pass |
| prefix-random-1024 | 1024 | 0.442187× | 0.427013× | 0.965685× | pass/pass |
| prefix-random-4096 | 4096 | 0.791442× | 0.740440× | 0.935559× | pass/pass |
| prefix-random-8192 | 8192 | 1.045636× | 0.964176× | 0.922096× | pass/pass |
| prefix-random-16384 | 16384 | 1.450357× | 1.334732× | 0.920278× | pass/pass |
| prefix-random-32767 | 32767 | 1.754403× | 1.621148× | 0.924045× | pass/pass |
| prefix-random-32768 | 32768 | 1.768308× | 1.625701× | 0.919354× | pass/pass |
| prefix-random-32769 | 32769 | 1.743155× | 1.613550× | 0.925649× | pass/pass |
| prefix-random-33027 | 33027 | 1.741515× | 1.609466× | 0.924176× | pass/pass |
| prefix-random-65535 | 65535 | 1.947976× | 1.807134× | 0.927698× | pass/pass |
| prefix-random-65536 | 65536 | 1.943942× | 1.803834× | 0.927926× | pass/pass |
| prefix-random-65537 | 65537 | 1.939510× | 1.797067× | 0.926558× | pass/pass |
| prefix-random-131072 | 131072 | 2.052112× | 1.904836× | 0.928232× | pass/pass |
| prefix-random-262144 | 262144 | 2.096220× | 1.950907× | 0.930679× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.122997× | 1.978261× | 0.931825× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.361362× | 0.344438× | 0.953165× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.546519× | 0.508155× | 0.929803× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.671631× | 0.609315× | 0.907217× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.785537× | 0.728443× | 0.927318× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.897099× | 0.828943× | 0.924026× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.904721× | 0.832744× | 0.920443× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.909496× | 0.837297× | 0.920616× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.914234× | 0.840424× | 0.919266× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.983958× | 0.906406× | 0.921183× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.987985× | 0.908596× | 0.919645× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.986940× | 0.908324× | 0.920344× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.026781× | 0.944224× | 0.919596× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.050517× | 0.965063× | 0.918655× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.063008× | 0.974739× | 0.916962× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.298581× | 0.280859× | 0.940644× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.498741× | 0.456230× | 0.914764× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.629331× | 0.572061× | 0.909000× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.748757× | 0.693738× | 0.926520× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.863273× | 0.798102× | 0.924507× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.864253× | 0.796153× | 0.921204× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.886825× | 0.818790× | 0.923282× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.903761× | 0.835108× | 0.924037× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.965269× | 0.890848× | 0.922901× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.966657× | 0.890565× | 0.921283× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.962653× | 0.888201× | 0.922660× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.010884× | 0.933784× | 0.923731× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.030722× | 0.951214× | 0.922862× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.035608× | 0.954143× | 0.921336× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r278-checked-append-run-7d6f1e528b44.tar.gz). SHA-256 7d6f1e528b443c0e994ee9c24bb9b08b4bef144116b1bf0d7ac04bd6805d83d7.
