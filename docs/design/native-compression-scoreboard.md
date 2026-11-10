# Current compression scoreboard

**Retained experimental compiler: R253. R254 combination archived.** Adding the existing broad identity-forwarding pass after current-predicate proof hurts large random/text in both confirmation cohorts. The source compiler and source algorithm remain unchanged.

These are fresh R254 matched observations: both columns share the same new reference denominator. The retained R253 implementation is unchanged from the [R253 improvement](native-current-predicates.md); its historical measurements remain there. Eight stays below 1x. The largest winning controlled size remains 4 KiB.

All 81 exact streams, independent inflation, aliases, output capacity and private cleanup pass. Both Mech variants have identical heap/copy/zero fields and source-phase counts. The candidate has larger-call instruction increases and fails several unchanged timing gates, so it is not retained. Ordinary IR215 and the compact entry’s additional omitted managed contract remain unchanged. [Design, every result, current profile, qualification and evidence](native-predicate-forwarding-combination.md).

## Every retained below-reference configuration

| Group | Case | Bytes | Retained R253 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.416931x** |
| original25 | eight-overlap | 8 | **0.407556x** |
| original25 | 320-overlap | 320 | **0.464849x** |
| original25 | eight-literals-prefix | 8 | **0.417555x** |
| original25 | 128-biased-dynamic | 128 | **0.423843x** |
| original25 | 32-high-entropy-stored | 32 | **0.432775x** |
| original25 | random-64 | 64 | **0.461064x** |
| original25 | random-256 | 256 | **0.542358x** |
| original25 | random-1024 | 1024 | **0.639798x** |
| original25 | repeated-1024 | 1024 | **0.555818x** |
| original25 | reymont-prefix-1024 | 1024 | **0.800973x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.803887x** |
| prefix56 | prefix-random-1024 | 1024 | **0.648466x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.582820x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.901405x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.556795x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.866526x** |

## All 81 matched measurements

Ratios use pooled observation medians. Changes use balanced block means of archived R254 versus retained R253. Positive changes are regressions. Near-resolution empty/one/seven-byte samples and every adverse individual observation remain archived.

