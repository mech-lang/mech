# Measure conditional compares on selected loop exits

The two-site diagnostic takes **0.996732×** paired control over 81 cases, with both 1.05 gates passing. It exchanges one libdeflate win for another and leaves the Roc win set unchanged. The control and trial each win 53/81 (65.4%) against Roc in this cohort. That fresh control already beats Roc on natural 8 KiB; it is not a crossing caused by the change.

| Input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.171760× → 0.171720× | 2.175973× → 2.175462× | 0.999765× |
| Natural 8 KiB | 0.995271× → 0.997965× | 1.233042× → 1.236380× | 1.002708× |
| Natural 32,767 bytes | 1.102826× → 1.101038× | 1.002254× → 1.000630× | 0.998379× |
| Natural 32 KiB | 1.102243× → 1.098165× | 1.001664× → 0.997959× | 0.996301× |
| Natural 32,769 bytes | 1.106722× → 1.102342× | 1.005996× → 1.002014× | 0.996042× |
| Held-out slides | 1.132901× → 1.128747× | 0.989207× → 0.985580× | 0.996333× |
| Natural 256 KiB | 1.122933× → 1.120733× | 0.910116× → 0.908333× | 0.998041× |
| Natural 1 MiB | 1.129547× → 1.129017× | 0.911240× → 0.910813× | 0.999531× |
| Periodic 256 KiB | 0.567139× → 0.568893× | 0.999617× → 1.002709× | 1.003093× |

The full matrix keeps the libdeflate win count at 20/81, but wins against both move from ten to nine and either from 63 to 64. The 16-case screen gains two libdeflate wins and the eight-case confirmation gains one net. In confirmation the natural 32 KiB candidate is 1.000317× libdeflate, natural 32,767 bytes is 0.997561×, and held-out window 32 KiB is 0.999435×. Periodic 256 KiB is 1.000270× in confirmation and 0.997814× in the adverse check. The pairwise natural 32 KiB speedup recurs in all three cohorts even though its libdeflate crossing does not. Eight literals stays under 0.179× Roc across the measured cohorts.

Preserve the largest favorable and adverse results. Periodic 4 KiB improves 12.04% in the full run but slows 1.11% on retest. Its full-run paired order halves disagree on magnitude (0.800734× versus 0.966174×). The 32-byte high-entropy stored case slows 2.96% in the full run and 0.30% in the adverse check. Periodic 8 KiB slows 2.41% in the adverse check. The four cohorts contain 7,584, 245,664, 768 and 62,112 timed calls, with unchanged output bytes, aliases, cleanup and six payload distributions.

The already-forwarded loop stops on either of two integer comparisons. The trial complements them and branches to continuation only when both inverses hold. With the old carried flag gone, ARM64 selects the existing CMP/CCMP/branch lowering. Both original stores, source checks and stop routes remain. The matcher body shrinks from 10,176 to 10,152 bytes, with the same 576-byte frame. Static stack references rise from 436 to 440. This one-body diagnostic preserves every other linked byte except UUID and signature and all 1,348 named text addresses. The ordinary backend reproduces the machine body from the transformed verified IR. The full compiler route was not enabled for this two-site variant.

Separate hardware measurements show natural 32 KiB retiring 0.992824× instructions and taking 0.997194× cycles, natural 256 KiB at 0.991136× and 0.998693×, and slides at 0.990911× and 0.997784×. Eight literals has 1.000099× instructions and 1.053068× cycles in that independent cohort. The exact executable profiles place 1,156/4,187 running timer PCs in the matcher on natural 8 KiB, 1,975/5,364 on natural 256 KiB, 2,196/5,363 on slides, and 3,593/5,310 on held-out random. Profiles include validation. The candidate passes 162 linked checks over all 81 inputs, with exact reference streams and independent decompression.

Keep the two-site rule out of the compiler. Test the search-only geometry against the same retained control to see whether it preserves the periodic win while keeping the natural-text gain. The benchmark remains fixed at level 12, with source, revision-14 bytecode, allocator, capacities, runtime inputs, and complete-call boundary unchanged. Measured base `d36c21c3cfd449c76a92efb45114f5973aac5f5d`; timed worker SHA-256 `6751f2307ca1bbc3de6120ff8be5b3e5b0bb5a7b2bccec6db6668f6dc03fde5a`.

[All timing cohorts, hardware counters, exact-worker profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-d36c21c3cf-matcher-branch-flags.tar.gz). SHA-256 `8804dd2908d8ef688c2589bb184f8df4003c19c017082b1f310bc8c7d5a4f429`; all 208 files verified after download.
