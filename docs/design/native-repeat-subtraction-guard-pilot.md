# Recheck duplicate subtraction guards at fixed addresses

The 18-case trial takes **1.002611×** paired control. Both 1.05 gates pass. Its matched Roc win count stays 4/18 and libdeflate falls from nine to eight. Retain the previous compiler.

| Input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.175640× → 0.175920× | 2.256954× → 2.260556× | 1.001596× |
| Natural 8 KiB | 0.994262× → 0.997392× | 1.232524× → 1.236403× | 1.003148× |
| Natural 32 KiB | 1.099709× → 1.103766× | 0.997900× → 1.001581× | 1.003689× |
| Natural 32,769 bytes | 1.101000× → 1.103083× | 0.999140× → 1.001030× | 1.001891× |
| Reymont prefix, 32,769 bytes | 1.102772× → 1.101257× | 1.000527× → 0.999153× | 0.998627× |
| Held-out window, 32,767 bytes | 1.098790× → 1.105374× | 1.001771× → 1.007773× | 1.005992× |
| Held-out mixed, 131,073 bytes | 1.010384× → 1.013622× | 1.173645× → 1.177406× | 1.003205× |
| Held-out random, 65,537 bytes | 1.019045× → 1.025364× | 1.725131× → 1.735827× | 1.006200× |

This is a restricted retry of the [earlier complementary fact experiment](native-current-complement-facts.md), under the new branch-normalized allocation. On each of two false unsigned-less-than edges the same current SSA operands feed the unsigned-greater-or-equal subtraction guard. The intervening corridor contains only single-entry jumps. The diagnostic removes that redundant check and preserves the original zero arm, original subtraction and all unrelated error routes. Only one relocation-free matcher body changes, from 10,172 to 10,136 bytes. It retains the 576-byte frame and all 1,348 linked text addresses.

The 7,776 timed calls preserve the same six payload distributions, source and runtime input, revision-14 bytecode, level, allocator, buffers and timer. All 162 linked checks pass with exact reference bytes, independent inflate, capacity, alias and cleanup checks. The separate CPU cohort retires 0.996440× and 0.995554× instructions on natural 32 KiB and 256 KiB. On held-out random it retires 1.000703× instructions and takes 1.006065× cycles. The separate eight-literal cycle result is 1.079335× while instructions are 1.000025×. The exact worker's matcher accounts for 1,152/4,334 running timer PCs on natural 8 KiB, 1,963/5,364 on natural 256 KiB, 2,194/5,347 on slides and 3,565/5,267 on held-out random. These samples include validation. Ordinary full emission and all-81 timing were skipped for this rejected candidate.

Measured base `6e7dd1bed4614034887d11b574251fa06cc0dff5`; timed worker SHA-256 `f004775f6f80dd3e209c4bd12c60087ad100a572058e369f5fc678d62e5d08f3`.

[Matched observations, hardware counters, profile and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-6e7dd1bed4-repeated-subtract-guards.tar.gz). SHA-256 `854116fa440c8fd168f8f00f6fea6a49b0525978b5cbe2cc8a731bf30f222431`; 80 files verified after public download.
