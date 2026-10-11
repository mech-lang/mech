# General execution optimizations

This cohort preserves the fifteen counted Mech sources, complete workloads,
worker limits, output validation, and timing boundaries from `../20261010`.
Each source is measured in JIT and resident interpretation, with an AOT control
in the same three randomized rounds. The k-nucleotide control is freshly linked
with the updated counting runtime; the other controls retain their original
executables. Candidate endpoints are chosen
from all measured sources independently for each path. Prior measurements and
binaries are retained; the optimized runner has a separate executable name.

Native admission shares one canonical parsed document across sequence, control,
and grid frontends. Each still checks its own kinds and operations. Matrix
normalization reuses that syntax tree and reparses only changed text. The JIT
control backend consumes the already-admitted IR instead of compiling it again.
The parsed document is released immediately after successful admission.
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

The standard counting container's batch update keeps its table dispatch inlined
with its caller. The update loop and every input key remain unchanged. A separate
five-pair full-input confirmation against the original runner is retained in
`../20261010-optimization/inline-batch-check.json`; these diagnostic samples are
not used as chart samples. This cohort freezes that final runner before timing
all sources in fresh randomized rounds.

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

The command above builds the validation emitter. The measured runner was built
with the same environment using `../20261010-optimization/driver/Cargo.toml`
and `--bin mech-execution-modes-opt3`, then copied to
`.build/mech-execution-modes-final` before validation. `runner-build.toml` and
`runner-build.lock` retain that manifest and dependency lock; `snapshots.json`
pins the runtime source snapshot and immutable measured executables.

Then run `python3 conformance.py`, `python3 check_objects.py`, and
`python3 run.py`. The full suite retains three samples for each of 45 entries
(135 samples). Resident times include fresh input, source computation, complete
output and flush after loading and activation. JIT and AOT controls use full
fresh-process time; JIT includes compilation. Comparing loaded interpretation
with either other path excludes their startup costs and is explicitly labelled.

The completed cohort passed all 135 full-workload samples and 54 semantic checks,
with fifteen identical native objects and no discarded timing runs. Fastest
source medians compare with the original execution-mode cohort as follows:

| Benchmark / path | Before (s) | Final (s) | Speedup |
| --- | ---: | ---: | ---: |
| Spectral norm / resident | 5.184886 | 0.457014 | 11.35× |
| Pi digits / JIT | 0.719950 | 0.447466 | 1.61× |
| FASTA / JIT | 1.126172 | 0.765849 | 1.47× |
| Binary trees / JIT | 0.733148 | 0.525069 | 1.40× |
| Mandelbrot / JIT | 1.066208 | 0.783310 | 1.36× |
| Fannkuch / JIT | 2.476235 | 2.020614 | 1.23× |
| N-body / JIT | 1.628762 | 1.380457 | 1.18× |

Resident fannkuch measured 61.607569 s versus 59.393436 s previously (3.7%
slower). The raw samples are retained, including this result. No source/mode
median in the cohort was more than 5% slower than its original measurement.
The AOT controls' geometric mean elapsed-time ratio is recorded in `summary.json`;
the native code is unchanged, and k-nucleotide's newly linked counting runtime
is recorded separately. The explorer retains its earlier published AOT timings,
including the paired Rust/Mech floating-point FASTA comparison, while replacing
the JIT and resident timings with this completed cohort.
