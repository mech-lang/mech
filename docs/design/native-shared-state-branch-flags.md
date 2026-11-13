# Normalize a branch-only loop decision with shared state

The one-site diagnostic takes **0.997359×** the paired continuation-forwarding control across all 81 cases. Both 1.05 gates pass. The five new libdeflate wins all recur in the eight-case confirmation. The broad two-site version is [retained as separate evidence](native-matcher-branch-flags-pilot.md).

| Peer | Control wins | Trial wins |
| --- | ---: | ---: |
| Roc | 53/81 (65.4%) | 53/81 (65.4%) |
| libdeflate | 19/81 (23.5%) | **24/81 (29.6%)** |
| Both | 9/81 | 9/81 |
| Either | 63/81 (77.8%) | **68/81 (84.0%)** |

| Input | Control → trial / Roc | Control → trial / libdeflate | Confirmation / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.172033× → 0.172431× | 2.167034× → 2.172041× | — |
| Natural 8 KiB | 0.993263× → 0.989697× | 1.234347× → 1.229915× | 1.232009× |
| Natural 32,767 bytes | 1.100425× → 1.096678× | 1.003113× → 0.999698× | 0.995904× |
| Natural 32 KiB | 1.101716× → 1.096860× | 1.003496× → 0.999072× | 0.997658× |
| Natural 32,769 bytes | 1.101470× → 1.097267× | 1.002736× → 0.998910× | 0.996918× |
| Held-out window, 32 KiB | 1.100782× → 1.096814× | 1.000005× → 0.996401× | 0.997265× |
| Reymont prefix, 32,769 bytes | 1.103860× → 1.096453× | 1.003813× → 0.997077× | 0.996291× |
| Held-out slides, 65,537 bytes | 1.128471× → 1.122327× | 0.986449× → 0.981078× | 0.981131× |
| Held-out mixed, 131,073 bytes | 1.016273× → 1.010960× | 1.177214× → 1.171059× | — |
| Held-out random, 65,537 bytes | 1.026363× → 1.019332× | 1.728174× → 1.716335× | — |
| Periodic 256 KiB | 0.569483× → 0.567964× | 1.003099× → 1.000423× | 0.999814× |

Natural 8 KiB is under Roc in the screen, full and confirmation at 0.994664×, 0.989697× and 0.992474×. The paired control also beats Roc in all three, so this is a faster existing win within these cohorts. Its full-run order halves disagree on direction (1.003288× and 0.989579×); preserve both. Natural 256 KiB and 1 MiB are 0.905606× and 0.909139× libdeflate in the full run. The full cohort takes 0.998298× and 0.996957× the paired control on these inputs. The smaller full-run win counts in previous reports used different cohorts.

The screen, full, confirmation and adverse check contain 7,584, 245,664, 768 and 62,112 timed calls. Their paired elapsed ratios are 0.995157×, 0.997359×, 0.994995× and 0.993441×. The 32-byte high-entropy stored case is +6.45% in the full paired estimator, with the prescribed pooled and block gate estimates at +0.36% and +0.26%; its rerun is −0.60%. Periodic 8 KiB slows +1.89% in the full run and +0.64% in the adverse check. Periodic 256 KiB improves 0.27% in the full run but is 1.000514× libdeflate in the adverse check, where its control was under 1×. The complete matrix loses no peer win. The narrower adverse check retains that loss separately.

The preceding continuation pass proves the current stop decision can remain in control flow. This experiment requires that the remaining OR and low-bit mask, both comparison results, and both zero extensions have no other uses. The operands must be ordinary integer comparisons, and the typed backedge and typed exit must forward a common current value. The compiler complements the comparisons, conjoins their results, and swaps the two existing successors. The stores, source checks, argument values, failure origins, aliases and timing boundary stay fixed. The existing ARM64 conditional-compare selector handles the new conjunction. No source function, field name, benchmark input or compression operation enters the proof.

The exact machine excerpt replaces CMP, two CSETs, the second CMP, ORR, UXTB and the Boolean branch with CMP, CCMP and a flags branch at one loop exit. The 10,176-byte matcher becomes 10,172 bytes and its frame stays 576 bytes. The diagnostic preserves all 1,348 linked text addresses and every byte outside that body, UUID and signature. Code produced from the general IR proof matches the timed diagnostic body byte for byte.

| Separate hardware counter cohort | Trial / control instructions | Trial / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999917× | 1.006101× |
| Natural 8 KiB | 0.996859× | 0.997381× |
| Natural 32 KiB | 0.993140× | 0.996100× |
| Natural 256 KiB | 0.991618× | 0.997656× |
| Held-out slides, 65,537 bytes | 0.991240× | 0.995511× |
| Held-out random, 65,537 bytes | 0.992333× | 0.994152× |
| Repeated, 65,537 bytes | 0.997301× | 0.998645× |

The exact worker's matcher accounts for 1,437/5,290 running timer PCs on natural 8 KiB, 1,949/5,328 on natural 256 KiB, 2,194/5,353 on slides and 3,443/5,079 on held-out random. Profiles include validation; these are sample counts. All six payload distributions match in every timed case. The fixed-address worker passed 162 linked exact-stream, independent inflation, capacity, alias and cleanup checks. Focused Rust JIT qualification passes 21,574 calls against an independent result and store oracle, including unsigned endpoints, and refuses shared predicates, raw input words and unrelated state.

Measured production base `d36c21c3cfd449c76a92efb45114f5973aac5f5d`; integration base `f24765bb1de56feb110d939bf827a96d2aad0944`. The pinned level-12 source, revision-14 bytecode, capacities, runtime input, allocator and complete-call boundary stay fixed. Timed Object `f7727b1d87931f2eac6ee6cc1392a302183cdd554ec353cca1bbc78400013256`; worker `408600bc9d1ac236022cf7601d13b561f492efd80b1d06fb16425cd4591bc139`.

[All elapsed cohorts, machine code, hardware counters and exact-worker profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-d36c21c3cf-shared-state-branch-flags.tar.gz). SHA-256 `2170149b60c9aa7ccc91d72cbb42f9d46c33f76ab600e4e1d7e05bdf2b94c76a`; all 208 files verified from the public download.

Enable the rule with `MECH_NORMALIZE_SELECTED_LOOP_TEST=1` after `MECH_FORWARD_SELECTED_CONTINUATIONS=1`. It is disabled by default and uses the existing ARM64 lowering. The six-feature optimized build (`aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`) uses nightly 2026-03-03 with warnings as errors. Source and decoded revision-14 bytecode emit the exact timed Object and worker. The only descriptor change is the vector matcher's reported code size. Disabling the rule restores the prior Object, decoded descriptor and bytecode exactly.

Across source and decoded, 648 linked stream checks and 136,192 linked phase positions pass, including unchanged rejection origin and recovery. The separate managed benchmark preserves all 13 bundle hashes and 88 exact outputs at 0.727765× and 0.723144× Rust. Changed-file formatting, compiler planning quarantine, host catalog, resident routing and interactive architecture all pass. Ordinary hosted CI remains separate from these local receipts.

[Normal compiler patch, focused and linked checks, and managed qualification](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f24765bb1d-shared-state-branch-qualification.tar.gz). SHA-256 `a3fad38688f3528d9c45757d6807c8914317c28b8a02858fda00f694555849ec`; all 90 files verified after downloading. Both archives replay with `python3 -B verify-round.py`.
