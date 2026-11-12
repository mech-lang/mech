# Canonical Boolean masks in wide comparisons

The candidate recognizes canonical Boolean masks in the existing wide equality lowering. The compiled source program and the 81-case fixture inventory were unchanged. The candidate and retained worker use the compact diagnostic entry.

## Linked matcher

| Executable | Code bytes | Stack bytes | 128-bit loads | Certified widening headers |
| --- | ---: | ---: | ---: | ---: |
| mech-merged | 9760 | 560 | 0 | 0 |
| mech-wide-mask | 11340 | 624 | 60 | 6 |

The first linked original images each passed two complete calls for every fixture, with exact reference streams, independent decompression, capacity, aliases and cleanup.

## First five-case matched screen

The four actors ran the same 24 rotated permutations per case. All 6,528 measured calls and their validation receipts are included. Both the candidate gains and the adverse measurements appear below.

| Input | Baseline/Roc | Candidate/Roc | Candidate/libdeflate | Candidate/baseline |
| --- | ---: | ---: | ---: | ---: |
| eight-literals | 0.177209× | 0.177660× | 2.278769× | 1.002548× |
| reymont-prefix-32769 | 1.244799× | 1.268403× | 1.156681× | 1.018962× |
| prefix-natural-262144 | 1.276973× | 1.300714× | 1.050802× | 1.018592× |
| prefix-periodic-1048576 | 1.702936× | 0.787019× | 1.335407× | 0.462154× |
| prefix-repeated-1048576 | 1.610734× | 0.774997× | 1.217524× | 0.481145× |

## Complete 16-case screen

The primary and four-actor cohorts retained 56,592 and 154,752 calls respectively. The two forms of both compilers passed 648 linked checks. The focused Rust run passed 6 tests using the release-profile code-generator dependencies. The extracted harness retains the exact widening pass and JIT tests. The full linked runs cover source-plan admission and image generation.

The unmodified pass refused the masked witness and passed the unmasked control. The standard Cargo attempt stopped during math dependency compilation, with zero tests executed.

The candidate stopped at the pilot under the recorded acceptance rules.
The primary paired geometric mean is 0.918883×. The matched priority geometric mean is 1.012282×.
The screen requires both priority witnesses to improve in the primary and matched cohorts. Reymont 32,769 B and natural 256 KiB slowed in both cohorts.

| Gate | Misses |
| --- | --- |
| pooled_gate_1_05 | None |
| block_gate_1_05 | None |

| Input | Candidate/baseline primary | Candidate/baseline matched | Candidate/Roc | Candidate/libdeflate | Heap distributions |
| --- | ---: | ---: | ---: | ---: | --- |
| empty | 1.000000× | 0.992342× | 2.377668× | 0.018656× | Agree |
| one-stored | 1.000000× | 0.993688× | 2.247406× | 0.020133× | Agree |
| seven-stored-prefix | 1.000000× | 1.003716× | 2.703808× | 0.022497× | Agree |
| eight-literals | 1.003269× | 0.995529× | 0.180208× | 2.238402× | Agree |
| reymont-prefix-32769 | 1.017530× | 1.017211× | 1.266319× | 1.150970× | Agree |
| heldout-window-32768 | 1.014274× | 1.016267× | 1.263753× | 1.152489× | Agree |
| prefix-natural-32767 | 1.018927× | 1.017876× | 1.260797× | 1.152595× | Agree |
| prefix-natural-32768 | 1.013369× | 1.016273× | 1.265159× | 1.151916× | Agree |
| prefix-natural-65537 | 1.014648× | 1.016650× | 1.278523× | 1.076866× | Agree |
| prefix-natural-131072 | 1.014343× | 1.015807× | 1.287816× | 1.060821× | Agree |
| prefix-natural-262144 | 1.019555× | 1.018451× | 1.279453× | 1.055827× | Agree |
| prefix-natural-1048576 | 1.021583× | 1.027415× | 1.323107× | 1.055511× | Agree |
| prefix-random-16384 | 1.007060× | 1.009194× | 1.141994× | 3.109556× | Agree |
| prefix-random-1048576 | 1.004944× | 1.004471× | 1.710737× | 2.418394× | Agree |
| prefix-periodic-1048576 | 0.463452× | 0.463199× | 0.789893× | 1.340718× | Agree |
| prefix-repeated-1048576 | 0.480562× | 0.481673× | 0.772258× | 1.210677× | Agree |

CPU and allocation counters were collected in separate cohorts. The profiles below include validation.

| Profile input | Running timer PCs | Candidate/baseline retired instructions | Candidate/baseline cycles |
| --- | ---: | ---: | ---: |
| eight-literals | 5259 | 1.000282× | 0.997226× |
| prefix-natural-1048576 | 5292 | 1.027211× | 1.018138× |
| prefix-natural-262144 | 5355 | 1.023543× | 1.013152× |
| prefix-random-1048576 | 5350 | 1.015112× | 1.004128× |
| prefix-random-16384 | 5233 | 1.013308× | 1.003864× |
| reymont-prefix-32769 | 3612 | 1.022140× | 1.013162× |

Four additional captures cover periodic and repeated 1 MiB using the same linked workers.

| Input | Baseline matcher PCs / total | Candidate matcher PCs / total | Candidate fixed 32-byte route PCs | Candidate/baseline retired instructions | Candidate/baseline cycles |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-periodic-1048576 | 4,385 / 5,546 | 2,940 / 5,320 | 269 | 0.571204× | 0.469541× |
| prefix-repeated-1048576 | 4,116 / 5,361 | 2,807 / 5,352 | 246 | 0.576007× | 0.487460× |

Natural 256 KiB samples 88 of 5,355 running PCs in the fixed 32-byte route at source header 1372. That route still executes a guard for the 64-byte tier and recurring 16-byte limit checks. These samples identify reached instructions. A separate experiment is required to measure their cost.

The candidate beats Roc on 3/16 pilot cases and libdeflate on 3/16. The published 81-case scoreboard remains at the measured integration baseline pending a full adoption campaign.

## Provenance

Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Fixture inventory SHA-256 `387582249cce52fcb7f5f551a0b7af3977aa3954438f5ecf10ef2f0fb41f11a4`. Compiler patch SHA-256 `bad32416c55cd63374549d363228dfbc70a40a6047cf1272ada27ff241deec34`. The comparison fixes level 12, runtime input, warmup, output ownership and the timing boundary. The evidence replay uses the stored observations and does not execute a compiler or compressor.

[Raw receipts, hashes and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/canonical-wide-mask-full-fcafe1c0b395.tar.gz).
