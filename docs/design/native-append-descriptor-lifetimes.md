# R279 — Separate the append loop’s descriptor lifetimes

**Retain R279 with the R272 runtime.** R279 changes paired elapsed by -0.509% across 81 fixed level-12 cases. Timing gates pass 81/81 and 81/81. The control beats Roc on 45/81 cases and the candidate beats it on 45/81 (55.56%). The separate library comparison gives the candidate 3/81 wins against libdeflate.

The two primary cohorts change elapsed by -0.501% and -0.517%. The largest paired slowdown is +1.291% on empty.

Cases crossing below 1× Roc include none. Cases crossing above include none.

Pooled gate misses include none. Block gate misses include none.

Measured and published parent 31e48dbb2e745c5d48180aa55530161241cde8cd. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the verified R278 closure and the three-file experimental overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14.

## Descriptor placement and proof

The existing no-growth caller passes current views in a private, initialized input stack packet. Its input and output packets are disjoint. Neither can be a source array backing address. A phase consumes the input synchronously, so a reload of the same word during that phase observes the same descriptor. The compiler selects aligned I64 loads within the known packet extent. It replaces their ordinary-loop uses only when the loop header dominates all uses, and declines values used to compute the existing seed arguments or checks.

The first variant removes nine words used only by the ordinary path from the values live in the short loop. A shared return block restores them to the ordinary loop’s block parameters. The broader variant also loads five shared descriptors at short-loop entry and restores all 14 words at the return. Both keep the actual capacity and owner gate, the original array bounds tests and source error order. No store occurs on the guard edge that returns to the ordinary visit.

## Linked machine code

The root and 342 other bodies match R278. Exactly four no-growth bodies change. The frame grows from 256 to 272 bytes for both variants. Each body changes from 1,744 bytes to 1,728 in the first variant and 1,760 in the broader variant. The short loop in the broader variant has no stack accesses. The matched path still reloads the cache pointer, and the source selector gains a spill across the reduction.

| Static linked span | R278 instructions / stack accesses | Ordinary-only split | Both paths split |
|---|---:|---:|---:|
| Continued readable empty visit | 34 / 3 | 34 / 3 | 31 / 0 |
| Ordinary header to first candidate | 44 / 4 | 44 / 5 | 44 / 5 |
| Matched append to next ordinary header | 11 / 0 | 10 / 1 | 10 / 1 |
| Nonempty header returns from short loop | 15 / 1 | 27 / 5 | 34 / 7 |

The libdeflate peer visit uses 16 instructions and no stack transfers. The register assignment removes three loads from every continuing empty visit and adds work when a run returns to the ordinary path. The longer return path is paid once per exit. [Peer binary comparison](native-libdeflate-comparison.md).

| Screen | Geometric elapsed change | Natural 4 KiB | Natural 1 MiB | Pooled and block gates |
|---|---:|---:|---:|---:|
| split split | -0.396% | -1.030% | -0.562% | 27/27 / 27/27 |
| descriptor split | -0.295% | -0.867% | -0.727% | 27/27 / 27/27 |
| descriptor split-all | -0.452% | -1.105% | -0.450% | 27/27 / 27/27 |

## Matched frontier

Both peer ratios use the same fresh four-actor comparison. Elapsed change comes from the two primary cohorts. The source, runtime, inputs, allocator, output ownership and full compression timing boundary are pinned.

| Input | R278 / Roc | R279 / Roc | R278 / libdeflate | R279 / libdeflate | Elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.201945× | 0.201739× | 2.532281× | 2.529701× | -0.042% |
| prefix-random-8192 | 0.968319× | 0.973673× | 3.468744× | 3.487925× | -0.494% |
| prefix-natural-4096 | 1.103297× | 1.093769× | 1.886769× | 1.870476× | -0.964% |
| prefix-natural-8192 | 1.323797× | 1.317903× | 1.644648× | 1.637326× | -0.875% |
| prefix-periodic-1048576 | 0.975436× | 0.969030× | 1.649626× | 1.638792× | -0.658% |
| prefix-repeated-1048576 | 0.954446× | 0.948688× | 1.501559× | 1.492502× | -0.638% |
| prefix-natural-32768 | 1.506811× | 1.496483× | 1.370388× | 1.360995× | -0.620% |
| prefix-natural-1048576 | 1.527874× | 1.522098× | 1.232274× | 1.227616× | -0.248% |
| prefix-random-1048576 | 1.976420× | 1.975365× | 2.793264× | 2.791773× | -0.070% |

## Qualification and profiles

