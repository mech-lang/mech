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
12.6% in the Object and 9.7% in the linked worker. The final Object has
4,323 unconditional branches to the shared tail. The remaining site uses
the conditional branch at Object text offset `0xda10c`, which targets
`0xdad3c` directly.

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
This screen aggregates across cases. Individual cases require their own
paired intervals.

The largest first-cohort slowdown was 2.58% on periodic 8 KiB, which
improved 0.26% in the repeat. The largest repeat slowdown was 2.45% on
seven stored bytes. Both existing 1.05 gates pass all cases in both
cohorts. The Apple M3 baseline's published libdeflate counts are 38/81 and
35/81. The new paired observations are from the Apple M1.
[Per-case ratios, build switches and artifact hashes](../../benchmarks/mech-deflate/native-profile/shared-cold-epilogues-apple-m1-20261114.json)

## Per-case follow-up

The follow-up uses a 0.5% measurement resolution. Its 5th–95th percentile
intervals resample complete paired blocks within each run order. The first
study repeated 30 cases selected by the original per-case intervals or two
adverse point estimates. An independent study repeated 11 of those cases
with longer schedules.

With two control processes reading the same executable file and one
candidate process reading a separate file, the 8 KiB random input with
an output prefix was 0.65% and 0.85% slower. The 4 KiB random input with
an output prefix was 0.39% and 0.82% slower. Most of the difference
appeared on the first call of each actor's batch. Two diagnostic builds
restored the addresses of code and data after the checked root, either
within each page or exactly. Both reproduced the adverse signal.

A byte-identical control put in the candidate's separate file showed the
same effect on the 8 KiB input (+0.93%, interval +0.56% to +1.29%) and
the 4 KiB input (+1.50%, +1.05% to +2.00%). The longer measurement puts
all three workers on distinct executable files and uses 24 calls per actor
per block, with 144 balanced blocks per case. The control builds still have
identical hashes.

| Complete-call change vs. paired controls | Three-file run | Independent three-file repeat |
| --- | ---: | ---: |
| Random 4 KiB with output prefix | +0.014% [−0.092%, +0.122%] | −0.062% [−0.182%, +0.059%] |
| Random 8 KiB with output prefix | +0.011% [−0.081%, +0.105%] | −0.130% [−0.230%, −0.030%] |

All five cases in both longer candidate runs have their whole interval
within ±0.5%. An additional three-file run with three byte-identical
controls has the same scale of differences. The shared-epilogue candidate
was unchanged for these runs. The packet retains the adverse observations
from the earlier eight-call runs and the other follow-ups. This 0.5%
bound applies to the five M1 cases in the 24-call configuration.

The replay verifies all 736,992 original raw timed observations, per-case
figures and peer-win totals, the six payload counters, the exact streams
and witness outcomes, and the emitted Object and linked worker inventory.
The packet includes build inputs and disassembly. It contains the hashes
for the natural PDF fixtures, whose bytes are supplied separately for
native re-runs.
[Apple M1 raw measurements and replay packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/shared-cold-epilogues-apple-m1-1e1c275a8136.tar.gz)

The packet's SHA-256 is
`1e1c275a813688ac7df93075ca7581f5dc7b9dada6df104f3c51e6454dd37010`.

## Correctness checks

The source and decoded bytecode produce identical candidate Objects,
descriptors, and workers. Each form passes 162 exact linked stream checks.
Two linked control fixtures compare 648 outcomes across configurations,
including 20 errors, partial work, shared aliases, untouched failure
outputs, and owner cleanup. They preserve the original error statuses and
source origins. The ordinary managed compiler emits byte-identical Object
and descriptor for the partial-work fixture with the switch set or unset.

The Linux native-plan CI job runs eight tests against the vendored ARM64
backend. They verify branches to the shared restore, separate success
returns, B-key pointer authentication, prologue unwind records, branch
reach, large-frame restores, unframed returns, platform and calling
convention eligibility, diagnostic layouts, and context reuse.
[First CI run of the eight-test step](https://github.com/mech-lang/mech/actions/runs/43654475272/job/132630034017)

The measured binaries were compiled from the original #991 change. When
the local full rebuild ran out of disk space, the qualification reused the
published compiler's pinned unchanged dependency artifacts and rebuilt the
modified backend and producers. The disabled Object, descriptor, and
worker reproduce the published hashes. Timing covers complete successful
compression calls. The error witnesses verify results and cleanup.
