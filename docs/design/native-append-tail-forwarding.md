# R271 — Keep the previous append in a native loop register

**Retain R270.** The evaluated exact proof changes paired elapsed by -0.149% across 81 cases. In the fresh pairing, 27/81 baseline and 28/81 candidate cases run below 1× Roc. Pooled and block gates pass 81/81 and 80/81.

Cases crossing below 1× include prefix-periodic-32768. Cases crossing above 1× include none. Pooled misses were none. Block misses were seven-stored-prefix.

The measured parent and published parent are 8830db78789aee692d89583d25458bcc32f300b3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The pinned level-12 source, bytecode, algorithm, tables, logical capacities, allocation sites, mimalloc and output protocol are unchanged.

## Compiler proof and linked code

The no-growth call already checks the current writable row, sole ownership, positive initialized length, and enough actual capacity for every reached unit append. Only that body can carry the last stored U64 in SSA. It loads the initial last cell once, then updates the carried word immediately after each real store. A current-traversal fact identifies the exact one-based last index for every selected predecessor of the read. It recognizes the current length directly and an exact subtraction and addition of one. An unrelated view, uncertain index or later revision of the value declines the substitution. The source bounds handling, ordinary and growing paths, aliases, assertions and final publication remain in their existing order.

Four no-growth functions select the proof. The checked root machine bytes and 57,072-byte frame match the baseline. The inspected continuing no-match route takes 50 instructions at R270, 51 in the first guarded build and 48 in the exact build. The two recurring stack transfers and 240-byte frame remain. The recurring previous-node payload load becomes one initial load per selected phase. The static route is directly checked against the linked image. Whole-call counters include all executed paths.

## Timings and counters

Primary A changes -0.361% and B +0.063%. Both use 24 balanced blocks per input. Independent CPU and heap cohorts cover the same 81 inputs. All measured Mech allocation, requested byte, copy, zeroing, peak and private cleanup fields agree.

The initial 19-case screen missed the pooled gate for eight-literals and the block gate for seven-stored-prefix, eight-literals. The exact screen missed the pooled gate for none and the block gate for seven-stored-prefix. All screen and campaign observations remain in the packet.

For the seven-byte stored stream the pooled medians are both 83 ns. The median block mean rises from 77.9 to 85.9 ns in A and 78.6 to 85.8 ns in B. The separate CPU cohort records 6,441 median instructions on both builds. The observed elapsed regression fails the unchanged gate. Its cause remains unverified.

| Input | R270 / Roc | R271 / Roc | Paired elapsed change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.426027× | 0.415835× | -2.392% | +0.013% | -4.601% |
| seven-stored-prefix | 2.235377× | 2.446560× | +9.447% | +0.000% | -1.111% |
| prefix-repeated-33027 | 0.994085× | 0.985664× | -0.847% | -0.515% | -1.967% |
| prefix-repeated-65536 | 1.049266× | 1.045274× | -0.380% | -0.530% | -0.461% |
| prefix-periodic-32767 | 1.005398× | 1.006864× | +0.146% | -0.524% | -1.071% |
| prefix-periodic-32768 | 1.011546× | 0.996063× | -1.531% | -0.524% | -1.373% |
| prefix-periodic-32769 | 1.012291× | 1.006396× | -0.582% | -0.519% | -1.295% |
| prefix-periodic-33027 | 1.017222× | 1.008418× | -0.865% | -0.517% | -1.687% |
| prefix-natural-1048576 | 1.760395× | 1.787901× | +1.562% | -0.302% | +1.559% |
| prefix-random-1048576 | 2.119436× | 2.078712× | -1.921% | -0.842% | -1.705% |

## Five-case matched libdeflate follow-up

This fresh four-actor cohort uses all 24 actor orders, the original capacities and a newly allocated level-12 libdeflate 1.26 compressor per call. Each row preserves exact compressed bytes, independent inflate and held aliases. Each displayed ratio has Mech as its numerator. The previous 81-case library comparison remains a separate cohort. [Source, LOC and full R270 library results](native-libdeflate-comparison.md).

| Input | R270 / libdeflate | R271 / libdeflate | R270 / Roc | R271 / Roc |
|---|---:|---:|---:|---:|
| eight-literals | 5.41915× | 5.23260× | 0.42774× | 0.41302× |
| prefix-natural-1048576 | 1.42871× | 1.44577× | 1.77006× | 1.79120× |
| prefix-random-1048576 | 2.98270× | 2.95429× | 2.10776× | 2.08768× |
| prefix-periodic-32768 | 2.15498× | 2.16713× | 1.00449× | 1.01016× |
| prefix-repeated-33027 | 1.93682× | 1.93021× | 0.99852× | 0.99511× |

