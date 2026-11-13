# Isolating the composed ordered reduction

Changing only the four composed reductions takes **0.993395× control time** across the 81-case suite. Both 1.05 gates pass for every case. Fifty-seven paired estimates improve. The isolated executable keeps all 52 Roc wins and moves from 15 to 16 libdeflate wins in the same run. Seven cases beat both peers.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.997940× | 0.170898× | 0.170546× | 2.193033× | 2.188515× |
| Natural 8 KiB | 0.981852× | 1.019612× | **1.001108×** | 1.268283× | 1.245266× |
| Natural 256 KiB | 0.981276× | 1.170778× | 1.148857× | 0.949422× | **0.931645×** |
| Natural 1 MiB | 0.977936× | 1.187437× | 1.161237× | 0.956212× | **0.935114×** |
| Repeated 32 KiB | 0.995841× | 0.510011× | 0.507890× | 1.001895× | **0.997728×** |
| Repeated 65,535 bytes | 1.001493× | 0.565094× | 0.565938× | 0.998150× | **0.999640×** |
| Periodic 128 KiB | 1.015264× | 0.579698× | 0.588547× | 1.051822× | 1.067878× |

The composed and ordinary 16-case screens take 0.996548× and 1.000653× their controls, respectively. The full comparison runs 245,664 calls, and each screen runs 56,640. Natural 8 KiB remains about 0.11% above parity in the full run. The 32 KiB repeated crossing occurs only in the full cohort, whose control was slightly above parity. The largest adverse estimate is periodic 128 KiB. Its two timing-order halves are 1.034248× and 0.996629×. All three comparisons use the same four actors, output-ownership conditions, startup descriptor and timer protocol.

The [combined countdown trial](native-reduction-loop-registers-pilot.md) already kept all named linked code addresses fixed. This check compared 1,348 text symbol entries, including repeated names. Both isolated Objects use four exact emitted bodies. The composed Object also updates two cost-table relocations in each selected body. After relinking, each selected body exactly matches the full compiler trial. The two isolated workers and the original control agree elsewhere after masking the selected code, UUID and signature. The matcher stays at `0x100062c80` and the root at `0x10008b180`.

The composed-only counter cohort retires 15.309M → 14.708M instructions and spends 3.684M → 3.628M cycles for natural 8 KiB. At natural 256 KiB it retires 586.983M → 565.698M instructions and spends 170.017M → 166.153M cycles. The ordinary-only 8 KiB count is effectively equal. Exact-worker profiles place 2,374 of 4,380 timer PCs in the composed parser for natural 8 KiB. The repeated 65,535-byte and periodic 8 KiB profiles sampled the ordinary parser and zero composed-parser PCs. The attribution includes validation. The isolated timing and counter effects on the repeated boundary case are small and vary across cohorts.

Both isolated workers pass 162 exact linked calls, with independent inflate, capacity, aliases and zero private owners at exit. The six private-payload metrics agree in all measured cases. The counters cover six cases per variant. Five profile captures completed after retrying one internal profiler-export error. The ordinary-only experiment stopped after its screen. The retained compiler stays at `de01ea502cc8e3aaec55e9a3f6ea3a6769c18abd` while source-based reproduction of the composed-only Object is being qualified.

[Paired data, link audit, profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-4fff511238-reduction-variant-isolation.tar.gz). SHA-256 `880aa7ab3ec605f54489483b98691dcd98030c3043176f4a68290446171fe1ec`. Run `python3 -B verify-isolation.py` after unpacking.
