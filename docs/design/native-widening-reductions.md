# R308 — Widen unsigned dot products in native code

**Retain R308 with the R272 runtime.** Paired elapsed changes -1.170% across 81 fixed level-12 cases. The pooled and block timing gates pass 81/81 and 81/81. R304 beats Roc on 45/81 in the fresh primary cohorts. R308 beats Roc on 46/81 (56.79%). The separate four-way cohort gives R308 3/81 below libdeflate.

The two primary cohort changes are -1.237% and -1.103%. The highest paired ratio is +2.000% on seven-stored-prefix. Crossings below parity include prefix-natural-4096. Crossings above include none.

Pooled gate misses include none. Block gate misses include none.

Measured parent 9283b56817d59042eb73420d0e05a3bc9ec2216f. Published parent d09e3a4b78126b888e241599752b13a514e089d7. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3, with the R304 closure and three changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, tables, limits, allocator, capacities and full-call timing boundary are identical.

## Complete source proof and direct result

The compiler proves a complete active visit and an exhausted visit over two read-only current views. The active visit reads U32 and U8 at the same logical cursor, widens both, adds their product to the U64 accumulator and advances by one. The exhausted visit returns the current accumulator. The proof rejects unmodeled effects and other escaping values. Two Huffman loops match the recipe, at source headers 11763, 22093.

The current views must be flat and cover the source end. At least four elements must remain. The maximum U32 × U8 product times the end bounds every nonnegative prefix, including all partial vector sums. The seed must fit alongside that bound. If any condition fails, the ordinary scalar CFG runs from the original header and preserves its checks and origins.

The caller borrows both views synchronously. Four register arguments carry their pointers and the current interval. The helper returns one sum in a register. The caller publishes the scalar and uses existing owner retirement on the bypass edge. Both shared phase packet sizes agree.

## Machine evidence

1 of 347 existing Object bodies change. The root goes from 998,904 to 979,776 code bytes and from 57,104 to 57,440 frame bytes. All 227 shared linked compact addresses agree. The new helper has 37 instructions, processes four terms per vector iteration with UMULL and UMULL2, and handles zero to three remaining terms in a scalar tail.

The linked scalar baseline retains two default-selection comparisons, two indexed-read bounds checks and three checked arithmetic branches in the active visit at header 22093. It also moves and retires views within that visit. The seven source decisions and their exact ARM64 instructions are recorded in the packet. The paired timing includes the combined effects of guard hoisting, borrowing, direct return and vector arithmetic.

The exact libdeflate Huffman routine uses MUL and MLA on four 32-bit lanes. Mech admits full-width U32 × U8 products, which may exceed 32 bits. A separately compiled safe Rust bounded witness uses 64-bit UMLAL and UMLAL2. The present Mech helper uses separate widening multiplies and additions.

The 27-case primary screen changes paired elapsed -0.323% and passes both 1.05 gates on every case. All full-case observations remain in the archive.

## Matched ratios

| Input | R304 / Roc | R308 / Roc | R304 / libdeflate | R308 / libdeflate | Paired elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.195052× | 0.183332× | 2.496556× | 2.346542× | -6.322% |
| prefix-natural-4096 | 1.003873× | 0.991145× | 1.697860× | 1.676334× | -1.078% |
| prefix-random-16384 | 1.123333× | 1.113617× | 3.060576× | 3.034103× | -0.902% |
| prefix-natural-32768 | 1.368349× | 1.367406× | 1.245028× | 1.244170× | -0.253% |
| prefix-natural-1048576 | 1.405550× | 1.406094× | 1.134229× | 1.134668× | +0.022% |
| prefix-random-1048576 | 1.696041× | 1.694171× | 2.401077× | 2.398430× | -0.130% |
| prefix-periodic-1048576 | 0.741126× | 0.740038× | 1.255028× | 1.253186× | -0.109% |
| prefix-repeated-1048576 | 0.743341× | 0.738997× | 1.169736× | 1.162900× | -0.248% |

The four-actor cohort contains 245,664 measured calls. Libdeflate also agrees on 81/81 raw streams. Peer ratios come from those balanced blocks. Paired elapsed change and the parity score use the primary cohorts.

## Separate hardware counts and exact image profiles

The six payload and heap counters agree on all 81 cases.

| Input | R304 instructions | R308 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 772,940.5 | 720,168.5 | -6.827% | -0.796% |
| prefix-natural-4096 | 9,604,816.5 | 9,534,063.0 | -0.737% | -1.236% |
| prefix-random-16384 | 11,004,495.5 | 10,929,638.5 | -0.680% | -1.453% |
| prefix-natural-32768 | 80,072,793.5 | 79,875,366.0 | -0.247% | +0.003% |
| prefix-natural-1048576 | 2,912,520,842.5 | 2,912,157,190.0 | -0.012% | +0.022% |
| prefix-random-1048576 | 641,385,019.5 | 641,116,450.0 | -0.042% | +0.502% |
| prefix-periodic-1048576 | 517,900,748.5 | 517,506,333.5 | -0.076% | -0.168% |
| prefix-repeated-1048576 | 525,876,504.5 | 525,229,908.0 | -0.123% | -0.249% |

