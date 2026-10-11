# General execution optimizations

This cohort preserves the fifteen counted Mech sources, complete workloads,
worker limits, output validation, and timing boundaries from `../20261010`.
Each source is measured in JIT and resident interpretation, with an unchanged
AOT control in the same three randomized rounds. Candidate endpoints are chosen
from all measured sources independently for each path. Prior measurements and
binaries are retained; the optimized runner has a separate executable name.

Native admission shares one canonical parsed document across sequence, control,
and grid frontends. Each still checks its own kinds and operations. Matrix
normalization reuses that syntax tree and reparses only changed text. The JIT
control backend consumes the already-admitted IR instead of compiling it again.
The autonomous numeric runner retains the caller's loading/admission timing.

Resident scalar coefficient calls are interpreted in independent tiles of at
most 32 elements. Each source arithmetic instruction still executes for every
coefficient. Typed elementwise loops amortize VM dispatch and allow the host
compiler to vectorize general operations. Unsigned range proofs must cover the
entire invocation before checked arithmetic can become wrapping arithmetic;
otherwise the original checked scalar interpreter remains active. Constant
power-of-two divisors become shifts. Scalar accumulation order, transposed
indices, and tail bounds are preserved. Coefficients and results are recomputed
for every matrix product; no workload-specific formula or caching is present.

`conformance.py` checks 27 independent fixtures in both modes (54 checks),
including rectangular matrices, both multiplication orientations, tile lengths
1/31/32/33/65, variable integer division, unused parameters, eager arithmetic
errors before effects, scans, captures, allocations, and packed-window tails.
`check_objects.py` independently verifies that all fifteen emitted AOT objects
match the original bytes. `run.py` rechecks small and full outputs for every
source/mode/control, freezes source and binary hashes, monitors AC power and
compiler contention, and retains raw process/preparation/execution measurements.
Only completed full-cohort measurements are packaged in the website.

Build with the same native CPU and GMP configuration as the original cohort:

```sh
C_INCLUDE_PATH=/opt/homebrew/include LIBRARY_PATH=/opt/homebrew/lib \
RUSTFLAGS='-D warnings -C target-cpu=native -L native=/opt/homebrew/lib' \
CARGO_TARGET_DIR=/tmp/mech-execution-modes-target \
cargo build --release --offline --manifest-path driver/Cargo.toml
```

Then run `python3 conformance.py`, `python3 check_objects.py`, and
`python3 run.py`. The full suite retains three samples for each of 45 entries
(135 samples). Resident times include fresh input, source computation, complete
output and flush after loading and activation. JIT and AOT controls use full
fresh-process time; JIT includes compilation. Comparing loaded interpretation
with either other path excludes their startup costs and is explicitly labelled.
