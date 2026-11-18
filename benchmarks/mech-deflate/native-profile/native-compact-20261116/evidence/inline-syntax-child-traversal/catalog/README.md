# Overlap cold catalog work and parsing

In the fresh-process source build, regular parsing and built-in catalog
construction are independent. The probe starts catalog construction inside
the measured source process and joins it before semantic lowering. A
thread start failure uses the synchronous catalog path. Both modes reject
the malformed fixture at parsing. The comparison changes one setting in
the same compiler executable for its serial and overlapping arms.

The candidate compiler also includes the owned syntax iterator, which
saves semantic time in isolation. The measured deployment decision uses
the combined source and the verified frame-copy recipe. [Iterator comparison](../README.md)

| Apple M1, ten balanced blocks | Source through final link | Parser | Synchronous catalog time or join wait |
| --- | ---: | ---: | ---: |
| Maintained Mech | 2.48924 s | 0.78228 s | 61.073 ms |
| Candidate with serial catalog | 2.49044 s | 0.78860 s | 60.558 ms |
| Candidate with catalog overlap | 2.42741 s | 0.78940 s | 0.019 ms |
| Roc | 2.98099 s | — | — |
| libdeflate | 0.47964 s | — | — |

The paired saving is **+61.03 ms (+56.95 to +62.39)** against maintained Mech and
**+60.88 ms (+58.29 to +62.57)** within the same compiler. The ranges are 90% paired intervals.
The catalog worker itself took **62.5 ms** with overlap
and **60.6 ms** serially. The through-link clock includes
reading the source, starting the catalog work, parsing, lowering,
native emission, and final linking. Standard deployment support is prebuilt.
Mech takes **0.815× Roc** and **5.058× libdeflate**.
The frame-copy source recipe uses this measured compiler and enables catalog overlap.

The bytecode, encoder, complete metadata, linked worker, and stripped binary
match the maintained build. The exact encoder is **71,192 B** and the
stripped executable is **292,568 B**. The linked worker passes **162**
observations and carries its full **81/81 Roc and 61/81 libdeflate** scores
in both orders. [Build inputs](plan.json) · [Timings](observations.json.gz) ·
[Exact linked worker](worker/qualification.json)
