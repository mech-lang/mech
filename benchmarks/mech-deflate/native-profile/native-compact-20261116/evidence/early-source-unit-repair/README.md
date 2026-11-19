# Source-unit repair, still experimental

Compiler source is `44fc9f05304111c6e75cac54a3dc5ec5c4f443bf` in PR #995.
The selected encoder remains **63,532 B**, with **81/81 Roc and 62/81
libdeflate wins in both complete orders**. Both compiler configurations,
normal and allocation-checked emission reproduce its exact object, complete
metadata and canonical bytecode. Linking the minimal configuration with the
fixed support reproduces qualified worker `834c7a7e…` exactly. Runtime
qualification is reused only for that identical worker.

The source-unit experiment now has source-relative extraction hints,
caller-owned ownership homes, shared numeric call packets, borrowed private
output scratch, and register preference scoped to the root. Hints only locate
candidates: extraction still proves the current control graph, ABI, ownership
and initialized inputs. Synthetic unit calls and returns do not masquerade as
ordinary source operations. No original Mech codec or canonical bytecode changes,
LLVM dependency, or machine-byte implementation are introduced.

| Same experimental compiler | Whole encoder | Root / frame | Helper / frame |
| --- | ---: | ---: | ---: |
| Selected encoder | 63,532 B | 46,788 / 3,296 B | — |
| Early unit, old native hints | 69,556 B | 48,672 / 3,424 B | 7,348 / 1,072 B |
| Source-relative hints | 66,204 B | 42,248 / 3,680 B | 6,288 / 1,120 B |
| Owner homes and shared packets | 65,964 B | 42,220 / 3,168 B | 6,056 / 944 B |
| Also shared output scratch | 65,964 B | 42,220 / 3,168 B | 6,052 / 816 B |
| Also root-scoped register preference | **65,948 B** | **42,220 / 3,168 B** | **6,044 / 800 B** |

The last case saves 3,608 B from the unrepaired experiment and reaches the
**5,280 B** active-stack ceiling exactly. It remains **2,416 B larger** than
the selected encoder. It passes all 162 output/inflation/alias observations,
six checked failures and 12 exact recoveries. Four source-boundary tests,
five complete production Words tests, five complete phase-storage tests and
nine scalar success/error probes pass. The standalone test harnesses use an
unexecuted panic adapter for the unrelated CodePlan constructor; they do not
prove that constructor.

The experiment has **no full runtime-win qualification**. Its 17-fixture
screen, with 96 balanced paired blocks in each order, keeps 17 Roc wins but
has 12 C wins against the control's 13. `320-overlap` changes from
0.995061 / 0.994957× C to **1.007444 / 1.009468× C**. Natural 4 KiB remains
1.063345 / 1.065092× C. Keep these gains and receipts available for repair;
the selected 81/62 encoder has not been replaced.

## Complete builds

Eighteen balanced fresh-process orders cover nine actors. Every actor has
its compiler and fixed support prebuilt; the measured boundary is original
source through final executable linking. Strip is separate. Every Mech trial
checks its complete object, metadata and canonical bytecode hashes.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Earlier compiler, selected encoder | 63,532 B | 2.241802 s | 2.252804 s | 276,072 B |
| Current minimal compiler, selected encoder | **63,532 B** | **2.245045 s** | **2.256023 s** | **276,072 B** |
| Experimental compiler, split disabled | 63,532 B | 2.215508 s | 2.226393 s | 276,072 B |
| Repaired early unit | 65,948 B | 2.282298 s | 2.293045 s | 276,072 B |
| Roc | 57,696 B | 2.993610 s | 3.004575 s | 191,912 B |
| libdeflate | 37,080 B | 0.491877 s | 0.502805 s | 155,752 B |

The current minimal compiler is a paired **1.058 ms slower** than its earlier
compiler, 90% interval **−8.222 to +19.025 ms**, 9/18 faster: no established
change. The experimental compiler configuration's apparent median advantage
also has an interval spanning zero. The repaired unit adds a paired
**66.899 ms**, interval **63.081–71.099 ms**, against the same experimental
compiler with the split disabled. No compile-time improvement is claimed.
Do not combine this cohort with earlier cohorts as causal savings.

The selected current build has paired ratios **0.739423× Roc / 4.540212× C**;
its encoder is **1.101151× Roc / 1.713376× C**. Phase medians are parsing
706.538 ms, semantics 225.121 ms, artifact preparation 6.921 ms, encoding
2.667 ms, native pipeline 1,226.643 ms and final linking 35.812 ms. Catalog
work overlaps parsing; separately computed medians do not add.

The four compiler captures include semantic preparation through exit, across
threads, including waits; sample counts are not exclusive elapsed timers.
The additional early-unit planning repeatedly computes full continuation
liveness. The corrected identical-body inventory finds one 296 B candidate;
it does not prove safe linker folding. A prior invalid inventory used absolute
Mach-O addresses as section offsets and must not be used as a size claim.

## Reproduce and inspect

Run `python3 verify.py` in this directory to check archive hashes, committed
source snapshots, emitted artifacts, six normal/checked replay receipts,
matched raw build medians and paired ratios, and both focused raw runtime
orders. This is a receipt verifier; it does not rerun the compiler or tests.

With a compiler built from the named source, run:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/source-unit-replay
python3 reproduce.py /path/to/compile_profile /tmp/unit-replay --unit
```

The unit replay needs the `early-region-emission` feature. The default replay
works with the minimal compiler. Both allocation modes must reproduce all
three pinned artifacts. Source snapshots, compiler build plans, test logs,
actual worker bindings, failure/recovery/stack receipts, raw focus observations,
compiler profiles and all nine actors' raw build records are archived here.
The existing missing archived allocator path still blocks the unsafe audit;
there is no CI-green claim.

Immediate runtime work targets natural 4 KiB's scorer. The next C win remains
unearned; preserve 81 Roc and every existing 62 C win while pursuing it.
