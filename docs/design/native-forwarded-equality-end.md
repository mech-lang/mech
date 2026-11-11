# R303 — Forward a proved equality endpoint

**Retain R303 with the R272 runtime.** Candidate paired elapsed changes -1.436% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R301 beats Roc on 45/81 cases and R303 beats it on 45/81 (55.56%). The separate four-actor cohort gives R303 46/81 below Roc and 3/81 below libdeflate.

The two primary cohorts change elapsed by -1.524% and -1.347%. The largest paired slowdown is +0.270% on reymont-prefix-1024. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Measured parent 4debec08de0feeb0884f13aa620c7167a1729682. Published parent ac79c1c3659866338ef98f8e3d5d7c3c3dcd61d6. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R301 closure and four changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The short scan proves that its entire prefix lies in the saved equal interval at the same displacement. A unique effect-free chain from the bound exit must pass the current cursor and both ordered addresses into the continuation. The selected edge passes the already loaded endpoint through the original admission. Other arrivals pass zero and use the checked lookup. The forwarded route clamps the advance to the current limit and rounds it down to whole words. Only the original successful bound exit records the new end.

The current artifact selects one handoff, source header 2713 to 2733. One of 347 bodies changes. The vector matcher grows from 13,696 to 13,832 code bytes and from 768 to 784 frame bytes. The original assertions, captured span gates, interval slot inventory and checked continuation remain in the linked body.

| Linked successful route | R301 | R302 | R303 |
|---|---:|---:|---:|
| Prefix instructions | 36 | 31 | 31 |
| Prefix stack loads | 2 | 7 | 8 |
| Continuation proof instructions | 25 | 27 | 8 |
| Continuation proof stack loads | 4 | 4 | 0 |
| Continuation proof stack stores | 3 | 3 | 0 |

The continuation count starts after the unchanged checked admission. The exact pinned peers and safe Rust excerpts show the corresponding short scan and checked interval geometry.

The four-actor 27-case screen changes paired elapsed -2.252%. Its empty case misses the block gate. The independent three-actor confirmation changes elapsed -1.921% and passes both gates on all 27. Both sets of observations are retained.

| Screen case | R303 / R301, four actors | R303 / R302, four actors | R303 / R301, confirmation |
|---|---:|---:|---:|
| eight-literals | 1.002536× | 1.003593× | 0.994456× |
| prefix-natural-4096 | 0.988600× | 0.980797× | 1.002535× |
| prefix-periodic-1048576 | 0.964013× | 0.960210× | 0.962684× |
| prefix-repeated-1048576 | 0.966486× | 0.963788× | 0.966134× |

## Headline matched ratios

| Input | R301 / Roc | R303 / Roc | R301 / libdeflate | R303 / libdeflate | R303 / R301 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.195000× | 0.195287× | 2.487166× | 2.490820× | +0.008% |
| prefix-random-8192 | 0.810979× | 0.813350× | 2.946582× | 2.955200× | -1.097% |
| prefix-natural-4096 | 1.002560× | 0.997365× | 1.722593× | 1.713668× | -0.379% |
| prefix-natural-32768 | 1.370493× | 1.369492× | 1.248901× | 1.247989× | -0.127% |
| prefix-natural-1048576 | 1.408331× | 1.405020× | 1.136660× | 1.133988× | -0.178% |
| prefix-random-1048576 | 1.719274× | 1.711657× | 2.428797× | 2.418036× | -0.303% |
| prefix-periodic-1048576 | 0.779972× | 0.751472× | 1.321756× | 1.273459× | -3.772% |
| prefix-repeated-1048576 | 0.777212× | 0.751014× | 1.221658× | 1.180478× | -3.367% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

Natural 4 KiB measures 0.997365× Roc in the peer cohort and 1.017269× in the primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- The focused JIT comparison executes 23,408 compiled calls and checks 327,712 results. It covers alternate entry, refusal by the continuation gate, changed bases, changed cursor, an intervening effect, a merged path, a different view, and short or overflowing spans. Four inherited equality tests and 48 cleanup tests pass with warnings as errors.
- A stale receipt path stopped the first qualification command after both image checks. The corrected runner resumed with those receipts and entered the measurement campaign once.
- The 87 direct parser controls preserve the unchanged body encodings and the independent oracle. Captured-window, continuation, source-window, and ordered-span controls also pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R301 instructions | R303 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 773,152.0 | 773,041.0 | -0.014% | +4.157% |
| prefix-random-8192 | 5,842,660.0 | 5,820,529.5 | -0.379% | +0.235% |
| prefix-natural-4096 | 9,661,349.0 | 9,662,279.0 | +0.010% | -0.054% |
| prefix-natural-32768 | 80,615,364.0 | 80,659,836.0 | +0.055% | +0.197% |
| prefix-natural-1048576 | 2,932,916,098.5 | 2,936,173,328.5 | +0.111% | +0.134% |
| prefix-random-1048576 | 657,597,937.0 | 655,475,884.5 | -0.323% | +0.056% |
| prefix-periodic-1048576 | 552,102,848.0 | 525,256,437.0 | -4.863% | -3.560% |
| prefix-repeated-1048576 | 559,942,319.0 | 533,196,279.5 | -4.777% | -3.365% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,576,064 to 54,576,064 bytes. This includes the harness.

