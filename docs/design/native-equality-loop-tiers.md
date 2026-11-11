# R270 — Keep equality comparison widths in separate loops

**Retain R270.** Paired elapsed changes -0.73680% across all 81 cases. The score moves from 26/81 to 27/81 (33.33%). Repeated 33,027 bytes crosses from 1.015880× to 0.992047× Roc. The retained cases below parity all remain below parity in this comparison. Periodic 32 KiB is the closest remaining case at 1.002207×.

PR base 92f75db84e2523693648084bea16cd52ad60777a. Published parent 962a8273a6c4fb42e7439746be0e674fb101c016. Measured R268 parent 1e6459bee97ef35b55794c95df9fb72cbf455bb2. Apple ARM64, Cranelift 0.131.3 and regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, tables, bounds, search limits, allocation sites and output ownership are pinned. The compact call covers compression, scratch construction, output transfer and private cleanup. The additional ordinary managed admission, accounting and recovery contract is outside that timing boundary.

## Compiler change and native code

The existing certificate admits two immutable readable byte windows, a pure equality loop and one current induction value. The initial edge may enter the 64-byte tier. Its continuing edge tests the next complete group directly. When less than 64 bytes remain, the traversal enters the 16-byte tier, then the original eight-byte and byte tail. A mismatch restarts scalar comparison at the original group position. The scalar latch stays in that loop because the first differing word is in the rejected group. Each advance is no greater than the admitted limit. The positive-length guard supplies the invariant last-readable position for each width. Access widths and flags, selected bounds, first mismatch, error order and source results remain intact.

On the selected linked matching path, a continuing 64-byte group changes 29 to 28 instructions, and a continuing 16-byte group changes 13 to 10. Each audited group stays free of stack transfers. The emitted latch still reconstructs the last-readable bound, which the small C and Rust controls compute once outside their loops. The vector matcher grows from 41,576 to 41,780 code bytes and its frame falls from 960 to 928 bytes. The general matcher grows from 51,868 to 52,028 bytes and its frame falls from 960 to 944 bytes. The original scalar mismatch and sub-vector tail use the same source word and byte operations.

## Full acceptance

Both 1.05 gates pass 81/81. Primary A changes -0.43558% and B -1.03710%. The matching screen was 19 cases and both gates passed on every input. The primary cohorts use 24 blocks each with six balanced actor orders, matched 200 ms warmup and within-block geometric ratios. The independent CPU and heap cohorts complete all 81. All measured Mech payload allocation, requested bytes, private peak, initialized copy, zeroing and cleanup families match the parent.

| Case | R268 / Roc | R270 / Roc | Paired elapsed change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.426039× | 0.426594× | +0.130% | +0.001% | -1.502% |
| prefix-repeated-32767 | 0.995901× | 0.973148× | -2.285% | -1.420% | -2.485% |
| prefix-repeated-32768 | 0.995036× | 0.973240× | -2.190% | -1.419% | -2.831% |
| prefix-repeated-32769 | 0.999921× | 0.974652× | -2.527% | -1.405% | -2.281% |
| prefix-repeated-33027 | 1.015880× | 0.992047× | -2.346% | -1.381% | -2.742% |
| prefix-periodic-32768 | 1.033342× | 1.002207× | -3.013% | -1.415% | -2.064% |
| prefix-periodic-32767 | 1.030812× | 1.008713× | -2.144% | -1.415% | -3.185% |
| prefix-natural-32768 | 1.683321× | 1.696043× | +0.756% | +0.183% | -0.061% |
| prefix-natural-1048576 | 1.750283× | 1.764205× | +0.795% | +0.205% | +0.061% |
| prefix-random-1048576 | 2.102051× | 2.114769× | +0.605% | +0.753% | +0.363% |

## Correctness and profiles

