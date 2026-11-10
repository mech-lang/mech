# Current compression scoreboard

**Retained experimental executable: R245. 11 of25 complete compression cases are below1x Roc.** R247 phase-only threading is archived with its repeatable natural-text gain. All three variants below use the same fresh R247 reference cohort. The retained compiler has not changed; historical reports keep their original denominators.

Natural32 improves2.248360x->2.145959x (-4.5545% block elapsed), but periodic worsens1.593197x->1.632102x (+2.4420%) in both cohorts despite restoring the exact R245 root body/frame/symbolic relocations. Root threading is not required for this penalty; its microarchitectural cause is not established. All25 historical1.05 gates pass in both cohorts. Sixteen pooled block means worsen. No below1x case is gained/lost.

These are complete level12 experimental native runs, runtime inputs and matched live output ownership. OrdinaryIR215 and standalone extra-contract omissions remain unchanged. Every25-case row stays mandatory. Ratios use pooled observation medians; changes use balanced block means, so directions may differ. Below1x is a point estimate;0/1/7-byte calls are near clock resolution.

## All current retained wins

| Case | Bytes | Retained R245 / Roc |
|---|---:|---:|
| eight-literals-prefix | 8 | **0.503448x** |
| eight-literals | 8 | **0.503905x** |
| eight-overlap | 8 | **0.508717x** |
| 128-biased-dynamic | 128 | **0.520970x** |
| 32-high-entropy-stored | 32 | **0.546296x** |
| random-64 | 64 | **0.569897x** |
| 320-overlap | 320 | **0.578674x** |
| repeated-1024 | 1024 | **0.641270x** |
| random-256 | 256 | **0.662601x** |
| random-1024 | 1024 | **0.742282x** |
| reymont-prefix-1024 | 1024 | **0.927292x** |

## Every case and candidate regression

| Case | Bytes | Retained R245 / Roc | Candidate R247 / Roc | Full R246 / Roc | Candidate block change | Instruction change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | 1.976190x | +1.5294% | +0.0000% |
| one-stored | 1 | 1.976190x | 1.976190x | 1.976190x | -0.7924% | +0.0000% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | 1.976190x | +0.3884% | +0.0000% |
| eight-literals | 8 | 0.503905x | 0.507425x | 0.495672x | +0.8161% | -0.0406% |
| eight-overlap | 8 | 0.508717x | 0.511321x | 0.511684x | +0.1454% | -0.0485% |
| 320-overlap | 320 | 0.578674x | 0.575161x | 0.581272x | -0.2121% | -1.4081% |
| eight-literals-prefix | 8 | 0.503448x | 0.508766x | 0.517386x | +1.1239% | -0.0407% |
| 128-biased-dynamic | 128 | 0.520970x | 0.521617x | 0.527115x | +0.9321% | -0.5643% |
| 32-high-entropy-stored | 32 | 0.546296x | 0.542923x | 0.538615x | +0.7785% | -0.1547% |
| 16384-distribution-change | 16384 | 2.741508x | 2.724473x | 2.707563x | -0.6214% | -4.7182% |
| 33027-periodic-window | 33027 | 1.593197x | 1.632102x | 1.648828x | +2.4420% | -3.0373% |
| random-64 | 64 | 0.569897x | 0.570611x | 0.575472x | +0.3530% | -0.2900% |
| random-256 | 256 | 0.662601x | 0.674516x | 0.665405x | +1.8982% | -0.8539% |
| random-1024 | 1024 | 0.742282x | 0.756571x | 0.749156x | +1.7790% | -1.8639% |
| repeated-1024 | 1024 | 0.641270x | 0.652534x | 0.653799x | +1.8469% | -2.8340% |
| random-32769 | 32769 | 2.732096x | 2.806148x | 2.790419x | +2.7105% | -3.7421% |
| repeated-32769 | 32769 | 1.506688x | 1.541355x | 1.556664x | +2.3009% | -3.0455% |
| reymont-prefix-1024 | 1024 | 0.927292x | 0.909146x | 0.906615x | -1.7196% | -2.9622% |
| reymont-prefix-16384 | 16384 | 2.213907x | 2.099295x | 2.092803x | -5.1769% | -5.7930% |
| reymont-prefix-32769 | 32769 | 2.248360x | 2.145959x | 2.145357x | -4.5545% | -5.5141% |
| heldout-window-32767 | 32767 | 2.251295x | 2.146943x | 2.145632x | -4.6352% | -5.5254% |
| heldout-window-32768 | 32768 | 2.244875x | 2.142933x | 2.142139x | -4.5411% | -5.5232% |
| heldout-slides-65537 | 65537 | 2.306372x | 2.215506x | 2.210606x | -3.9398% | -5.0748% |
| heldout-random-65537 | 65537 | 2.985697x | 3.055233x | 3.055083x | +2.3290% | -3.7807% |
| heldout-mixed-131073 | 131073 | 2.506422x | 2.511497x | 2.509036x | +0.2025% | -5.6181% |

