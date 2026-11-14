# Reuse the remaining span at short reduction entry

Keep the [composed countdown baseline](native-composed-countdown-qualification.md). The trial takes **0.998011×** paired control time across all 81 cases. Natural 256 KiB improves by 0.46–0.73% in three elapsed cohorts, and both timing gates pass everywhere. Natural 8 KiB crosses below Roc in the screen, then slows in the full matrix and confirmation. The full matrix adds no peer wins.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.002106× | 0.171273× | 0.171634× | 2.187494× | 2.192102× |
| Natural 8 KiB | 1.007263× | 1.001581× | 1.008855× | 1.247623× | 1.256684× |
| Natural 32 KiB | 0.999645× | 1.125286× | 1.124886× | 1.024464× | 1.024101× |
| Natural 256 KiB | 0.995138× | 1.147152× | 1.141574× | 0.929794× | 0.925274× |
| Natural 1 MiB | 0.998252× | 1.153089× | 1.151073× | 0.929965× | 0.928340× |
| Repeated 65,535 bytes | 1.000014× | 0.566753× | 0.566761× | 1.001498× | 1.001512× |
| Repeated 65,537 bytes | 1.000966× | 0.568800× | 0.569349× | 1.002241× | 1.003209× |
| Held-out slides 65,537 bytes | 0.996871× | 1.156523× | 1.152904× | 1.011279× | 1.008114× |

The fresh control and trial each win 52/81 (64.2%) against Roc, 15/81 (18.5%) against libdeflate and six against both. The retained baseline's published cohort has 16 libdeflate wins. The 16-case screen takes 0.995685× control; natural 8 KiB goes from 1.001154× to 0.998938× Roc. The eight-case confirmation takes 0.996111× control; natural 8 KiB goes from 1.000564× to 1.004024× Roc. Natural 256 KiB improves in all six order halves. In the confirmation the natural 1 MiB halves disagree on magnitude (0.997726× and 0.939148× control). Periodic 4 KiB has the worst full-matrix increase at 1.014080× control and is flat in confirmation. Thirty-eight full-matrix paired estimates improve. The screen, full matrix and confirmation include 56,640, 245,664 and 6,816 timed calls. All results and both order halves are retained.

The short route has already established that `remaining = stop - start` is positive. After one visit, the singleton branch finishes immediately. On the continuing edge `remaining >= 2`; using `remaining - 1` is equivalent to `stop - (start + 1)`. The experiment uses that identity only where the existing biased ordinal enters the composed countdown. The short first visit, long vector route, strict ties, checked fallback, and source work are preserved.

In four of 357 audited Object bodies, the continuing handoff goes from five instructions to three, including its branch. The recurring scalar loop still has 11 instructions and no explicit stack transfers. The pinned Roc scalar reduction also has 11 instructions. The libdeflate reduction uses a conditional branch on strict improvement. Register allocation changes inside the four bodies, but their sizes and frames remain 1,328 and 224 bytes, all 1,348 linked text symbol entries keep their addresses, and no relocations move. A byte comparison confines changes to those four bodies plus the linked UUID and signature. The remaining first visit and vector completion are visible in the linked disassembly.

| Separate hardware counter cohort | Instructions before | Instructions after | After / before instructions | After / before cycles |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 626,885.5 | 626,920 | 1.000055× | 1.025698× |
| Natural 8 KiB | 14,708,965.5 | 14,649,747 | 0.995974× | 1.002612× |
| Natural 256 KiB | 565,710,875 | 562,434,132 | 0.994208× | 0.997450× |
| Natural 1 MiB | 2,019,769,877.5 | 2,008,767,208 | 0.994553× | 0.997331× |

Process lifetime physical peaks are unchanged in the six-case counter cohort. Exact-worker timer samples in composed parser bodies number 2,182/4,069 for natural 8 KiB and 3,005/5,362 for natural 256 KiB. None of the 5,252 running samples for repeated 65,535 bytes lands in a composed parser body. Samples and counters are separate cohorts.

Parent `4de09acf2643f706110c0c747517080f11a90792`; compiler patch SHA-256 `314040a910e5a0df75c05ab3a1e56212adf9b56583f5b1dd4b013755364e29f8`. The six-feature optimized native build uses nightly 2026-03-03 and warnings as errors. The pinned revision-14 bytecode emits Object `27ddf758cd6c0d95177309c2dd8c51d9602ddbb6e25c7621fff5b9e262826687` in 585.21 seconds. The timed worker SHA-256 is `cda982cda5ab7cb8e1d6dc93353aef2bfecfc911a807635d2dca3aa01e01b127`. The two descriptor launches pass 324 complete linked checks, and the direct phase oracle checks 68,096 positions with the original rejection origin and successful retry. The all-case run preserves exact reference streams, independent inflation, capacities and aliases; all six private payload measures agree with the control. The initial idle guard refused a run during another build; the run started when that build finished. The evidence replay was corrected to accept the eight-case confirmation count. Source-form emission, the managed performance guard and full CI were skipped for the declined source patch.

[Compiler patch, measurements, linked disassembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-4de09acf26-short-reduction-entry.tar.gz). SHA-256 `f6bacce45488e185de662fd7933282438459a6ff41f0bf00d10ecb5f06677bc7`. Run `python3 -B verify-round.py` after unpacking.
