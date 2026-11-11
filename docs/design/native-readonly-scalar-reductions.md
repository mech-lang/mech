# R273 — Borrow read-only arrays through scalar reductions

**Retain R272.** The 19-case pilot changes paired elapsed by -0.722% across the case ratios. All 19 pooled gates pass. The empty and one-byte cases miss the block gate. The full-suite score stays at 31/81 below Roc and 3/81 below libdeflate.

Measured and published parent e8e87ab7f84a41fe1455bc216136729d90ea3d72. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The candidate retains the R272 allocation helper, pinned level-12 source, bytecode, tables, limits, capacities, source allocation sites and complete-call boundary.

## What changed in the executable

The canonical Huffman literal-cost recurrence has a complete numeric proof for two immutable views and a scalar result. Its current dimensions, values and aliases remain dynamic. The compact phase selector previously required a returned array or frame. The pilot admits complete, single-loop, read-only scalar reductions over at least two arrays. Existing entry binding transports their current views once. On success the callee writes the exact scalar source word to the existing result packet. Terminal liveness uses that same absolute word. On failure the status and reached source origin return before result publication. The synchronous caller holds the source views through the phase and retires them at the exit.

Thirteen phases select this rule. At canonical header 22093, the retained profile attributes 57 running samples to the two indexed reads and 22 to the scalar branch. The reached native body has two payload loads and no stack traffic or owner operations on the inspected 31-instruction continuing visit. Its entry and error exits still use the existing compact calling convention. Duplicate comparisons and checked arithmetic survive in that visit.

The linked root grows from 1,039,432 to 1,045,380 code bytes and from 57,072 to 61,968 frame bytes. The same prologue instruction sequence reserves the larger frame with different register and stack assignments. The cause of the short-input timing change remains open.

## SIMD in the measured binaries

The retained Mech binary compares 64-byte match groups with 128-bit loads, vector XOR and OR. It also rebases 16-bit tree entries with UQSUB. Libdeflate uses SQADD in the table slide and Clang emits MUL and MLA on four 32-bit lanes for the C Huffman cost routine. The corresponding Mech pilot remains a checked scalar reduction. A bounded widening reduction would need to prove every selected read and checked sum safe or fall back to the current scalar order. The archived excerpts are from the exact timed workers.

## Same-block measurements

The screen uses 12 balanced blocks per case and 200 ms warmups. Empty and one-byte pooled medians are 83 ns for both workers. Their median block means are shown below. The independent four-actor follow-up uses all 24 orders, the same raw streams and capacities, and a fresh libdeflate 1.26 compressor per measured call. Both ratios use Mech as numerator.

| Case | R272 median block mean | R273 median block mean | Change |
|---|---:|---:|---:|
| empty | 69.49 ns | 76.34 ns | +9.86% |
| one-stored | 74.14 ns | 83.97 ns | +13.26% |

| Case | R272 / Roc | R273 / Roc | R272 / libdeflate | R273 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.200730× | 0.193350× | 2.558463× | 2.464403× |
| prefix-natural-1048576 | 1.755205× | 1.755324× | 1.419926× | 1.420023× |
| prefix-random-1048576 | 2.106051× | 2.107540× | 2.972370× | 2.974471× |
| prefix-periodic-32768 | 0.983626× | 0.979445× | 2.097602× | 2.088687× |
| prefix-periodic-33027 | 0.995631× | 0.996128× | 2.124380× | 2.125439× |
| prefix-repeated-4096 | 0.536310× | 0.518285× | 2.011944× | 1.944322× |

## Counters, profiles and correctness

Eight-literal instructions change from 812,277.0 to 762,610.5, a -6.11% change. Cycles change from 198,942.5 to 197,414.0. The six independent counter cases retain identical median allocation counts, initialized copy bytes and private peak. Eight-literal process lifetime peak increases by 245,760 bytes.

Three profiles use the exact tested linked candidate. They include harness and validation work. The natural 1 MiB profile places 2,940/5,366 samples in the no-growth path parser and 2,070/5,366 in the vector matcher. At this size those routines dominate the next decision.

- 162 complete original-image calls cover all 81 streams with exact bytes, independent inflate, unchanged input and output aliases, and zero final private ownership.
- 30 linked controls compare the disabled control with candidate original and decoded forms. The candidate artifacts match byte-for-byte. The tests exercise both matrix and vector routes, short and empty arrays, zero visits, full-width values and success → checked overflow → retry. The failure preserves status, source origin, prior publication and cleanup.
- The first standalone witness used the heap-free compact entry and failed to link against the heap-enabled symbol. A reachable one-word append puts the witness on the same entry as the compressor. All corrected controls pass.
- The timing scope is a 19-case screen, six matched library cases, six independent counter cases and three CPU profiles. R272 supplies the retained 81-case acceptance data.

The packet contains the exact four-file compiler diff, including disabled source-map instrumentation, and the source, toolchain, Object, worker and runtime hashes. Replaying the numerical results uses the archived observations and requires no compiler build or compression run.

## All pilot timings

| Case | R272 / Roc | R273 / Roc | R273 / R272 | Pooled / block gate |
|---|---:|---:|---:|---|
| empty | 1.976190× | 1.976190× | 1.000000× | pass/fail |
| one-stored | 1.976190× | 1.976190× | 1.000000× | pass/fail |
| seven-stored-prefix | 1.976190× | 1.976190× | 1.000000× | pass/pass |
| eight-literals | 0.195827× | 0.189151× | 0.959650× | pass/pass |
| 33027-periodic-window | 0.996498× | 0.985869× | 0.987476× | pass/pass |
| prefix-natural-32768 | 1.688133× | 1.680648× | 0.999380× | pass/pass |
| prefix-natural-1048576 | 1.756767× | 1.755903× | 0.999550× | pass/pass |
| prefix-random-32768 | 1.723387× | 1.731728× | 1.003083× | pass/pass |
| prefix-random-1048576 | 2.100993× | 2.101145× | 1.000293× | pass/pass |
| prefix-periodic-32767 | 0.971537× | 0.969863× | 1.001797× | pass/pass |
| prefix-periodic-32768 | 0.970135× | 0.970423× | 0.996830× | pass/pass |
| prefix-periodic-32769 | 0.980932× | 0.980373× | 0.997827× | pass/pass |
| prefix-periodic-33027 | 0.995778× | 0.984592× | 0.985733× | pass/pass |
| prefix-repeated-4096 | 0.531380× | 0.515236× | 0.949082× | pass/pass |
| prefix-repeated-32767 | 0.925347× | 0.924209× | 0.994617× | pass/pass |
| prefix-repeated-32768 | 0.924660× | 0.923469× | 0.995703× | pass/pass |
| prefix-repeated-32769 | 0.951242× | 0.948459× | 0.993432× | pass/pass |
| prefix-repeated-33027 | 0.970617× | 0.972047× | 1.001765× | pass/pass |
| prefix-repeated-65536 | 1.036604× | 1.036781× | 0.998553× | pass/pass |

[Verified patch and timing observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r273-readonly-scalar-reductions-49733d995008.tar.gz). SHA-256 49733d9950080237a57cf59f1a8bb5fc4140aff4eaebd55b4e7ac9ac077bb9ff.
