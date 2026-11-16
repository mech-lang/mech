# First measured compression recovery

**Retain R355. The original-artifact recovery trial still has measured regressions.**

The first recovery candidate beats Roc on 46/81 cases (56.79%) and libdeflate on 4/81. R355 beats Roc on 47/81 and libdeflate on 7/81 in the same run. The merged anchor had 22/81 wins against Roc and 3/81 against libdeflate.

The paired candidate-to-R355 geometric mean is 1.084745×. 72 paired point estimates are above 1×, including 51 above 1.05×. One case loses parity with Roc and three lose parity with libdeflate. The archive keeps every individual observation, including 6,977 zero timer readings.

| Input | R355/Roc | Candidate/Roc | R355/libdeflate | Candidate/libdeflate | Candidate/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.183933× | 0.178122× | 2.305833× | 2.232991× | 0.968410× |
| reymont-prefix-32769 | 1.182129× | 1.198784× | 1.079882× | 1.095096× | 1.014089× |
| prefix-natural-32768 | 1.186010× | 1.204880× | 1.079891× | 1.097073× | 1.015911× |
| prefix-natural-131072 | 1.196970× | 1.224868× | 0.985227× | 1.008190× | 1.023307× |
| prefix-natural-262144 | 1.196193× | 1.229536× | 0.969307× | 0.996326× | 1.027874× |
| prefix-natural-1048576 | 1.210713× | 1.247498× | 0.976233× | 1.005894× | 1.030383× |
| prefix-random-1048576 | 1.487587× | 1.649353× | 2.104287× | 2.333115× | 1.108743× |
| prefix-periodic-1048576 | 0.637757× | 0.753722× | 1.076131× | 1.271807× | 1.181833× |
| prefix-repeated-1048576 | 0.645947× | 0.741555× | 1.015080× | 1.165324× | 1.148012× |

## Separate counter and profile cohorts

| Input | Retired instructions / R355 | CPU cycles / R355 |
| --- | ---: | ---: |
| prefix-natural-262144 | 1.029566× | 1.010465× |
| prefix-periodic-1048576 | 1.018373× | 1.104864× |
| prefix-repeated-1048576 | 1.016457× | 1.149442× |
| prefix-random-1048576 | 1.079163× | 1.099386× |
| eight-literals | 0.895452× | 0.993137× |

The timed candidate and the counter companion use the same original generated Object. The counter results are separate medians across 20 calls per actor per case. Periodic and repeated inputs recover most of the extra retired instructions, with a remaining cycle and elapsed gap. The five candidate profiles use the exact elapsed-timing executable.

At natural 256 KiB the restored composed parser accounts for 55.7% of candidate running samples, matching the archived R355 share. At random 1 MiB, the other parser paths account for 18.9% versus 9.2% in the separate archived R355 profile. The profile shares include validation and describe sampled PCs.

## What the generated code recovered

The exact timed matcher contains 60 wide ARM64 loads and six certified live widening headers. The failed merge had zero of each. The instruction selector now expresses comparison results as an I8 comparison masked with one. Matching that exact Boolean representation lets the existing equality proof recognize the loop again.

The candidate emits four no-growth parser variants with composed read-only offset lookups. They share the 65,536-byte immutable table and keep the live cache and cost view checks. The candidate also uses the retained register allocator loop-depth calculation.

The checked append pass still refuses all four parser variants. After recognizing the canonical Boolean mask, it reaches a fused unsigned 32-bit load. In the inspected phase that is the first cache-header read, followed later by a second unsigned 32-bit literal-cost read. The existing trace recognizes plain loads and therefore declines before emitting the fast continuation.

The three stored-input cases are still slower with the merged output-transfer wrapper. The earlier same-Object diagnostic established that the difference survives with the original generated body. Its timed boundary copies the output view into an out-of-line transfer call.

## Qualification and comparison boundary

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f` plus patch SHA-256 `4153645f90ced5c2cd5f70e5ecaedeb34e2de591e96cdc6234529506bf0ae6ce`. Benchmark source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Original Object SHA-256 `f024a8a9cd1c7e14b887b5310bb5b98cb370d358f62882fda26edcd2ffacef2b`. Linked worker SHA-256 `01ffe627f3c4b3c99dd69d9e0757ee61def44b06eb266a890f49709470467860`.

The narrow locked release build took 84.70 seconds. Original image emission took 628.14 seconds. The original worker passed 162 linked calls with exact bytes, independent inflation, aliases and cleanup. The fixed four-actor kernel took 136.63 seconds, with 200 ms warmup per actor per case and 245,664 recorded calls. Timings include compression, scratch construction, output transfer and private cleanup. Validation remains outside the timer.

Both Mech actors use the compact diagnostic entry. Each uses its own independently qualified descriptor. The candidate was screened with its original artifact. Decoded qualification, repeated recovery acceptance and the separate #990 managed guard remain required on a retention candidate. The merged checkpoint passed the unchanged #990 Rust guard in both cohorts.

[Raw paired observations, linked profile receipts and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/compression-first-recovery-2659d624f3ba.tar.gz).