Running samples include worker and validation activity. They describe the exact measured image.

| Input | Running samples | Widening helper | Root | Matcher |
|---|---:|---:|---:|---:|
| eight-literals | 5465 | 21 | 1552 | 705 |
| prefix-natural-1048576 | 5367 | 0 | 85 | 2325 |
| prefix-natural-32768 | 5356 | 0 | 178 | 2080 |
| prefix-natural-4096 | 5611 | 0 | 403 | 1684 |
| prefix-periodic-1048576 | 5369 | 0 | 82 | 3532 |
| prefix-random-1048576 | 5352 | 0 | 12 | 3344 |
| prefix-repeated-1048576 | 5379 | 0 | 191 | 3249 |

## Checks and recovery

- Both original and decoded images pass 324 exact complete calls with independent inflation, held aliases, original capacities and zero final private ownership. Object and metadata hashes match.
- The 45 linked controls observe both routes, full-width products, scalar tails, short and nonflat views, zero visits, overflow, lexical precedence and success after failure. Status, source origin, prior publication, and heap counts agree with the scalar build.
- The compact entry enforces source bounds, overflow and assertions. The ordinary managed resource accounting and recovery contract is outside this timing scope.

## Scratch initialization audit

The exact R304 executable already supplies the 6,002,052-byte cache through anonymous zero pages. The matchfinder tree reads original zero entries at its first hash lookup. The cache has seven checked replacement sites in the root that can read the old word before a store and call the ordinary writable helper. That helper can copy the full logical view on refusal of exclusive use.

The separately instrumented runtime supplies two initialized sentinel patterns after the same allocation. The baseline and sentinel runs make 486 linked calls across all 81 cases. They preserve the expected streams and show zero cache copies in these calls. Eight literals touch eight cache words. Natural 1 MiB touches 1,160,740 cache words. The source proof still needs to account for the old-value and copy paths before changing the initialization contract. R304 keeps its current allocation behavior.

The closest candidate case above parity is prefix-random-16384 at 1.108522× Roc. A matched scalar helper with the same entry proof can isolate the recurring check cost from widening and direct result transport.

## All primary cases

