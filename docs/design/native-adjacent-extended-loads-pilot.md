# Share addresses across extending loads

Keep the [retained branch-fed selection](native-branch-fed-selection-pilot.md). This independent trial starts from its composed-countdown predecessor to measure the address change alone. The 16-case comparison takes **0.999682×** paired control. Both 1.05 gates pass on all 16. The candidate retains eight wins over Roc and changes libdeflate wins from five to six; the one crossing is repeated 65,535 bytes. Larger natural inputs are slower in the screen, and natural 8 KiB moves away from parity. The rule remains out of the active compiler.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.000177× | 0.174415× | 0.174445× | 2.223444× | 2.223838× |
| Natural 8 KiB | 1.002594× | 1.002404× | 1.005004× | 1.246148× | 1.249381× |
| Natural 32 KiB | 1.004286× | 1.125797× | 1.130622× | 1.022398× | 1.026779× |
| Natural 256 KiB | 1.002245× | 1.145004× | 1.147575× | 0.925404× | 0.927482× |
| Natural 1 MiB | 1.003630× | 1.153810× | 1.157999× | 0.931331× | 0.934712× |
| Held-out random 65,537 bytes | 0.992247× | 1.033159× | 1.025149× | 1.738494× | 1.725016× |
| Random 1 MiB | 1.001233× | 1.163095× | 1.164530× | 1.645264× | 1.647293× |
| Repeated 65,535 bytes | 0.996385× | 0.565031× | 0.562988× | 1.002061× | 0.998438× |
| Repeated 65,537 bytes | 1.003360× | 0.567830× | 0.569738× | 1.002317× | 1.005684× |

The existing adjacent-address pass classifies a plain access by element width. An extending load has a wider result than its memory access. The experiment derives the width from the signed or unsigned load opcode, leaving its original instruction, flags and ordering intact. The existing same-pointer, same-index and straight-line checks still apply. On the actual vector matcher, seven uses share an address expression, including the second tree slot at the first search. Its linked address setup uses one add of two bytes instead of rebuilding the scaled index. The retained comparator binaries form addresses for the adjacent slots from their existing tree pointer. No compressor source or limits change.

With the pass disabled, re-emission reproduces the retained 10,188-byte matcher exactly. With it enabled, that body shrinks to 10,156 bytes. The diagnostic keeps all other linked bytes and every function address fixed apart from its UUID and signature; it pads the replaced body with eight unreachable instructions. The measured counter worker verifies the same replacement. The source patch extends the pass and its existing tests. There are 450 focused executions including signed values, wrapping indices, intervening reads, alternate edges and separate bases. All 162 linked checks pass, as do 56,640 timed calls with exact raw outputs, independent inflation, capacities, held aliases and zero live private owners. The full matrix and normal whole-program emission were not run for this unretained pilot.

| Separate counter input | Instructions / control | Cycles / control |
| --- | ---: | ---: |
| Natural 8 KiB | 0.995940× | 1.002056× |
| Natural 256 KiB | 0.996743× | 0.999838× |
| Natural 1 MiB | 0.996711× | 0.999856× |
| Random 1 MiB | 0.984484× | 0.992977× |
| Repeated 65,537 bytes | 0.986515× | 1.000479× |
| Periodic 1 MiB | 0.986872× | 1.005849× |

The separate hardware measurements preserve the instruction reduction and the mixed cycle changes. In exact-worker timer profiles, the matcher accounts for 1,176/4,196 running samples on natural 8 KiB and 3,577/5,302 on the held-out random case. These samples include validation and do not count executed instructions. The six private payload metrics agree throughout the timing screen.

Measured parent `dbca8209b74a4e2ab4e8bde8bf754b1b6490cbed`; compiler patch SHA-256 `ec040f905c27934963eb17bcbe77a0828071fc786c85a923bc0afd5b774ba27e`; exact trial worker `c061ef9723adde6917c916a4fe07f1b1a801bd2c052109292e084253846744ef`. The test harness and pinned backend use Nightly 2026-03-03, warnings as errors, and the register-allocation checker for the linked-body reproduction. The archive preserves both initial scratch module-path errors and the passing reruns.

[Patch, exact code, paired observations, counters and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-dbca8209b7-extended-adjacent-loads.tar.gz). SHA-256 `8f8d73c617481f96f2fc6fa755a7e4b6b99231032ff945730459c7dba5d27027`. Run `python3 -B verify-round.py` after unpacking.
