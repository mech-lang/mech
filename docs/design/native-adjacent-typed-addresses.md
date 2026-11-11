# R304 — Share adjacent typed addresses

**Retain R304 with the R272 runtime.** Candidate paired elapsed changes -0.810% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R303 beats Roc on 45/81 cases and R304 beats it on 45/81 (55.56%). The separate four-actor cohort gives R304 46/81 below Roc and 3/81 below libdeflate.

The two primary cohorts change elapsed by -0.860% and -0.759%. The largest paired slowdown is +0.252% on one-stored. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Measured and published parent 6f50ac6d77fc05ef292e746fa32ae966e2c251fa. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R303 closure and four changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The complete-call timer includes scratch construction, output transfer and cleanup. The 81 pinned streams, capacities, allocator and measured compression API follow the existing qualification.

## Current proof and generated code

The cleanup pass recognizes typed I16, I32 and I64 memory operations whose address adds a live base to a scaled index. For the same base, index and element width, it normalizes wrapping constants and uses the first address plus an exact adjacent displacement. A unique unconditional predecessor chain carries the group across blocks. A join, an unrelated memory operation or another effect ends the group. The adjusted immediate must fit signed 32 bits. The substitution preserves operation order, memory types and flags, and the placement of every checked guard.

One of 347 machine bodies changes. The vector matcher goes from 13,832 to 13,840 code bytes and 784 to 752 frame bytes. The four inspected CLIF operations share the address value at displacements 0, 2, 0 and 2. ARM64 uses one loaded table base and one shared scaled index. Two base-plus-two adjustments survive instruction selection.

| Linked adjacent access span | Instructions | Table-base stack loads | Access order |
|---|---:|---:|---|
| R303 | 20 | 4 | read, read, write, write |
| R304 | 15 | 1 | read, read, write, write |

The local linked Roc span uses 12 instructions and shares a scaled base. The local libdeflate span uses 5 instructions with one effective address, an immediate displacement and interleaved reads and writes. These spans include different surrounding value calculations. The independent safe Rust slice witness also uses a single computed address and retains the read-read-write-write order.

The 27-case screen changes paired elapsed -1.069% and passes both timing gates on all 27. Random 1 MiB changes -0.954% and eight literals changes -0.205%.

## Headline matched ratios

| Input | R303 / Roc | R304 / Roc | R303 / libdeflate | R304 / libdeflate | R304 / R303 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.194810× | 0.193882× | 2.499098× | 2.487201× | +0.014% |
| prefix-random-8192 | 0.820479× | 0.812653× | 2.918651× | 2.890812× | -0.299% |
| prefix-natural-4096 | 1.001705× | 0.996317× | 1.719334× | 1.710086× | -0.856% |
| prefix-natural-32768 | 1.373552× | 1.369323× | 1.251734× | 1.247880× | -0.185% |
| prefix-natural-1048576 | 1.403966× | 1.404636× | 1.132615× | 1.133156× | +0.032% |
| prefix-random-1048576 | 1.713010× | 1.700575× | 2.422950× | 2.405362× | -0.866% |
| prefix-periodic-1048576 | 0.751401× | 0.740975× | 1.274684× | 1.256997× | -1.405% |
| prefix-repeated-1048576 | 0.752914× | 0.742142× | 1.184204× | 1.167260× | -1.087% |

The four-actor cohort contains 245,664 timed calls. Reference ratios use its same-case balanced blocks. Elapsed changes and the primary parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 original and decoded calls produce the expected bytes and independently inflate. The two forms have identical Objects and metadata. Capacities and held aliases agree, and both workers release all private payload and owners.
- The focused JIT test executes 150 compiled calls across six shapes. It checks adjacent addresses, displaced bases, wrapping indices, a different pointer, a distant offset, an intervening effect and a joined path. All 49 cleanup tests pass with warnings as errors.
- The payload request count, requested bytes, copy and zeroing counters, peak private payload and residual private bytes agree across all 81 cases.

| Input | R303 instructions | R304 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 772,992.5 | 772,972.5 | -0.003% | -2.459% |
| prefix-random-8192 | 5,820,731.5 | 5,721,687.0 | -1.702% | -1.416% |
| prefix-natural-4096 | 9,662,029.5 | 9,603,818.0 | -0.602% | -0.770% |
| prefix-natural-32768 | 80,660,662.0 | 80,071,323.0 | -0.731% | -0.044% |
| prefix-natural-1048576 | 2,936,183,909.0 | 2,912,500,641.0 | -0.807% | -0.219% |
| prefix-random-1048576 | 655,478,451.0 | 641,384,717.0 | -2.150% | -0.932% |
| prefix-periodic-1048576 | 525,256,581.5 | 517,901,907.5 | -1.400% | -1.375% |
| prefix-repeated-1048576 | 533,180,015.5 | 525,855,401.0 | -1.374% | -1.356% |

