# R287 — Use checked view lengths for loop indices

**Retain R287 with the R272 runtime.** Candidate paired elapsed changes -0.463% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R286 beats Roc on 45/81 cases and R287 beats it on 45/81 (55.56%). The independent four-actor cohort gives R287 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.575% and -0.351%. The largest paired elapsed ratio has a change of +0.262% on prefix-periodic-8192. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent e29b33e7836b9af8ba77d9d6ddc1dd0b95a849bc. Measured parent e29b33e7836b9af8ba77d9d6ddc1dd0b95a849bc. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R286 backend and allocator and a small compiler overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The candidate uses the pinned input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary.

## Compiler rule and emitted code

The existing width solver includes every incoming loop value, including backedges. It now handles addition by a constant, power-of-two multiplication and cancelling offsets at the operation’s actual modular width. A check comparing an index to a current length can use a lower bound for that exact SSA length established on a dominating selected edge. Unproved incoming values keep their full type widths. The new proof is used in the existing private-vector phase optimization.

The two original length assertions require at least 262,144 entries. The actual matcher indices are at most 65,535. All twelve checks comparing the carried write indices with that same current length become jumps to their original success blocks. Both assertions still execute. The one changed machine body among 347 is the vector matcher. It shrinks from 27,748 to 27,252 code bytes and from a 928- to an 864-byte frame. The linked body has 12 fewer conditional branches, 33 fewer single stack loads and six fewer single stack stores.

The exact Roc and libdeflate binaries write pending links through register-held indices or addresses. The separate safe Rust slice witness emits one entry length comparison and direct halfword accesses. Each of these patterns is visible in the archived ARM64 excerpts.

The initial 27-case screen changes paired elapsed by -0.306%. The screen records 0 pooled and 0 block misses.

## Headline matched ratios

| Input | R286 / Roc | R287 / Roc | R286 / libdeflate | R287 / libdeflate | R287 / R286 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.198129× | 0.198445× | 2.519734× | 2.523749× | +0.037% |
| prefix-random-8192 | 0.859466× | 0.857887× | 3.082899× | 3.077236× | -1.053% |
| prefix-natural-4096 | 1.075146× | 1.067306× | 1.832369× | 1.819007× | -0.500% |
| prefix-natural-1048576 | 1.479486× | 1.466281× | 1.195625× | 1.184954× | -0.584% |
| prefix-random-1048576 | 1.780821× | 1.764568× | 2.518956× | 2.495966× | -0.712% |
| prefix-periodic-1048576 | 0.846272× | 0.868962× | 1.410258× | 1.448070× | +0.005% |
| prefix-repeated-1048576 | 0.836137× | 0.836853× | 1.314402× | 1.315527× | -0.195% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 1.0077× R286.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- Forty-three compiler cleanup tests pass with warnings as errors. The native length fixture compares 500 original and optimized argument pairs against a separate model. It checks the selected proof, an insufficient floor, a different length, an unrestricted backedge, and a join reached without the proof.
- The first test run exposed stale verifier metadata in the test helper after a branch rewrite. Recomputing the dominator tree there resolved all three test failures. The measured compiler uses the existing cleanup and verification path.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R286 instructions | R287 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 817,496.0 | 819,007.5 | +0.185% | +3.158% |
| prefix-random-8192 | 6,359,459.0 | 6,347,654.5 | -0.186% | -1.647% |
| prefix-natural-4096 | 10,696,303.5 | 10,690,349.5 | -0.056% | -1.168% |
| prefix-natural-1048576 | 3,280,230,119.5 | 3,296,058,402.0 | +0.483% | -0.982% |
| prefix-random-1048576 | 725,118,579.0 | 724,652,064.5 | -0.064% | -0.739% |
| prefix-periodic-1048576 | 618,721,265.5 | 616,606,810.0 | -0.342% | +0.045% |
| prefix-repeated-1048576 | 626,397,743.5 | 624,296,547.0 | -0.335% | -0.061% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,559,680 bytes before and 54,576,064 bytes after. These process-wide samples include the harness.

Cases with opposite timing directions across cohorts total 27. The packet preserves both ratios and every original observation.

