# Guarded byte histogram: qualified throughput improvement

**63,612 B, retaining every prior individual win: 81/81 Roc and 62/81 libdeflate.** Random 1 MiB takes **2.44–2.73% less time** than the same-run 63,388 B reference. Natural 4 KiB improves 0.30–0.60%; it remains above C, so there is no 63rd C win. The 224 B cost buys a measured improvement across larger inputs. The smaller 63,388 B recipe remains available.

Compiler source is `7ea6c80d8db874e10392bdb2a3d38e6205ca876d`; compiler `08e3c13132397e7f7e78d358ef06ee35dc320c203b32ec10eea2e96ad18020c8` was acquired from the preceding commit plus the exact five-file overlay subsequently committed there. Both source forms are hash bound. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency or codec rewrite is introduced.

## What changed

The compiler recognizes a unit-stride byte histogram with fixed current views and unique writable ownership. One entry guard checks the entire input span and at least 256 histogram bins. A specialized clone then removes four per-byte checks: position addition, input bounds, histogram-read bounds and histogram-write bounds. An unsuccessful guard enters the original checked function before source effects. The per-byte **U32 counter overflow check remains**: zero initialization and a maximum increment count have not been assumed.

The proof has fixed work bounds and requires exact packet words, the original induction recurrence, unsigned byte loads and the same histogram extent. Other strides, shifted indices, changed limits and proof misses leave the graph unchanged. The tests compare old/new results, partial histogram contents and error origins across **11,760 native calls**, including short views, wrapping positions and counters at zero, MAX−1 and MAX.

The diagnostic fast body executes **13,631,584 instructions / 3,145,732 branches** on random 1 MiB, versus **23,068,768 / 7,340,036** before. These counts exclude the new fixed entry wrapper and do not forecast whole-encoder time. All diagnostic objects are separate from the source-built timing worker.

## Runtime and confidence

The exact worker is `a2f692f524448d7b1c972abf5f1cc92ac412c1fcd1424ce4ade4b9bae526481c`. Two 81-case point screens retained 81 Roc / 62 C wins, followed by complete **96-block forward and reverse** orders. All C wins were confidence supported in both. The original reverse order had two uncertain Roc comparisons at natural 65,536/65,537 B, including one also uncertain for the control. All actors showed large timing regime changes; every observation remains archived.

The **entire reverse order** was repeated at **192 balanced blocks per case**. It independently supports all 81 Roc / 62 C wins. Pooling the original 96 and repeated 192 reverse blocks, stratified by acquisition and relative actor order, also supports every prior named win. Qualification therefore uses **forward 96 / pooled reverse 288**, with the original reverse uncertainty retained and visible. It does not silently replace or delete the adverse data. The verifier replays every individual required win from raw observations, rather than relying on aggregate counts.

| Case, original full cohort | Forward / C | Reverse / C | Candidate / same-run control |
| --- | ---: | ---: | ---: |
| Random 1 MiB | 1.325151× | 1.324508× | 0.972701× / 0.975622× |
| Natural 4 KiB | 1.027657× | 1.031202× | 0.994006× / 0.997030× |

The reverse repeat measures random 1 MiB at **1.326165× C**. Reaching parity on that input still needs roughly 24.5% less candidate time. Keep the broader goal separate from the next natural-4-KiB win.

All **162 output/inflation/alias checks, six checked failures, twelve injected failures and twelve successful recoveries** pass. Peak active stack remains **4,864 B**, below 5,280 B. Register-allocation-checked emission reproduces all three artifacts exactly. Every timed call verifies its output and aliases and the workers verify complete cleanup.

## Complete builds

| Actor | Encoder text | Source through final executable link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Same compiler, histogram guard disabled | 63,388 B | 2.217692 s | 2.228513 s | 276,072 B |
| Guarded histogram | **63,612 B** | **2.214983 s** | 2.225963 s | 276,072 B |
| Roc | 57,696 B | 2.991646 s | 3.002677 s | 191,912 B |
| libdeflate | 37,080 B | 0.481387 s | 0.492066 s | 155,752 B |

Eight balanced serial fresh-process trials include final linking. All compilers and fixed support are prebuilt equally; bootstrap and strip are separate. Every generated Mech object, complete metadata and bytecode is checked. Paired guarded-build ratios are **0.740659× Roc / 4.602734× C / 0.998456× disabled**. No established compiler-time improvement is claimed from this small difference. Runtime is the improvement here; executable size remains unchanged.

## Reproduce and continue

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/bounded-histogram-replay
```

From the benchmark root:

```sh
python3 rebuild.py /tmp/mech-bounded-histogram --variant m1-bounded-byte-histogram
```

Normal and checked modes must reproduce the exact artifacts. New hardware or different support requires its own timing qualification. The package retains all discovery alternatives, both full point screens, original full96 orders, reverse192 repeat, build trials, native tests, failures, recoveries, counters and exact compiler input overlays. The entire GPU suite is not claimed; the two relevant histogram tests passed. Earlier wide-equality tests remain separate evidence. The existing missing archived allocator continues to block the repository unsafe audit.

The named rebuild and the public normal/checked replay also reproduce all three artifacts from the current compiler. Their receipts and exact source overlay are in `current-source-replay/`: it contains an inactive experimental range utility beyond the histogram integration commit. That utility removes **zero checks from this codec** and contributes no claimed gain. The named command is an artifact reproduction check; its isolated source-to-object time is not the matched complete-build metric above.

The collector's 262,144-entry floor covers an already optimized history table. The remaining observation-counter checks use a separate small table and need their own extent proof or a read/write continuation proof. Producer-defined scorer roles, collector position/window relationships and a real zero-initialization/count proof remain further opportunities.
