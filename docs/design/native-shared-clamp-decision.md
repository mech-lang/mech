# Reuse a current clamp decision

The fixed-address candidate takes **0.989616×** the published control across all 81 cases, with both 1.05 gates passing. Wins against at least one peer rise **67 → 70** in the matched run. The published compiler scoreboard remains **68/81** during compiler qualification.

| Cohort | Cases | Candidate / control | Roc wins | libdeflate wins | Both wins | At least one win |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Screen against the prior experiment | 18 | 0.998607× | 4 → 4 | 12 → 12 | 2 → 2 | 14 → 14 |
| Full against the published control | 81 | **0.989616×** | **53 → 53** | **23 → 29** | **9 → 12** | **67 → 70** |
| Confirmation | 8 | 0.994360× | 1 → 1 | 0 → 2 | 0 → 0 | 1 → 3 |
| Adverse cases | 10 | 0.992098× | 6 → 6 | 5 → 5 | 2 → 3 | 9 → 8 |

| Full-cohort input | Control → candidate / Roc | Control → candidate / libdeflate | Candidate / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.173782× → 0.172070× | 2.199360× → 2.177696× | 0.990150× |
| Natural 8 KiB | 0.988134× → 0.990449× | 1.227014× → 1.229888× | 1.002342× |
| Held-out window, 32,767 bytes | 1.101324× → 1.097286× | 1.002007× → **0.998332×** | 0.996333× |
| Natural 32 KiB | 1.101126× → 1.097905× | 0.999393× → 0.996470× | 0.997075× |
| Natural 33,027 bytes | 1.101851× → 1.098282× | 1.001622× → **0.998377×** | 0.996761× |
| Reymont prefix, 32,769 bytes | 1.097852× → 1.095109× | 1.000413× → **0.997914×** | 0.997502× |
| Held-out mixed, 131,073 bytes | 1.013087× → 1.008958× | 1.173893× → 1.169109× | 0.995925× |
| Held-out random, 65,537 bytes | 1.020738× → 1.007090× | 1.728671× → 1.705558× | 0.986629× |
| Periodic 256 KiB | 0.567652× → 0.557772× | 1.001581× → **0.984149×** | 0.982595× |
| Random 1 MiB | 1.151406× → 1.139279× | 1.628834× → 1.611678× | 0.989467× |

Natural 33,027 bytes and the smaller held-out window confirm at 0.997510× and 0.998214× libdeflate. Held-out random is 1.001921× Roc in the screen and 1.008966× in confirmation. The two order halves improve in all three comparisons. Natural 32 KiB crosses the other way in the adverse cohort, 0.999802× → 1.003794× libdeflate, with a 0.40% paired slowdown. Periodic 256 KiB confirms a win against both in the adverse cohort. Eight literals slows 0.45% in the screen and 0.27% in confirmation, and improves 0.26% in the adverse cohort. The largest full increases are empty at +1.25% and seven stored bytes at +1.04%. The matcher is unused in both cases.

The published matcher compares remaining length with 258 twice in each of the search and skip dispatches. The reference binaries reuse the comparison for both clamps. The trial routes the existing first branch through one typed merge with both results. On the near-end path it preserves the maximum and the original minimum of the nice length and current remaining length. On the other path it forwards the two prior limits. The slide and subsequent checks keep their positions. The proof uses the identical current SSA operands and confirms that the bypassed blocks have no other entries or observable work.

The measured matcher shrinks from 10,020 to 9,976 bytes over the prior experiment and from 10,172 bytes over the published control. Static stack references fall 420 → 413 over the prior experiment, with the same 576-byte frame. Two conditional branches disappear from the steady paths. The variants that compute both choices speculatively produce more stack traffic and did not progress to timing. The fixed-address executable still differs from the published worker only in the matcher body, UUID and signature.

| Separate counter cohort | Candidate / control instructions | Candidate / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999885× | 1.026290× |
| Natural 256 KiB | 0.994917× | 0.999471× |
| Held-out mixed, 131,073 bytes | 0.990989× | 0.997588× |
| Held-out random, 65,537 bytes | 0.979866× | 0.992864× |
| Random 1 MiB | 0.979733× | 0.985212× |
| Repeated, 65,537 bytes | 0.986225× | 0.984645× |

Four profiles use the exact timed worker. The matcher accounts for 3,536/5,266 timer PCs on held-out random and 3,777/5,365 on random 1 MiB. These include validation. The 162 independent linked calls and 267,264 timed calls preserve the pinned level-12 source, revision-14 bytecode, scratch capacities, allocator, runtime inputs, exact streams, independent inflation, and final cleanup. Candidate Object `d4219d2768962b6857506009f14c4e5902e702ca9da1f204af562c6e0dbfa8be`, worker `b029865174a14b6fb607b7e6f465b1616e979ef7d07e3e8a5e5a807213934b8e`. Compiler re-emission and normally placed timing are pending.

[Measured cohorts, generated code, counters, profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-03aded8154-shared-clamp-decision.tar.gz). SHA-256 `47caeccbcb3c391583e135222efeb0a173a6077718188a7ccfa93bcd91b12f34`. All 237 files passed public replay.