Natural text at 1 MiB retires 15,828,282 more instructions and changes cycles by -0.982%. The changed machine body has a different register assignment. The whole-call counters leave the extra instructions to be attributed to individual routes.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser | Vector matcher | Matcher stack loads / stores |
|---|---:|---:|---:|---:|
| eight-literals | 5325 | 2 | 660 | 18 / 6 |
| prefix-natural-1048576 | 5511 | 2750 | 2358 | 1261 / 139 |
| prefix-natural-32768 | 5374 | 2655 | 2091 | 1017 / 151 |
| prefix-natural-4096 | 5583 | 1617 | 1676 | 594 / 176 |
| prefix-periodic-1048576 | 5362 | 699 | 3742 | 1831 / 322 |
| prefix-random-1048576 | 5516 | 458 | 3514 | 1700 / 555 |
| prefix-repeated-1048576 | 5357 | 658 | 3468 | 1714 / 276 |

The sampled vector matcher contains two gate spans that combine the same overflow and flat-view predicates twice. They account for 183 running samples on 1 MiB periodic input and 169 on 1 MiB repeated input. The exact current matcher profile identifies the hottest sites and their stack-access categories in the packet. The compact timing worker runs outside the ordinary path’s managed admission, resource accounting and recovery.

The closest candidate case above parity is prefix-natural-4096 at 1.076438× Roc. Emit one copy of each required predicate in a captured-window gate. Two reached matcher entries repeat the same overflow and flat-view checks for their paired captures. Keep distinct current operands and the checked fallback.

## All primary timings

