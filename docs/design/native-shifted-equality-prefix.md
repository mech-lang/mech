# R302 — Share shifted equality across a short scan and its continuation

**Retain R301.** The 27-case screen changes paired elapsed by -0.098%. The pooled gate passes 27/27 and the block gate passes 26/27. Empty input misses the block gate at 1.079561×. The retained 81-case score is 45/81 (55.56%) below Roc and 3/81 below libdeflate.

Measured and published parent 4debec08de0feeb0884f13aa620c7167a1729682. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3 and regalloc2 0.15.2. The compiler patch applies to the R301 closure. The pinned level-12 source, bytecode, fixtures, capacities, tables, limits, R272 allocator and complete-call timing boundary stay fixed.

The certified short scan and the subsequent scan use one existing 24-byte interval record for the current immutable view. At the original admitted entry, a matching displacement and complete coverage let the short scan enter its original loop header at the limit. On a miss, the original short comparisons execute. Only their successful bound exit can record that prefix. The continuation extends the record when the previous interval covers or abuts its current start. Scope entry resets the record. A mismatched or shorter interval keeps the original comparisons.

The linked proof pairs source headers 2713 and 2733 under scope 2398. One of 347 bodies changes. The vector matcher grows from 13,696 to 13,804 code bytes and from 768 to 784 frame bytes. The physical interval slot count stays fixed. The entry assertion, short-path selection and input prefix-load guards retain their conditions and destinations.

| Reached code | R301 instructions | R302 instructions | R301 stack loads | R302 stack loads | R301 input loads | R302 input loads |
|---|---:|---:|---:|---:|---:|---:|
| Successful fixed prefix | 36 | 31 | 2 | 7 | 4 | 0 |
| Long-scan proof | 25 | 27 | 4 | 4 | 0 | 0 |

The prefix counts follow the exact linked successful path. The baseline takes two visits to its 16-byte loop. The candidate reads the proof, loads the input base again, and reaches the continuation through the scalar bound exit. The long-scan proof then rereads the three proof words. The pinned Roc worker uses a ten-instruction word cycle for the short span. The pinned C worker unrolls the four word comparisons. The safe Rust witness uses a single checked interval test and keeps the byte loads on its fallback route.

## Paired screen and matched peer comparison

| Input | R302 / R301 screen | R301 / Roc | R302 / Roc | R301 / libdeflate | R302 / libdeflate |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.995747× | 0.198964× | 0.198009× | 2.537128× | 2.524952× |
| prefix-natural-4096 | 0.997654× | 1.013953× | 1.008731× | 1.727236× | 1.718340× |
| prefix-natural-1048576 | 1.003258× | 1.406352× | 1.410974× | 1.132503× | 1.136226× |
| prefix-random-1048576 | 0.998198× | 1.716092× | 1.713998× | 2.426102× | 2.423141× |
| prefix-periodic-1048576 | 1.002748× | 0.780503× | 0.783998× | 1.324637× | 1.330569× |
| prefix-repeated-4096 | 0.961174× | 0.404184× | 0.409652× | 1.528313× | 1.548990× |
| prefix-repeated-1048576 | 1.002057× | 0.778046× | 0.780436× | 1.221569× | 1.225321× |

The 27-case screen contains 56,988 measured calls. The separate four-actor comparison contains 6,720. Repeated 4 KiB favors R302 in the first screen and R301 in the matched peer cohort. Both observations are preserved.

## Counter and profile evidence

| Input | R301 instructions | R302 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 772,828 | 772,924 | +0.012% | +0.712% |
| prefix-natural-4096 | 9,662,275 | 9,661,944 | -0.003% | -0.567% |
| prefix-natural-1048576 | 2,933,004,456 | 2,930,254,623 | -0.094% | +0.579% |
| prefix-random-1048576 | 657,607,786 | 656,871,574 | -0.112% | +0.044% |
| prefix-periodic-1048576 | 552,104,424 | 547,033,193 | -0.919% | +0.271% |
| prefix-repeated-4096 | 3,039,798 | 3,020,212 | -0.644% | +0.010% |
| prefix-repeated-1048576 | 559,943,706 | 554,892,356 | -0.902% | +0.330% |

Each counter cohort retains 24 complete-call samples per actor. Lifetime physical peak is in the counter receipt.