## Correctness and profiles

- 729 native tests pass with seven inherited exclusions. The focused test checks same-view, positive length and exact index.
- Original and decoded full Objects and descriptors are byte-identical. All 324 complete smokes preserve exact bytes, independent inflate, aliases and zero final private ownership.
- 2,894 matching calls reproduce the byte-identical R270 images and results. The same frozen resident and independent matching oracles remain in place.
- 108 linked controls cover zero and repeated visits, nonzero initial tails, actual growth, failed growth, assertions, alias fallback and later native re-entry, unchanged publication and retry. There are ten fresh resident seed checks, four default-work denials, four successful values and two lexical rejections.
- Four exact-image profiles follow the independent primary and counter cohorts. Samples include the worker and validation and represent sampled CPU time.
- The initial test link ran out of disk space. Reclaiming old build outputs allowed the unchanged test command to finish. The compact timing boundary includes scratch, compression, output transfer and cleanup. The additional ordinary managed contract and full hosted suite remain outside this receipt.

| Input | Running samples | Top linked symbol | Top symbol samples |
|---|---:|---|---:|
| eight-literals | 5509 | `external_or_unmapped` | 3509 |
| prefix-natural-1048576 | 5597 | `_mech_compact_numeric_15296_no_growth` | 3149 |
| prefix-periodic-32768 | 5348 | `_mech_compact_numeric_837_vector` | 3274 |
| prefix-repeated-33027 | 5559 | `_mech_compact_numeric_837_vector` | 3192 |

## All 81 paired timings

