# R286 — Retain spill copy facts across adjacent blocks

**Retain R286 with the R272 runtime.** Candidate paired elapsed changes -3.002% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R285 beats Roc on 45/81 cases and R286 beats it on 45/81 (55.56%). The independent four-actor cohort gives R286 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -2.952% and -3.052%. The smallest paired elapsed gain is -0.232% on eight-literals. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent a50d4f5b20fdd19aaf6063614bd7834e5632f376. Measured parent a50d4f5b20fdd19aaf6063614bd7834e5632f376. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R285 Mech and Cranelift source closure plus one change in regalloc2 0.15.2. Apple ARM64, Cranelift 0.131.3 and IR/work/ABI/bytecode 215/107/39/14. The candidate uses the pinned input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary.

## Compiler rule and emitted code

The register allocator used to discard its current copy facts at every block boundary. It now preserves them across contiguous blocks when every next block has exactly the preceding block as its sole predecessor and is reached by one actual edge. An entry, join or gap clears the facts. The existing scan invalidates copies on definitions, register clobbers, scratch uses and intervening moves.

The measured vector matcher keeps the first reload of x0 from [sp, #440] in VCode block 511, uses it in the comparison, and retains the loaded value through blocks 512 to 514. The reload at block 515 is gone. Three of 347 machine bodies change. The vector matcher removes 13 stack loads and 60 stack stores. The root, general matcher and vector matcher shrink by 1,992, 284 and 292 code bytes respectively. Their stack frames remain 57,072, 1,056 and 928 bytes.

The pinned Roc matcher keeps its window delta in a register through its comparison and tree update. The pinned libdeflate matcher keeps the signed link value in a register from the tree load through the comparison and next input read. The exact linked ARM64 excerpts show the register lifetimes in each executable.

The initial 27-case screen changes paired elapsed by -3.516%. The screen records 0 pooled and 0 block misses.

## Headline matched ratios

| Input | R285 / Roc | R286 / Roc | R285 / libdeflate | R286 / libdeflate | R286 / R285 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.196468× | 0.195031× | 2.528546× | 2.510044× | -0.232% |
| prefix-random-8192 | 0.969709× | 0.907995× | 3.321527× | 3.110141× | -3.981% |
| prefix-natural-4096 | 1.080632× | 1.062951× | 1.859157× | 1.828738× | -1.608% |
| prefix-natural-1048576 | 1.485025× | 1.475257× | 1.197336× | 1.189461× | -0.536% |
| prefix-random-1048576 | 1.816262× | 1.773796× | 2.569302× | 2.509229× | -2.139% |
| prefix-periodic-1048576 | 0.901823× | 0.849858× | 1.524770× | 1.436910× | -5.848% |
| prefix-repeated-1048576 | 0.884717× | 0.836596× | 1.391071× | 1.315408× | -5.416% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 1.0050× R285.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- Five focused post-allocation tests exercise the real move resolver and check its output with the register allocator checker. The original allocator retains the redundant load in the positive control and passes the four refusal controls. The candidate removes it and preserves the required loads at a join, after clobber or spill-slot rewrite, and across a gap in instruction order. Forty-two compiler cleanup tests pass with warnings as errors.
- Both changed matcher bodies pass the optional allocator checker before root emission. The root checker attempt exits with SIGKILL after 108.33 s at the machine-code stage. The cause is unconfirmed. Normal root emission and the linked controls pass.
- The first full-run setup still selected the prior round’s worker path and failed before timing. The corrected run uses the pinned candidate and retains the first receipt.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R285 instructions | R286 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 813,282.5 | 812,635.5 | -0.080% | +0.585% |
| prefix-random-8192 | 6,534,847.5 | 6,361,883.0 | -2.647% | -4.176% |
| prefix-natural-4096 | 10,795,685.0 | 10,694,086.0 | -0.941% | -1.983% |
| prefix-natural-1048576 | 3,309,111,539.0 | 3,280,352,228.0 | -0.869% | -0.613% |
| prefix-random-1048576 | 748,031,082.5 | 725,115,204.0 | -3.063% | -2.336% |
| prefix-periodic-1048576 | 642,796,009.5 | 618,714,238.0 | -3.746% | -5.696% |
| prefix-repeated-1048576 | 650,441,519.0 | 626,403,775.0 | -3.696% | -5.306% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,576,064 bytes before and 54,592,448 bytes after. These process-wide samples include the harness.

Cases with opposite timing directions across cohorts total 1. The one-byte stored case changes +0.541% and -1.107%. The replay retains both ratios and all observations for each case.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser | Vector matcher | Matcher stack loads / stores | At adjacent reloads |
|---|---:|---:|---:|---:|---:|
| eight-literals | 5330 | 9 | 651 | 16 / 7 | 0 / 0 |
| prefix-natural-1048576 | 5362 | 2655 | 2309 | 1205 / 141 | 0 / 0 |
| prefix-natural-32768 | 5364 | 2640 | 2100 | 1024 / 154 | 0 / 0 |
| prefix-natural-4096 | 5601 | 1580 | 1699 | 575 / 156 | 0 / 0 |
| prefix-periodic-1048576 | 5362 | 700 | 3729 | 1833 / 312 | 0 / 0 |
| prefix-random-1048576 | 5364 | 431 | 3403 | 1017 / 424 | 0 / 0 |
| prefix-repeated-1048576 | 5358 | 659 | 3438 | 1695 / 309 | 0 / 0 |

The measured matcher still has two consecutive loads of the same spill slot into x8 and x0. The pair accounts for zero observed samples in seven profiles. Three loads of the tree length account for 465 samples on 1 MiB natural text and 129 on 4 KiB natural text. The CLIF audit covers both existing length assertions and all twelve guards against that length. The largest index bound is 65,535, with a required floor of 262,144. The audit includes loop backedges. The compact timing worker runs outside the ordinary path’s managed admission, resource accounting and recovery.

The current parser’s continuing empty route has 30 instructions with zero stack accesses. The comparable libdeflate span has 16. Mech’s route from a nonempty header in that loop back to the ordinary header executes 34 instructions with seven stack accesses. The packet includes current samples for both spans. The short loop still computes the cache ordinal twice and checks the previous-node index after forwarding the previous append.

The closest candidate case above parity is prefix-natural-4096 at 1.077548× Roc. Carry bounds on every incoming loop index to a comparison against the same current view length. The two existing length assertions establish the floor for twelve surviving tree write checks. Preserve both assertions and prove all loop entries and backedges before changing the generated branches.

## All primary timings

| Input | Bytes | R285 / Roc | R286 / Roc | R286 / R285 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.130583× | 2.084840× | 0.978530× | pass/pass |
| one-stored | 1 | 2.030085× | 2.024274× | 0.997137× | pass/pass |
| seven-stored-prefix | 7 | 2.273872× | 2.252402× | 0.990558× | pass/pass |
| eight-literals | 8 | 0.195864× | 0.195410× | 0.997679× | pass/pass |
| eight-overlap | 8 | 0.184776× | 0.183748× | 0.994440× | pass/pass |
| 320-overlap | 320 | 0.215635× | 0.213240× | 0.988892× | pass/pass |
| eight-literals-prefix | 8 | 0.198039× | 0.196894× | 0.994219× | pass/pass |
| 128-biased-dynamic | 128 | 0.264614× | 0.259189× | 0.979497× | pass/pass |
| 32-high-entropy-stored | 32 | 0.261478× | 0.259688× | 0.993151× | pass/pass |
| 16384-distribution-change | 16384 | 1.561682× | 1.522915× | 0.975176× | pass/pass |
| 33027-periodic-window | 33027 | 0.790414× | 0.744064× | 0.941360× | pass/pass |
| random-64 | 64 | 0.277418× | 0.274618× | 0.989907× | pass/pass |
| random-256 | 256 | 0.342803× | 0.333662× | 0.973336× | pass/pass |
| random-1024 | 1024 | 0.416288× | 0.399583× | 0.959872× | pass/pass |
| repeated-1024 | 1024 | 0.271451× | 0.266282× | 0.980957× | pass/pass |
| random-32769 | 32769 | 1.477942× | 1.433094× | 0.969656× | pass/pass |
| repeated-32769 | 32769 | 0.769075× | 0.728184× | 0.946831× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.530941× | 0.517507× | 0.974696× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.410758× | 1.401550× | 0.993473× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.462465× | 1.454217× | 0.994360× | pass/pass |
| heldout-window-32767 | 32767 | 1.461305× | 1.454478× | 0.995328× | pass/pass |
| heldout-window-32768 | 32768 | 1.462865× | 1.454220× | 0.994090× | pass/pass |
| heldout-slides-65537 | 65537 | 1.490947× | 1.484413× | 0.995618× | pass/pass |
| heldout-random-65537 | 65537 | 1.633320× | 1.589807× | 0.973359× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.467392× | 1.446278× | 0.985612× | pass/pass |
| prefix-natural-1024 | 1024 | 0.530111× | 0.516970× | 0.975210× | pass/pass |
| prefix-natural-4096 | 4096 | 1.095155× | 1.077548× | 0.983922× | pass/pass |
| prefix-natural-8192 | 8192 | 1.291700× | 1.279666× | 0.990684× | pass/pass |
| prefix-natural-16384 | 16384 | 1.413296× | 1.400300× | 0.990804× | pass/pass |
| prefix-natural-32767 | 32767 | 1.458965× | 1.451336× | 0.994771× | pass/pass |
| prefix-natural-32768 | 32768 | 1.463517× | 1.455466× | 0.994499× | pass/pass |
| prefix-natural-32769 | 32769 | 1.462342× | 1.455179× | 0.995102× | pass/pass |
| prefix-natural-33027 | 33027 | 1.460959× | 1.451339× | 0.993415× | pass/pass |
| prefix-natural-65535 | 65535 | 1.464272× | 1.455684× | 0.994135× | pass/pass |
| prefix-natural-65536 | 65536 | 1.462587× | 1.453712× | 0.993932× | pass/pass |
| prefix-natural-65537 | 65537 | 1.456608× | 1.449875× | 0.995377× | pass/pass |
| prefix-natural-131072 | 131072 | 1.469428× | 1.464336× | 0.996535× | pass/pass |
| prefix-natural-262144 | 262144 | 1.470654× | 1.461403× | 0.993710× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.486721× | 1.478746× | 0.994636× | pass/pass |
| prefix-random-1024 | 1024 | 0.416498× | 0.400071× | 0.960560× | pass/pass |
| prefix-random-4096 | 4096 | 0.712356× | 0.679102× | 0.953319× | pass/pass |
| prefix-random-8192 | 8192 | 0.898734× | 0.862959× | 0.960194× | pass/pass |
| prefix-random-16384 | 16384 | 1.237190× | 1.192747× | 0.964077× | pass/pass |
| prefix-random-32767 | 32767 | 1.489936× | 1.442823× | 0.968379× | pass/pass |
| prefix-random-32768 | 32768 | 1.494153× | 1.446030× | 0.967792× | pass/pass |
| prefix-random-32769 | 32769 | 1.486093× | 1.439629× | 0.968734× | pass/pass |
| prefix-random-33027 | 33027 | 1.491552× | 1.443167× | 0.967561× | pass/pass |
| prefix-random-65535 | 65535 | 1.665179× | 1.619767× | 0.972728× | pass/pass |
| prefix-random-65536 | 65536 | 1.662544× | 1.617515× | 0.972915× | pass/pass |
| prefix-random-65537 | 65537 | 1.661851× | 1.616774× | 0.972876× | pass/pass |
| prefix-random-131072 | 131072 | 1.756928× | 1.709541× | 0.973028× | pass/pass |
| prefix-random-262144 | 262144 | 1.798491× | 1.755714× | 0.976215× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.815465× | 1.776631× | 0.978609× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.334848× | 0.328143× | 0.979976× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.482692× | 0.464238× | 0.961769× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.568882× | 0.545751× | 0.959340× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.685090× | 0.645691× | 0.942490× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.769137× | 0.730466× | 0.949722× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.772285× | 0.728544× | 0.943361× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.775814× | 0.734519× | 0.946772× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.787003× | 0.739689× | 0.939881× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.838530× | 0.789683× | 0.941747× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.840862× | 0.791680× | 0.941510× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.842525× | 0.794405× | 0.942886× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.878532× | 0.825659× | 0.939817× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.888609× | 0.836802× | 0.941698× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.898212× | 0.845681× | 0.941515× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.269676× | 0.263624× | 0.977557× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.428941× | 0.412286× | 0.961170× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.530213× | 0.510465× | 0.962755× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.652250× | 0.615492× | 0.943645× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.741399× | 0.700442× | 0.944757× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.741556× | 0.699782× | 0.943668× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.759662× | 0.725172× | 0.954598× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.784183× | 0.742141× | 0.946387× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.830079× | 0.787284× | 0.948445× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.831997× | 0.787190× | 0.946146× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.831469× | 0.785537× | 0.944758× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.868131× | 0.820927× | 0.945625× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.882859× | 0.834911× | 0.945691× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.884657× | 0.836747× | 0.945844× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r286-adjacent-spill-reloads-b72b8eab23ff.tar.gz). SHA-256 b72b8eab23ff1cebc1ef7c312773fb468046fb6fa3fefe16ca0f7c30385ebe48.
