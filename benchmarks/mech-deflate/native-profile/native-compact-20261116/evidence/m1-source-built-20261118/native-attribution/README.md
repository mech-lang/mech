# Native compiler attribution on Apple M1

The checked 65,276-byte recipe at `90d133253a` produced identical object,
canonical bytecode and complete metadata in four paired traced and
untraced builds. The control's native median was 1.179 s. The trace added
a paired median 137 ms. These coordinator spans identify where the work
runs. Complete-build decisions use the uninstrumented measurements.
[Trace and baseline observations](observations.json.gz) · [Instrumentation](instrumentation.patch.gz)

| Native interval | Traced median |
| --- | ---: |
| Root CFG cleanup and outlining | 407 ms |
| Phase construction, cleanup and code publication | 210 ms |
| Phase worker wait, within phase construction | 14 ms |
| Root machine emission | 94 ms |
| Tail layout preparation | 88 ms |
| Root guard derivation | 77 ms |
| Retirement and consumption proofs | 45 ms |
| Object writing | 0.16 ms |

The root cleanup reduces 16,157 blocks and 42,255 instructions to
10,446 blocks and 19,281 instructions. SSA value compaction retains
122,162 of the 831,378 allocated value slots. Root and phase spans include
their nested passes, and adjacent measurements can overlap.
[Exclusive spans and counts](result.json)

## Retirement guard transitions

The prototype reused a transition only within the same retirement site and
only for the same known facts, expressions and closure state. Each owner
still ran its own liveness and read checks. The count-only run reused
2,963 of 95,820 transitions. All builds emitted byte-identical bytecode,
metadata, objects and deployment executables.

Eight balanced source-to-final-link pairs with prebuilt support took medians
of 2.172 s for the reference and 2.176 s for the prototype. The paired
median increased by 4.7 ms, with one faster pair. Source CPU was 2.205 s
and 2.203 s, and peak RSS was 392.6 MB and 392.5 MB, respectively.
The prototype was rejected. [Plan](guard-transitions-plan.md) ·
[Paired measurements and counts](guard-transitions.json.gz) ·
[Prototype patch](guard-transitions.patch.gz)
