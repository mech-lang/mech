# R298 — Measure shared immutable comparison window bounds

**Retain R297.** The R298 pilot changes paired elapsed by -0.223% across 27 cases and passes both screen gates on all 27. The repetitive 1 MiB cases are essentially flat. The recurring admission still reloads two more stack values.

Measured and published parent c2aa0ebc8984e608791e0fca42fcc0ea8f44d7b9. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64 and the same R297 compiler closure, R272 runtime, workload, scratch and complete-call timing boundary. The source patch is against the compiler closure in the R297 evidence packet.

A source-derived certificate already admits a repeated equality scan. If both captured windows use the same current immutable view and limit and all four bounds are still needed, the later base has the smaller readable suffix. The compiler checks that base against the view length before relying on the subtraction. The remaining predicates and the original checked fallback apply. The emitter still handles distinct views and independent limits separately.

Exactly two of 347 linked bodies change. The general matcher shrinks 50,840 → 50,812 code bytes and grows 1,008 → 1,024 frame bytes. The vector matcher grows 13,828 → 13,988 code bytes and shrinks 784 → 768 frame bytes. The inspected hot guard uses 25 instructions and five stack loads, down from 28 instructions with three stack loads. Its two CLIF copies each form one unsigned maximum. The linked branch still selects the same native or checked route.

The original candidate passes 162 full linked calls across the unchanged 81 cases, including independent inflate, aliases, capacities and zero live private ownership. Three focused tests pass, including 39,366 calls comparing original and revised compiled gates with a checked geometry oracle. The first link ran out of disk space. The identical retry after reclaiming older compiler build outputs passed.

| Paired screen input | R298 / R297 | R298 / Roc | Gates |
|---|---:|---:|---|
| empty | 1.000000× | 2.012195× | pass/pass |
| one-stored | 1.000000× | 1.976190× | pass/pass |
| seven-stored-prefix | 1.000000× | 1.976190× | pass/pass |
| eight-literals | 0.993188× | 0.193291× | pass/pass |
| 33027-periodic-window | 0.991454× | 0.693960× | pass/pass |
| prefix-natural-4096 | 1.003849× | 0.979337× | pass/pass |
| prefix-natural-8192 | 1.004564× | 1.219888× | pass/pass |
| prefix-natural-32768 | 1.001930× | 1.372270× | pass/pass |
| prefix-natural-1048576 | 0.999676× | 1.406735× | pass/pass |
| prefix-random-8192 | 0.992569× | 0.809792× | pass/pass |
| prefix-random-32768 | 0.992255× | 1.372441× | pass/pass |
| prefix-random-1048576 | 1.001446× | 1.710982× | pass/pass |
| prefix-periodic-32767 | 1.004813× | 0.687217× | pass/pass |
| prefix-periodic-32768 | 0.990432× | 0.683227× | pass/pass |
| prefix-periodic-32769 | 0.995392× | 0.688133× | pass/pass |
| prefix-periodic-33027 | 0.997808× | 0.701220× | pass/pass |
| prefix-periodic-65536 | 0.996798× | 0.740032× | pass/pass |
| prefix-periodic-131072 | 0.993552× | 0.770861× | pass/pass |
| prefix-periodic-1048576 | 0.999038× | 0.791239× | pass/pass |
| prefix-repeated-4096 | 0.984523× | 0.396617× | pass/pass |
| prefix-repeated-32767 | 1.001130× | 0.658658× | pass/pass |
| prefix-repeated-32768 | 0.998704× | 0.659343× | pass/pass |
| prefix-repeated-32769 | 0.993499× | 0.680883× | pass/pass |
| prefix-repeated-33027 | 1.006706× | 0.710748× | pass/pass |
| prefix-repeated-65536 | 0.994957× | 0.740860× | pass/pass |
| prefix-repeated-262144 | 1.000558× | 0.786567× | pass/pass |
| prefix-repeated-1048576 | 1.001401× | 0.787445× | pass/pass |

The two time profiles are separate from the elapsed screen and include harness and validation.

| Input | Running samples R297 / R298 | Full guard samples R297 / R298 |
|---|---:|---:|
| prefix-periodic-1048576 | 5366 / 5593 | 358 / 328 |
| prefix-repeated-1048576 | 5377 / 5422 | 323 / 313 |

This pilot stops at the screen. The decoded candidate, full timing gates, hardware counters and matched libdeflate comparison remain part of full adoption. A follow-up can combine the checked difference and unsigned rejection into one lowering and verify whether the spill loads disappear.

[Verified compiler patch, timing screen and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r298-paired-capture-bounds-b4feef6b0d2d.tar.gz). SHA-256 b4feef6b0d2db753d309ff07abaa6939a559c12b9c44408e941957c91a49484c.
