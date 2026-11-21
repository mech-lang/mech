# Sequential scalar visits and a proved one-visit tail

These opt-in compiler variants keep sequential scalar updates inside each two-entry group. The single-tail variant removes the scalar backedge only after the existing admission proves exactly one entry remains. The original 2,719-line Mech codec and canonical bytecode are unchanged.

**None replaces the current 81/63 working build.** All sixteen discovery orders lose the existing natural 8 KiB libdeflate win. The historical `qualified` field in `result.json` records the earlier 81/62 histogram reference; the selected eight-entry build is documented in [the current scoreboard](../../../CURRENT.md).

| Variant | Encoder bytes | Complete build median | Natural 4 KiB / C, forward / reverse | Natural 8 KiB / C, forward / reverse |
| --- | ---: | ---: | ---: | ---: |
| scalar-loop-tail-probe-profiled | 64,108 | 2.220542 s | 1.020491 / 1.016618 | 1.009596 / 1.013841 |
| scalar-loop-head-probe-profiled | 64,092 | 2.220593 s | 1.015722 / 1.016038 | 1.007492 / 1.012215 |
| scalar-once-tail-probe-profiled | 64,076 | 2.220864 s | 1.032617 / 1.038783 | 1.015949 / 1.015006 |
| scalar-once-head-probe-profiled | 64,060 | 2.218731 s | 1.022752 / 1.016335 | 1.010235 / 1.010877 |

Compared with the legacy paired helper, sequential updates save 16 bytes and 20,271 dynamic instructions on natural 4 KiB. Proving a single remaining tail entry saves another 32 bytes, 31,164 instructions and 8,439 branches. It still performs 158,550 stack loads versus 17,310 for the selected unpaired scorer. Both improvements are retained as repair candidates.

The archive includes four configurations of 20 optimized tests (67,584 scalar-pair fixture calls per configuration), 41 exact older recipe replays, all four normal/checked portable source replays, 162 successful encoding/inflation/alias observations per worker, six checked errors, twelve failure/recovery observations, and a 4,736-byte peak stack. The default selection is unchanged. The repository-wide unsafe audit still has the unrelated missing archived allocator blocker.

Runtime discovery uses 21 cases and 24 balanced blocks, forward and reverse, against both unpaired and legacy paired controls. All sixteen orders are included. Full 81-case results are not inferred from those panels. Fourteen balanced fresh-process complete-build trials per actor include final executable linking with each compiler and support library already built. Diagnostic counters, build measurements and runtime clocks ran separately.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/encoder-size-gap/scalar-sequential-study/verify.py
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/encoder-size-gap/scalar-sequential-study/reproduce.py /path/to/compile_profile /tmp/scalar-tail-replay --role scalar-once-head-probe-profiled
```

Compiler-source basis: `30c1c440c39f4a7ff43d8e8e5f10d8cd6bad2006`. Raw observations, source pins, generation commands, profiles, artifact identities and build stage data are in the archives and `result.json`.
