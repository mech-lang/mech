# Continuing reduction countdown timing

The existing scalar continuation of the admitted ordered minimum keeps its source index, reconstructs the same index plus the fixed forward-read offset to form the selected item, and compares the next index with the stop. It moves the winning cost and next index at the latch. The pinned reference parser keeps the biased index and a remaining count in its 11-instruction loop.

The fixed-layout control initializes that same biased index and positive remaining count on both scalar continuation entries. The loop uses the biased index directly for the forward read and selected item, keeps the minimum in its original register, and decrements the count. The reverse read updates the same address, both U32 cost additions wrap, and one strict comparison drives both selects. The incumbent and earliest index win ties. On exit the original continuation receives the current minimum and selected item. The first visit, singleton, vector prefix, fallback and capacity checks keep their original instructions. The recurring loop goes from 16 to 11 instructions, with no stack transfer on either route.

| Input | Candidate / control | Control / Roc | Candidate / Roc | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 1.004465× | 0.173109× | 0.173881× | 2.200585× |
| Natural 4 KiB | 0.990243× | 0.864614× | 0.856178× | 1.446219× |
| Natural 8 KiB | 0.985764× | 1.024215× | 1.009634× | 1.253211× |
| Natural 32 KiB | 0.993072× | 1.150888× | 1.142914× | 1.037274× |
| Natural 256 KiB | 0.992101× | 1.171156× | 1.161905× | 0.937726× |
| Natural 1 MiB | 0.993542× | 1.179372× | 1.171755× | 0.944861× |
| Held-out random 65,537 bytes | 1.000115× | 1.036650× | 1.036770× | 1.740680× |
| Random 1 MiB | 0.997378× | 1.166379× | 1.163321× | 1.643863× |
| Periodic 1 MiB | 0.998663× | 0.587344× | 0.586559× | 0.993159× |
| Repeated 65,537 bytes | 0.999641× | 0.566964× | 0.566761× | 1.001109× |

The ten-case screen takes **0.995484×** control in 7,008 timed calls with 24 balanced blocks per case. Both 1.05 gates and the private payload agreement pass 10/10. All five natural sizes improve in both order halves. Eight literals slows in both halves and retains its reference win. Each peer win count is unchanged in this screen. The closest affected natural case, 8 KiB, has a remaining 0.95% elapsed gap to parity in this cohort.

The two ad hoc signed workers derive from the retained binary at `0882815df6b7103a84f17e55f93c17f91126c21f`. Only four 17-word spans and the signatures differ. All 324 linked calls spanning 81 exact cases preserve output bytes, independent inflation, capacity, aliases and cleanup. Separate candidate profiles cover natural 8 KiB, 256 KiB and 1 MiB. The archive includes both balanced order halves and the first archive verifier's corrected case-count assertion. This is the machine control for a separate compiler candidate.

[Exact instructions, all timing blocks, profile receipts and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-0882815df6-reduction-countdown-layout.tar.gz). SHA-256 `6a31a815cd7d21c2944c92bcae70dda7869da37f788202e0a2a631466317a875`. Run `python3 -B verify.py` after unpacking.
