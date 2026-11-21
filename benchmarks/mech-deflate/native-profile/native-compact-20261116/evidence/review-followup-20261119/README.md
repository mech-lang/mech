# Review follow-up: finalist precision and remaining work

**The qualified build is 63,388 B, with 81/81 Roc and 62/81 libdeflate wins. There is no 63rd C win. Both finalists remain useful.** The [full qualification](../encoder-size-gap/word-complement-current-read-study/README.md) checks every prior individual win in two complete 96-block orders. This study adds the requested direct comparison and counters; it does not substitute a 19-case panel for full qualification.

The original Mech codec and canonical bytecode are unchanged. Shipping objects come from the Mech compiler with Cranelift. Conditional machine-code instrumentation below is confined to untimed diagnostic clones.

## Both finalists in one cohort

The 63,532 B control, 63,388 B word/complement, 63,628 B vector/narrow, Roc and C run together in every block. Each workload uses all **120 permutations of the five actors**, in both complete case orders. The representative 19-case panel includes small inputs, natural and random sizes, boundaries, distribution change, and fragile overlap/periodic wins. All timed output, inflation, alias, allocation and cleanup checks pass. `finalists/precision.json` contains paired estimates and 90% intervals from the raw blocks, stratified by numerator/denominator order.

| Case | Word/C, forward / reverse | Vector/C, forward / reverse |
| --- | ---: | ---: |
| Natural 4 KiB | 1.037325 / 1.029387 | **1.031229 / 1.021762** |
| Random 4 KiB | 1.640012 / 1.626320 | **1.582781 / 1.609595** |
| Random 8 KiB | 1.656806 / 1.661203 | **1.611756 / 1.605645** |
| Distribution change | 1.346644 / 1.346699 | **1.337727 / 1.338156** |
| Periodic 1 KiB | **0.962602 / 0.967993** | 0.981316 / 0.989645 |
| 320-overlap | **0.989051 / 0.988764** | 0.997519 / 1.000442 |

Word/vector on natural 4 KiB is **1.005912×** (interval 1.001467–1.010307) forward and **1.007463×** (1.004247–1.010792) reverse. Vector/narrow is faster here in this cohort; the earlier separate C ratios did not establish the opposite ranking. Word/complement retains the size and fragile-win advantages. Vector/narrow's reverse overlap result is uncertain around parity (0.998664–1.002287 interval), so it is a repair candidate rather than a promoted full-score replacement. Keep both.

These combined recipes already include DP forwarding and compared-byte reuse. Their gains cannot be added to the earlier isolated gains. The full word qualification and this panel have different cohorts; report each separately.

## Adverse secondary run retained and repeated

The original **word-reuse/complement, random 1 MiB, reverse** run remains archived, all 24 blocks included. Its **1.015314× Roc** ratio and sharp regime change are reproduced in `original-regime-change.json`. This is worker `3625b042…`, distinct from the qualified word-current worker `fab30315…`.

The exact original worker and complete original 19-case reverse order were repeated with 24 balanced blocks. Random 1 MiB measured **0.960743× Roc / 1.361828× C**; median candidate 27.657 ms, Roc 28.787 ms and C 20.337 ms. The earlier 60 ms jump did not recur. Original and repeat remain separate evidence; no blocks were removed. Power/thermal/load snapshots accompany the repeat. This does not retroactively qualify that alternative or erase the original adverse result.

## Incremental first-word probe

Compiler commit `ce5abdb0d13cae82cf01f4935a43ab667971ce0c` adds an admitted 8-byte probe before vectors. Failure forwards the already loaded XOR into the original mismatch continuation; success advances eight bytes before the vector tier. The production native tests cover 2,577,920 comparisons and refusal cases. The full repository feature build now passes all **10 relevant wide-equality tests**, including the first-word paths.

The diagnostic conditional counts are per probe, not vector iterations:

| Case | Probes | Immediate word mismatches | Continue into vectors | Short scalar tails |
| --- | ---: | ---: | ---: | ---: |
| Natural 1 KiB | 710 | 424 | 285 | 1 |
| Natural 4 KiB | 5,562 | 2,983 | 2,579 | 0 |
| Natural 8 KiB | 12,086 | 8,170 | 3,916 | 0 |
| Random 1 MiB | 1,962 | 1,962 | 0 | 0 |
| Distribution change | 449 | 449 | 0 | 0 |

Random 4/8 KiB does not reach the admitted word probes. Each case executes twice with matching raw counters and exact-output/inflation/alias checks. The following remaining-length guard separately counts vector entry and short tails. Profiled first-word alternatives pass output/failure/stack gates at 63,740 B, 63,756 B and 63,692 B; their latency remains unmeasured here. Compare each against the corresponding vector recipe with other features fixed before making a performance claim.

Commit `00765f3be` fixes the review's no-state issue: `word-reuse` leaves IR unchanged when no saved interval is admitted, and reports no reuse. The repository strategy test checks exact graph equality on that miss. The source-only word strategy remains available. The corrected compiler now reproduces the exact qualified artifacts and linked worker in all four word/word-reuse and normal/checked combinations, with no reuse metadata.

## Scorer winner frequency

On the exact qualified scorer body, a conditional counter observes whether the final vector argmin replaces the existing winner. This is per **completed vector reduction**, not per SIMD chunk:

| Case | Reductions | Vector replacements | Fraction |
| --- | ---: | ---: | ---: |
| Natural 1 KiB | 440 | 153 | 34.77% |
| Natural 4 KiB | 3,550 | 1,449 | 40.82% |
| Natural 8 KiB | 6,036 | 2,516 | 41.68% |

The measured random and distribution-change cases execute no vector reductions. A cost-only reduction with later winner reconstruction therefore has a workload-specific budget: about 59% of natural-4-KiB reductions do not replace the winner, while about 41% still need a winner. It offers no direct saving on those random paths. Current code already defers horizontal argmin until the reduction ends; do not budget that work as though it happened every SIMD chunk.

## Histogram common-path target

Fresh counters on the qualified histogram show **90,136 instructions / 28,673 branches** for 4 KiB and **23,068,768 instructions / 7,340,036 branches** for random 1 MiB. The latter makes four calls and processes every byte once. The five defensive checks remain: position addition, input read, histogram read, U32 increment, and histogram write. Raw block instructions and counts are retained.

The next implementation connects an entry-checked input span, byte-valued indices, fixed histogram extent and unique writable ownership. The first stage should retain the U32 increment check unless zero initialization and a maximum increment count are actually proved. Proof misses must preserve the original checked path. Collector position/window facts, observation read/write checks and producer-defined scorer roles remain broader targets.

## Verification and limits

Run `python3 verify.py`. It replays both five-actor orders and every reported interval from raw observations, checks the original adverse ratio and the complete repeat, verifies raw conditional counter observations and histogram totals, and binds the 10-test result to its exact source. Acquisition scripts are included; their original local paths are provenance, not a claim that a new machine inherits the runtime score.

The relevant repository run is 10 passed tests, with 1,350 filtered out; the entire GPU suite is not claimed. The previous disk-exhausted attempt remains retained. The pre-existing missing allowlisted archived allocator continues to block the repository unsafe audit. No scalar entry, ownership check, error origin or resource accounting is retired by this work.
