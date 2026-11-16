# R268 — Thread numeric phases after isolating growth

**Retain the growth-isolated threaded candidate for the compact executable.**
Paired elapsed changes -3.59045% across the fixed 81 cases. Cases faster than Roc move from 22/81 (27.16%) to 26/81 (32.10%) for the candidate.

PR base `92f75db84e2523693648084bea16cd52ad60777a`. Published parent `fd529dd01f388828d079e5bf6f26c398eacb67a7`. Measured parent compiler R264 `9676a9cbad760dfc44f9b69aee746e1ca9080e13`. IR/work/ABI/bytecode 215/107/39/14. Cranelift 0.131.3 and regalloc2 0.15.2.

Measurements cover the compact executable's complete compression, scratch construction, output transfer and private cleanup. The ordinary managed admission, accounting and recovery contract lies outside that boundary.

## Frontier and safety gates

The independent 13-case screen established the named new condition from R260. The general growth branch is absent and the cache cursor stays in a register throughout the audited empty-match visit. Threading three proved pure edges with six local repair parameters cuts 61 to 50 instructions and keeps both remaining stack transfers. The no-growth body shrinks from 1,424 to 1,376 bytes with a 240-byte frame. The general phase retains growth and the ordinary execution path keeps its own admission and resource semantics.

In that screen, repeated 32,768 bytes reached 0.994544× Roc, against 1.005465× for no growth alone in the same blocks. The separate full comparison below uses the retained R264 baseline and fresh reference observations.

- Two primary cohorts change -3.49875% and -3.68206%.
- The pooled and block gates pass 81/81 and 81/81.
- Cases moving below 1× repeated-32769, prefix-repeated-32767, prefix-repeated-32768, prefix-repeated-32769.
- Cases moving back to 1× or above none.
- Pooled gate misses none. Block gate misses none.

| Case | R264 / Roc | Candidate / Roc | Elapsed vs R264 | Instructions vs R264 | Cycles vs R264 |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.429721× | 0.426573× | -0.733% | -0.063% | -0.030% |
| prefix-repeated-32767 | 1.019817× | 0.993460× | -2.584% | -8.346% | -3.129% |
| prefix-repeated-32768 | 1.020098× | 0.986508× | -3.293% | -8.348% | -3.807% |
| prefix-repeated-32769 | 1.021545× | 0.992240× | -2.869% | -8.275% | -3.408% |
| prefix-repeated-33027 | 1.034875× | 1.010755× | -2.331% | -8.191% | -3.807% |
| prefix-periodic-32768 | 1.049861× | 1.020948× | -2.754% | -8.329% | -3.328% |
| prefix-natural-32768 | 1.817045× | 1.679269× | -7.582% | -12.426% | -7.818% |
| prefix-natural-1048576 | 1.888199× | 1.753719× | -7.122% | -11.924% | -7.085% |
| prefix-random-1048576 | 2.163206× | 2.118901× | -2.048% | -10.947% | -2.285% |

Each primary cohort uses 24 balanced blocks per case with the same six actor orders and matched 200 ms warmups. Ratios are formed within each block. Hardware CPU and heap counters each have separate 81-case cohorts. The measured Mech heap payload, copying, zeroing and private peaks agree across all 81: True. The comparison retains the same pinned source, bytecode, tables, capacities, search limits, allocations, output reuse and timing boundary.

## Qualification

- 727 native tests passed with seven inherited exclusions and warnings as errors. Seven tests exercise the reused edge-threading proof.
- Both original and decoded complete images have identical Object bytes and metadata. Both equal the screened image. All 324 complete smokes agree with pinned reference bytes and independent inflate and preserve aliases and cleanup.
- Two artifact forms pass the existing 40 path cases, seven focused capacity and failure controls, and two shared-owner cases each. Real growth, injected denial on the second payload request, selected assertion, prior publication and retry agree with the independent controls.
- Prior compact entry observations cover 404 current path entries per matrix, each with actual capacity 305,257 and a unique owner. The new experiment changes edge emission inside that selected body.
- Coverage is Apple ARM64. Hosted CI, the full linked/AOT integration suite, and the full 6.6 MB input are outside this receipt.

The historical fixed-placement experiment and the exact register-allocation trace remain linked. [Fixed-placement control](native-empty-path-placement-control.md) · [Growth-state proof and allocator trace](native-loop-state.md).

