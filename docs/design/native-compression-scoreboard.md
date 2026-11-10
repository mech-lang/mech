# Current compression scoreboard

**Retained experimental compiler: R255.** The R253 Mech source and algorithm remain unchanged; a bounded Cranelift AArch64 emitter optimization removes a duplicate comparison in four reduction bodies. Text 1 MiB retires 0.6974% fewer instructions, with essentially flat elapsed (+0.0809%). This is not a substantial text timing win.

Both all-81 matched confirmations pass every unchanged 1.05 gate, and eight remains below 1x. All heap/copy/zero figures remain equal. Root, 132 other routines and seven vector bodies are identical; all 142 retained linked function starts and linked root bytes match the control. The largest winning controlled size remains 4 KiB.

Ratios below use the fresh R255 reference denominator. R253 historical measurements stay in its [report](native-current-predicates.md). At 1 MiB: text **2.174604x**, random **3.070799x**, periodic **1.691775x**, repeated **1.582587x**; eight **0.434532x**. Ordinary IR215 and the compact entry’s additional omitted managed contract remain unchanged. [Patch, qualification, every measurement, current profile and verified evidence](native-shared-comparison.md).

## Every retained below-reference configuration

| Group | Case | Bytes | R255 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.434532x** |
| original25 | eight-overlap | 8 | **0.437646x** |
| original25 | 320-overlap | 320 | **0.491495x** |
| original25 | eight-literals-prefix | 8 | **0.441553x** |
| original25 | 128-biased-dynamic | 128 | **0.435588x** |
| original25 | 32-high-entropy-stored | 32 | **0.450853x** |
| original25 | random-64 | 64 | **0.470429x** |
| original25 | random-256 | 256 | **0.549964x** |
| original25 | random-1024 | 1024 | **0.649841x** |
| original25 | repeated-1024 | 1024 | **0.571373x** |
| original25 | reymont-prefix-1024 | 1024 | **0.809736x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.820394x** |
| prefix56 | prefix-random-1024 | 1024 | **0.656630x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.579668x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.882739x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.562690x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.866064x** |

## All 81 matched measurements

Ratios use pooled observation medians. Changes use balanced block means against matched R253. Positive changes are regressions; every adverse observation is retained. Instructions and cycles come from an independent cohort.

