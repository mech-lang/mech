PolyBench/C native MINI regressions

The thirty Mech programs use the F64 MINI profile and matrix operators,
broadcasting, comprehensions, pattern matching and finite-state machines. The FSM ports specialize their closed
payload kinds to the upstream MINI dimensions. All thirty kernels are covered;
this fixture does not qualify larger datasets.

cases.json contains the C initialization inputs and all live-out values from its unmodified PolyBench/C 4.2.1 beta initializer and kernel. The C
adapters were checked independently against untouched upstream dump output.
The fixture records the upstream archive URL/SHA256, each C source path/SHA256,
the Mech source hashes, dataset, floating-point flags and canonical data layout.
The inputs and expected results were generated from C, not from Mech execution.
No C compiler, network connection or external benchmark checkout is needed to
run this regression. Expected values are finite; comparisons use rtol=1e-10
and atol=1e-9.

Matrices are canonical row-major snapshots. For doitgen and heat-3d, the first
two tensor axes are flattened into rows. C kernels use the supported
DATA_TYPE_IS_DOUBLE option, including
Deriche (whose default is F32) and the graph tables (whose default is integer).
Nucleotide bases remain byte-valued C inputs and use exactly representable
F64 cells in Mech. C identifiers containing underscores become Mech
names containing hyphens. The first trisolv row uses an explicit zero-prefix
case because the maintained range operation rejects an empty range. Deriche
expresses exp(x) using the maintained math/pow operation with Euler's constant;
its complete C outputs are checked with the same tolerance as the other kernels.

Run from the repository root:
cargo test -p mech-gpu --no-default-features --features canonical-native-program --test canonical_artifact_polybench

The test checks every live-out of original and decoded artifacts, emits their
Objects, preserves input aliases, verifies Mech source hashes and checks that
the shared managed-memory account is empty after each completed case. All
execution and library dimensions have explicit finite limits. The full-suite
work allowance is 10^15 conservative work units; matrix regression tests
separately retain tight admission-denial, rollback and recovery checks.

The polybench_qualification example accepts a source path, a JSON object of
named C initialization inputs, and an optional sample count. Matrices contain
shape:[rows,columns] and canonical row-major values; U64 parameters contain
u64:value, and scalar F64 inputs are JSON numbers. A zero sample count checks
one fresh native invocation without timing samples. A positive sample count
adds warmed, cache-flushed checked invocations. Parsing, compilation and input
import stay outside those intervals; resource checks and output creation stay
inside. The explicit resident-reference mode is a separate diagnostic path.

First complete-suite timing baseline (Apple M3, MINI, F64)

baseline-m3-mini.json pins the compiled source commit, driver and C executable
hashes, input hashes, toolchains, five raw samples per actor/comparison and the
medians below. The 600 observations alternate Mech/C order and flush 32 MiB
before each timed invocation. Parsing, JIT compilation and input import are
outside Mech timing; finite checks, execution and output creation are inside.
C kernel time excludes setup. The separate C fresh comparison includes array
allocation and initialization copies. A process's untimed warm-up result is
validated against every C live-out; timed calls must complete successfully.

All rows use MINI. The earlier 3mm MEDIUM comparison (about 1.26x C runtime)
uses a different problem size. This is the first complete-suite baseline, with
no further compiler optimization after the 30 ports were completed.

Mech/C is the runtime multiplier: 2x means Mech takes twice C's time.
These exploratory medians include substantial setup/checking overhead at MINI
sizes. F64 FSMs use canonical matrix state and carry large recurrence costs.
The measurements qualify the checked CPU path; GPU/SIMD selection remains
follow-up work.

kernel              C kernel ms  Mech checked ms   Mech/C x  Mech/C fresh x
2mm                     0.002042         0.157292      77.03           56.25
3mm                     0.002541         0.096875      38.12           19.85
adi                     0.044667       966.202292   21631.38        21202.15
atax                    0.001208         0.095625      79.13           42.34
bicg                    0.001459         0.128292      87.94           33.44
cholesky                0.006708       158.733750   23662.23        20536.60
correlation             0.004292         1.407500     327.94          200.74
covariance              0.004416         0.222833      50.46           29.18
deriche                 0.028917      1259.650292   43560.74        35553.69
doitgen                 0.003167         0.071167      22.47           11.82
durbin                  0.001333         5.198625    3899.39         2195.22
fdtd-2d                 0.006916        72.621833   10500.55         7334.14
floyd-warshall          0.065542       317.310458    4841.31         4687.48
gemm                    0.004416         0.159208      36.05           24.07
gemver                  0.001791         0.466958     260.73          120.82
gesummv                 0.001083         0.121666     112.38           30.29
gramschmidt             0.003584        99.972917   27896.34        18052.80
heat-3d                 0.017750        74.166542    4178.38         3714.46
jacobi-1d               0.000583         4.179083    7168.14         1898.66
jacobi-2d               0.007667        73.692708    9612.12         7796.67
lu                      0.011583       241.279417   20830.73        19084.08
ludcmp                  0.012041       249.535750   20723.72        16230.06
mvt                     0.001291         0.179375     138.96           70.57
nussinov                0.030000      3589.172708  119638.18       113810.07
seidel-2d               0.110375      5537.376375   50168.86        49447.51
symm                    0.003750         0.817875     218.10          131.15
syr2k                   0.006708         1.506208     224.54          180.94
syrk                    0.005500         1.348042     245.10          173.87
trisolv                 0.001084         3.909291    3606.16         1080.09
trmm                    0.003208         0.704500     219.61          124.29

