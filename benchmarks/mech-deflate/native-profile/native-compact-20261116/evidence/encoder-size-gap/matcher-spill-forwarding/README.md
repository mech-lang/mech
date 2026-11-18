# Reuse stack spill values in the match collector

Spill forwarding keeps a register value when it still matches the private spill slot. The smaller collector uses a block layout generated from the forwarded code. Layout weighting includes natural 4 KiB, natural 16 KiB, natural 32,769 bytes, periodic 1 KiB and random 4 KiB. Natural 8 KiB and 320-overlap are held out.

The opt-in encoder emits **65,516 B**, a saving of 96 B. Encoder text is **1.136× Roc** and **1.767× libdeflate**. Linked validation matched 162 successful observations and six checked failures. Peak stack is 4,784 B.

[Layout comparison](layout-comparison.json) · [Exact text](size.json) · [Linked checks](linked-checks.json)

## Compression runtime

| Order | Roc wins | libdeflate wins | Natural 4 KiB vs libdeflate | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 1.06557× | 0.98515× |
| Reverse | 81/81 | 62/81 | 1.08026× | 0.99109× |

V3 returned **Retain**. Both complete orders time through private heap cleanup with 96 balanced blocks per case.

[Replay](full-replay.json.gz) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

## Executed collector instructions

| Case | Instructions before | After | Stack stores before | After |
| --- | ---: | ---: | ---: | ---: |
| prefix-natural-4096 | 1,670,020 | 1,642,466 | 71,402 | 39,683 |
| prefix-natural-8192 | 3,448,168 | 3,392,587 | 144,295 | 80,364 |
| 320-overlap | 65,882 | 64,909 | 2,911 | 1,940 |

Instruction counts come from separate instrumented workers.

[Instruction counts](matcher-instruction-counts.json)

## Fresh source through final link

Mech takes **2.201 s**, or **0.740× Roc** and **4.586× libdeflate**, in eight balanced Apple M1 trials. Compilers and fixed deployment support are prebuilt.

[Measurements and phases](deployment.json) · [Raw builds](build-comparison.json.gz)

The SSA definition filter reproduced the qualified control object and worker. The median paired build was 2.1 ms slower.

[SSA timing and identity](ssa-definition-filter.json)

## Source replay

Build the compiler with the checked-load source patch and documented direct-source flags. Run this directory’s replay.py with that rebuild directory and a fresh output path.

[Compiler source and rebuild](../select-checked-default-address/README.md#replay) · [Flags and checksums](candidate.json) · [Verified replay](verified-replay.json)
