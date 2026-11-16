# Reuse a completed private overwrite

The compact native compiler can reuse an array produced by an earlier
visit of an enclosing tail loop. The source must clear the incoming
array and use that cleared storage throughout a deterministic
append-only region. The compiler follows the result back through the
latch, checks the region's entry and exit, and identifies every input
that the next visit must compare. Eligibility follows typed
operations, current definitions and owner liveness.

The optimization requires a successful prior visit in the same
activation. The prior overwrite completes without a payload
allocation. The next visit requires a live sole owner and sufficient
capacity for the previous result. It compares current and prior
initialized integer arrays with equal geometry before moving the
existing result owner to the continuation. The fallback executes the
original checked graph. In the benchmark, tallying, Huffman
construction, actual-cost evaluation and pass selection still execute.

## Apple M3 complete-call result

Both 81-case runs use all 24 orders of the four timed actors, the
pinned level-12 source, and the existing borrowed-output and cleanup
boundary. Each verifies 245,664 complete calls and passes both
existing 1.05 gates on all cases.

| Result | Full | Repeat |
| --- | ---: | ---: |
| Faster than the pinned reference | 81/81 (100%) | 81/81 (100%) |
| Faster than libdeflate and the pinned reference | 38/81 (46.9%) | 35/81 (43.2%) |
| Candidate / paired control geometric mean | 0.996686× | 0.995890× |
| Held-out slides / pinned reference | 0.970861× | 0.954116× |
| Natural 1 MiB / pinned reference | 0.913648× | 0.912731× |
| Eight literals / pinned reference | 0.164584× | 0.160349× |

The paired control scores 80/81 and 81/81. Three descriptive repeat
intervals cross parity. Empty input is the largest matched slowdown,
at 3.13% and 2.13%. An independent hardware-counter cohort measures
4.16% fewer instructions on slides and 9.35% fewer on natural 1 MiB.
Eight literals takes 5.58% longer in that counter cohort.

A separate census records eight reuse selections per complete set.
They remove 1,436,174 positions and 3,767,115 match records across
five inputs. The held-out slide case contributes 26,885 positions and
65,329 records. All six private payload counters agree in the primary
observations.

## Review and reproduction

The source and decoded forms emit the same native Object and each
pass 162 complete linked compression calls. The generic linked
controls check results, earlier errors, untouched failure outputs,
and cleanup. A separate live-alias witness confirms that the checked
arm executes on shared backing. The compiler build denies warnings
and all five architecture/catalog checks pass. The ordinary managed
entry has its own resource and publication contract. These timings
measure the compact benchmark configuration with
`MECH_COMPACT_REUSE_OVERWRITTEN_RESULT=1`.

The primary binaries were built from the change on
`1c09216698a3f4c3d5c33da150d1a9b3d1b48eaa`. The final build of the
combined compiler on `8e3c03512b4df9ae479fe7f586d195166791b3c5`
reproduces the exact timed Object and worker. Publication parent
`51b18380c07cbbdd4e0c0ffd472c3820289dbc0d` only adds documentation
and receipts.

[Build commands, compiler and source hashes, raw observations and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/native-repeated-overwrite-f50c39e49f25.tar.gz).
SHA-256 `f50c39e49f25d4b9bfbcd80417e538fdcf69dbd58353d796a621f3061ce57c6a`.
Run `python3 -B replay.py` after unpacking to verify 215 case
receipts and 704,736 timing observations, including the rejected
paired reduction.