Timing directions differ between cohorts on 12 cases. The packet retains both observations.

## Exact executable profiles

Running CPU samples include validation and the harness. Each range uses the corresponding linked image addresses. Retired instructions and cycles use a separate counter cohort.

| Input | Running samples before / after | Vector matcher before / after | Adjacent accesses before / after | Matcher stack-load samples before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5410 / 5461 | 696 / 646 | 243 / 227 | 16 / 13 |
| prefix-natural-1048576 | 5509 / 5491 | 2376 / 2375 | 28 / 9 | 698 / 656 |
| prefix-natural-32768 | 5368 / 5422 | 2089 / 2081 | 40 / 20 | 587 / 598 |
| prefix-natural-4096 | 5605 / 5566 | 1679 / 1673 | 191 / 144 | 517 / 479 |
| prefix-periodic-1048576 | 5513 / 5386 | 3667 / 3477 | 10 / 7 | 1968 / 1841 |
| prefix-random-1048576 | 5513 / 5412 | 3445 / 3399 | 91 / 63 | 1013 / 1363 |
| prefix-repeated-1048576 | 5509 / 5485 | 3336 / 3298 | 0 / 1 | 1845 / 1767 |

The literal tally accounts for 1042/5412 samples on 1 MiB random input. The closest primary candidate above parity is prefix-natural-4096 at 1.009321× Roc. Audit literal-tally entry geometry and its repeated indexed update on random input. The source masks the literal ordinal to eight bits. Test whether a current frequency-table bound can authorize the whole literal recurrence while preserving the checked alternate path.

## Brief coverage

