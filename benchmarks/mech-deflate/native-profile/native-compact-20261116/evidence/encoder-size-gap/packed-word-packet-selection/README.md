# Qualified packet/narrow-word selection

**Working selection: `m1-narrow-packet-words`, 63,804 B, 81/81 Roc and 63/81 libdeflate in both complete 96-block orders.** Every required named point win is retained, and all required confidence checks also pass in both directions. This saves 48 B versus the preceding 63,852 B working build and 64 B versus the original eight-entry 63-win build.

Natural 4 KiB measures **0.981214× / 0.992852× C**. The point-win requirement remains the selection rule; confidence is reported alongside it. The one narrow discovery loss did not discard this candidate; complete 96-block evaluation established all 63 named C wins.

Source through final executable link on Apple M1 takes **2.222473s**, paired **0.742624× Roc / 4.538331× C**, in the existing 18-trial matched cohort. Roc takes3.005122s and C0.490298s. Stripped executable **276,072B**; peak stack **4,736B**. Encoder text **1.106× Roc / 1.721× C**. No material compile-time gain is established. All compilers and fixed support are prebuilt equally; final linking is included and stripping/bootstrap are separate.

`rebuild.py` defaults to this recipe on Apple AArch64. Explicit selection: `--variant m1-narrow-packet-words`. Actual named-recipe normal/checked emission reproduces the measured object, full metadata and canonical bytecode exactly. [Current source, both full orders, every named win, confidence bounds and reconstruction](result.json). [Prior twelve-recipe study, tests, gates, counters and build evidence](../packed-word-repair-study/README.md).

Original 2,719-line Mech source and canonical bytecode remain unchanged; no LLVM dependency is added. The next milestone is **64/81 C**, preserving all 63 existing C names and 81/81 Roc; the full goal remains **1× libdeflate across all 81 cases**. The next major diagnostic is a current Mech/C phase-and-work comparison across the remaining losses, as requested in the new brief. Existing unsafe audit still has the unrelated missing archived allocator blocker.


## Both full81-case orders

| Order | Roc / C wins | Required names lost | Natural4KiB / C |90% interval |
| --- | ---: | ---: | ---: | ---: |
| forward96 |81 /63 |0 |0.981214 |-2.401% to-1.370% |
| reverse96 |81 /63 |0 |0.992852 |-1.326% to-0.071% |

All required point and confidence checks pass. Raw outputs, cleanup, artifact/metadata identities, fixture inventory and host validation have no finding. The complete96-block observations are preserved, including all per-case balanced blocks and source policy.

The already published twelve-recipe study binds this exact packet worker, its source at9b1460933, profiles, output/error/recovery/stack gates, normal/checked emission, source tests, dynamic instruction counts and18 balanced complete-build trials. This study references that immutable evidence by hash and adds the independent full81-case outcome and an exact named-recipe replay. It does not repeat or selectively pool the preceding timings.

The memory packet qualifier is restricted to the scorer family; the packed low-word simplification is unchanged. Normalization guards, private array witnesses, strict earliest winners and full stored items remain. The measured worker is `5d03ec0c3275ff61857dbe6e13433e8ea459b4c9e2b4eaa034abd60e2713e4d4`.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-packet --variant m1-narrow-packet-words
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/encoder-size-gap/packed-word-packet-selection/verify.py
```

The public verifier replays source/artifact bindings, actual named normal/checked emission, both full96 orders, all81 Roc/63 C required point wins and confidence outcomes, and binding to unchanged18-trial build evidence. Original source, canonical bytecode, checked errors/recovery and stack cap remain. Ordinary managed-entry admission/recovery and the entire GPU suite remain separate scopes.

The optional parallel low-word carry repair at source 89dd67efb passes 22 optimized tests in four configurations, including 30,720 new loop/multi-edge native calls per configuration, and 57 older recipes reproduce exactly. Its bounded two-order, 21-case evaluation takes 2.42–2.51% less time on random 1 MiB and retains observed wins, but adds no 64th C win. Full 81-case qualification remains pending and it is unselected. [Completed bounded study and new current Mech/C work attribution](../../current-phase-work-study/README.md).
