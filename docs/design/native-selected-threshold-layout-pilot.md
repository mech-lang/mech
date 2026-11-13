# Repeated threshold comparison timing

Both matcher routes compare the same current remaining length against the same threshold twice. The first branch's common edge reaches the second comparison with its operands and flags intact. Redirecting that edge past the second comparison preserves the first branch's short-window move, the second branch's short-window behavior, and the subsequent assertion and load order.

The diagnostic uses two signed copies of the retained grouped-tally worker at `79106a2b31bdaa12ab12f747664341dc86f6f81b`. The matcher keeps the same layout, with branch destinations changed at `0x100062dfc` and `0x100063a18`. The two executables agree outside those words and signature bytes. Each copy passes 162 linked checks covering all 81 exact streams, independent inflation, capacity, aliases and cleanup.

| Input | Candidate / control | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 1.005688× | 0.172353× | 2.187072× | 2.199512× |
| Natural 8 KiB | 1.000670× | 1.026333× | 1.268646× | 1.269496× |
| Held-out random 65,537 bytes | 0.997479× | 1.026257× | 1.739086× | 1.734702× |
| Random 1 MiB | 0.997497× | 1.161068× | 1.648718× | 1.644592× |
| Periodic 1 MiB | 1.001511× | 0.582166× | 0.988460× | 0.989953× |
| Repeated 65,537 bytes | 0.997153× | 0.569238× | 1.007137× | 1.004270× |

The nine-case screen measures **0.999536× control** over 6,912 timed calls and passes both timing gates. The repeated boundary case's order halves are 1.002168× and 0.992164× control. The exact worker was profiled on that case and random 1 MiB. The screen retains the same peer wins in both variants. The split order halves and remaining libdeflate gap leave this as timing evidence. A compiler change would need its own measurement and qualification.

[Machine witness, raw timing blocks, exact-worker profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-79106a2b31-selected-threshold-layout-pilot.tar.gz). SHA-256 `08d756a1793613d34848ef56af5aa2b2987c3f2a9709aca82ab1732d709ee050`. Run `python3 -B verify.py` after unpacking.