| Input | Running samples before / after | Selected prefix PCs before / after | Suffix proof PCs before / after | Matcher stack-load PCs before / after |
|---|---:|---:|---:|---:|---:|
| prefix-natural-4096 | 5580 / 5604 | 0 / 0 | 0 / 0 | 481 / 449 |
| prefix-periodic-1048576 | 5370 / 5347 | 350 / 310 | 307 / 335 | 1758 / 2251 |
| prefix-random-1048576 | 5384 / 5364 | 0 / 0 | 0 / 0 | 1547 / 932 |
| prefix-repeated-1048576 | 5377 / 5351 | 326 / 250 | 257 / 363 | 1630 / 2036 |

Sample counts come from the exact linked worker and include harness and validation.

## Qualification and disposition

- 324 original and decoded linked compression calls across all 81 cases preserve exact streams, independent inflation, capacities, held aliases and zero final private ownership. The original and decoded Objects and metadata match.
- The new focused JIT test passes 13,376 compiled calls and 147,136 result checks. It covers displacements, shifts, fresh scopes, mismatches, checked admission, intervening effects, a different view and a partial final word. Four inherited equality tests and 48 cleanup tests pass. Builds use warnings as errors.
- The first build declined the real matcher and reproduced the control executable exactly. Its negative test exposed an intervening effect at the continuation entry. The revised bounded search reaches the nearest eligible continuation and verifies the complete entry block.
- Full adoption timing and memory cohorts stop after the screen. The matched peer, counter and profile cohorts diagnose the measured candidate. R301 supplies the full-suite score.

Use a single proved end across the short scan and its continuation. Require the selected short exit to forward the exact current cursor and bases. On that route carry the proved end as one scalar through the existing continuation checks and read the saved record only once. Other entries retain the current checked lookup.

## All screen observations

| Input | R301 / Roc | R302 / Roc | R302 / R301 | Pooled / block gates |
|---|---:|---:|---:|---|
| empty | 1.976190× | 1.976190× | 1.000000× | pass/miss |
| one-stored | 1.976190× | 1.976190× | 1.000000× | pass/pass |
| seven-stored-prefix | 1.976190× | 1.976190× | 1.000000× | pass/pass |
| eight-literals | 0.192921× | 0.191973× | 0.995747× | pass/pass |
| 33027-periodic-window | 0.698788× | 0.700617× | 1.000432× | pass/pass |
| prefix-natural-4096 | 1.017590× | 1.020886× | 0.997654× | pass/pass |
| prefix-natural-8192 | 1.218242× | 1.215258× | 1.002124× | pass/pass |
| prefix-natural-32768 | 1.366062× | 1.371051× | 1.000591× | pass/pass |
| prefix-natural-1048576 | 1.406053× | 1.412004× | 1.003258× | pass/pass |
| prefix-random-8192 | 0.812616× | 0.819443× | 0.999201× | pass/pass |
| prefix-random-32768 | 1.379133× | 1.374953× | 0.995919× | pass/pass |
| prefix-random-1048576 | 1.715925× | 1.711794× | 0.998198× | pass/pass |
| prefix-periodic-32767 | 0.672057× | 0.672591× | 1.003631× | pass/pass |
| prefix-periodic-32768 | 0.675295× | 0.680159× | 1.001094× | pass/pass |
| prefix-periodic-32769 | 0.679316× | 0.681009× | 0.997566× | pass/pass |
| prefix-periodic-33027 | 0.692726× | 0.697012× | 0.989744× | pass/pass |
| prefix-periodic-65536 | 0.733130× | 0.735374× | 1.003327× | pass/pass |
| prefix-periodic-131072 | 0.758641× | 0.763505× | 1.001914× | pass/pass |
| prefix-periodic-1048576 | 0.782052× | 0.783883× | 1.002748× | pass/pass |
| prefix-repeated-4096 | 0.411719× | 0.394412× | 0.961174× | pass/pass |
| prefix-repeated-32767 | 0.647248× | 0.650986× | 1.008740× | pass/pass |
| prefix-repeated-32768 | 0.655676× | 0.653574× | 0.999999× | pass/pass |
| prefix-repeated-32769 | 0.673083× | 0.678161× | 1.009616× | pass/pass |
| prefix-repeated-33027 | 0.702238× | 0.700872× | 0.994876× | pass/pass |
| prefix-repeated-65536 | 0.731109× | 0.734278× | 1.003030× | pass/pass |
| prefix-repeated-262144 | 0.777468× | 0.780447× | 1.001944× | pass/pass |
| prefix-repeated-1048576 | 0.777787× | 0.779727× | 1.002057× | pass/pass |

[Verified compiler patch, timing screen and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r302-prefix-equality-reuse-4a2d9d0abdda.tar.gz). SHA-256 4a2d9d0abdda9f3e607426bdc2dcc3593be2f1d0ad32e9623d2cb891576afa37.
