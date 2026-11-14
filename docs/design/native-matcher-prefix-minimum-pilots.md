# Compare prefix minima in the native matcher

The fixed-address search-only pilot takes **0.998521×** paired control across 16 cases. The shared minimum after search and skip merges takes **1.001467×** across the same 16 cases in a separate matched cohort. Both timing gates pass throughout. Neither trial gains a peer win in its own cohort.

| Input | Search control → trial / Roc | Search control → trial / libdeflate | Search trial / control | Shared minimum trial / control |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.171248× → 0.171719× | 2.172560× → 2.178542× | 1.002754× | 1.002731× |
| Natural 8 KiB | 1.005272× → 1.002423× | 1.249886× → 1.246345× | 0.997167× | 1.002033× |
| Natural 32 KiB | 1.119165× → 1.117222× | 1.014555× → 1.012793× | 0.998264× | 1.008520× |
| Natural 256 KiB | 1.141322× → 1.134006× | 0.920418× → 0.914518× | 0.993590× | 0.999729× |
| Natural 1 MiB | 1.144754× → 1.140846× | 0.927973× → 0.924805× | 0.996586× | 1.002613× |
| Held-out slides, 65,537 bytes | 1.143682× → 1.143294× | 1.001455× → 1.001115× | 0.999661× | 1.006769× |
| Periodic 256 KiB | 0.568581× → 0.567058× | 1.002726× → 1.000040× | 0.997322× | 1.000267× |
| Random 1 MiB | 1.173872× → 1.166167× | 1.657510× → 1.646630× | 0.993436× | 0.996513× |
| Repeated, 65,537 bytes | 0.557303× → 0.557336× | 0.984071× → 0.984129× | 1.000059× | 1.001111× |

Each cohort keeps its own peer denominator. The shared-minimum cohort's unmodified control is 1.070758× Roc on natural 8 KiB, versus 1.005272× in the search cohort. The shared-minimum candidate is 1.072935× against that control's peer. Its natural 32 KiB case slows 0.85%, and held-out slides slows in both order halves. Preserve these observations separately from the search-only improvement; do not pool them.

Both pilots start at `77a4d5cdc976460adf21eb709f0edf08787321b0`, holding the pinned source, revision-14 bytecode, allocator, startup descriptor, capacities, runtime inputs and full compression boundary fixed. The retained vector matcher has four pure unsigned-minimum diamonds after the original tree stores and child loads. Its true and false arms deliver exactly the compared operands to the same join. The pinned Roc body uses a conditional select in this part of the descent. The libdeflate excerpt uses a different state layout; it is an independent machine control.

Each candidate replaces only the vector matcher and preserves every linked text address. The unmodified Cranelift output matches all 10,188 original matcher bytes before replacement. Selecting all four minima locally would grow the body to 10,264 bytes, outside the 10,240-byte slot, and raises the static stack reference count from 437 to 482. The search-only version computes the minimum on each original search arm and jumps to its typed continuation, reducing the body to 10,160 bytes and the static count to 436. The shared version forwards only the existing left/right prefixes on both incoming edges and calculates their minimum after the stop join. The redundant result parameters disappear, but the body grows to 10,224 bytes and 478 static stack references. Both keep the original 576-byte frame; the counts include cold code and are not dynamic spill counts. Other branch and merge variants were not relinked; their preflight measurements are retained.

The search-only trial retires 0.998366×, 0.997035× and 0.996476× instructions on natural 8 KiB, 32 KiB and 256 KiB. Its paired cycle ratios in the separate cohort are 0.985032×, 0.995525× and 0.998211×. On repeated 65,537 bytes instructions rise to 1.002711×. The shared version raises 256 KiB instructions to 1.000784× and cycles to 1.005351×. On slides, it takes 1.007829× cycles; the search trial takes 0.995048×. Eight literals is 1.00275× in both elapsed screens and stays under 0.175× Roc; the single-call counter cohorts show +2.95% and +4.82% cycles there and are not the primary timing result.

The exact candidate profiles assign 1,246 of 4,428 running timer PCs to the search-only matcher on natural 8 KiB and 2,235 of 5,354 on slides. The shared-minimum candidate assigns 1,354 of 4,799 and 2,287 of 5,361, respectively. These samples include validation. The two screens each preserve 7,584 balanced calls and the six payload distributions. Each linked diagnostic passes 162 complete checks with exact reference bytes, independent inflate, intact aliases and cleanup. Initial harness setup used the wrong allocator metadata key and did not launch any timed call; restoring the pinned key fixed it. The first shared-merge IR preflight left an undefined SSA name in an unreachable block. Keeping that definition fixed the reader rejection before linking or timing.

Keep the production compiler at the prior baseline. The search-only experiment is a measured lead, but none of the 16 cases changed parity and its effect is smaller than the code opportunity on the same recurring route. Test forwarding the existing stop and continue edges, which can remove the flag transport without changing either branch's stores. Ordinary full compiler emission and the 81-case acceptance matrix were not run for these two diagnostics.

[Replay both matched screens, hardware counters, profiles and linked code](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-77a4d5cdc9-matcher-prefix-minimum.tar.gz). SHA-256 `55fa5a9fb97a1d12d5c2ea86d9deef2c79a94409fa2f623971787706c34b97ba`. Run `python3 -B verify-round.py`; all 189 files were verified from the downloaded archive.