| Input | Bytes | R270 / Roc | R271 / Roc | R271 / R270 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.127682× | 2.129195× | 1.000711× | pass/pass |
| one-stored | 1 | 2.039603× | 2.021776× | 0.991260× | pass/pass |
| seven-stored-prefix | 7 | 2.235377× | 2.446560× | 1.094473× | pass/fail |
| eight-literals | 8 | 0.426027× | 0.415835× | 0.976076× | pass/pass |
| eight-overlap | 8 | 0.416382× | 0.425702× | 1.022384× | pass/pass |
| 320-overlap | 320 | 0.453086× | 0.450491× | 0.994273× | pass/pass |
| eight-literals-prefix | 8 | 0.420068× | 0.424366× | 1.010231× | pass/pass |
| 128-biased-dynamic | 128 | 0.414057× | 0.415353× | 1.003129× | pass/pass |
| 32-high-entropy-stored | 32 | 0.425863× | 0.430231× | 1.010257× | pass/pass |
| 16384-distribution-change | 16384 | 1.742820× | 1.737061× | 0.996696× | pass/pass |
| 33027-periodic-window | 33027 | 1.012888× | 1.002454× | 0.989699× | pass/pass |
| random-64 | 64 | 0.434639× | 0.437567× | 1.006737× | pass/pass |
| random-256 | 256 | 0.471413× | 0.481896× | 1.022236× | pass/pass |
| random-1024 | 1024 | 0.526099× | 0.529183× | 1.005861× | pass/pass |
| repeated-1024 | 1024 | 0.500465× | 0.502282× | 1.003632× | pass/pass |
| random-32769 | 32769 | 1.755158× | 1.729341× | 0.985291× | pass/pass |
| repeated-32769 | 32769 | 0.985580× | 0.969427× | 0.983610× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.648824× | 0.650352× | 1.002355× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.630729× | 1.659655× | 1.017738× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.690859× | 1.720538× | 1.017553× | pass/pass |
| heldout-window-32767 | 32767 | 1.690420× | 1.714653× | 1.014335× | pass/pass |
| heldout-window-32768 | 32768 | 1.692936× | 1.711595× | 1.011021× | pass/pass |
| heldout-slides-65537 | 65537 | 1.773315× | 1.799509× | 1.014771× | pass/pass |
| heldout-random-65537 | 65537 | 1.918691× | 1.879213× | 0.979425× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.758918× | 1.753308× | 0.996811× | pass/pass |
| prefix-natural-1024 | 1024 | 0.646732× | 0.650008× | 1.005065× | pass/pass |
| prefix-natural-4096 | 4096 | 1.218778× | 1.231120× | 1.010127× | pass/pass |
| prefix-natural-8192 | 8192 | 1.437099× | 1.478007× | 1.028466× | pass/pass |
| prefix-natural-16384 | 16384 | 1.622648× | 1.646026× | 1.014407× | pass/pass |
| prefix-natural-32767 | 32767 | 1.690270× | 1.715878× | 1.015150× | pass/pass |
| prefix-natural-32768 | 32768 | 1.693316× | 1.718007× | 1.014582× | pass/pass |
| prefix-natural-32769 | 32769 | 1.692223× | 1.712146× | 1.011773× | pass/pass |
| prefix-natural-33027 | 33027 | 1.693246× | 1.714005× | 1.012260× | pass/pass |
| prefix-natural-65535 | 65535 | 1.731510× | 1.759140× | 1.015957× | pass/pass |
| prefix-natural-65536 | 65536 | 1.731065× | 1.759092× | 1.016190× | pass/pass |
| prefix-natural-65537 | 65537 | 1.765967× | 1.757585× | 0.995253× | pass/pass |
| prefix-natural-131072 | 131072 | 1.738684× | 1.765148× | 1.015221× | pass/pass |
| prefix-natural-262144 | 262144 | 1.750452× | 1.776495× | 1.014878× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.760395× | 1.787901× | 1.015625× | pass/pass |
| prefix-random-1024 | 1024 | 0.530827× | 0.535079× | 1.008011× | pass/pass |
| prefix-random-4096 | 4096 | 0.926523× | 0.899423× | 0.970751× | pass/pass |
| prefix-random-8192 | 8192 | 1.159828× | 1.127939× | 0.972505× | pass/pass |
| prefix-random-16384 | 16384 | 1.501808× | 1.468545× | 0.977852× | pass/pass |
| prefix-random-32767 | 32767 | 1.773768× | 1.739195× | 0.980508× | pass/pass |
| prefix-random-32768 | 32768 | 1.765067× | 1.735135× | 0.983042× | pass/pass |
| prefix-random-32769 | 32769 | 1.765847× | 1.723403× | 0.975964× | pass/pass |
| prefix-random-33027 | 33027 | 1.770153× | 1.731488× | 0.978157× | pass/pass |
| prefix-random-65535 | 65535 | 1.948515× | 1.911692× | 0.981102× | pass/pass |
| prefix-random-65536 | 65536 | 1.946682× | 1.908188× | 0.980226× | pass/pass |
| prefix-random-65537 | 65537 | 1.940613× | 1.908467× | 0.983435× | pass/pass |
| prefix-random-131072 | 131072 | 2.026582× | 2.006941× | 0.990308× | pass/pass |
| prefix-random-262144 | 262144 | 2.072317× | 2.050625× | 0.989532× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.119436× | 2.078712× | 0.980785× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.520100× | 0.520744× | 1.001238× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.714982× | 0.708068× | 0.990330× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.806574× | 0.794960× | 0.985602× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.907784× | 0.895420× | 0.986380× | pass/pass |
| prefix-periodic-32767 | 32767 | 1.005398× | 1.006864× | 1.001457× | pass/pass |
| prefix-periodic-32768 | 32768 | 1.011546× | 0.996063× | 0.984693× | pass/pass |
| prefix-periodic-32769 | 32769 | 1.012291× | 1.006396× | 0.994176× | pass/pass |
| prefix-periodic-33027 | 33027 | 1.017222× | 1.008418× | 0.991345× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.077298× | 1.072975× | 0.995987× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.078358× | 1.068208× | 0.990587× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.082256× | 1.073589× | 0.991992× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.117168× | 1.110863× | 0.994357× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.137052× | 1.131001× | 0.994678× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.154750× | 1.153088× | 0.998560× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.509299× | 0.506174× | 0.993865× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.700245× | 0.683557× | 0.976169× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.806493× | 0.798533× | 0.990130× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.887222× | 0.884750× | 0.997213× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.979050× | 0.972125× | 0.992927× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.976460× | 0.972454× | 0.995897× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.980234× | 0.968794× | 0.988329× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.994085× | 0.985664× | 0.991529× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.048556× | 1.039710× | 0.991564× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.049266× | 1.045274× | 0.996195× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.049757× | 1.041071× | 0.991726× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.089448× | 1.087026× | 0.997777× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.108995× | 1.108301× | 0.999374× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.118286× | 1.117038× | 0.998884× | pass/pass |

[Verified compiler patch and receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r271-append-tail-forwarding-6d46e68498eb.tar.gz). SHA-256 6d46e68498ebfea413827efdd77a18059b86c43ba15d1e16bd2020947f1bf177.