| Input | Bytes | R286 / Roc | R287 / Roc | R287 / R286 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.141608× | 2.104633× | 0.982735× | pass/pass |
| one-stored | 1 | 2.063170× | 2.005858× | 0.972222× | pass/pass |
| seven-stored-prefix | 7 | 2.283778× | 2.245630× | 0.983296× | pass/pass |
| eight-literals | 8 | 0.199413× | 0.199486× | 1.000367× | pass/pass |
| eight-overlap | 8 | 0.185475× | 0.185540× | 1.000352× | pass/pass |
| 320-overlap | 320 | 0.215423× | 0.215536× | 1.000527× | pass/pass |
| eight-literals-prefix | 8 | 0.197028× | 0.196465× | 0.997145× | pass/pass |
| 128-biased-dynamic | 128 | 0.261839× | 0.260978× | 0.996711× | pass/pass |
| 32-high-entropy-stored | 32 | 0.265437× | 0.264631× | 0.996964× | pass/pass |
| 16384-distribution-change | 16384 | 1.526168× | 1.517571× | 0.994367× | pass/pass |
| 33027-periodic-window | 33027 | 0.747063× | 0.745622× | 0.998072× | pass/pass |
| random-64 | 64 | 0.278980× | 0.279169× | 1.000680× | pass/pass |
| random-256 | 256 | 0.335987× | 0.335326× | 0.998031× | pass/pass |
| random-1024 | 1024 | 0.399671× | 0.398333× | 0.996653× | pass/pass |
| repeated-1024 | 1024 | 0.265891× | 0.265670× | 0.999168× | pass/pass |
| random-32769 | 32769 | 1.431515× | 1.421369× | 0.992912× | pass/pass |
| repeated-32769 | 32769 | 0.731375× | 0.726978× | 0.993988× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.517288× | 0.514460× | 0.994534× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.406770× | 1.394028× | 0.990942× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.456125× | 1.444689× | 0.992146× | pass/pass |
| heldout-window-32767 | 32767 | 1.451679× | 1.440447× | 0.992263× | pass/pass |
| heldout-window-32768 | 32768 | 1.452983× | 1.444928× | 0.994456× | pass/pass |
| heldout-slides-65537 | 65537 | 1.484353× | 1.473754× | 0.992860× | pass/pass |
| heldout-random-65537 | 65537 | 1.590950× | 1.579489× | 0.992796× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.449275× | 1.440521× | 0.993959× | pass/pass |
| prefix-natural-1024 | 1024 | 0.522606× | 0.518802× | 0.992720× | pass/pass |
| prefix-natural-4096 | 4096 | 1.081842× | 1.076438× | 0.995004× | pass/pass |
| prefix-natural-8192 | 8192 | 1.281668× | 1.280055× | 0.998742× | pass/pass |
| prefix-natural-16384 | 16384 | 1.403252× | 1.392055× | 0.992020× | pass/pass |
| prefix-natural-32767 | 32767 | 1.454475× | 1.442789× | 0.991966× | pass/pass |
| prefix-natural-32768 | 32768 | 1.451798× | 1.442137× | 0.993346× | pass/pass |
| prefix-natural-32769 | 32769 | 1.454894× | 1.443646× | 0.992269× | pass/pass |
| prefix-natural-33027 | 33027 | 1.455005× | 1.442843× | 0.991641× | pass/pass |
| prefix-natural-65535 | 65535 | 1.462004× | 1.449120× | 0.991188× | pass/pass |
| prefix-natural-65536 | 65536 | 1.460685× | 1.448768× | 0.991842× | pass/pass |
| prefix-natural-65537 | 65537 | 1.449493× | 1.439805× | 0.993316× | pass/pass |
| prefix-natural-131072 | 131072 | 1.460656× | 1.450950× | 0.993355× | pass/pass |
| prefix-natural-262144 | 262144 | 1.463801× | 1.455551× | 0.994364× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.479071× | 1.470430× | 0.994158× | pass/pass |
| prefix-random-1024 | 1024 | 0.399246× | 0.399195× | 0.999872× | pass/pass |
| prefix-random-4096 | 4096 | 0.673138× | 0.668228× | 0.992706× | pass/pass |
| prefix-random-8192 | 8192 | 0.859489× | 0.850438× | 0.989469× | pass/pass |
| prefix-random-16384 | 16384 | 1.189036× | 1.177172× | 0.990022× | pass/pass |
| prefix-random-32767 | 32767 | 1.436711× | 1.429731× | 0.995142× | pass/pass |
| prefix-random-32768 | 32768 | 1.441992× | 1.430866× | 0.992284× | pass/pass |
| prefix-random-32769 | 32769 | 1.433901× | 1.425014× | 0.993802× | pass/pass |
| prefix-random-33027 | 33027 | 1.449453× | 1.437543× | 0.991783× | pass/pass |
| prefix-random-65535 | 65535 | 1.621856× | 1.608505× | 0.991768× | pass/pass |
| prefix-random-65536 | 65536 | 1.615374× | 1.608389× | 0.995676× | pass/pass |
| prefix-random-65537 | 65537 | 1.615658× | 1.602992× | 0.992161× | pass/pass |
| prefix-random-131072 | 131072 | 1.708687× | 1.699127× | 0.994405× | pass/pass |
| prefix-random-262144 | 262144 | 1.757191× | 1.751712× | 0.996882× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.779791× | 1.767113× | 0.992877× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.330483× | 0.329799× | 0.997930× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.467791× | 0.466091× | 0.996365× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.544207× | 0.545632× | 1.002618× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.649302× | 0.646784× | 0.996121× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.730949× | 0.727748× | 0.995622× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.733308× | 0.731791× | 0.997931× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.736906× | 0.735551× | 0.998162× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.747134× | 0.745177× | 0.997382× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.792227× | 0.792880× | 1.000824× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.794655× | 0.793913× | 0.999066× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.794668× | 0.792732× | 0.997564× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.825089× | 0.823809× | 0.998449× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.837586× | 0.838067× | 1.000575× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.846125× | 0.846169× | 1.000052× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.265170× | 0.265049× | 0.999540× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.414615× | 0.412892× | 0.995844× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.507790× | 0.507859× | 1.000136× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.616220× | 0.616709× | 1.000792× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.701519× | 0.701341× | 0.999747× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.701839× | 0.702153× | 1.000447× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.723751× | 0.723454× | 0.999590× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.748098× | 0.742991× | 0.993172× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.786846× | 0.786736× | 0.999861× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.785947× | 0.784442× | 0.998086× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.788925× | 0.788018× | 0.998850× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.819982× | 0.819684× | 0.999636× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.834918× | 0.835476× | 1.000669× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.837451× | 0.835822× | 0.998054× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r287-current-matcher-lengths-9737d0b95997.tar.gz). SHA-256 9737d0b959977f3f7c8a579212e7bfe440ddee5375d94bc7d88f63aea3b3a65c.