qualification-m3-mini.json records all 30 C comparisons and regression checks.

Compiler optimization protocol and performance history

optimization-protocol.json freezes 15 optimization programs and 15 holdout
programs before the first optimization round. Close variants stay together.
The optimization programs are 2mm, 3mm, atax, bicg, cholesky, correlation,
covariance, deriche, doitgen, durbin, gemm, gramschmidt, mvt, nussinov and
seidel-2d. The other fifteen are the holdout. Every original baseline timing
was already visible; this is a fixed holdout for subsequent tuning, not a blind
experiment. Future rounds disclose previous holdout exposure. A separately
selected application will be needed for fresh external validation.

The Mech programs and all C-derived inputs/results remain pinned. Changes
target general compiler/runtime machinery shared by other programs. Candidate
profiles and tuning timings use only the optimization partition. Freeze the
candidate before any holdout timing and retain all holdout results, including
regressions; do not retune that candidate against holdout per-case results.
Correctness tests may run on both partitions throughout development.

performance-history.json starts with the original baseline and appends one
entry per frozen candidate. The primary aggregate is the equal-weight geometric
mean of baseline Mech time / candidate Mech time, separately for optimization,
holdout and all kernels. Larger speedup is better. Mech/C runtime ratios use
separate kernel-only and fresh-allocation comparisons; smaller is better.
Median/minimum/maximum speedups, regressions over 5%, C wins, worst C ratio and
summed medians accompany the geometric means. Summed medians are secondary
because they heavily weight the slowest programs. A missing case makes an
aggregate incomplete; a failed numerical comparison invalidates the candidate.

scripts/prepare-polybench-c.py preserves the original adapter generator and
builds the C side from a PolyBench/C 4.2.1 beta checkout. Obtain the archive URL
and SHA256 from cases.json, verify the archive, and extract it outside the
repository. This recipe verifies every C source hash, generated input and full
live-out. The timing runner additionally requires executable hashes identical
to the pinned C baseline; changing the C toolchain requires an explicit new
baseline rather than silently changing comparisons.

Run from the repository root, with paths chosen for this host:

python3 scripts/prepare-polybench-c.py --suite /path/to/polybench-c-4.2.1-beta --output /path/to/c-mini
CARGO_TARGET_DIR=/path/to/target cargo build --release -p mech-gpu --no-default-features --features canonical-native-program --example polybench_qualification
python3 scripts/benchmark-polybench.py freeze --label round-01 --driver /path/to/target/release/examples/polybench_qualification --output /path/to/frozen.json
python3 scripts/benchmark-polybench.py run --partition optimization --driver /path/to/target/release/examples/polybench_qualification --c-build /path/to/c-mini --freeze /path/to/frozen.json --output /path/to/optimization.json
python3 scripts/benchmark-polybench.py run --partition holdout --driver /path/to/target/release/examples/polybench_qualification --c-build /path/to/c-mini --freeze /path/to/frozen.json --output /path/to/holdout.json
python3 scripts/benchmark-polybench.py summarize --optimization /path/to/optimization.json --holdout /path/to/holdout.json --output /path/to/progress.json --history hosts/gpu/tests/fixtures/polybench/performance-history.json

Use an immutable copy of the original baseline executable with
--reference-driver /path/to/baseline-driver on freeze and both run commands to
add contemporaneous paired baseline observations. This separates historical
machine drift from compiler improvement. Five observations per actor and C
comparison alternate execution order; all actors run sequentially. Do not run
builds or profiles alongside measurements. The candidate receipt pins compiler
source and executable fingerprints, protocol/fixture/runner hashes and machine
metadata. Source or driver changes after freezing are rejected. Keep the raw
partition receipts alongside each aggregate entry.