| Input | Bytes | R304 / Roc | R308 / Roc | R308 / R304 | Pooled / block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.160738× | 2.110996× | 0.976979× | pass/pass |
| one-stored | 1 | 2.040542× | 2.038123× | 0.998815× | pass/pass |
| seven-stored-prefix | 7 | 2.219915× | 2.264309× | 1.019998× | pass/pass |
| eight-literals | 8 | 0.194296× | 0.182012× | 0.936779× | pass/pass |
| eight-overlap | 8 | 0.185862× | 0.174190× | 0.937198× | pass/pass |
| 320-overlap | 320 | 0.211513× | 0.199990× | 0.945524× | pass/pass |
| eight-literals-prefix | 8 | 0.195928× | 0.183316× | 0.935631× | pass/pass |
| 128-biased-dynamic | 128 | 0.257996× | 0.247239× | 0.958305× | pass/pass |
| 32-high-entropy-stored | 32 | 0.261017× | 0.249967× | 0.957669× | pass/pass |
| 16384-distribution-change | 16384 | 1.430886× | 1.421792× | 0.993645× | pass/pass |
| 33027-periodic-window | 33027 | 0.663204× | 0.657254× | 0.991029× | pass/pass |
| random-64 | 64 | 0.273402× | 0.263201× | 0.962690× | pass/pass |
| random-256 | 256 | 0.327494× | 0.316758× | 0.967218× | pass/pass |
| random-1024 | 1024 | 0.385916× | 0.375851× | 0.973918× | pass/pass |
| repeated-1024 | 1024 | 0.254679× | 0.243638× | 0.956649× | pass/pass |
| random-32769 | 32769 | 1.351422× | 1.344186× | 0.994646× | pass/pass |
| repeated-32769 | 32769 | 0.650787× | 0.646602× | 0.993569× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.493916× | 0.481048× | 0.973948× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.319851× | 1.312953× | 0.994774× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.370716× | 1.368576× | 0.998438× | pass/pass |
| heldout-window-32767 | 32767 | 1.369765× | 1.370513× | 1.000545× | pass/pass |
| heldout-window-32768 | 32768 | 1.372244× | 1.370470× | 0.998707× | pass/pass |
| heldout-slides-65537 | 65537 | 1.404695× | 1.402248× | 0.998258× | pass/pass |
| heldout-random-65537 | 65537 | 1.513893× | 1.507789× | 0.995968× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.363131× | 1.361252× | 0.998622× | pass/pass |
| prefix-natural-1024 | 1024 | 0.495925× | 0.481619× | 0.971154× | pass/pass |
| prefix-natural-4096 | 4096 | 1.008593× | 0.997721× | 0.989221× | pass/pass |
| prefix-natural-8192 | 8192 | 1.211479× | 1.202505× | 0.992593× | pass/pass |
| prefix-natural-16384 | 16384 | 1.321755× | 1.316237× | 0.995825× | pass/pass |
| prefix-natural-32767 | 32767 | 1.365006× | 1.364702× | 0.999777× | pass/pass |
| prefix-natural-32768 | 32768 | 1.369605× | 1.366144× | 0.997473× | pass/pass |
| prefix-natural-32769 | 32769 | 1.380278× | 1.370388× | 0.992835× | pass/pass |
| prefix-natural-33027 | 33027 | 1.373200× | 1.368149× | 0.996322× | pass/pass |
| prefix-natural-65535 | 65535 | 1.376537× | 1.373748× | 0.997974× | pass/pass |
| prefix-natural-65536 | 65536 | 1.376351× | 1.375856× | 0.999640× | pass/pass |
| prefix-natural-65537 | 65537 | 1.372249× | 1.369828× | 0.998236× | pass/pass |
| prefix-natural-131072 | 131072 | 1.382898× | 1.384314× | 1.001024× | pass/pass |
| prefix-natural-262144 | 262144 | 1.386983× | 1.390855× | 1.002791× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.403848× | 1.404155× | 1.000219× | pass/pass |
| prefix-random-1024 | 1024 | 0.388880× | 0.378292× | 0.972772× | pass/pass |
| prefix-random-4096 | 4096 | 0.651126× | 0.636323× | 0.977264× | pass/pass |
| prefix-random-8192 | 8192 | 0.816493× | 0.807169× | 0.988580× | pass/pass |
| prefix-random-16384 | 16384 | 1.118608× | 1.108522× | 0.990983× | pass/pass |
| prefix-random-32767 | 32767 | 1.365273× | 1.356761× | 0.993765× | pass/pass |
| prefix-random-32768 | 32768 | 1.370025× | 1.362543× | 0.994538× | pass/pass |
| prefix-random-32769 | 32769 | 1.361232× | 1.376948× | 1.011545× | pass/pass |
| prefix-random-33027 | 33027 | 1.367913× | 1.361726× | 0.995477× | pass/pass |
| prefix-random-65535 | 65535 | 1.544743× | 1.540323× | 0.997139× | pass/pass |
| prefix-random-65536 | 65536 | 1.539001× | 1.533303× | 0.996297× | pass/pass |
| prefix-random-65537 | 65537 | 1.538135× | 1.532578× | 0.996387× | pass/pass |
| prefix-random-131072 | 131072 | 1.621982× | 1.621661× | 0.999802× | pass/pass |
| prefix-random-262144 | 262144 | 1.672763× | 1.670837× | 0.998849× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.699541× | 1.697326× | 0.998697× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.316219× | 0.306529× | 0.969355× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.430749× | 0.421952× | 0.979578× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.499820× | 0.491988× | 0.984331× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.583844× | 0.578150× | 0.990249× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.645510× | 0.643228× | 0.996466× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.650139× | 0.645470× | 0.992818× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.655167× | 0.652720× | 0.996266× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.660377× | 0.656067× | 0.993475× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.697133× | 0.695565× | 0.997751× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.695921× | 0.696087× | 1.000239× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.699010× | 0.694403× | 0.993409× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.725673× | 0.724305× | 0.998115× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.736058× | 0.734834× | 0.998338× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.741284× | 0.740477× | 0.998912× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.253717× | 0.243548× | 0.959919× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.387181× | 0.379845× | 0.981054× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.464635× | 0.458027× | 0.985778× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.554118× | 0.547590× | 0.988220× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.625826× | 0.623938× | 0.996983× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.626714× | 0.623991× | 0.995655× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.645678× | 0.642383× | 0.994896× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.667534× | 0.662073× | 0.991819× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.696802× | 0.693818× | 0.995717× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.700819× | 0.696960× | 0.994494× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.700320× | 0.696643× | 0.994750× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.732696× | 0.728449× | 0.994204× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.741468× | 0.739061× | 0.996754× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.741249× | 0.739413× | 0.997523× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r308-widening-reduction-5bb6b1a20290.tar.gz). SHA-256 5bb6b1a20290792c94ecb77e24fcf4ea755161d66aad2437c30845a16cd7958a.
