# N-body: Mech, K, and BQN

Every endpoint runs 50,000,000 steps on the same Apple M3 with AC power and Low Power Mode disabled. Compilation is outside the timer.

Times are median process wall time ± sample standard deviation from five sequential fresh-process runs per endpoint. The ratio uses the compact Mech median. These are the fastest tested endpoints, not a claim of the best possible program in each language.

| Implementation | Normalized LOC | Time (s ± SD) | Time / Mech compact |
|---|---:|---:|---:|
| Mech (AOT, compact) | 32 | 1.124 ± 0.057 | 1.00× |
| Mech (AOT, optimized) | 37 | 1.055 ± 0.002 | 0.94× |
| K (Growler, compact/optimized) | 36 | 23.340 ± 0.085 | 20.77× |
| BQN (CBQN, compact) | 26 | 31.417 ± 0.056 | 27.96× |
| BQN (CBQN, optimized) | 39 | 18.840 ± 0.051 | 16.76× |

## Fastest-source selection

K’s published Amber and Growler sources are byte-identical. Native Growler was faster than native Amber in sequential pilots. Reciprocal reuse, separate coordinate arithmetic, and grouped reductions did not beat the published cached-pair advance algorithm. Our 36-line K source retains that algorithm, simplifies setup/reporting, and puts each assignment or sequential statement on a separate executable line.

BQN compact is 26 LOC: the published full-pair advance algorithm with direct matrix data instead of named body records. The initial conditions and output requirements are unchanged. Momentum offset and output run directly at script level, without a redundant main block.

BQN optimized is 39 LOC, reduced from our first 82-line variant. It uses ten unique body pairs, flat coordinate arrays, cached gathers, scaled velocities/masses, and two ordinary array reductions. Direct matrix data removes the named body records and their extraction. Energy and advance use the same state arrays, removing duplicate initialization, state copies, and final reshaping. The advance loop still has seven executable statements.

All local K and BQN variants keep the computation in the language, with no FFI or specialized native executor. They are derived local sources, not upstream submissions. The original source snapshots and their measurements are retained separately.

BQN optimized versus compact speedup: 1.67×. Mech optimized versus optimized BQN: 17.86× faster in this measurement.

## Validation and LOC

Each endpoint passes independent C-reference checks at 1, 2, 10, 1,000, 10,000, and 1,000,000 steps. Every timed 50-million-step output passes the same reference within the existing 1e−8 energy tolerance. Initial and final energies are checked.

LOC separates packed, nested, and chained assignments. The new K and BQN sources have one assignment or executable statement per physical line; their normalized and physical counts agree. Matrix literals use one coordinate row per line. This changes the source representation rather than the counting rule. Source and binary hashes are pinned. Both Mech endpoints use the verified v0.4 integration binaries without compiler changes.

Mech source excludes its general repetition/output runner, while the array scripts include those calls. Timings include repetition and output for every endpoint. The spectral-norm and k-nucleotide array sources are not covered by this timing run.

## Measurement details

Rounds are randomized with seed 2026100914. All 25 source-compaction confirmation samples are retained. Conditions are checked at both sample boundaries and every ten seconds for longer runs. No power change or competing compiler activity was detected. Sample SD describes the five observations; it is not a confidence interval for the median.

The confirmation uses a blocking process wait with a separate 180-second stall watchdog. The earlier published-source run used Python subprocess timeout polling, which can add process-reaping delay. All endpoints, including Mech, were remeasured with the same blocking method for this comparison. The earlier results are preserved separately and are not pooled with these samples.

## Evidence

- [Final confirmation samples, order, conditions, outputs, hashes, and checks](comparison-compact.json)
- [Final confirmation measurement script](measure_compact.py)
- [Prior optimized-source confirmation](comparison.json) and [script](measure_fast.py)
- [Original published-source run](results.json) and [its script](measure.py)
- [Selection pilots, first pass](candidate-pilots.json), [second pass](candidate-pilots-2.json), [third pass](candidate-pilots-3.json), [shorter-source pilots](compaction-pilots.json)
- [Runtime builds and commits](runtime-builds.json)
- [Performance and LOC chart](performance-loc.png)
- [BQN compact](sources/nbody-compact.bqn), [BQN optimized](sources/nbody-optimized-short.bqn), [K compact/optimized](sources/nbody-optimized-short.k)
- [Compacted LOC audit](compacted-loc-audit.json) and [source rewrite script](compact_sources.py)
- [Published BQN source](sources/nbody.bqn), [published K source](sources/nbody.growler.k), [first 82-line BQN variant](sources/nbody-flat-cached-reduce.bqn)
- [Original normalized LOC audit](/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-png/iversonian-sources/normalized-loc-audit.json)
