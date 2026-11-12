# Runtime comparison with the exact retained Object

Both Mech workers link the exact R355 generated Object and mimalloc and use the same emitted descriptor. The hybrid replaces the linked worker and runtime closure. Link placement can also contribute to the measured difference.

The 81-case geometric mean is **1.011750× R355**. The hybrid has 69 slower and 12 faster point estimates. 3 cases exceed 1.05× R355. The best hybrid result is prefix-repeated-262144 at 0.988384× R355.

| Actor | Faster than Roc | Faster than libdeflate | Faster than both |
| --- | ---: | ---: | ---: |
| R355 | 47/81 (58.02%) | 8/81 (9.88%) | 0/81 (0.00%) |
| Hybrid | 47/81 (58.02%) | 7/81 (8.64%) | 0/81 (0.00%) |

Roc parity is unchanged case by case. prefix-natural-65537 crossed above libdeflate parity.

## Largest adverse point estimates

| Input | Hybrid/R355 | R355 ns | Hybrid ns |
| --- | ---: | ---: | ---: |
| seven-stored-prefix | 1.156521× | 83.0 | 83.0 |
| empty | 1.147678× | 83.0 | 83.0 |
| one-stored | 1.127991× | 83.0 | 83.0 |
| prefix-natural-32767 | 1.046374× | 5,683,666.0 | 5,674,354.0 |
| prefix-periodic-8192 | 1.023964× | 245,479.5 | 250,083.0 |
| random-32769 | 1.018647× | 1,290,146.0 | 1,300,416.5 |
| prefix-periodic-32767 | 1.018071× | 694,396.0 | 706,271.0 |
| eight-literals-prefix | 1.017718× | 52,417.0 | 53,292.0 |
| 128-biased-dynamic | 1.017269× | 70,167.0 | 70,834.0 |
| prefix-periodic-65535 | 1.017148× | 1,338,708.5 | 1,361,292.0 |

The ratio column uses paired block means. The last two columns show pooled medians.
The archived result and raw packets retain all 81 cases.

## Headline cases

| Input | R355/Roc | Hybrid/Roc | R355/libdeflate | Hybrid/libdeflate | Hybrid/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.182827× | 0.185735× | 2.306067× | 2.342737× | 1.015902× |
| reymont-prefix-32769 | 1.188488× | 1.182799× | 1.090262× | 1.085044× | 0.995213× |
| prefix-natural-32768 | 1.192611× | 1.192707× | 1.084607× | 1.084694× | 1.000081× |
| prefix-natural-131072 | 1.197609× | 1.199391× | 0.986157× | 0.987625× | 1.001488× |
| prefix-natural-262144 | 1.190443× | 1.194289× | 0.973806× | 0.976952× | 1.003231× |
| prefix-natural-1048576 | 1.212719× | 1.215345× | 0.977673× | 0.979790× | 1.002165× |
| prefix-random-1048576 | 1.491031× | 1.494054× | 2.109486× | 2.113762× | 1.002027× |
| prefix-periodic-1048576 | 0.637937× | 0.647421× | 1.080527× | 1.096591× | 1.014867× |
| prefix-repeated-1048576 | 0.658100× | 0.653977× | 1.026426× | 1.019995× | 0.993735× |

## Scope and qualification

The matched four-actor cohort ran 245,664 measured calls in 24 rotated orders per case. It preserves 6,767 zero timer samples; all block means used for ratios are positive. The 162 hybrid linked smoke calls passed exact output, independent inflation, capacity, alias and cleanup checks.

Timing uses the compact diagnostic entry and includes scratch construction, final ownership transfer and private cleanup. Warmup and validation stay outside the timer. The case shifts are descriptive point estimates from one matched cohort. The result does not establish recovery or replace the retained R355 baseline.

R355 Object SHA-256 `803a2d556384a12f3732b17953152e54982b3f2f23be36e55cd04f5d909a4094`. Shared descriptor SHA-256 `ec3bd0369218c1905806b52701342ef5356e0248466bf85277f88ff3e8103a29`. Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.

## Residual stored-input overhead

The linked wrapper copies a 48-byte output view to an out-of-line `take_u8` call. The retained worker inlines that transfer. The hybrid also goes through the Rust allocator shim for payload and owner allocation and copies more heap state during explicit destruction. The exact same generated compression body runs in both workers. These instruction differences are the next isolated runtime witnesses.

[Raw observations and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r355-same-object-runtime-a6a803ae058e.tar.gz).
