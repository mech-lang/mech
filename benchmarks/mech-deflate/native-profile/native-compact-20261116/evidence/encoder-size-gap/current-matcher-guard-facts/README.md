# Current matcher guard facts and layout

The selected recipe is **`m1-matcher-guard-facts`**: **63,692 B encoder text**, **81/81 Roc and 62/81 libdeflate wins in both complete orders**, with every previous winning fixture and all 81/62 precision-supported wins held. It combines allocator spill pairing with two generic impossible-guard proofs and a fresh matcher layout. It saves **64 B** from the qualified paired recipe and **112 B** from the earlier 63,804 B reference. Original Mech source and canonical bytecode are unchanged.

The actual fixed-support worker is `88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4`. The checked root stays **46,788 B**, frame **3,296 B**, peak active stack **4,848 B** under the unchanged 5,280 B cap. Stripped deployment stays **276,072 B**. All 162 output/inflation/alias observations, six failures, 12 recoveries and ten unsigned-guard tests pass. Normal and allocation-checked compilers reproduce identical artifacts. No LLVM dependency is added.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Prior selected, same compiler | 63,756 B | 2.211203 s | 2.222270 s | 276,072 B |
| Current matcher facts + layout | 63,692 B | 2.214612 s | 2.225813 s | 276,072 B |
| Roc | 57,696 B | 2.983165 s | 2.994082 s | 191,912 B |
| libdeflate | 37,080 B | 0.480880 s | 0.492001 s | 155,752 B |

Eight balanced fresh-process builds per actor include final linking, with fixed support prebuilt for every actor. Mech complete-build time is **0.74294× Roc / 4.61093× C**. It is **1.000656× the same-compiler reference**: a paired **1.453 ms cost**, 90% interval **-0.561–5.621 ms**, 3/8 faster. This is not a demonstrated build-time gain. Do not compare separate cohort medians as a causal improvement.

The encoder is **1.717691× C / 1.103924× Roc**, still **26,612 B** above C. The broader 1× libdeflate code and build-time goal remains unfinished. There is **no 63rd C win**.

## Runtime and remaining costs

| Case | Current/reference, forward / reverse |
| --- | ---: |
| `random-1024` | 0.973334× / 0.975276× |
| `prefix-random-1024` | 0.972567× / 0.974160× |
| `prefix-random-4096` | 0.966632× / 0.969427× |
| `prefix-random-8192` | 0.968930× / 0.964531× |
| `prefix-random-1048576` | 0.980341× / 0.980292× |
| `prefix-natural-4096` | 0.999014× / 0.998583× |

The 1 KiB slowdowns reported for the prior pairing-only pass are recovered. There are **no established slowdowns** against that immediate reference; three comparisons remain uncertain in the v3 replay. Every individual peer win is retained. The numbers above belong to the **combined guard proof and fresh layout**; their separate effects have not been measured.

Natural 4 KiB remains the closest C loss at **1.055687× / 1.065612× C** in this cohort, requiring about **6.2%** less Mech time to win both point comparisons. The [case table](runtime-case-ratios.json), complete raw orders and statistical replay contain every case. Keep it as the next-win check while optimizing the shared large-random collector path identified by the [19-mechanism history audit](../../history-and-profiles-20261119/README.md). Those sample profiles bind the old 63,804 B worker, not this new worker.

On 1 MiB random input, actual untimed matcher counters change **241,346,097 → 233,154,478** original instructions, **45,024,643 → 41,017,489** branches and **9,698,800 → 8,653,030** stack-store instructions. Stack-load instructions remain **22,267,325**. This is 8.19 million fewer instructions, including 4.01 million fewer branches and 1.05 million fewer stack stores. Actual full-worker time improves about 2% on that case, and about 3–4% on random 4/8 KiB. The counter clones are never used for latency.

The natural 4 KiB matcher trace grows slightly, from 1,508,125 to 1,512,193 instructions, while full-worker latency is roughly unchanged. That is retained under the peer-win rule. Do not reject a small instruction or elapsed-time increase without checking its real peer results.

## Source proof and layout preparation

The existing source compiler's `MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS=1` proves two impossible failure edges in the current private vector collector. It preserves operations, debits, memory-access and first-error ordering. Tests cover overflow, signed/shared entries, narrow loads and modular intervals. Mech source and bytecode are unmodified. The unprofiled body falls from 6,176 B to 6,096 B and matched whole text falls by 80 B; valid final layout makes the whole saving 64 B.

Old matcher offsets are not reused. Untimed block counters train the new source-emitted layout on six declared cases, normalized by each trace's peak visits. All 581 edges are resolved on the random traces; 577/581 are resolved on the natural/periodic traces. The four remaining flows use the historical matcher's documented residual-edge estimates only to schedule blocks. Predictions are not elapsed measurements. Both other profiles have identical actual ordered graphs.

The first scorer-only edge solver failed on the ambiguous matcher flows and is retained. The first offline refinement recomputed a whole-layout score for every local move and was interrupted. The replacement uses affected-edge deltas, checking accepted improvements with the full score; it changes profile preparation, not compiler/build timing. The actual shipping object is source-compiled. Diagnostic counters alone instrument cloned instructions.

Current phase medians are **0.707648 s parse**, **0.214311 s source semantics**, **0.006829 s artifact creation**, **0.002540 s encoding** and **1.218448 s native emission**. Final link is **0.036247 s**, strip **0.010855 s**. Catalog join wait is 22 microseconds; catalog work runs during parsing. Independent medians need not add. Compiler building is outside this source-through-link boundary.

## Reproduce and verify

From `native-compact-20261116`, build from this checkout:

```sh
python3 rebuild.py /tmp/mech-m1-matcher-guard-facts \
  --variant m1-matcher-guard-facts --mimalloc-object /path/to/mimalloc.o
```

From this evidence directory, an existing source-built compiler can reproduce both normal and allocation-checked artifacts:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-matcher-guard-replay
python3 verify.py
```

The verifier checks artifact and graph bindings, all prior fixture wins, both full statistical orders, independently recalculated raw paired ratios, stack/recovery and eight matched complete builds per actor. A differently linked support worker needs its own runtime qualification.

Source base: `5a89f646664b86a0b808137c01c7627b2142552a`. Compiler SHA: `6711160f915f00bb783831fc14f0144ba7949c06ea9df264edd727a62d44e715`. Object SHA: `470e6899a3cfec1e2fc0f9d3d8aef1d427de7d621e6b9ec87b350cb6103d9b38`. The [compiler producer and source equivalence](source-equivalence.json) bind the source implementation, not just the built artifacts. All adverse results, raw counters, actual worker checks, preparation scripts and layout models are archived here.

Keep narrowing remaining checked collector live ranges and tracing hot word/root/link bounds; 22.3 million stack-load instructions remain on 1 MiB. The secondary scorer cursor/countdown exit-pointer repair and the early codebook architecture remain available for measured repair.