[Absolute times, both cohorts, counters/memory, code/relocation identities, fresh random64 profiles, exact source and qualification](native-phase-only-loop-threading.md). [Last retained compiler](native-vector-phase-specialization.md). Every measured round and retain/archive decision remains in benchmarks/mech-deflate/experiments.json.

## Additional controlled-prefix coverage (R248)

The retained compiler remains R245. The original25 table above keeps its R247 two-cohort observations. R248 adds56 controlled-prefix configurations using the same normal executables. All81 independently agree with the reference on11 work categories, exact bytes, inflate and aliases. New prefix results use one separate balanced primary cohort; they are additional coverage, not a new compiler speedup. Some inputs duplicate earlier configurations. The near-parity4KiB repeated point is provisional.

### Every new prefix configuration below1x

| Input | Bytes | Retained R245 / Roc |
|---|---:|---:|
| natural | 1024 | **0.927359x** |
| random | 1024 | **0.747654x** |
| periodic | 1024 | **0.660161x** |
| repeated | 1024 | **0.656157x** |
| repeated | 4096 | **0.994270x** |

### Every controlled prefix

| Input | Bytes | Retained R245 / Roc | Archived R247 / Roc |
|---|---:|---:|---:|
| natural | 1024 | 0.927359x | 0.905046x |
| natural | 4096 | 1.732205x | 1.627041x |
| natural | 8192 | 2.000372x | 1.896303x |
| natural | 16384 | 2.182917x | 2.071087x |
| natural | 32767 | 2.223104x | 2.120104x |
| natural | 32768 | 2.222372x | 2.119049x |
| natural | 32769 | 2.221505x | 2.116417x |
| natural | 33027 | 2.223451x | 2.117863x |
| natural | 65535 | 2.234642x | 2.124137x |
| natural | 65536 | 2.246042x | 2.139343x |
| natural | 65537 | 2.241262x | 2.135143x |
| natural | 131072 | 2.202592x | 2.095699x |
| natural | 262144 | 2.204170x | 2.105780x |
| natural | 1048576 | 2.244033x | 2.154004x |
| random | 1024 | 0.747654x | 0.752581x |
| random | 4096 | 1.446860x | 1.401550x |
| random | 8192 | 1.794987x | 1.783976x |
| random | 16384 | 2.305161x | 2.334046x |
| random | 32767 | 2.735059x | 2.786486x |
| random | 32768 | 2.740593x | 2.779905x |
| random | 32769 | 2.708798x | 2.770201x |
| random | 33027 | 2.734139x | 2.771481x |
| random | 65535 | 2.984825x | 3.042485x |
| random | 65536 | 2.984127x | 3.045634x |
| random | 65537 | 2.966156x | 3.032974x |
| random | 131072 | 3.109431x | 3.193102x |
| random | 262144 | 3.162576x | 3.249457x |
| random | 1048576 | 3.235679x | 3.319774x |
| periodic | 1024 | 0.660161x | 0.661563x |
| periodic | 4096 | 1.020835x | 1.021173x |
| periodic | 8192 | 1.195133x | 1.224742x |
| periodic | 16384 | 1.393867x | 1.403244x |
| periodic | 32767 | 1.552908x | 1.594756x |
| periodic | 32768 | 1.554712x | 1.592894x |
| periodic | 32769 | 1.552052x | 1.598976x |
| periodic | 33027 | 1.550876x | 1.588684x |
| periodic | 65535 | 1.683792x | 1.725236x |
| periodic | 65536 | 1.681518x | 1.722979x |
| periodic | 65537 | 1.682484x | 1.717511x |
| periodic | 131072 | 1.747571x | 1.794815x |
| periodic | 262144 | 1.781511x | 1.834278x |
| periodic | 1048576 | 1.813792x | 1.866506x |
| repeated | 1024 | 0.656157x | 0.655389x |
| repeated | 4096 | 0.994270x | 0.987782x |
| repeated | 8192 | 1.152558x | 1.178996x |
| repeated | 16384 | 1.339269x | 1.369642x |
| repeated | 32767 | 1.470852x | 1.513713x |
| repeated | 32768 | 1.473996x | 1.512046x |
| repeated | 32769 | 1.478072x | 1.519796x |
| repeated | 33027 | 1.486106x | 1.520354x |
| repeated | 65535 | 1.591148x | 1.627116x |
| repeated | 65536 | 1.594817x | 1.633477x |
| repeated | 65537 | 1.593469x | 1.634433x |
| repeated | 131072 | 1.664661x | 1.705084x |
| repeated | 262144 | 1.684936x | 1.733150x |
| repeated | 1048576 | 1.690176x | 1.739905x |

[Complete current work comparison, absolute times, ns/byte, instructions/cycles, heap/copy/peak memory and verified inputs/evidence](native-work-ledger-prefix-scaling.md). Official text continuation and the older repeated-prefix heldout stay distinct.
