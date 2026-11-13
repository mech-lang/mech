# Matcher comparison branch trials

The current matcher computes a shared stop decision from two integer comparisons. It combines the complement with three further comparisons for a branch and uses the original stop decision again on a later continuation. The pinned executable profiles showed a recurring subtraction in that branch. The trials retain the shared word. One exposes an I8 conjunction for the AArch64 conditional-compare lowerer. The other adds three false edges through a single-entry sequence of total integer operations and branches on the last comparison. The original selected blocks and their effects are preserved.

The old fixed-address worker was the first control. The second comparison reuses the normal layout of the published executable at `e885dd503cbc1fa52185773d6e7c8d936a0489d7`, with every address and byte outside the matcher body, signature and UUID verified. The linker retains the same startup metadata. The control matcher is 9,976 bytes with 413 static stack references. The conditional-compare trial emits 9,940 bytes and three additional conditional compares. Short-circuiting emits 9,956 bytes and the original conditional-compare count. Both have 412 static stack references and retain the 576-byte complete frame. An unmeasured form that duplicates the shared comparisons emits 10,144 bytes, 457 stack references and a 592-byte complete frame.

| Trial and cohort | Cases | Mech / paired control | Roc wins | libdeflate wins | Both wins | Either win |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Short-circuit, fixed screen | 18 | 0.993575× | 4 → 5 | 10 → 12 | 2 → 2 | 12 → 15 |
| Conditional compare, fixed screen | 18 | 0.996346× | 4 → 5 | 12 → 12 | 2 → 2 | 14 → 15 |
| Short-circuit, published layout | 81 | 0.997813× | 53 → 54 | 28 → 28 | 11 → 11 | 70 → 71 |
| Conditional compare, published layout | 81 | 0.996182× | 53 → 54 | 28 → 27 | 11 → 11 | 70 → 70 |
| Short-circuit, confirmation | 8 | 0.989534× | 1 → 2 | 1 → 4 | 0 → 0 | 2 → 6 |
| Conditional compare, confirmation | 8 | 0.994303× | 1 → 2 | 4 → 3 | 0 → 0 | 5 → 5 |
| Short-circuit, adverse check | 10 | 0.999930× | 6 → 6 | 6 → 6 | 3 → 3 | 9 → 9 |
| Conditional compare, adverse check | 10 | 0.998465× | 6 → 6 | 6 → 6 | 3 → 3 | 9 → 9 |

The win counts use fresh peer timings in each cohort. The published historical control counts were 53 Roc, 27 libdeflate, 12 both and 68 either. The isolated full comparisons each have their own paired control. Both timing gates pass in every cohort.

| Input | Peer | Short-circuit control → trial, full | Short-circuit trial, confirmation | Conditional-compare control → trial, full |
| --- | --- | ---: | ---: | ---: |
| Held-out random 65,537 | Roc | 1.011953× → 0.992046× | 0.989139× | 1.011586× → 0.998684× |
| Held-out mixed 131,073 | Roc | 1.011944× → 1.003434× | 1.000604× | 1.011714× → 1.001953× |
| Natural 33,027 | libdeflate | 0.997126× → 0.997193× | 0.999341× | 0.999870× → 1.002932× |
| Random 1 MiB | Roc | 1.141062× → 1.106722× | 1.107635× | 1.138866× → 1.117810× |
| Eight literals | Roc | 0.171110× → 0.171109× | 0.170889× | 0.170506× → 0.171613× |
| Eight literals | libdeflate | 2.179017× → 2.178995× | 2.190129× | 2.179289× → 2.193438× |

Short-circuiting advances to maintained compiler qualification. In the separate counter cohort, its instruction and cycle ratios to control are 0.967274× and 0.984269× on held-out random, 0.964810× and 0.972627× on random 1 MiB, and 0.988733× and 0.993131× on held-out mixed. Peak physical footprint ratios are 1.000708×, 1.000619× and 1.000704×. Eight literals has a 1.054513× cycle ratio with 0.999841× instructions and neutral full elapsed. The largest full elapsed estimates against control are +4.06% for seven stored bytes and +2.03% for repeated 8 KiB. Periodic 8 KiB is +0.76% in the adverse check. The conditional-compare trial loses natural 33,027 bytes in the full cohort and held-out window 32,767 in confirmation. Those cases, both order halves, and every other adverse observation are in the replay.

The first published-layout wrapper stopped after 162 successful linked checks because its write-once receipt helper was asked to replace an existing receipt. The retry attached the control to the existing verified worker and started the first timing. The two published-layout cohorts replay 534,528 timed calls, 324 linked checks, eight exact-worker profiles and 16 separate counter cases. The fixed-address archive replays 15,552 timed calls, 324 linked checks and eight profiles. SHA-256 `502aab1e9e1f896e0c861664ceaa038f3f89f4c06e2bc1c7ba9b692df274c725` and `fa5321c84fd8ac0b9eb5bdfe114b557055ca5b7e5aee66460b2a3a4885c901a8`, respectively. [Published-layout evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-e885dd503c-matcher-branch-qualification.tar.gz), [fixed-address evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-e885dd503c-matcher-branch-screen.tar.gz).