| Case | Bytes | R253 / Roc | Retained R255 / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -5.4878% | +0.0000% |
| one-stored | 1 | 1.976190x | 1.976190x | -1.2829% | +0.0000% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -0.1105% | +0.0000% |
| eight-literals | 8 | 0.431510x | 0.434532x | +0.4137% | +0.0000% |
| eight-overlap | 8 | 0.429237x | 0.437646x | +1.9227% | -0.0013% |
| 320-overlap | 320 | 0.488720x | 0.491495x | +0.3227% | -0.0030% |
| eight-literals-prefix | 8 | 0.428324x | 0.441553x | +1.1018% | +0.0000% |
| 128-biased-dynamic | 128 | 0.435443x | 0.435588x | -0.2427% | -0.0124% |
| 32-high-entropy-stored | 32 | 0.446701x | 0.450853x | +0.1834% | -0.0018% |
| 16384-distribution-change | 16384 | 2.351248x | 2.331244x | -0.8508% | -0.0789% |
| 33027-periodic-window | 33027 | 1.437834x | 1.426207x | -0.8086% | -0.0028% |
| random-64 | 64 | 0.461296x | 0.470429x | +2.5398% | -0.0013% |
| random-256 | 256 | 0.543337x | 0.549964x | +1.6390% | -0.0011% |
| random-1024 | 1024 | 0.642684x | 0.649841x | +0.9291% | +0.0017% |
| repeated-1024 | 1024 | 0.567953x | 0.571373x | +0.4921% | +0.0045% |
| random-32769 | 32769 | 2.515242x | 2.514559x | -0.0271% | -0.0003% |
| repeated-32769 | 32769 | 1.369651x | 1.367773x | -0.1372% | -0.0029% |
| reymont-prefix-1024 | 1024 | 0.809739x | 0.809736x | +0.0335% | -0.2761% |
| reymont-prefix-16384 | 16384 | 2.059503x | 2.050391x | -0.4425% | -0.6741% |
| reymont-prefix-32769 | 32769 | 2.134637x | 2.129041x | -0.2621% | -0.6618% |
| heldout-window-32767 | 32767 | 2.136502x | 2.128159x | -0.3905% | -0.6647% |
| heldout-window-32768 | 32768 | 2.135478x | 2.127728x | -0.3629% | -0.6664% |
| heldout-slides-65537 | 65537 | 2.210322x | 2.205205x | -0.2315% | -0.6058% |
| heldout-random-65537 | 65537 | 2.797955x | 2.781194x | -0.5990% | +0.0004% |
| heldout-mixed-131073 | 131073 | 2.271093x | 2.272213x | +0.0493% | -0.2818% |
| prefix-natural-1024 | 1024 | 0.819083x | 0.820394x | -0.1762% | -0.2773% |
| prefix-natural-4096 | 4096 | 1.564692x | 1.542794x | -1.3995% | -0.6865% |
| prefix-natural-8192 | 8192 | 1.863595x | 1.860608x | -0.1603% | -0.7147% |
| prefix-natural-16384 | 16384 | 2.064067x | 2.047143x | -0.8200% | -0.6678% |
| prefix-natural-32767 | 32767 | 2.139014x | 2.131957x | -0.3299% | -0.6661% |
| prefix-natural-32768 | 32768 | 2.133446x | 2.127455x | -0.2808% | -0.6618% |
| prefix-natural-32769 | 32769 | 2.131820x | 2.128181x | -0.1707% | -0.6615% |
| prefix-natural-33027 | 33027 | 2.132253x | 2.128295x | -0.1856% | -0.6662% |
| prefix-natural-65535 | 65535 | 2.130423x | 2.128858x | -0.0735% | -0.7306% |
| prefix-natural-65536 | 65536 | 2.126442x | 2.124778x | -0.0782% | -0.7314% |
| prefix-natural-65537 | 65537 | 2.157687x | 2.127389x | -1.4042% | -1.0571% |
| prefix-natural-131072 | 131072 | 2.126584x | 2.129745x | +0.1486% | -0.7570% |
| prefix-natural-262144 | 262144 | 2.137119x | 2.140011x | +0.1353% | -0.7681% |
| prefix-natural-1048576 | 1048576 | 2.172846x | 2.174604x | +0.0809% | -0.6974% |
| prefix-random-1024 | 1024 | 0.652985x | 0.656630x | +0.4637% | +0.0019% |
| prefix-random-4096 | 4096 | 1.221196x | 1.221582x | +0.0316% | -0.0007% |
| prefix-random-8192 | 8192 | 1.586681x | 1.610085x | +1.4750% | -0.0001% |
| prefix-random-16384 | 16384 | 2.127065x | 2.118671x | -0.3946% | +0.0000% |
| prefix-random-32767 | 32767 | 2.535862x | 2.539888x | +0.1588% | -0.0003% |
| prefix-random-32768 | 32768 | 2.543242x | 2.544112x | +0.0342% | -0.0002% |
| prefix-random-32769 | 32769 | 2.509991x | 2.505762x | -0.1685% | +0.0005% |
| prefix-random-33027 | 33027 | 2.536627x | 2.532534x | -0.1614% | +0.0005% |
| prefix-random-65535 | 65535 | 2.800753x | 2.791049x | -0.3465% | +0.0038% |
| prefix-random-65536 | 65536 | 2.805868x | 2.792846x | -0.4641% | +0.0003% |
| prefix-random-65537 | 65537 | 2.798431x | 2.780617x | -0.6365% | +0.0006% |
| prefix-random-131072 | 131072 | 2.927314x | 2.927954x | +0.0219% | -0.0005% |
| prefix-random-262144 | 262144 | 2.996320x | 2.995264x | -0.0352% | -0.0002% |
| prefix-random-1048576 | 1048576 | 3.086275x | 3.070799x | -0.5014% | -0.0870% |
| prefix-periodic-1024 | 1024 | 0.575065x | 0.579668x | +0.0366% | -0.0200% |
| prefix-periodic-4096 | 4096 | 0.895778x | 0.882739x | -1.4557% | -0.0106% |
| prefix-periodic-8192 | 8192 | 1.077191x | 1.073716x | -0.3226% | -0.0077% |
| prefix-periodic-16384 | 16384 | 1.258795x | 1.253892x | -0.3895% | -0.0027% |
| prefix-periodic-32767 | 32767 | 1.434478x | 1.428904x | -0.3886% | -0.0028% |
| prefix-periodic-32768 | 32768 | 1.432484x | 1.424933x | -0.5271% | -0.0028% |
| prefix-periodic-32769 | 32769 | 1.431303x | 1.428118x | -0.2225% | -0.0028% |
| prefix-periodic-33027 | 33027 | 1.435375x | 1.425971x | -0.6552% | -0.0027% |
| prefix-periodic-65535 | 65535 | 1.555393x | 1.552524x | -0.1845% | -0.0032% |
| prefix-periodic-65536 | 65536 | 1.553665x | 1.551618x | -0.1317% | -0.0032% |
| prefix-periodic-65537 | 65537 | 1.550408x | 1.548151x | -0.1456% | -0.0033% |
| prefix-periodic-131072 | 131072 | 1.627421x | 1.622137x | -0.3247% | -0.0028% |
| prefix-periodic-262144 | 262144 | 1.662838x | 1.665242x | +0.1445% | -0.0028% |
| prefix-periodic-1048576 | 1048576 | 1.693067x | 1.691775x | -0.0763% | -0.0030% |
| prefix-repeated-1024 | 1024 | 0.568261x | 0.562690x | -0.1971% | -0.0024% |
| prefix-repeated-4096 | 4096 | 0.865119x | 0.866064x | +0.1092% | -0.0107% |
| prefix-repeated-8192 | 8192 | 1.043351x | 1.040479x | -0.2753% | -0.0077% |
| prefix-repeated-16384 | 16384 | 1.220106x | 1.220539x | +0.0355% | -0.0027% |
| prefix-repeated-32767 | 32767 | 1.366338x | 1.364688x | -0.1208% | -0.0028% |
| prefix-repeated-32768 | 32768 | 1.370750x | 1.370144x | -0.0443% | -0.0041% |
| prefix-repeated-32769 | 32769 | 1.374246x | 1.369132x | -0.3721% | -0.0041% |
| prefix-repeated-33027 | 33027 | 1.375017x | 1.370911x | -0.2986% | -0.0041% |
| prefix-repeated-65535 | 65535 | 1.481564x | 1.472921x | -0.5834% | -0.0029% |
| prefix-repeated-65536 | 65536 | 1.478125x | 1.475253x | -0.1943% | -0.0075% |
| prefix-repeated-65537 | 65537 | 1.476846x | 1.472387x | -0.3019% | -0.0028% |
| prefix-repeated-131072 | 131072 | 1.544740x | 1.541676x | -0.1984% | -0.0048% |
| prefix-repeated-262144 | 262144 | 1.573771x | 1.574080x | +0.0197% | -0.0027% |
| prefix-repeated-1048576 | 1048576 | 1.585071x | 1.582587x | -0.1567% | -0.0037% |
