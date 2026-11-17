# Exclusive native compiler work

Both opt-level-3 diagnostic variants reproduce the same bytecode, complete
metadata, 71,768-byte encoder Object and linked worker. The trace uses one
monotonic clock with per-thread parent spans and queued job IDs. Its wall
clock begins at canonical plan lowering and ends after object publication.
Per-thread elapsed time can overlap. The replay retains that overlap.
[Frozen inputs](plan-inputs.json) · [Replay and identities](exclusive-replay.json)

| Shared-owner coordinator phase | Exclusive time |
| --- | ---: |
| Canonical plan lowering | 17.445 ms |
| Semantic preparation | 296.995 ms |
| SSA construction | 280.450 ms |
| CFG cleanup | 691.530 ms |
| Cranelift optimization | 46.430 ms |
| ISA lowering and code generation | 76.508 ms |
| Coordinator wait | 18.224 ms |
| Object publication | 0.394 ms |

The 23 worker jobs use about 30.8 ms of active elapsed time, and coordinator
wait is about 18 ms. The exact counts of the graph and definition inventory
initializers move from 36 to 35 and 90 to 79 respectively. Read-role
initializations remain nine. Their executor is the coordinator. All timed
facts together move from 22.739 to 21.687 ms. The rest of the native wall
change tracks cleanup variability in this one diagnostic pair.
[Uninstrumented matched build](../shared-program-facts/README.md)

The sizeable SSA and cleanup windows justify an initial source-level
scalar demand inventory. The first slice covers pure SSA aggregate and
carrier transfers, with pre-read filtering and a complete all-fields path
whenever demand proof is unavailable. A separate ablation disables the
demand mask. The present trace measures costs and leaves target code intact.

The exact [diagnostic patch](trace.patch.gz) is based on dae46c6867.
Apply the additional [shared-owner change](shared-facts-only.patch.gz)
only on the shared side. The as-run compiler and link argv in each capture
pin inputs and output identities. The executable paths identify the
original environment. Fresh locally built copies of both compilers can
run [capture.py](capture.py) and [replay.py](replay.py). The captured data can
be checked offline with [replay-archive.py](replay-archive.py). [Scoped scalar demand pilot](SCALAR-DEMAND.md)
