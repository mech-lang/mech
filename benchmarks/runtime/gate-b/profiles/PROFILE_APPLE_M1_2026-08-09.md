# Resident EKF turn profile: Apple M1

This profile isolates the one-instance, 4,096-turn Gate B resident EKF on an
Apple M1. Fixture construction, correctness validation, allocation reporting,
and Criterion analysis are excluded from the layer timings and flame graph.
All profiled steady-state lanes report zero allocations.

## Layer subtraction

The profiling-only `mech-resident-prepared` lane adds candidate summary and
state hashing to the scheduled turn, then publishes without preparing or
appending a retained receipt.

| Cumulative layer | Episode | Per turn | Increment |
| --- | ---: | ---: | ---: |
| Resident kernel | 0.5303 ms | 129.5 ns | 129.5 ns |
| Scheduled and published | 0.6702 ms | 163.6 ns | 34.1 ns |
| Summary/hash and published | 1.1300 ms | 275.9 ns | 112.3 ns |
| Complete receipt/ledger turn | 1.5069 ms | 367.9 ns | 92.0 ns |

The absolute times are slower than the earlier Gate B run because this profile
was collected under Time Profiler and a different thermal state. The
incremental attribution is the useful result: candidate summary/hash consumes
about 55% and receipt/ledger work about 45% of the 204 ns turn shell above
scheduled execution.

## Sampling profile

The Time Profiler recording sampled a sustained complete-turn run at 1 kHz.
The SVG keeps only the Criterion hot-turn closure and removes fixture setup and
post-timing validation stacks.

Prominent inclusive frames:

| Frame | Samples |
| --- | ---: |
| `prepare_scheduled_turn` | 56.8% |
| EKF scheduled kernel | 36.7% |
| `candidate_state_hash` | 14.0% |
| `prepare_commit` | 34.9% |
| `accepted_record` | 21.3% |
| ledger `prepare` | 6.6% |
| retained-ledger `append` | 5.2% |

Percentages are inclusive and therefore overlap. The capacity controller's
mutex-backed `bind` and `commit_prepared` transitions appear beneath ledger
prepare/append. They are real costs, but record construction/validation and
state hashing are larger targets.

## Finding

There is an obvious performance-mode boundary. A successful turn currently
hashes all 96 candidate bytes, constructs and validates a 64-byte receipt, and
appends that receipt through the retained ledger even though the computation
and scheduler have already completed. A fail-stop performance mode that omits
the diagnostic hash and per-turn retained receipt should approach the 164 ns
scheduled/published result from this run, below the measured 248 ns Julia EKF
turn.

Open [`resident-ekf-turn-apple-m1.svg`](resident-ekf-turn-apple-m1.svg) in a
browser for the interactive flame graph. Hover for inclusive sample counts,
click a frame to zoom, and use the search control to highlight functions.
