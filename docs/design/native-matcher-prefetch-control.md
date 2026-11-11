# R281 — Isolate the reference matcher’s prefetch hints

The exact Apple ARM64 libdeflate 1.26 worker was compared against a locally signed copy with its four near-optimal matcher prefetch hints replaced by equal-size no-ops. The function addresses and other instructions stay fixed. Both workers retain the original allocator, hash and tree operations, source code, level-12 input fixtures and full-call timer. The address calculations before each hint remain in both executables.

All 81 cases produce their pinned C streams and capacities in 162 complete smoke calls with independent inflation and held input/output aliases. The separate 11-case screen records 20,208 timed calls in 24 balanced paired blocks per case. Both workers use the existing 200 ms warmup, and output verification remains outside the timer.

| Input | C without hints / original C | Original first | Control first |
|---|---:|---:|---:|
| eight-literals | 1.000765× | 0.997828× | 1.003711× |
| prefix-natural-4096 | 0.999433× | 1.015057× | 0.984049× |
| prefix-natural-32768 | 1.009532× | 1.009501× | 1.009563× |
| prefix-natural-1048576 | 1.009383× | 1.009805× | 1.008961× |
| prefix-random-8192 | 1.002256× | 1.002876× | 1.001636× |
| prefix-random-32768 | 1.029100× | 1.031855× | 1.026353× |
| prefix-random-1048576 | 1.054166× | 1.061220× | 1.047159× |
| prefix-periodic-32768 | 0.998723× | 0.997300× | 1.000149× |
| prefix-periodic-1048576 | 0.999861× | 1.004801× | 0.994945× |
| prefix-repeated-33027 | 1.001646× | 1.002340× | 1.000953× |
| prefix-repeated-1048576 | 0.999702× | 1.004583× | 0.994845× |

Removing the hints changes elapsed by +2.91% on 32 KiB random input and +5.42% on 1 MiB random input. Both order halves favor the hints for those cases. The 4 KiB natural-text frontier and large periodic and repeated cases are close to unchanged. The first Mech pilot will target a future indexed write whose address is already proved in bounds in the same live private view. It will keep hint address generation, normal accesses, proof failures and the full recurring register allocation visible in the linked binary.

Ratios above 1× favor the original prefetch instructions. The order columns reconstruct both halves of the observed blocks. The screen covers the pinned library’s implementation. Mech would need its own eligibility, address-safety and layout checks before any adoption.

R280 remains the Mech compiler baseline. Its exact linked matcher shows repeated stack loads around consecutive input-window checks and around tree writes. Both the prefetch screen and that machine evidence are relevant to choosing the next matcher experiment. [R280 qualification and profiles](native-masked-wrapping-arithmetic.md).

[Verified control and paired observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r281-matcher-prefetch-control-4d5838f78e92.tar.gz). SHA-256 4d5838f78e92473c6d63d98d96ec9a7d228ce3021df1d75ee33f8777aba844f0.
