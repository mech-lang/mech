# Ordinary buffer ownership across frame transport

Previous goal turn: progress. Published checkpoint `ba198160dc51162da49d962866fe4a19e0aa051c`
adds actual CPU/allocation/heap measurements and backtrace attribution. The
qualified round-37 layout experiment remains unconfirmed for elapsed time.

Target the real complete eight-byte level-12 computation, whose 305257-word U64
path buffer has 39 capacity allocations versus one in the pinned reference, and
whose 262144-entry U16 tree has four additional full copies. Cache allocation is
already one. Preserve source/bytecode, original algorithm and allocation sites,
tables, limits, allocator, timing boundary, errors, aliases and failure recovery.

First observe the existing selected buffer route. Distinguish absent static
consume candidates from actual retained arena/frame aliases. Use weak frozen
identities and raw backing addresses; the observation must not introduce a
strong owner, change work, or masquerade as a performance measurement. Save the
actual reached sites and owner inventories immediately.

Then add the smallest general compiler/runtime change supported by that witness
and a meaningful acceptance test. Reuse maintained liveness, private frame lanes,
ownership transfer and ledger machinery. Do not discard observable aliases or
invent a benchmark-specific operation, second owner or executor.

Qualify correctness, scalar/frame controls and architecture before balanced
eight-byte timing. Repeat the instruction/allocation/peak scorecard and inspect
the resulting binary/profile. Compare against the frozen round-37 executable,
use one shared reference per comparison and retain the historical 0.300 ms ruler.
Commit each observation cohort immediately. No larger-input timing or CI wait.
