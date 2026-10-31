# Diagnose repeated path-buffer capacity reservations

Parent `70847471ba06c9e395f9628eb1b19c7b11ea82f6` is published and qualified;
round 51 is progress, with parity still unachieved. Its longer matched result is
13.915864× the pinned reference. Preserve both timing cohorts and the adverse
instruction count.

Current allocation receipts contain 35 requests of 2,442,056 bytes each. The
older round-37 stack census already attributes 39 requests of that size to
copy_buffer_with_sublist: the U64 path buffer reserves 305257 words. A Clear
fallback reserves capacity but copies no words. Do not equate requested capacity
with copied bytes or redo that historical analysis as a new finding.

First refresh attribution on the exact retained graph/Object/image. Then record
the selected buffer operation, source length/capacity, consuming-candidate set,
and whether a physical fallback occurs. Distinguish missing static consumption
proof from actual remaining ownership. If necessary inventory current matching
arena/frame owners in a separate diagnostic overlay. Keep instrumentation out
of performance claims and out of production execution.

Use this evidence to choose the smallest general ownership/identity improvement.
Before expanding compiler work, supply a minimal source witness, the exact real
compressor operation, and an acceptance test. Preserve observable aliases,
schema/shape validation, error precedence, charges, cleanup and retry. No
benchmark-specific source-ID or capacity rule is permitted in an optimization.

Keep the full eight-byte computation, pinned source/bytecode, tables, actual
tree/cache/path capacities, allocator, output clear/reuse and timer unchanged.
Commit every diagnostic/measurement cohort immediately. If a change is made,
repeat matched complete compression, hardware, memory and sampling measurements
and retain contrary results as well as improvements.