Timing directions differ between cohorts on 14 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Vector matcher before / after | Successful continuation before / after | Matcher stack-load samples before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5340 / 5410 | 729 / 696 | 0 / 0 | 18 / 16 |
| prefix-natural-1048576 | 5356 / 5509 | 2309 / 2376 | 0 / 0 | 707 / 698 |
| prefix-natural-32768 | 5366 / 5368 | 2061 / 2089 | 0 / 0 | 643 / 587 |
| prefix-natural-4096 | 5580 / 5605 | 1683 / 1679 | 0 / 0 | 481 / 517 |
| prefix-periodic-1048576 | 5370 / 5513 | 3608 / 3667 | 307 / 38 | 1758 / 1968 |
| prefix-random-1048576 | 5384 / 5513 | 3375 / 3445 | 0 / 0 | 1547 / 1013 |
| prefix-repeated-1048576 | 5377 / 5509 | 3340 / 3336 | 257 / 37 | 1630 / 1845 |

The archive also retains the corresponding R302 samples and exact instruction ranges.

The closest candidate case above parity is prefix-natural-4096 at 1.017269× Roc. Measure common address generation for the two neighboring typed table slots on the ordinary matcher route. The current search emits separate scaled addresses. The linked C worker uses one base plus an immediate two-byte displacement. The next candidate must preserve current access order, geometry and intervening effects.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R301 / Roc | R303 / Roc | R303 / R301 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.139246× | 2.104060× | 0.983552× | pass/pass |
| one-stored | 1 | 2.005267× | 1.988089× | 0.991433× | pass/pass |
| seven-stored-prefix | 7 | 2.228143× | 2.226171× | 0.999115× | pass/pass |
| eight-literals | 8 | 0.196284× | 0.196301× | 1.000084× | pass/pass |
| eight-overlap | 8 | 0.185697× | 0.185824× | 1.000684× | pass/pass |
| 320-overlap | 320 | 0.210638× | 0.210554× | 0.999603× | pass/pass |
| eight-literals-prefix | 8 | 0.195088× | 0.194565× | 0.997317× | pass/pass |
| 128-biased-dynamic | 128 | 0.256159× | 0.256369× | 1.000820× | pass/pass |
| 32-high-entropy-stored | 32 | 0.263794× | 0.262257× | 0.994174× | pass/pass |
| 16384-distribution-change | 16384 | 1.444851× | 1.438165× | 0.995373× | pass/pass |
| 33027-periodic-window | 33027 | 0.690772× | 0.664784× | 0.962379× | pass/pass |
| random-64 | 64 | 0.275633× | 0.274372× | 0.995427× | pass/pass |
| random-256 | 256 | 0.328596× | 0.327307× | 0.996077× | pass/pass |
| random-1024 | 1024 | 0.389777× | 0.389317× | 0.998821× | pass/pass |
| repeated-1024 | 1024 | 0.256419× | 0.254176× | 0.991256× | pass/pass |
| random-32769 | 32769 | 1.378008× | 1.370193× | 0.994329× | pass/pass |
| repeated-32769 | 32769 | 0.679608× | 0.657340× | 0.967233× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.494778× | 0.496115× | 1.002702× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.324106× | 1.325087× | 1.000741× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.370998× | 1.368939× | 0.998499× | pass/pass |
| heldout-window-32767 | 32767 | 1.372935× | 1.368793× | 0.996983× | pass/pass |
| heldout-window-32768 | 32768 | 1.370029× | 1.366461× | 0.997396× | pass/pass |
| heldout-slides-65537 | 65537 | 1.407030× | 1.406486× | 0.999613× | pass/pass |
| heldout-random-65537 | 65537 | 1.531862× | 1.523686× | 0.994663× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.373944× | 1.367434× | 0.995262× | pass/pass |
| prefix-natural-1024 | 1024 | 0.496899× | 0.497382× | 1.000971× | pass/pass |
| prefix-natural-4096 | 4096 | 1.021138× | 1.017269× | 0.996211× | pass/pass |
| prefix-natural-8192 | 8192 | 1.213538× | 1.213526× | 0.999990× | pass/pass |
| prefix-natural-16384 | 16384 | 1.325080× | 1.326511× | 1.001079× | pass/pass |
| prefix-natural-32767 | 32767 | 1.370000× | 1.371308× | 1.000955× | pass/pass |
| prefix-natural-32768 | 32768 | 1.373838× | 1.372088× | 0.998726× | pass/pass |
| prefix-natural-32769 | 32769 | 1.369665× | 1.369606× | 0.999957× | pass/pass |
| prefix-natural-33027 | 33027 | 1.380025× | 1.373041× | 0.994939× | pass/pass |
| prefix-natural-65535 | 65535 | 1.388281× | 1.386538× | 0.998744× | pass/pass |
| prefix-natural-65536 | 65536 | 1.389538× | 1.382704× | 0.995081× | pass/pass |
| prefix-natural-65537 | 65537 | 1.376179× | 1.372440× | 0.997283× | pass/pass |
| prefix-natural-131072 | 131072 | 1.388357× | 1.385932× | 0.998253× | pass/pass |
| prefix-natural-262144 | 262144 | 1.390184× | 1.389893× | 0.999791× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.407014× | 1.404509× | 0.998220× | pass/pass |
| prefix-random-1024 | 1024 | 0.391557× | 0.391584× | 1.000069× | pass/pass |
| prefix-random-4096 | 4096 | 0.652580× | 0.643335× | 0.985833× | pass/pass |
| prefix-random-8192 | 8192 | 0.828409× | 0.819318× | 0.989025× | pass/pass |
| prefix-random-16384 | 16384 | 1.133826× | 1.124678× | 0.991931× | pass/pass |
| prefix-random-32767 | 32767 | 1.378571× | 1.372680× | 0.995727× | pass/pass |
| prefix-random-32768 | 32768 | 1.380825× | 1.377039× | 0.997258× | pass/pass |
| prefix-random-32769 | 32769 | 1.378119× | 1.369345× | 0.993633× | pass/pass |
| prefix-random-33027 | 33027 | 1.386677× | 1.381061× | 0.995950× | pass/pass |
| prefix-random-65535 | 65535 | 1.560967× | 1.549861× | 0.992885× | pass/pass |
| prefix-random-65536 | 65536 | 1.559463× | 1.552496× | 0.995532× | pass/pass |
| prefix-random-65537 | 65537 | 1.553971× | 1.545555× | 0.994584× | pass/pass |
| prefix-random-131072 | 131072 | 1.652689× | 1.644896× | 0.995285× | pass/pass |
| prefix-random-262144 | 262144 | 1.694119× | 1.690808× | 0.998046× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.714345× | 1.709155× | 0.996972× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.322452× | 0.319448× | 0.990683× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.450721× | 0.436863× | 0.969253× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.525935× | 0.509292× | 0.968357× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.603250× | 0.585372× | 0.970364× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.687431× | 0.661004× | 0.961557× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.679076× | 0.656826× | 0.967235× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.686050× | 0.663272× | 0.966798× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.694089× | 0.668275× | 0.962809× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.731516× | 0.704602× | 0.963208× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.734967× | 0.708358× | 0.963796× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.732510× | 0.705547× | 0.963191× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.764551× | 0.735910× | 0.962539× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.772716× | 0.744775× | 0.963841× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.781620× | 0.752135× | 0.962277× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.258893× | 0.256784× | 0.991857× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.408782× | 0.394644× | 0.965413× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.482228× | 0.467196× | 0.968829× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.577647× | 0.559133× | 0.967950× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.654892× | 0.632926× | 0.966459× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.654396× | 0.632380× | 0.966356× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.677452× | 0.656418× | 0.968951× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.695947× | 0.672061× | 0.965678× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.729623× | 0.704318× | 0.965318× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.732180× | 0.705501× | 0.963562× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.731614× | 0.706510× | 0.965688× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.766573× | 0.740331× | 0.965768× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.779049× | 0.752482× | 0.965899× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.776908× | 0.750746× | 0.966325× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r303-single-equality-proof-cb6dea7f94a7.tar.gz). SHA-256 cb6dea7f94a7e7b41afd1299500c9986362133d6d9d1a73c84f1c88211d1005f.