- Warnings-as-errors build and 728 native tests pass, with seven inherited exclusions.
- The four focused equality tests pass, including first mismatch and the original scalar backedge.
- Original and decoded full Objects and metadata match byte for byte. All 324 full smokes preserve exact reference bytes, independent inflate, input and output aliases, capacity and cleanup.
- There are 5,788 matching controls across original and decoded forms, with retained resident oracles. The existing 98 linked path, growth, failure, publication, retry and shared-owner controls pass across both forms.
- Eight exact candidate-image profiles complete after the primary and counter cohorts. The repeated 33,027-byte matcher takes 3,178 of 5,595 timer samples and its no-growth parser takes 1,005.
- The full linked/AOT suite, multi-megabyte inputs and hosted CI remain outside this local receipt.

The separate four-actor libdeflate comparison includes all 81 cases and identical streams. The independent C profile and linked code expose the scratch initialization difference on eight literals and the register-carried cost on repeated inputs. [Mech ratios to libdeflate and Roc, source counts and machine comparison](native-libdeflate-comparison.md).

## All 81 paired timing cases

| Case | Bytes | R268 / Roc | R270 / Roc | R270 / R268 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.109414× | 2.121494× | 1.005727× | pass/pass |
| one-stored | 1 | 1.970415× | 1.973469× | 1.001550× | pass/pass |
| seven-stored-prefix | 7 | 2.253644× | 2.267674× | 1.006226× | pass/pass |
| eight-literals | 8 | 0.426039× | 0.426594× | 1.001303× | pass/pass |
| eight-overlap | 8 | 0.432584× | 0.428794× | 0.991238× | pass/pass |
| 320-overlap | 320 | 0.446807× | 0.447097× | 1.000650× | pass/pass |
| eight-literals-prefix | 8 | 0.421521× | 0.419817× | 0.995957× | pass/pass |
| 128-biased-dynamic | 128 | 0.420318× | 0.419014× | 0.996899× | pass/pass |
| 32-high-entropy-stored | 32 | 0.430517× | 0.425989× | 0.989482× | pass/pass |
| 16384-distribution-change | 16384 | 1.734930× | 1.725974× | 0.994838× | pass/pass |
| 33027-periodic-window | 33027 | 1.037000× | 1.010613× | 0.974555× | pass/pass |
| random-64 | 64 | 0.445965× | 0.445772× | 0.999566× | pass/pass |
| random-256 | 256 | 0.487429× | 0.486243× | 0.997566× | pass/pass |
| random-1024 | 1024 | 0.541977× | 0.544608× | 1.004855× | pass/pass |
| repeated-1024 | 1024 | 0.520759× | 0.511955× | 0.983094× | pass/pass |
| random-32769 | 32769 | 1.762657× | 1.749355× | 0.992453× | pass/pass |
| repeated-32769 | 32769 | 0.999318× | 0.974289× | 0.974954× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.654065× | 0.662586× | 1.013029× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.612874× | 1.619983× | 1.004408× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.679008× | 1.685744× | 1.004012× | pass/pass |
| heldout-window-32767 | 32767 | 1.682790× | 1.691183× | 1.004988× | pass/pass |
| heldout-window-32768 | 32768 | 1.680373× | 1.689431× | 1.005390× | pass/pass |
| heldout-slides-65537 | 65537 | 1.760992× | 1.768700× | 1.004377× | pass/pass |
| heldout-random-65537 | 65537 | 1.920075× | 1.914866× | 0.997287× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.761695× | 1.761107× | 0.999667× | pass/pass |
| prefix-natural-1024 | 1024 | 0.657412× | 0.655760× | 0.997487× | pass/pass |
| prefix-natural-4096 | 4096 | 1.237186× | 1.222959× | 0.988501× | pass/pass |
| prefix-natural-8192 | 8192 | 1.446042× | 1.433519× | 0.991340× | pass/pass |
| prefix-natural-16384 | 16384 | 1.618431× | 1.622594× | 1.002573× | pass/pass |
| prefix-natural-32767 | 32767 | 1.683335× | 1.693691× | 1.006152× | pass/pass |
| prefix-natural-32768 | 32768 | 1.683321× | 1.696043× | 1.007557× | pass/pass |
| prefix-natural-32769 | 32769 | 1.681450× | 1.691087× | 1.005731× | pass/pass |
| prefix-natural-33027 | 33027 | 1.682692× | 1.686147× | 1.002053× | pass/pass |
| prefix-natural-65535 | 65535 | 1.714856× | 1.725693× | 1.006319× | pass/pass |
| prefix-natural-65536 | 65536 | 1.712977× | 1.727238× | 1.008325× | pass/pass |
| prefix-natural-65537 | 65537 | 1.713606× | 1.756481× | 1.025020× | pass/pass |
| prefix-natural-131072 | 131072 | 1.721483× | 1.737255× | 1.009162× | pass/pass |
| prefix-natural-262144 | 262144 | 1.735458× | 1.748615× | 1.007582× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.750283× | 1.764205× | 1.007955× | pass/pass |
| prefix-random-1024 | 1024 | 0.554619× | 0.558281× | 1.006603× | pass/pass |
| prefix-random-4096 | 4096 | 0.933380× | 0.933972× | 1.000635× | pass/pass |
| prefix-random-8192 | 8192 | 1.147094× | 1.134303× | 0.988849× | pass/pass |
| prefix-random-16384 | 16384 | 1.500790× | 1.492745× | 0.994640× | pass/pass |
| prefix-random-32767 | 32767 | 1.764995× | 1.755899× | 0.994846× | pass/pass |
| prefix-random-32768 | 32768 | 1.764100× | 1.756334× | 0.995598× | pass/pass |
| prefix-random-32769 | 32769 | 1.758343× | 1.765868× | 1.004279× | pass/pass |
| prefix-random-33027 | 33027 | 1.765674× | 1.780156× | 1.008202× | pass/pass |
| prefix-random-65535 | 65535 | 1.935169× | 1.935412× | 1.000126× | pass/pass |
| prefix-random-65536 | 65536 | 1.939397× | 1.946470× | 1.003647× | pass/pass |
| prefix-random-65537 | 65537 | 1.933999× | 1.932716× | 0.999337× | pass/pass |
| prefix-random-131072 | 131072 | 2.027135× | 2.026599× | 0.999736× | pass/pass |
| prefix-random-262144 | 262144 | 2.072159× | 2.070432× | 0.999166× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.102051× | 2.114769× | 1.006050× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.522771× | 0.524656× | 1.003606× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.735757× | 0.719413× | 0.977786× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.830266× | 0.813964× | 0.980365× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.935408× | 0.913487× | 0.976565× | pass/pass |
| prefix-periodic-32767 | 32767 | 1.030812× | 1.008713× | 0.978561× | pass/pass |
| prefix-periodic-32768 | 32768 | 1.033342× | 1.002207× | 0.969869× | pass/pass |
| prefix-periodic-32769 | 32769 | 1.040292× | 1.020556× | 0.981028× | pass/pass |
| prefix-periodic-33027 | 33027 | 1.037988× | 1.010329× | 0.973354× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.103554× | 1.073551× | 0.972812× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.104586× | 1.074817× | 0.973050× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.102831× | 1.076073× | 0.975737× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.142080× | 1.116883× | 0.977938× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.166816× | 1.140872× | 0.977766× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.180210× | 1.154409× | 0.978139× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.516827× | 0.517574× | 1.001446× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.721071× | 0.704035× | 0.976374× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.816781× | 0.796523× | 0.975198× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.905592× | 0.882756× | 0.974783× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.995901× | 0.973148× | 0.977154× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.995036× | 0.973240× | 0.978095× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.999921× | 0.974652× | 0.974729× | pass/pass |
| prefix-repeated-33027 | 33027 | 1.015880× | 0.992047× | 0.976540× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.072680× | 1.047776× | 0.976784× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.072757× | 1.048936× | 0.977794× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.071602× | 1.046255× | 0.976347× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.116737× | 1.092761× | 0.978530× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.133196× | 1.109526× | 0.979112× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.138972× | 1.115310× | 0.979225× | pass/pass |

[Verified source and full measurements](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r270-equality-tiers-and-library-comparison-11d6708c6ba7.tar.gz). SHA-256 11d6708c6ba7497587cab1ab6cab518e0a80d9900c535ba07c9d07272f8f26d1.
