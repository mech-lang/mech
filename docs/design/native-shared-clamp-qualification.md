# Qualify shared clamp decisions through the compiler

The exact normally linked compiler candidate takes **0.992946×** the paired control across 81 cases, with both 1.05 gates passing. It gains five libdeflate wins in the full paired campaign. Three were already faster than Roc, so wins against at least one peer rise **66 → 68**. The previous published campaign also scored 68/81, with 24 libdeflate and nine joint wins. The new campaign has 27 and 12.

| Cohort | Cases | Candidate / control | Roc wins | libdeflate wins | Both wins | At least one win |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Screen | 18 | 0.995390× | 4 → 4 | 10 → 12 | 1 → 2 | 13 → 14 |
| Full | 81 | **0.992946×** | **53 → 53** | **22 → 27** | **9 → 12** | **66 → 68** |
| Confirmation | 8 | 0.996567× | 2 → 2 | 1 → 2 | 0 → 0 | 3 → 4 |
| Adverse cases | 10 | 0.995009× | 6 → 6 | 5 → 5 | 2 → 2 | 9 → 9 |
| Stored-case repeat | 10 | 0.997807× | 5 → 5 | 5 → 5 | 0 → 0 | 10 → 10 |

The five full-run crossings against libdeflate are the 32 KiB held-out window, natural 32,767 bytes, periodic 128 and 256 KiB, and repeated 32,769 bytes. Natural 33,027 bytes is 1.001481× in full, 0.999536× in confirmation and 0.997248× in the stored-case repeat. Both order halves improve in the latter two. Held-out random is 1.010857× Roc in full and 1.009265× in confirmation, with both order halves improving in screen, full and confirmation. Held-out mixed is 1.011405× Roc in full and 1.011269× in confirmation. The eight-literal case ranges from 0.169477× to 0.173606× Roc. The primary geometric mean includes every case.

Seven stored bytes gives a +7.07% paired estimate in full and −0.64% on the same worker in the repeat. Both gates pass in both runs. The stored route bypasses the changed matcher. Eight literals gives +0.62% on that repeat and a separate cycle increase of 3.69%. Periodic 8 KiB gives +1.22% full, −0.64% in the adverse set and −0.40% in the stored-case repeat. Random 1 MiB improves 1.58% full and 1.01% in confirmation. [Fixed-address comparison](native-shared-clamp-decision.md)

The candidate uses three opt-in compiler passes, in this order.

1. `MECH_PACK_PRIVATE_HALFWORD_READS=1` packs adjacent unsigned halfword reads in a fixed private view when the caller's current length covers both cells and the intervening path has no observable operation. The two stores keep their original order and widths.
2. `MECH_REUSE_ADMITTED_VIEW_LENGTHS=1` removes repeated unsigned length branches that the caller's guard proves true for the exact immutable packet word. Guard refusal resumes the source route before the phase can act.
3. `MECH_SHARE_CLAMP_DECISIONS=1` forwards both scalar results through one typed merge. The two original tests must use the same current unsigned values. The pass verifies single entries through the skipped code and preserves the selected minimum and both live results. It declines intervening effects, other operands, alternate entries, overlapping rewrites and values used outside the skipped path.

All three run inside the existing compact vector pipeline. The candidate matches the 9,976-byte matcher measured in the fixed-address comparison. The prior body is 10,172 bytes, both frames are 576 bytes, and the other 356 recorded bodies match. Normal linking moves later functions, and the fixed-address 70/81 result is a separate cohort. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`, Object `43fb37b569e9ff9efdf4a8f59eef3e109411b61e90eda42b66abc1965a280d63`, worker `b3980f806ac79b417120a8932aaf6fadb1af362151ae43fe79816c05054ead37`.

| Separate counter cohort | Candidate / control instructions | Candidate / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999864× | 1.036898× |
| Natural 33,027 bytes | 0.993677× | 0.996683× |
| Held-out mixed, 131,073 bytes | 0.991010× | 0.994527× |
| Held-out random, 65,537 bytes | 0.979880× | 0.993824× |
| Random 1 MiB | 0.979745× | 0.987417× |
| Repeated, 65,537 bytes | 0.986195× | 0.985951× |

All six measured payload distributions agree for every case. Peak physical memory agrees in the counter cohort. On natural 33,027 bytes, the matcher accounts for 1,670 of 4,481 sampled timer PCs and the composed parser accounts for 2,197. Held-out random has 3,511 of 5,222 in the matcher. Sampling includes validation and the counter runs are separate from elapsed timing.

The optimized compiler build uses nightly 2026-03-03 with `-D warnings` and `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`. Source emission takes 241.94 seconds including 57.92 seconds through the frontend. Decoded emission takes 184.17 seconds. With all three new rules disabled, Object, descriptor and bytecode match the prior image exactly. Both forms pass 648 linked complete checks and 136,192 direct phase positions, with exact reference streams, independent inflate, aliases, cleanup and failure recovery. The clamp JIT test runs 20,160 calls and the preceding load and length witnesses run 6,468 and 1,440. Formatting and all four architecture checks pass. The separate managed target keeps 13 bundle files byte-identical and passes 88 exact outputs at 0.701562× and 0.709662× Rust. Its timing completed before the report generator hit a missing aggregate patch filename. Replaying the completed observations with the exact patch hashes supplies the receipt.

The paired control uses `6e7dd1bed4614034887d11b574251fa06cc0dff5`. Compiler base `e602b097ccbf8c29e2d56b745102074bb1bbece3` adds the faster guard analysis. The three source commits are `60ffd8fe3755fdfd88f8f4594bdc90b6ee62590f`, `fd596e7848dd039eb765eb6bcefa4f4786350c81`, and `6b9793281cdbd17bb73611ad7c05aeb8e64fa0e5`. The timed compact entry has the three flags above in addition to the retained benchmark configuration. The published packet records the full flags, source and runtime pins, and exact commands. The full repository suite and ordinary hosted CI were outside this qualification.

[433,536 timed calls, exact profiles, patches and qualification replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-e602b097cc-shared-clamp-qualification.tar.gz). SHA-256 `496367bce96917670d95551485ad90dfd547558207773b91626d3caabce511d4`. All 305 files passed public replay.
