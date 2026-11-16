# Forward selected loop continuations

The compiler keeps a loop's existing stop decision in control flow. The complete paired comparison takes **0.996746×** the retained packed-read control across 81 cases. Both 1.05 gates pass everywhere. Mech wins **52/81 (64.2%)** against Roc and **20/81 (24.7%)** against libdeflate, unchanged from the matched control; 10 cases beat both and 62 beat either.

| Input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.173380× → 0.172433× | 2.213065× → 2.200984× | 0.994541× |
| Natural 8 KiB | 1.001701× → 1.000490× | 1.242496× → 1.240994× | 0.998791× |
| Natural 32 KiB | 1.116563× → 1.103885× | 1.016331× → 1.004790× | 0.988645× |
| Held-out slides, 65,537 bytes | 1.142925× → 1.129954× | 0.999851× → 0.988504× | 0.988651× |
| Held-out mixed, 131,073 bytes | 1.024641× → 1.015927× | 1.186324× → 1.176235× | 0.991495× |
| Natural 256 KiB | 1.136736× → 1.116967× | 0.922314× → 0.906274× | 0.982609× |
| Natural 1 MiB | 1.145262× → 1.129484× | 0.923214× → 0.910495× | 0.986223× |
| Held-out random, 65,537 bytes | 1.034213× → 1.027689× | 1.751637× → 1.740589× | 0.993692× |

The 16-case screen and eight-case confirmation take 0.992603× and 0.992930× control. Natural 8 KiB reaches 0.997146× Roc in confirmation and 1.000275× in the adverse check. The latter check places the control below parity and the candidate just above. Natural 8 KiB is near parity across these runs. Slides stays below libdeflate in all four cohorts, at 0.983863×–0.988504×. Natural 256 KiB improves in all three cohorts where it was measured. Periodic 8 KiB is the largest full-matrix regression (+1.29%), and slows +1.83% in the adverse check. The same check records +1.57% on repeated 8 KiB, +1.52% on periodic 16 KiB and +1.47% on random 8 KiB. All adverse checks pass both gates and their geomean is 1.003188×. Preserve both order halves and the altered peer wins from these narrower cohorts; the full win set remains the scoreboard.

The pass is opt-in with `MECH_FORWARD_SELECTED_CONTINUATIONS=1` in the compact native vector pipeline. It begins with a canonical I64 OR of two extended integer comparisons. Both selected arms must reach the same typed merge through single-entry paths. Stores, calls and checks on those arms stay in place. From each arm's final jump, the pass symbolically follows only direct jumps, Boolean masks or complements, and branches proved by the current selected value. It stops at the typed loop entry or exit. Destination arguments must be available at the original jump. It verifies that the proposed CFG cannot reach an old live-out before redefining it, and declines conflicting shortcuts. A later visit evaluates its own inputs. No source identity, input size or compression operation selects the rule.

The matcher's search and skip each stop on child-range exhaustion or the depth limit. The retained machine code combines the comparisons, branches to the clear path when needed, then complements, transports and tests the result again. The new edges leave both original tree stores and their order intact while removing that flag from the recurring continuation. The four rewritten edges reduce one vector body from 10,188 to 10,176 bytes, with the 576-byte frame unchanged. Static stack references in allocated VCode fall from 437 to 436. The diagnostic and normal Object are byte-identical; all other Object bytes and all 1,348 linked text addresses are unchanged. The fixed-address worker matches its compiler-built equivalent exactly.

| Separate hardware counter cohort | Trial / control instructions | Trial / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999951× | 1.014542× |
| Natural 8 KiB | 0.995994× | 0.992052× |
| Natural 32 KiB | 0.993732× | 0.983630× |
| Natural 256 KiB | 0.993360× | 0.984133× |
| Held-out slides, 65,537 bytes | 0.992687× | 0.981291× |
| Held-out random, 65,537 bytes | 0.992625× | 0.994164× |
| Repeated, 65,537 bytes | 1.002709× | 0.997653× |

The exact candidate's matcher accounts for 1,321/4,715 running timer PCs on natural 8 KiB, 1,965/5,323 on natural 256 KiB, 2,196/5,325 on slides and 3,497/5,199 on held-out random. These sample counts include validation. Both independent peer machine excerpts keep the continuing decision in flags; conditional compare is a separately measured follow-up.

The timed diagnostic is based on `77a4d5cdc976460adf21eb709f0edf08787321b0`; the normal compiler patch is based on `643c100ebb70d7dc5628407dede26062aef80a09`. Nightly 2026-03-03 builds the six-feature `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` producer with warnings as errors. Original source and decoded revision-14 bytecode emit Object `258613e5d1a8bad18e180c587b329a923f6883188fca6ec3eac26fc8f4e6e1be` and executable `a3eae71d94fb7c85a67f85cfd12438faa232c116b90e6c63c27015a31c512b8e`. In each form, the descriptor changes only the vector-body size; its ABI, origins and storage contract are identical. The producer exited successfully even though the first receipt incorrectly required the unchanged metadata hash. The corrected byte audit reuses the finished Objects and confirms the one expected size field. The disabled option restores the prior Object, descriptor and bytecode hashes.

The matched elapsed cohorts contain 7,584, 245,664, 6,816 and 56,064 calls. All six payload distributions match. The diagnostic passes 162 linked checks; both ordinary forms pass 648 more across exact and shared descriptors, preserving exact streams, independent inflation, capacities, aliases and cleanup. The separate linked phase witness checks 136,192 positions with identical rejection origin and successful retry. Two focused Rust JIT tests pass, including 82,944 native invocations against an independent oracle, and refusal with raw flags, shared selected paths, effects in the bypass and escaping values. An initial standalone test build failed on harness visibility and a test borrow; both were corrected. The managed comparison passes 88 exact outputs at 0.712064× and 0.720110× Rust with all 13 bundle hashes unchanged. Changed-file formatting, compiler planning quarantine, native host catalog, resident routing and interactive architecture pass.

[Timing, hardware counters, fixed-address machine code and profile replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-77a4d5cdc9-matcher-selected-continuations.tar.gz). SHA-256 `b97d773271ca8d5122608bcc0d53091cf5320dce39279b47932bded9082b094a`; 204 files verified from the public download.

[Compiler patch, focused, linked and managed qualification](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-643c100ebb-selected-continuations-qualification.tar.gz). SHA-256 `0d4d690d62e0dad2d7f25407ae89d2d2622b713b3755d8ac913ea6d53aaf72d7`; 95 files verified from the public download. Run `python3 -B verify-round.py` in either archive.