- 486 complete linked calls check exact bytes, independent inflation, aliases, capacities and final cleanup. The chosen original and decoded images match exactly.
- The retained direct-phase receipts cover 45 calls across the baseline and the original and decoded variant images. The control varies the length-cost, offset-cost and slot tables before a short run returns to a vector match. It also exercises the readable bounds, an assertion failure and retry. The exported diagnostic symbol preserves the actual phase instructions.
- The four-actor cohort includes 245,664 timed calls. The separate logical heap counts match in all 81 cases.

| Input | R278 instructions | R279 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,122.5 | 811,974.0 | -0.018% | +4.197% |
| prefix-random-8192 | 7,039,747.5 | 6,964,447.5 | -1.070% | -0.356% |
| prefix-natural-4096 | 11,286,556.0 | 11,236,514.5 | -0.443% | -1.612% |
| prefix-natural-8192 | 23,135,890.5 | 23,030,215.0 | -0.457% | -1.623% |
| prefix-periodic-1048576 | 673,116,298.5 | 667,045,438.5 | -0.902% | -0.667% |
| prefix-repeated-1048576 | 680,669,456.0 | 674,600,663.0 | -0.892% | -0.591% |
| prefix-natural-32768 | 96,401,680.0 | 95,788,326.5 | -0.636% | -1.858% |
| prefix-natural-1048576 | 3,515,858,707.0 | 3,489,549,732.0 | -0.748% | -1.382% |
| prefix-random-1048576 | 826,383,182.5 | 817,555,992.0 | -1.068% | -0.001% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,526,912 bytes before and 54,658,048 bytes after. This is process-wide and includes the harness. The logical heap and copied bytes use the independent memory cohort.

CPU time is sampled from the exact linked candidate in a separate session. The listed short-loop offsets are the instructions on the continuing visit. These observations are sample locations.

| Input | Running samples | Path body | Short-loop instructions | Matcher body |
|---|---:|---:|---:|---:|
| eight-literals | 5452 | 4 | 2 | 679 |
| prefix-natural-1048576 | 5515 | 2667 | 10 | 2469 |
| prefix-natural-32768 | 5333 | 2522 | 28 | 2181 |
| prefix-natural-4096 | 5565 | 1533 | 41 | 1823 |
| prefix-periodic-1048576 | 5354 | 801 | 658 | 3741 |
| prefix-random-1048576 | 5509 | 545 | 538 | 3552 |
| prefix-repeated-1048576 | 5511 | 778 | 644 | 3606 |

The closest candidate case still above parity is prefix-natural-4096 at 1.101818× Roc in the primary cohorts. The ordinary path still emits 64-bit adds followed by a low-word mask for wrapped costs. The next compiler experiment will use that type information to shorten the matched path.

## All primary timings

