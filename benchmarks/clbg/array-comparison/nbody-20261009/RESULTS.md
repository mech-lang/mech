# N-body: Mech, K, and BQN

Every endpoint runs 50,000,000 steps on the same Apple M3 with AC power and Low Power Mode disabled. Compilation is outside the timer.

Times are median process wall time ± sample standard deviation from five sequential fresh-process runs per endpoint. The ratio uses the compact Mech median. These are the fastest tested endpoints, not a claim of the best possible program in each language.

| Implementation | Normalized LOC | Time (s ± SD) | Time / Mech compact |
|---|---:|---:|---:|
| Mech (AOT, compact) | 20 | 1.124 ± 0.009 | 1.00× |
| Mech (AOT, optimized) | 25 | 1.053 ± 0.002 | 0.94× |
| K (Growler, compact/optimized) | 36 | 23.312 ± 0.026 | 20.73× |
| BQN (CBQN, compact) | 26 | 31.388 ± 0.067 | 27.92× |
| BQN (CBQN, optimized) | 39 | 18.890 ± 0.084 | 16.80× |

## Mech source simplification

Mech compact is 20 LOC and optimized is 25 LOC. Both end with the plain expression `energy`; Mech already returns its final statement. The general native runner now uses that program result by default, while named output selection remains available for the retained older sources.

Direct mass and coordinate matrices replace separately named planet data and their concatenation. The existing transpose operator preserves the original body/coordinate layout. The year and time-step constants are inlined, along with one-use pair selectors and the two energy terms. Every assignment and sequential statement remains on its own line. The optimized form retains its pairwise contribution arrays and calculation.

Native f64 lowering now handles general fixed-shape transpose by permuting operand references. This applies to constants and live matrices and contains no n-body recognition. Three transpose regression cases cover rectangular live matrices, double vector transpose, and constant state initialization.

Both shorter sources produce byte-identical Cranelift native objects and identical generated runner C to their respective 32/37-line baselines. All 30 state components and energy outputs are bit-identical at 0, 1, 10, 1,000, 10,000, 1,000,000, and 50,000,000 steps. The runtime performs the same work.

## Fastest-source selection

K’s published Amber and Growler sources are byte-identical. Native Growler was faster than native Amber in sequential pilots. Reciprocal reuse, separate coordinate arithmetic, and grouped reductions did not beat the published cached-pair advance algorithm. Our 36-line K source retains that algorithm, simplifies setup/reporting, and puts each assignment or sequential statement on a separate executable line.

BQN compact is 26 LOC: the published full-pair advance algorithm with direct matrix data instead of named body records. The initial conditions and output requirements are unchanged. Momentum offset and output run directly at script level, without a redundant main block.

BQN optimized is 39 LOC, reduced from our first 82-line variant. It uses ten unique body pairs, flat coordinate arrays, cached gathers, scaled velocities/masses, and two ordinary array reductions. Direct matrix data removes the named body records and their extraction. Energy and advance use the same state arrays, removing duplicate initialization, state copies, and final reshaping. The advance loop still has seven executable statements.

All local K and BQN variants keep the computation in the language, with no FFI or specialized native executor. They are derived local sources, not upstream submissions. The original source snapshots and their measurements are retained separately.

BQN optimized versus compact speedup: 1.66×. Mech optimized versus optimized BQN: 17.93× faster in this measurement.

## Validation and LOC

Each endpoint passes independent C-reference checks at 1, 2, 10, 1,000, 10,000, and 1,000,000 steps. Every timed 50-million-step output passes the same reference within the existing 1e−8 energy tolerance. Initial and final energies are checked.

LOC separates packed, nested, and chained assignments. The new K and BQN sources have one assignment or executable statement per physical line; their normalized and physical counts agree. Matrix literals use one coordinate row per line. This changes the source representation rather than the counting rule. Source and binary hashes are pinned. Both Mech endpoints use the v0.4 integration with general native transpose support and default final-statement output selection. The new native compute objects match the original baselines byte for byte.

Mech source excludes its general repetition/output runner, while the array scripts include those calls. Timings include repetition and output for every endpoint. The spectral-norm and k-nucleotide array sources are not covered by this timing run.

## Measurement details

Rounds are randomized with seed 2026100915. All 25 samples in this confirmation are retained; the prior source-compaction confirmation remains separate. Conditions are checked at both sample boundaries and every ten seconds for longer runs. No power change or competing compiler activity was detected. Sample SD describes the five observations; it is not a confidence interval for the median.

The confirmation uses a blocking process wait with a separate 180-second stall watchdog. The earlier published-source run used Python subprocess timeout polling, which can add process-reaping delay. All endpoints, including Mech, were remeasured with the same blocking method for this comparison. The earlier results are preserved separately and are not pooled with these samples.

## Evidence

- [Final confirmation samples, order, conditions, outputs, hashes, and checks](comparison-mech-compact.json)
- [Final confirmation measurement script](measure_mech_compact.py)
- [Previous 32/37-line Mech confirmation](comparison-compact.json), [script](measure_compact.py), and [report](RESULTS-32-37.md)
- [Mech compact](sources/nbody-matrix-short.mec), [Mech optimized](sources/nbody-matrix-pairwise.mec)
- [Mech builds and complete-state comparison](../../nbody/experiments/20261009-source-compact/source-compaction.json) and [reproduction script](../../nbody/experiments/20261009-source-compact/prepare.py)
- [Prior optimized-source confirmation](comparison.json) and [script](measure_fast.py)
- [Original published-source run](results.json) and [its script](measure.py)
- [Selection pilots, first pass](candidate-pilots.json), [second pass](candidate-pilots-2.json), [third pass](candidate-pilots-3.json), [shorter-source pilots](compaction-pilots.json)
- [Runtime builds and commits](runtime-builds.json)
- [Performance and LOC chart](performance-loc.png)
- [BQN compact](sources/nbody-compact.bqn), [BQN optimized](sources/nbody-optimized-short.bqn), [K compact/optimized](sources/nbody-optimized-short.k)
- [Compacted LOC audit](compacted-loc-audit.json) and [source rewrite script](compact_sources.py)
- [Current LOC audit including the shorter Mech sources](mech-compacted-loc-audit.json)
- [Published BQN source](sources/nbody.bqn), [published K source](sources/nbody.growler.k), [first 82-line BQN variant](sources/nbody-flat-cached-reduce.bqn)
- [Original normalized LOC audit](/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-png/iversonian-sources/normalized-loc-audit.json)
