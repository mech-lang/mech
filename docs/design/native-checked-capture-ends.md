# R299 — Measure a checked end for paired immutable windows

**Retain R297.** The R299 pilot changes paired elapsed by -0.147% across 27 cases and passes both screen gates on all 27. The repetitive 1 MiB cases are essentially flat. The recurring admission reloads three more stack values.

Measured parent c2aa0ebc8984e608791e0fca42fcc0ea8f44d7b9. Published parent e90b7f3701a15b0a0bcc091b851f85caa8583678. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64 and the same R297 compiler closure, R272 runtime, workload, scratch and complete-call timing boundary. The source patch is against the compiler closure in the R297 evidence packet.

The same selected immutable capture proof as R298 checks the later base. This version adds the common limit to that base with overflow detection and checks the end against the same current length. Both checks must succeed. Other predicates and the original checked continuation still apply.

Exactly two of 347 bodies change. The general matcher moves from 50,840 to 48,828 code bytes and from 1,008 to 1,040 frame bytes. The vector matcher moves from 13,828 to 13,864 code bytes and from 784 to 768 frame bytes. The hot gate uses 28 instructions in both R297 and R299. The checked-end build uses six stack loads versus three in R297 and five in R298. It lowers the end with ADDS, then materializes and inverts carry. The limit is reloaded four times.

The exact R298 binary was retimed alongside the candidate. Its paired change in this 27-case screen is -0.188%. Both forms pass both screen gates.

The original candidate passes 162 full linked calls across the unchanged 81 cases, including independent inflate, aliases, capacities and zero live private ownership. Three focused tests pass, including 39,366 calls comparing original and revised compiled gates with a checked geometry oracle.

| Paired screen input | R298 / R297 | R299 / R297 | R299 / Roc | R299 gates |
|---|---:|---:|---:|---|
| empty | 1.000000× | 1.000000× | 1.976190× | pass/pass |
| one-stored | 1.000000× | 1.000000× | 1.976190× | pass/pass |
| seven-stored-prefix | 1.000000× | 1.000000× | 1.976190× | pass/pass |
| eight-literals | 0.999451× | 0.995763× | 0.192256× | pass/pass |
| 33027-periodic-window | 0.996803× | 0.998739× | 0.698327× | pass/pass |
| prefix-natural-4096 | 0.994346× | 0.996312× | 1.010441× | pass/pass |
| prefix-natural-8192 | 0.996080× | 1.001261× | 1.210111× | pass/pass |
| prefix-natural-32768 | 1.001604× | 1.000677× | 1.371707× | pass/pass |
| prefix-natural-1048576 | 1.000124× | 1.001684× | 1.409445× | pass/pass |
| prefix-random-8192 | 0.990938× | 0.998934× | 0.810168× | pass/pass |
| prefix-random-32768 | 0.995016× | 0.998313× | 1.374813× | pass/pass |
| prefix-random-1048576 | 0.999299× | 1.001806× | 1.710521× | pass/pass |
| prefix-periodic-32767 | 0.995767× | 0.997813× | 0.681624× | pass/pass |
| prefix-periodic-32768 | 0.998312× | 0.996498× | 0.685014× | pass/pass |
| prefix-periodic-32769 | 0.997452× | 0.994077× | 0.690939× | pass/pass |
| prefix-periodic-33027 | 0.992033× | 0.998693× | 0.696547× | pass/pass |
| prefix-periodic-65536 | 0.998724× | 0.995374× | 0.744772× | pass/pass |
| prefix-periodic-131072 | 0.999529× | 0.998853× | 0.773840× | pass/pass |
| prefix-periodic-1048576 | 0.999971× | 0.999994× | 0.791563× | pass/pass |
| prefix-repeated-4096 | 0.999133× | 1.001735× | 0.394424× | pass/pass |
| prefix-repeated-32767 | 0.999010× | 1.003385× | 0.663540× | pass/pass |
| prefix-repeated-32768 | 0.998297× | 0.996115× | 0.660701× | pass/pass |
| prefix-repeated-32769 | 0.998494× | 0.996833× | 0.683000× | pass/pass |
| prefix-repeated-33027 | 1.003210× | 0.992336× | 0.701415× | pass/pass |
| prefix-repeated-65536 | 0.996109× | 0.995489× | 0.741878× | pass/pass |
| prefix-repeated-262144 | 0.999597× | 0.999739× | 0.787952× | pass/pass |
| prefix-repeated-1048576 | 0.999978× | 0.999844× | 0.786802× | pass/pass |

The two time profiles are separate from the elapsed screen and include harness and validation.

| Input | Running samples R297 / R299 | Full guard samples R297 / R299 |
|---|---:|---:|
| prefix-periodic-1048576 | 5366 / 5678 | 358 / 365 |
| prefix-repeated-1048576 | 5377 / 5422 | 323 / 320 |

This pilot stops at the screen. The decoded candidate, full timing gates, hardware counters and matched libdeflate comparison remain part of full adoption. The next pilot carries the selected entry span proof to its fixed-width subwindows. The actual retained matcher has a selected minimum of five, a nonwrapping end, and an end within the current input length.

[Verified compiler patch, timing screen and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r299-checked-capture-ends-4bc9d12dd8ac.tar.gz). SHA-256 4bc9d12dd8aca8ab9ef552aec038f785924dc756f9dd407c9e7e70ad7f7db2ba.
