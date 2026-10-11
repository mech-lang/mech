# Mech execution paths

The fifteen existing Mech source candidates cover all ten full Benchmark Game
workloads. Every candidate is measured in both JIT and loaded interpretation;
fastest and fewest-LOC endpoints are selected independently for each path.

`mech-native-source SOURCE jit|resident [arguments] [numeric flags]` admits the
source's typed graph. The runner selects sequence, control, grid, or fixed-shape
reactive execution by structural admission. It contains no benchmark-name
routing or native benchmark executors. JIT uses the same generic lowerings and
optimization passes as AOT. Loaded interpretation executes typed instruction
plans and general standard-library operations, including source-defined matrix
coefficients, control flow, recursion, scanned output, and allocation regions.

JIT reports fresh-process wall time, including source loading and compilation.
Resident interpretation reports time after loading and activation, including
input, execution, standard-library work, formatting, complete output and flush.
Every sample starts with fresh initial state and input. Preparation and whole
process times are retained separately. These timing boundaries are shown on the
website; a loaded measurement excludes work included in the other paths.

`prepare.py` rebuilds matching AOT controls and checks every generated native
object against the original. All fifteen objects must be byte-identical before
measurement proceeds. `run.py` measures original and rebuilt AOT controls in
same randomized rounds as the new paths. Three fresh full samples per source,
mode, and control use the original arguments and settings, at most eight
workers, AC power and Low Power Mode off. Compiler contention and power changes
cause samples to be discarded. Complete output hashes and numeric references
are checked outside the timed regions, independently for every sample.

`conformance.py` exercises forty independent semantic checks across both modes:
parallel state copies, lazy branches, overflow and range limits, effect and
allocation counts, signed bigint aliases, counted scans and buffer boundaries,
rectangular matrices and transposes, unused coefficient arguments, eager error
semantics, captured iteration, packed histogram boundaries, and SIMD state tails.
`smoke.py` checks every source at its original small workload. It is a diagnostic
run; authoritative flags, environment and full measurements are in `builds.json`
and `timings.json`.

All benchmark computation remains in the counted Mech sources. Runtime and
compiler additions are reusable typed operations, not benchmark-specific kernels.
