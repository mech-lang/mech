# All-unit DFG construction baseline

These diagnostic runs cover DFG construction, copies, compaction replacement,
clear and destruction throughout native compilation. Both cases reproduce their
qualified object, full metadata and canonical bytecode exactly. This is work
accounting, not a latency comparison.

| Native work | Selected 63,804 B | Late codebook 64,444 B |
| --- | ---: | ---: |
| SSA arena positions created | 1,407,099 | 1,284,796 |
| Instruction arena positions created | 497,878 | 392,213 |
| Block arena positions created | 174,834 | 139,102 |
| Largest single SSA arena | 831,378 | 831,378 |
| Peak resident SSA positions | 955,192 | 955,192 |
| CFG constructions | 1,335 | 1,349 |
| Dominator constructions | 1,242 | 1,255 |
| Full verifier runs | 825 | 834 |

The late split removes some later construction/copy traffic while adding graph
and validation work. It does **not** eliminate the original 831,378-slot unit or
reduce the peak resident SSA count. An early split must improve total work and
the largest unit, not just final root layout. These counts do not establish
that all temporary work can be removed, or forecast elapsed savings.

“Created” includes function copies and compacted replacement tables. “Resident”
includes retired aliases until compaction; it is not semantic/register liveness.
All three arena kinds return to zero after each compilation. Maximum instruction
and block arenas include removed entries, so they differ from laid-out CLIF
instruction/block snapshots. Canonical plan/proof tables, value-list pools,
compaction scratch maps, allocator capacity and raw machine-stub construction
are outside these DFG counters. Complete build time, RSS and whole-object text
remain required measurements.

The observer is an **off-by-default Cargo feature**. Normal builds contain no
observer calls. Enabled builds use host atomics to account for graphs that move
between compiler threads. Focused tests exercise actual graph creation,
clone/clear/compaction and cross-thread destruction. Serialized graph imports
are also accounted for without changing the wire representation. Instrumented
build times are never compared with ordinary build times as an optimization.

`results.json.gz`, each case's `census.json`, `receipt.json.gz` and compile log
are the raw evidence. The source base and as-run patch are in `plan.json` and
`compiler-source.patch.gz`. Extract `new-source-files.tar.gz` as well when
reconstructing that historical patch: its new observer/test files were not yet
tracked when the patch was captured. This PR contains the implementation as
live source, including subsequent serializer compatibility coverage.

Build the diagnostic compiler from the benchmark consumer with
`--features construction-census --bin compile_profile`, keeping the established
release/compiler/allocator profile. Run it with the named recipe's native flags
and unchanged Mech source; it writes `construction-census.json` next to the
encoder and metadata. `collect-as-run.py` retains the original acquisition
paths and command. The parent `verify-plans.py` checks the portable captured
results and exact-artifact bindings without those host paths.