| Input | Bytes | R278 / Roc | R279 / Roc | R279 / R278 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.077468× | 2.104285× | 1.012908× | pass/pass |
| one-stored | 1 | 1.967716× | 1.976237× | 1.004330× | pass/pass |
| seven-stored-prefix | 7 | 2.239103× | 2.228145× | 0.995106× | pass/pass |
| eight-literals | 8 | 0.199921× | 0.199837× | 0.999580× | pass/pass |
| eight-overlap | 8 | 0.187286× | 0.187256× | 0.999844× | pass/pass |
| 320-overlap | 320 | 0.222011× | 0.221405× | 0.997273× | pass/pass |
| eight-literals-prefix | 8 | 0.200182× | 0.200256× | 1.000368× | pass/pass |
| 128-biased-dynamic | 128 | 0.268834× | 0.267722× | 0.995864× | pass/pass |
| 32-high-entropy-stored | 32 | 0.269136× | 0.268155× | 0.996355× | pass/pass |
| 16384-distribution-change | 16384 | 1.634306× | 1.635767× | 1.000894× | pass/pass |
| 33027-periodic-window | 33027 | 0.845982× | 0.842382× | 0.995744× | pass/pass |
| random-64 | 64 | 0.283353× | 0.282712× | 0.997740× | pass/pass |
| random-256 | 256 | 0.343149× | 0.342915× | 0.999319× | pass/pass |
| random-1024 | 1024 | 0.421666× | 0.421889× | 1.000528× | pass/pass |
| repeated-1024 | 1024 | 0.280789× | 0.280105× | 0.997566× | pass/pass |
| random-32769 | 32769 | 1.613159× | 1.607911× | 0.996747× | pass/pass |
| repeated-32769 | 32769 | 0.825493× | 0.817863× | 0.990757× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.540572× | 0.534732× | 0.989195× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.454527× | 1.440863× | 0.990606× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.498937× | 1.486889× | 0.991962× | pass/pass |
| heldout-window-32767 | 32767 | 1.500460× | 1.489042× | 0.992391× | pass/pass |
| heldout-window-32768 | 32768 | 1.497011× | 1.486953× | 0.993281× | pass/pass |
| heldout-slides-65537 | 65537 | 1.538902× | 1.533032× | 0.996185× | pass/pass |
| heldout-random-65537 | 65537 | 1.781260× | 1.780890× | 0.999793× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.556960× | 1.548695× | 0.994692× | pass/pass |
| prefix-natural-1024 | 1024 | 0.540267× | 0.534319× | 0.988989× | pass/pass |
| prefix-natural-4096 | 4096 | 1.112538× | 1.101818× | 0.990364× | pass/pass |
| prefix-natural-8192 | 8192 | 1.333811× | 1.322135× | 0.991246× | pass/pass |
| prefix-natural-16384 | 16384 | 1.446479× | 1.439368× | 0.995084× | pass/pass |
| prefix-natural-32767 | 32767 | 1.497834× | 1.489125× | 0.994185× | pass/pass |
| prefix-natural-32768 | 32768 | 1.502955× | 1.493630× | 0.993796× | pass/pass |
| prefix-natural-32769 | 32769 | 1.498163× | 1.490693× | 0.995014× | pass/pass |
| prefix-natural-33027 | 33027 | 1.500228× | 1.488875× | 0.992433× | pass/pass |
| prefix-natural-65535 | 65535 | 1.504564× | 1.492613× | 0.992057× | pass/pass |
| prefix-natural-65536 | 65536 | 1.505644× | 1.495563× | 0.993304× | pass/pass |
| prefix-natural-65537 | 65537 | 1.500802× | 1.488789× | 0.991995× | pass/pass |
| prefix-natural-131072 | 131072 | 1.503017× | 1.496089× | 0.995391× | pass/pass |
| prefix-natural-262144 | 262144 | 1.508062× | 1.502494× | 0.996308× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.525770× | 1.521987× | 0.997520× | pass/pass |
| prefix-random-1024 | 1024 | 0.426711× | 0.426544× | 0.999610× | pass/pass |
| prefix-random-4096 | 4096 | 0.738209× | 0.735386× | 0.996175× | pass/pass |
| prefix-random-8192 | 8192 | 0.971249× | 0.966454× | 0.995063× | pass/pass |
| prefix-random-16384 | 16384 | 1.340253× | 1.333055× | 0.994629× | pass/pass |
| prefix-random-32767 | 32767 | 1.615679× | 1.610040× | 0.996510× | pass/pass |
| prefix-random-32768 | 32768 | 1.612813× | 1.610149× | 0.998348× | pass/pass |
| prefix-random-32769 | 32769 | 1.611422× | 1.609794× | 0.998990× | pass/pass |
| prefix-random-33027 | 33027 | 1.617018× | 1.613375× | 0.997747× | pass/pass |
| prefix-random-65535 | 65535 | 1.811340× | 1.804686× | 0.996327× | pass/pass |
| prefix-random-65536 | 65536 | 1.805604× | 1.801386× | 0.997664× | pass/pass |
| prefix-random-65537 | 65537 | 1.797062× | 1.792548× | 0.997488× | pass/pass |
| prefix-random-131072 | 131072 | 1.906671× | 1.904803× | 0.999020× | pass/pass |
| prefix-random-262144 | 262144 | 1.949922× | 1.949598× | 0.999834× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.975580× | 1.974190× | 0.999296× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.346916× | 0.345598× | 0.996201× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.509620× | 0.503876× | 0.988729× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.613398× | 0.608237× | 0.991586× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.735315× | 0.728976× | 0.991379× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.830929× | 0.825900× | 0.993948× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.835675× | 0.828925× | 0.991922× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.838604× | 0.827052× | 0.986224× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.846836× | 0.838255× | 0.989867× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.905990× | 0.899710× | 0.993069× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.910473× | 0.902791× | 0.991563× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.908938× | 0.900843× | 0.991094× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.948416× | 0.940321× | 0.991465× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.965066× | 0.957876× | 0.992550× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.975018× | 0.968606× | 0.993425× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.279859× | 0.279108× | 0.997317× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.458301× | 0.455687× | 0.994297× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.569621× | 0.564245× | 0.990563× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.693692× | 0.690643× | 0.995604× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.798069× | 0.790298× | 0.990262× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.799772× | 0.794101× | 0.992908× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.822352× | 0.815901× | 0.992156× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.838360× | 0.833136× | 0.993768× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.891173× | 0.885034× | 0.993111× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.893567× | 0.885334× | 0.990787× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.892748× | 0.883556× | 0.989704× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.935558× | 0.928781× | 0.992757× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.950981× | 0.946336× | 0.995116× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.955600× | 0.949500× | 0.993616× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r279-append-descriptors-a78223cc3339.tar.gz). SHA-256 a78223cc333914abd2a298deba5e6e5e12e8643d6eeaca7105ade6556e7896b1.