- R275 measured shifted equality reuse, and R303 forwarded the proof into the long continuation. [R275](native-shifted-immutable-equality.md) · [R303](native-forwarded-equality-end.md)
- R274 carried the selected item through the reduction. R276 measured a singleton dispatch and retained the faster first-visit peel. [R274](native-capacity-phase-selected-item.md) · [R276](native-short-reduction-first-visit.md)
- R277 measured the full offset-slot map at −0.320% across all 81. The retained benchmark uses its original condensed map. [R277](native-dense-offset-slot-experiment.md)
- R278 introduced the checked empty run and R291 combined it with the completed reduction handoff. R304 measures the random-data matcher path. [R278](native-checked-append-run.md) · [R291](native-empty-and-handoff.md)
- R282 measured guarded prefetch and kept the earlier baseline after a block timing miss. Write-before-read scratch proof and bounded widening reductions are open. [R282](native-private-write-prefetch.md) · [R273](native-readonly-scalar-reductions.md)

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R303 / Roc | R304 / Roc | R304 / R303 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.090829× | 2.079469× | 0.994567× | pass/pass |
| one-stored | 1 | 1.985701× | 1.990702× | 1.002519× | pass/pass |
| seven-stored-prefix | 7 | 2.215773× | 2.211001× | 0.997846× | pass/pass |
| eight-literals | 8 | 0.193221× | 0.193248× | 1.000141× | pass/pass |
| eight-overlap | 8 | 0.184439× | 0.183709× | 0.996046× | pass/pass |
| 320-overlap | 320 | 0.211635× | 0.210909× | 0.996570× | pass/pass |
| eight-literals-prefix | 8 | 0.195195× | 0.195036× | 0.999186× | pass/pass |
| 128-biased-dynamic | 128 | 0.256296× | 0.255852× | 0.998264× | pass/pass |
| 32-high-entropy-stored | 32 | 0.261095× | 0.260397× | 0.997326× | pass/pass |
| 16384-distribution-change | 16384 | 1.437715× | 1.434138× | 0.997512× | pass/pass |
| 33027-periodic-window | 33027 | 0.674999× | 0.664079× | 0.983822× | pass/pass |
| random-64 | 64 | 0.274455× | 0.273609× | 0.996918× | pass/pass |
| random-256 | 256 | 0.328302× | 0.327368× | 0.997155× | pass/pass |
| random-1024 | 1024 | 0.389650× | 0.387915× | 0.995547× | pass/pass |
| repeated-1024 | 1024 | 0.254584× | 0.253434× | 0.995482× | pass/pass |
| random-32769 | 32769 | 1.382006× | 1.360882× | 0.984715× | pass/pass |
| repeated-32769 | 32769 | 0.665140× | 0.654868× | 0.984558× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.499028× | 0.498387× | 0.998717× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.326418× | 1.321381× | 0.996202× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.373861× | 1.372207× | 0.998796× | pass/pass |
| heldout-window-32767 | 32767 | 1.372662× | 1.371080× | 0.998848× | pass/pass |
| heldout-window-32768 | 32768 | 1.372968× | 1.371099× | 0.998639× | pass/pass |
| heldout-slides-65537 | 65537 | 1.407217× | 1.404411× | 0.998005× | pass/pass |
| heldout-random-65537 | 65537 | 1.529973× | 1.508308× | 0.985840× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.371271× | 1.365742× | 0.995968× | pass/pass |
| prefix-natural-1024 | 1024 | 0.496238× | 0.495277× | 0.998063× | pass/pass |
| prefix-natural-4096 | 4096 | 1.018038× | 1.009321× | 0.991438× | pass/pass |
| prefix-natural-8192 | 8192 | 1.218300× | 1.211038× | 0.994039× | pass/pass |
| prefix-natural-16384 | 16384 | 1.326606× | 1.322461× | 0.996876× | pass/pass |
| prefix-natural-32767 | 32767 | 1.369762× | 1.367346× | 0.998236× | pass/pass |
| prefix-natural-32768 | 32768 | 1.374235× | 1.371693× | 0.998150× | pass/pass |
| prefix-natural-32769 | 32769 | 1.371712× | 1.369054× | 0.998062× | pass/pass |
| prefix-natural-33027 | 33027 | 1.370177× | 1.369530× | 0.999528× | pass/pass |
| prefix-natural-65535 | 65535 | 1.380108× | 1.374519× | 0.995950× | pass/pass |
| prefix-natural-65536 | 65536 | 1.374943× | 1.371288× | 0.997342× | pass/pass |
| prefix-natural-65537 | 65537 | 1.370157× | 1.366499× | 0.997330× | pass/pass |
| prefix-natural-131072 | 131072 | 1.382523× | 1.382118× | 0.999707× | pass/pass |
| prefix-natural-262144 | 262144 | 1.387952× | 1.387803× | 0.999892× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.405851× | 1.406306× | 1.000324× | pass/pass |
| prefix-random-1024 | 1024 | 0.392263× | 0.390552× | 0.995638× | pass/pass |
| prefix-random-4096 | 4096 | 0.646258× | 0.638962× | 0.988711× | pass/pass |
| prefix-random-8192 | 8192 | 0.822245× | 0.819785× | 0.997008× | pass/pass |
| prefix-random-16384 | 16384 | 1.129065× | 1.115931× | 0.988368× | pass/pass |
| prefix-random-32767 | 32767 | 1.379589× | 1.361367× | 0.986792× | pass/pass |
| prefix-random-32768 | 32768 | 1.376912× | 1.359911× | 0.987653× | pass/pass |
| prefix-random-32769 | 32769 | 1.373219× | 1.356449× | 0.987788× | pass/pass |
| prefix-random-33027 | 33027 | 1.385221× | 1.365195× | 0.985543× | pass/pass |
| prefix-random-65535 | 65535 | 1.556920× | 1.536213× | 0.986700× | pass/pass |
| prefix-random-65536 | 65536 | 1.553728× | 1.534991× | 0.987940× | pass/pass |
| prefix-random-65537 | 65537 | 1.550800× | 1.533074× | 0.988570× | pass/pass |
| prefix-random-131072 | 131072 | 1.646988× | 1.629105× | 0.989142× | pass/pass |
| prefix-random-262144 | 262144 | 1.692632× | 1.673170× | 0.988502× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.708201× | 1.693402× | 0.991337× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.317592× | 0.315898× | 0.994666× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.430924× | 0.429946× | 0.997730× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.504692× | 0.501233× | 0.993148× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.589663× | 0.580548× | 0.984542× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.655456× | 0.646137× | 0.985782× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.661657× | 0.651515× | 0.984672× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.665749× | 0.656259× | 0.985745× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.674640× | 0.663343× | 0.983256× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.708223× | 0.696078× | 0.982851× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.710453× | 0.700615× | 0.986153× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.711261× | 0.699168× | 0.982997× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.738119× | 0.726508× | 0.984269× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.747696× | 0.736630× | 0.985200× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.751307× | 0.740750× | 0.985948× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.251857× | 0.251160× | 0.997230× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.386427× | 0.381557× | 0.987398× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.467652× | 0.463699× | 0.991548× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.560614× | 0.554330× | 0.988790× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.634173× | 0.625308× | 0.986020× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.635744× | 0.624991× | 0.983086× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.656308× | 0.650416× | 0.991022× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.684019× | 0.672597× | 0.983301× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.709454× | 0.697921× | 0.983744× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.708760× | 0.698397× | 0.985379× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.707452× | 0.697415× | 0.985812× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.740657× | 0.730779× | 0.986663× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.752075× | 0.741367× | 0.985762× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.749529× | 0.741380× | 0.989128× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r304-adjacent-table-addresses-6860652179ec.tar.gz). SHA-256 6860652179ec93fc949705ef55a40e60d99c1c2c8366cb6f6ebb67ced0169cdf.

The first verification of the downloaded archive ran out of local disk space. Reclaiming 294,774,936 bytes of reproducible R303 build outputs allowed the same downloaded files to replay successfully. All recorded measurement blocks were used as collected.