## Exact-image CPU sampling

| Case | Running samples | Numeric phases | Root |
|---|---:|---:|---:|
| eight-literals | 5364 | 15.03% | 16.22% |
| prefix-natural-1048576 | 5361 | 97.61% | 1.29% |
| prefix-natural-32768 | 5363 | 95.19% | 2.65% |
| prefix-periodic-1048576 | 5347 | 96.73% | 1.29% |
| prefix-periodic-32768 | 5351 | 89.68% | 1.38% |
| prefix-random-1048576 | 5372 | 98.62% | 0.30% |
| prefix-random-32768 | 5364 | 93.59% | 2.05% |
| prefix-repeated-32768 | 5501 | 91.46% | 1.56% |

The sampling cohort uses the exact linked candidate after the elapsed and counter cohorts. CPU-time sample shares include harness work.

## All 81 cases

| Input | Bytes | R264 / Roc | Candidate / Roc | Candidate / R264 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.112322× | 2.129693× | 1.008224× | pass/pass |
| one-stored | 1 | 2.014668× | 2.049372× | 1.017226× | pass/pass |
| seven-stored-prefix | 7 | 2.222783× | 2.273757× | 1.022933× | pass/pass |
| eight-literals | 8 | 0.429721× | 0.426573× | 0.992673× | pass/pass |
| eight-overlap | 8 | 0.422248× | 0.428712× | 1.015310× | pass/pass |
| 320-overlap | 320 | 0.457726× | 0.461384× | 1.007992× | pass/pass |
| eight-literals-prefix | 8 | 0.429026× | 0.435268× | 1.014548× | pass/pass |
| 128-biased-dynamic | 128 | 0.425276× | 0.421585× | 0.991322× | pass/pass |
| 32-high-entropy-stored | 32 | 0.435051× | 0.435870× | 1.001882× | pass/pass |
| 16384-distribution-change | 16384 | 1.869167× | 1.724696× | 0.922709× | pass/pass |
| 33027-periodic-window | 33027 | 1.065031× | 1.030446× | 0.967527× | pass/pass |
| random-64 | 64 | 0.444788× | 0.445606× | 1.001837× | pass/pass |
| random-256 | 256 | 0.488920× | 0.484026× | 0.989991× | pass/pass |
| random-1024 | 1024 | 0.544483× | 0.540001× | 0.991768× | pass/pass |
| repeated-1024 | 1024 | 0.524111× | 0.516486× | 0.985451× | pass/pass |
| random-32769 | 32769 | 1.789731× | 1.736482× | 0.970247× | pass/pass |
| repeated-32769 | 32769 | 1.026695× | 0.993748× | 0.967909× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.689215× | 0.660951× | 0.958992× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.745703× | 1.599310× | 0.916141× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.817414× | 1.675745× | 0.922049× | pass/pass |
| heldout-window-32767 | 32767 | 1.815460× | 1.675441× | 0.922874× | pass/pass |
| heldout-window-32768 | 32768 | 1.814944× | 1.675837× | 0.923355× | pass/pass |
| heldout-slides-65537 | 65537 | 1.891850× | 1.757500× | 0.928985× | pass/pass |
| heldout-random-65537 | 65537 | 1.958407× | 1.908600× | 0.974567× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.862721× | 1.757055× | 0.943273× | pass/pass |
| prefix-natural-1024 | 1024 | 0.686092× | 0.657827× | 0.958804× | pass/pass |
| prefix-natural-4096 | 4096 | 1.304825× | 1.203461× | 0.922316× | pass/pass |
| prefix-natural-8192 | 8192 | 1.569729× | 1.432782× | 0.912758× | pass/pass |
| prefix-natural-16384 | 16384 | 1.745651× | 1.603407× | 0.918515× | pass/pass |
| prefix-natural-32767 | 32767 | 1.818549× | 1.679393× | 0.923480× | pass/pass |
| prefix-natural-32768 | 32768 | 1.817045× | 1.679269× | 0.924176× | pass/pass |
| prefix-natural-32769 | 32769 | 1.813268× | 1.674958× | 0.923724× | pass/pass |
| prefix-natural-33027 | 33027 | 1.820390× | 1.678133× | 0.921854× | pass/pass |
| prefix-natural-65535 | 65535 | 1.867524× | 1.716194× | 0.918968× | pass/pass |
| prefix-natural-65536 | 65536 | 1.862284× | 1.713453× | 0.920082× | pass/pass |
| prefix-natural-65537 | 65537 | 1.896491× | 1.745632× | 0.920453× | pass/pass |
| prefix-natural-131072 | 131072 | 1.870897× | 1.719727× | 0.919199× | pass/pass |
| prefix-natural-262144 | 262144 | 1.885111× | 1.735765× | 0.920776× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.888199× | 1.753719× | 0.928779× | pass/pass |
| prefix-random-1024 | 1024 | 0.562871× | 0.545779× | 0.969635× | pass/pass |
| prefix-random-4096 | 4096 | 0.943460× | 0.917753× | 0.972753× | pass/pass |
| prefix-random-8192 | 8192 | 1.173249× | 1.136996× | 0.969100× | pass/pass |
| prefix-random-16384 | 16384 | 1.528447× | 1.489129× | 0.974276× | pass/pass |
| prefix-random-32767 | 32767 | 1.802395× | 1.764016× | 0.978706× | pass/pass |
| prefix-random-32768 | 32768 | 1.804345× | 1.766784× | 0.979183× | pass/pass |
| prefix-random-32769 | 32769 | 1.801818× | 1.762660× | 0.978267× | pass/pass |
| prefix-random-33027 | 33027 | 1.806107× | 1.771789× | 0.980999× | pass/pass |
| prefix-random-65535 | 65535 | 1.986127× | 1.934461× | 0.973987× | pass/pass |
| prefix-random-65536 | 65536 | 1.984304× | 1.934576× | 0.974940× | pass/pass |
| prefix-random-65537 | 65537 | 1.976151× | 1.923476× | 0.973345× | pass/pass |
| prefix-random-131072 | 131072 | 2.072050× | 2.025685× | 0.977624× | pass/pass |
| prefix-random-262144 | 262144 | 2.127622× | 2.079640× | 0.977448× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.163206× | 2.118901× | 0.979519× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.539825× | 0.530165× | 0.982105× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.734803× | 0.725732× | 0.987655× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.866716× | 0.830662× | 0.958402× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.950265× | 0.925808× | 0.974263× | pass/pass |
| prefix-periodic-32767 | 32767 | 1.053669× | 1.024417× | 0.972238× | pass/pass |
| prefix-periodic-32768 | 32768 | 1.049861× | 1.020948× | 0.972460× | pass/pass |
| prefix-periodic-32769 | 32769 | 1.060703× | 1.028582× | 0.969717× | pass/pass |
| prefix-periodic-33027 | 33027 | 1.066881× | 1.032813× | 0.968068× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.138051× | 1.102140× | 0.968445× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.137259× | 1.097941× | 0.965427× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.130310× | 1.095941× | 0.969593× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.179963× | 1.140272× | 0.966363× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.204608× | 1.164925× | 0.967057× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.223413× | 1.182641× | 0.966673× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.524648× | 0.510170× | 0.972405× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.723758× | 0.700720× | 0.968169× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.827273× | 0.802430× | 0.969971× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.915090× | 0.894517× | 0.977518× | pass/pass |
| prefix-repeated-32767 | 32767 | 1.019817× | 0.993460× | 0.974155× | pass/pass |
| prefix-repeated-32768 | 32768 | 1.020098× | 0.986508× | 0.967072× | pass/pass |
| prefix-repeated-32769 | 32769 | 1.021545× | 0.992240× | 0.971312× | pass/pass |
| prefix-repeated-33027 | 33027 | 1.034875× | 1.010755× | 0.976693× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.104436× | 1.068047× | 0.967052× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.103484× | 1.067621× | 0.967500× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.105340× | 1.068090× | 0.966300× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.149778× | 1.111162× | 0.966415× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.170241× | 1.133738× | 0.968807× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.178417× | 1.140929× | 0.968188× | pass/pass |

[Verified compiler closure and full measurements](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r268-thread-after-growth-isolation-c0f0679e0fe2.tar.gz). SHA-256 `c0f0679e0fe2bd769b04d051333be39ad6b6144b488d269a0daac2baa484197c`.