| Case | Bytes | Retained R253 / Roc | Archived R254 / Roc | Candidate elapsed change | Candidate instruction change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -5.3628% | -0.2079% |
| one-stored | 1 | 1.976190x | 1.976190x | +1.3993% | -0.1256% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | +0.0807% | +1.3861% |
| eight-literals | 8 | 0.416931x | 0.434926x | +3.9367% | -1.1652% |
| eight-overlap | 8 | 0.407556x | 0.432289x | +5.6347% | -1.2326% |
| 320-overlap | 320 | 0.464849x | 0.484281x | +2.5100% | -0.8817% |
| eight-literals-prefix | 8 | 0.417555x | 0.436693x | +3.2921% | -1.1652% |
| 128-biased-dynamic | 128 | 0.423843x | 0.445584x | +5.2414% | -1.0313% |
| 32-high-entropy-stored | 32 | 0.432775x | 0.460897x | +6.6654% | -0.6563% |
| 16384-distribution-change | 16384 | 2.341313x | 2.353894x | +0.5374% | +0.9242% |
| 33027-periodic-window | 33027 | 1.436220x | 1.426367x | -0.6860% | +0.0524% |
| random-64 | 64 | 0.461064x | 0.489083x | +5.8650% | -0.3857% |
| random-256 | 256 | 0.542358x | 0.563540x | +3.1700% | +0.6211% |
| random-1024 | 1024 | 0.639798x | 0.675071x | +4.5818% | +1.3592% |
| repeated-1024 | 1024 | 0.555818x | 0.579535x | +4.4252% | -0.5214% |
| random-32769 | 32769 | 2.513835x | 2.586962x | +2.9089% | +3.2197% |
| repeated-32769 | 32769 | 1.372088x | 1.359102x | -0.9465% | +0.0517% |
| reymont-prefix-1024 | 1024 | 0.800973x | 0.821765x | +2.1123% | +0.3909% |
| reymont-prefix-16384 | 16384 | 2.057270x | 2.061354x | +0.1985% | +0.8079% |
| reymont-prefix-32769 | 32769 | 2.126448x | 2.129332x | +0.1356% | +0.8179% |
| heldout-window-32767 | 32767 | 2.131203x | 2.134625x | +0.1606% | +0.8200% |
| heldout-window-32768 | 32768 | 2.129094x | 2.131727x | +0.1237% | +0.8204% |
| heldout-slides-65537 | 65537 | 2.211736x | 2.215980x | +0.1919% | +0.8075% |
| heldout-random-65537 | 65537 | 2.791868x | 2.862779x | +2.5399% | +3.2128% |
| heldout-mixed-131073 | 131073 | 2.269250x | 2.272002x | +0.1213% | +0.7683% |
| prefix-natural-1024 | 1024 | 0.803887x | 0.818777x | +1.6827% | +0.3909% |
| prefix-natural-4096 | 4096 | 1.572704x | 1.563078x | -0.6121% | +0.8101% |
| prefix-natural-8192 | 8192 | 1.862594x | 1.866224x | +0.1949% | +0.8415% |
| prefix-natural-16384 | 16384 | 2.058570x | 2.060594x | +0.0983% | +0.8005% |
| prefix-natural-32767 | 32767 | 2.132498x | 2.134013x | +0.0710% | +0.8203% |
| prefix-natural-32768 | 32768 | 2.130545x | 2.132447x | +0.0893% | +0.8202% |
| prefix-natural-32769 | 32769 | 2.131449x | 2.133620x | +0.1018% | +0.8187% |
| prefix-natural-33027 | 33027 | 2.124321x | 2.127111x | +0.1313% | +0.8181% |
| prefix-natural-65535 | 65535 | 2.141611x | 2.139072x | -0.1186% | +0.7097% |
| prefix-natural-65536 | 65536 | 2.130765x | 2.130849x | +0.0039% | +0.7084% |
| prefix-natural-65537 | 65537 | 2.162356x | 2.164367x | +0.0930% | +0.7042% |
| prefix-natural-131072 | 131072 | 2.134067x | 2.137101x | +0.1422% | +0.6949% |
| prefix-natural-262144 | 262144 | 2.145937x | 2.152422x | +0.3022% | +0.6855% |
| prefix-natural-1048576 | 1048576 | 2.180404x | 2.187823x | +0.3403% | +0.7747% |
| prefix-random-1024 | 1024 | 0.648466x | 0.678212x | +4.1172% | +1.3605% |
| prefix-random-4096 | 4096 | 1.212611x | 1.243166x | +2.5197% | +2.6654% |
| prefix-random-8192 | 8192 | 1.570189x | 1.615219x | +2.8678% | +3.1487% |
| prefix-random-16384 | 16384 | 2.122976x | 2.156758x | +1.5913% | +3.2477% |
| prefix-random-32767 | 32767 | 2.532465x | 2.605475x | +2.8830% | +3.2383% |
| prefix-random-32768 | 32768 | 2.541059x | 2.603612x | +2.4617% | +3.2383% |
| prefix-random-32769 | 32769 | 2.528863x | 2.592185x | +2.5040% | +3.2209% |
| prefix-random-33027 | 33027 | 2.532118x | 2.589776x | +2.2771% | +3.2199% |
| prefix-random-65535 | 65535 | 2.798722x | 2.864512x | +2.3507% | +3.1467% |
| prefix-random-65536 | 65536 | 2.803080x | 2.865412x | +2.2237% | +3.1467% |
| prefix-random-65537 | 65537 | 2.789249x | 2.848817x | +2.1356% | +3.1383% |
| prefix-random-131072 | 131072 | 2.933092x | 2.998981x | +2.2464% | +2.9654% |
| prefix-random-262144 | 262144 | 2.999043x | 3.067335x | +2.2771% | +3.0125% |
| prefix-random-1048576 | 1048576 | 3.083508x | 3.163219x | +2.5851% | +3.1225% |
| prefix-periodic-1024 | 1024 | 0.582820x | 0.589613x | +0.8903% | -0.4007% |
| prefix-periodic-4096 | 4096 | 0.901405x | 0.900914x | -0.0544% | -0.1667% |
| prefix-periodic-8192 | 8192 | 1.087992x | 1.078040x | -0.9147% | -0.0565% |
| prefix-periodic-16384 | 16384 | 1.273057x | 1.262416x | -0.8359% | +0.0108% |
| prefix-periodic-32767 | 32767 | 1.446102x | 1.428295x | -1.2314% | +0.0525% |
| prefix-periodic-32768 | 32768 | 1.446918x | 1.437282x | -0.6660% | +0.0524% |
| prefix-periodic-32769 | 32769 | 1.435549x | 1.417969x | -1.2247% | +0.0521% |
| prefix-periodic-33027 | 33027 | 1.436424x | 1.425200x | -0.7814% | +0.0524% |
| prefix-periodic-65535 | 65535 | 1.557299x | 1.545913x | -0.7312% | +0.0741% |
| prefix-periodic-65536 | 65536 | 1.556858x | 1.542617x | -0.9148% | +0.0742% |
| prefix-periodic-65537 | 65537 | 1.551953x | 1.539446x | -0.8059% | +0.0739% |
| prefix-periodic-131072 | 131072 | 1.627331x | 1.616391x | -0.6723% | +0.0853% |
| prefix-periodic-262144 | 262144 | 1.662538x | 1.651109x | -0.6875% | +0.0898% |
| prefix-periodic-1048576 | 1048576 | 1.693255x | 1.681601x | -0.6882% | +0.0906% |
| prefix-repeated-1024 | 1024 | 0.556795x | 0.566037x | +0.8693% | -0.5221% |
| prefix-repeated-4096 | 4096 | 0.866526x | 0.854134x | -1.4301% | -0.1798% |
| prefix-repeated-8192 | 8192 | 1.045081x | 1.030149x | -1.4288% | -0.0605% |
| prefix-repeated-16384 | 16384 | 1.230123x | 1.212366x | -1.4435% | +0.0108% |
| prefix-repeated-32767 | 32767 | 1.371927x | 1.359697x | -0.8914% | +0.0523% |
| prefix-repeated-32768 | 32768 | 1.372578x | 1.357252x | -1.1166% | +0.0578% |
| prefix-repeated-32769 | 32769 | 1.369621x | 1.355752x | -1.0126% | +0.0519% |
| prefix-repeated-33027 | 33027 | 1.376601x | 1.360710x | -1.1544% | +0.0523% |
| prefix-repeated-65535 | 65535 | 1.482930x | 1.467044x | -1.0713% | +0.0738% |
| prefix-repeated-65536 | 65536 | 1.480168x | 1.466074x | -0.9521% | +0.0737% |
| prefix-repeated-65537 | 65537 | 1.478054x | 1.465739x | -0.8332% | +0.0736% |
| prefix-repeated-131072 | 131072 | 1.544085x | 1.531743x | -0.7993% | +0.0848% |
| prefix-repeated-262144 | 262144 | 1.576509x | 1.565041x | -0.7275% | +0.0892% |
| prefix-repeated-1048576 | 1048576 | 1.583638x | 1.574199x | -0.5961% | +0.0906% |
