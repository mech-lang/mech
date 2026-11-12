# Time the combined matcher and terminal transfer

**R355 remains the retained comparison during recovery.**

The candidate beats Roc on 47/81 cases (58.02%) and libdeflate on 7/81. In the same run R355 wins 47/81 against Roc and 8/81 against libdeflate. The paired candidate-to-R355 geometric mean is 0.964195×. The archive preserves all 9 paired point estimates above 1×, 0 above 1.05×, and 6,765 zero timer observations.

| Input | R355/Roc | Candidate/Roc | R355/libdeflate | Candidate/libdeflate | Candidate/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.183469× | 0.172376× | 2.321243× | 2.180889× | 0.939535× |
| reymont-prefix-32769 | 1.187794× | 1.195370× | 1.084551× | 1.091469× | 1.006379× |
| prefix-natural-32768 | 1.175685× | 1.227246× | 1.086204× | 1.133841× | 1.043856× |
| prefix-natural-131072 | 1.196845× | 1.211535× | 0.985786× | 0.997886× | 1.012274× |
| prefix-natural-262144 | 1.197411× | 1.211514× | 0.970289× | 0.981717× | 1.011778× |
| prefix-natural-1048576 | 1.213135× | 1.218817× | 0.977829× | 0.982410× | 1.004684× |
| prefix-random-1048576 | 1.483072× | 1.414491× | 2.097430× | 2.000440× | 0.953758× |
| prefix-periodic-1048576 | 0.638075× | 0.601307× | 1.080367× | 1.018114× | 0.942378× |
| prefix-repeated-1048576 | 0.647277× | 0.614254× | 1.015991× | 0.964157× | 0.948982× |

## Direct comparison with the exact matcher executable

The seven-case attribution screen measures exact R355, the matcher executable, and the terminal relink in the same blocks. It retains 41,832 complete calls. The last two workers use the same exact generated Object.

| Input | Matcher/R355 | Relink/R355 | Relink/matcher |
| --- | ---: | ---: | ---: |
| eight-literals | 0.934916× | 0.944567× | 1.010323× |
| seven-stored-prefix | 1.127769× | 0.928865× | 0.823631× |
| prefix-natural-262144 | 1.012003× | 1.012229× | 1.000223× |
| prefix-periodic-262144 | 0.931517× | 0.948197× | 1.017907× |
| prefix-periodic-1048576 | 0.927030× | 0.944291× | 1.018620× |
| prefix-repeated-1048576 | 0.935650× | 0.948978× | 1.014244× |
| prefix-random-1048576 | 0.957445× | 0.954742× | 0.997177× |

## Natural-text follow-up

The full screening comparison showed a 4.4% adverse point estimate at natural 32 KiB. This separate 648-call screen repeats that case and eight neighboring or larger natural cases. It includes both workers sharing the matcher Object in the same timing blocks.

| Input | Matcher/R355 | Relink/R355 | Relink/matcher |
| --- | ---: | ---: | ---: |
| prefix-natural-32767 | 0.995328× | 0.996614× | 1.001292× |
| prefix-natural-32768 | 0.995530× | 0.998777× | 1.003261× |
| prefix-natural-32769 | 0.998030× | 0.999479× | 1.001452× |
| prefix-natural-65535 | 1.006038× | 1.005924× | 0.999887× |
| prefix-natural-65536 | 1.008636× | 1.006438× | 0.997822× |
| prefix-natural-65537 | 1.005238× | 1.006789× | 1.001543× |
| prefix-natural-131072 | 1.007814× | 1.007276× | 0.999466× |
| prefix-natural-262144 | 1.008765× | 1.010492× | 1.001712× |
| prefix-natural-1048576 | 1.005890× | 1.006184× | 1.000292× |

The three tiny stored inputs beat R355 in the full matrix. The 32 KiB natural result repeats at 0.998777× R355. At and above 64 KiB, the follow-up relink remains approximately 0.6% to 1.1% slower than R355, while its ratios to the exact matcher worker cluster within 0.3% of 1×. The two initial follow-up adapters stopped before timing on import and output-directory setup; the corrected capture kept a separate directory.

## Independent counters and samples

| Input | Retired instructions / matcher | CPU cycles / matcher | Matcher samples |
| --- | ---: | ---: | ---: |
| prefix-natural-262144 | 0.999937× | 1.006880× | 35.0% |
| prefix-periodic-1048576 | 0.999997× | 1.000348× | 62.3% |
| prefix-repeated-1048576 | 0.999221× | 0.994967× | 55.8% |
| prefix-random-1048576 | 0.999574× | 1.001072× | 59.1% |
| eight-literals | 0.999798× | 0.977753× | 13.1% |

The counter companion uses the exact candidate Object in a separate set of 20 balanced calls per actor per case. The profile uses the exact linked elapsed-timing worker in its own acquisition. Sample percentages include validation.

## Generated code and qualification

The runtime pops the last owner slot after the existing ownership and geometry checks. It keeps the previous vacancy handling for interior slots. The transfer routine is inlined. Thirteen focused tests and the independent five-case inlining screen cover the individual runtime change.

The new worker links the exact retained matcher Object, descriptor and source bytecode against the rebuilt runtime. The wide matcher still has 60 vector loads. The measurement covers the same complete-call timer. The original worker passes 162 linked checks for exact output, independent inflation, aliases and cleanup.

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f` plus patch SHA-256 `acaa458a7a8c23da6a92bcd0e1577cf01392ec140e6af2c3b663198864b366fa`. Benchmark source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Retained Object `a0a0452e95b8f9ae7b06a5b6c02896817613ee40903976cad50a46579ab59179`. Linked relink worker `f430d8bb4829629ad1133c5a43c0732413813d6adab220e00f7c01248c360e32`.

This is a fresh timing of the combined runtime and the retained original generated code. Fresh emission, decoded checks and the repeated recovery decision remain separate.

[Paired measurements, generated-code evidence and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/compression-matcher-terminal-b63a36b1c338.tar.gz).
