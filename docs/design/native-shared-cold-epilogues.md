# Share cold return epilogues in the compact native root

Set `MECH_COMPACT_SHARE_COLD_EPILOGUES=1` to share cold return tails in the
macOS ARM64 diagnostic compact root. Cranelift applies the change during final
machine-code emission, after it has allocated registers and computed the
function's frame. Each exit writes its status and source origin to the ABI
return registers at its original site. The first eligible cold return emits
the frame restore and authenticated return. Later cold returns branch to that
tail when it is in direct branch range. A one-instruction return keeps its
original encoding. Normal returns keep their own epilogues.

The tail comes from the same frame layout, saved-register set, and pointer
authentication mode for every eligible return. Cranelift's macOS ARM64 unwind
records describe the prologue. The compact emitter retains the explicit
source-location diagnostic layout when requested. The backend also retains
the original layout for machine CFG output and debug value labels or tags.

## Emitted size

The parent compiler is `c1280f3ecb665f7db10b2a2262193de9961a5e73` with the full
published overwrite-reuse switch set. Disabling sharing reproduces that build's
Object, descriptor, and linked worker byte for byte. Both emission runs use the
same source and canonical bytecode. The descriptor changes only in the measured
checked-root code size.
[Published baseline and build inputs](native-private-overwrite-reuse.md)

| Measurement | Control | Shared cold returns |
| --- | ---: | ---: |
| Checked-root code bytes | 830,740 | 691,696 |
| Copies of the 36-byte checked-root epilogue | 4,327 | 3 |
| Object text bytes | 1,102,084 | 962,948 |
| Object file bytes | 1,238,656 | 1,099,520 |
| Linked worker text bytes | 1,431,568 | 1,292,432 |
| Linked worker file bytes | 1,926,536 | 1,794,440 |
| Checked-root stack frame bytes | 49,296 | 49,296 |

The 270,592 bytes of Object text preceding the checked root match exactly.
The Object retains the same defined symbols and 67,500 constant bytes. The
linked worker's unwind sections retain their sizes. The text reductions are
12.6% in the Object and 9.7% in the linked worker.

## Apple M1 complete-call results

Each cohort used the pinned level-12 source, 81 fixtures, 200 ms of warmup
per actor and case, and all 24 orders of the control, candidate, Roc and
libdeflate. The repeat reversed the case and actor order. Each cohort
validated 245,664 timed calls against the pinned streams and capacities.
All six private-payload counters agree. This is the compact complete-call
boundary with input and output aliases and invocation cleanup.

| Matched result | Full | Reversed repeat |
| --- | ---: | ---: |
| Control → candidate cases faster than Roc | 81 → 81 of 81 | 81 → 81 of 81 |
| Control → candidate cases faster than libdeflate | 34 → 35 of 81 | 36 → 36 of 81 |
| Geometric mean of candidate / control | 1.000705× | 0.999067× |
| Control first within the block | 1.001967× | 0.998317× |
| Candidate first within the block | 0.999444× | 0.999817× |

In a separate repeatability cohort, the same control worker scored 81/81
against Roc in both actor slots and 37/81 and 38/81 against libdeflate. The
95th percentile of absolute paired differences was 1.015% across the 78
inputs outside the three sub-8-byte fixtures. For each case the repeatability
tolerance is the larger of that percentile and its own same-worker
difference. Zero cases exceeded that tolerance in both matched cohorts.

The largest first-cohort slowdown was 2.58% on periodic 8 KiB, which
improved 0.26% in the repeat. The largest repeat slowdown was 2.45% on
seven stored bytes. Both existing 1.05 gates pass all cases in both
cohorts. The Apple M3 baseline's published libdeflate counts are 38/81 and
35/81. The new paired observations are from the Apple M1.
[Per-case ratios, build switches and artifact hashes](../../benchmarks/mech-deflate/native-profile/shared-cold-epilogues-apple-m1-20261114.json)

## Correctness checks

The source and decoded bytecode produce identical candidate Objects,
descriptors, and workers. Each form passes 162 exact linked stream checks.
Two linked control fixtures compare 648 outcomes across configurations,
including 20 errors, partial work, shared aliases, untouched failure
outputs, and owner cleanup. They preserve the original error statuses and
source origins. The ordinary managed compiler emits byte-identical Object
and descriptor for the partial-work fixture with the switch set or unset.

The focused backend tests verify direct branches to the restore, separate
success returns, B-key pointer authentication, prologue unwind records and
the machine-CFG diagnostic path. They build from the locked release
profile's Cranelift libraries. When the local full rebuild ran out of disk
space, the qualification reused the published compiler's pinned unchanged
dependency artifacts and rebuilt the modified backend and producers. The
disabled Object, descriptor, and worker checks verify that build against
the published hashes.
