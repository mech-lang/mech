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
entry per frozen candidate. The primary score is the equal-weight geometric mean of median Mech / C
kernel runtime on all fifteen holdout programs; the goal is strictly below
1.0x. Baseline Mech / candidate Mech speedup is a secondary aggregate, reported
separately for optimization, holdout and all kernels; larger is better. Mech/C
runtime ratios use separate kernel-only and fresh-allocation comparisons;
smaller is better.
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

First compiler optimization round (Apple M3, MINI, F64)

The primary optimization score is now pinned in optimization-objective.json:
the equal-weight geometric mean of median Mech / C kernel runtime ratios on
all fifteen fixed holdout programs. Lower is better; the goal is strictly below
1.0x. Total median runtime and baseline speedup remain secondary measures.
Before the first optimization the recorded holdout score was 2020.085x C.
Round 01 scored 1610.274x C, with 1.634173s summed Mech median runtime.
The preserved original compiler replayed alongside round 01 scored 1981.563x C
and 2.004918s; the paired compiler speedup is 1.231x. Historical timing and
paired replay are different observations of the same original compiler.
New freezes pin the objective hash and new progress receipts include its score;
historical raw receipts and compiler fingerprints are preserved unchanged.

round-01-frozen.json pins compiler commit b791cd541ce2a39d46e3ea1236523fc111019b78.
The compiler transports packed scalar matrices through existing private tuple
and closed-enum frames, returns typed scalar words through the maintained read
boundary, and shares packed selected-assignment construction. Single-cell
planning prepays the identical scan charge only when finite admission permits
it. Bool/I64 matrix composition and turn-shaped recursive patterns are admitted
through the same checked machinery. No benchmark source or C binary changed.

The candidate was frozen before holdout timing. The two raw partition receipts
contain 900 sequential observations: five per actor (candidate, preserved
baseline and C) per kernel/fresh comparison. Warm-up live-outs are validated;
timed Mech calls must succeed, as in the original baseline boundary. Every
observation passed the finite full-output warm-up comparison; timed outputs
are not independently flattened for validation.

Speedup is baseline Mech time / candidate Mech time; larger is better.
The replay column uses the preserved baseline executable measured alongside
the candidate. Mech/C is a runtime multiplier; smaller is better.

partition       recorded x    replay x    Mech/C x    C fresh x
optimization         1.731       1.761      441.86       266.05
holdout              1.233       1.231     1610.27       962.11
all                  1.461       1.472      843.51       505.93

These are equal-weight geometric means. Median per-kernel speedups are only
1.014x on optimization, 1.048x on holdout and 1.025x overall: the largest gains
are concentrated in a few recurrence-heavy programs. Nussinov improves 26.50x,
Seidel 7.58x, Cholesky 4.03x and Deriche 3.80x against recorded Mech timings.
Holdout gains include Jacobi-1d 2.23x, LU 1.99x and LUDCMP 1.99x.

Five optimization kernels exceed a 5% slowdown against historical timings
(2mm, 3mm, atax, bicg, covariance); none do on holdout. Against the contemporary
baseline replay, Doitgen is the one >5% regression (1.080x slower); none of the
holdout kernels exceed 5%. These measurements do not establish the cause of
the discrepancy. Both comparisons and every case are retained.

All thirty kernels remain slower than C in this checked MINI comparison.
The worst ratio is Gramschmidt at 26,878.28x C, with only a 1.03x speedup; it
is the next profiling target in the optimization set. Matrix copying and checked
access overhead remain targets from the existing recurrence profiles.

Training profiles after typed collector storage (IR357)

round-02-training-profile.json records compiler commit
29ec7445554053a55bb6d4c941b243cc65cef922, the exact executable/source hashes,
25,222 execution samples, symbolized stacks, disjoint classifications and raw
profile hashes. Five compressed macOS sample call trees are retained under
profiles/round-02. Each process ran alone; sample collected seven seconds at a
nominal 1ms interval after a 2s startup delay. Only stacks below the canonical
runtime execute boundary enter the percentages. Parsing, JIT compilation,
input import, cache flush and output serialization are excluded by stack.

The five cases account for 99.58% of the optimization partition's summed
round-01 median runtime. This prioritizes total CPU work; it does not change
the equal weighting of all fifteen programs in the primary holdout score.

case             library calls   visit admission   frame transport   collection
seidel-2d               76.5%              7.9%              6.7%         0.0%
deriche                 82.5%              5.4%              6.2%         1.3%
nussinov                26.8%             20.3%             32.4%         0.0%
gramschmidt             43.1%             23.1%              0.0%        14.0%
cholesky                70.1%              9.7%              2.7%         5.0%

These columns are selected disjoint categories, not a complete 100% table;
the receipt also includes typed reads, helper dispatch and loop cleanup.
Weighted by those five round-01 medians, ordinary library calls account for
70.3% of execution samples. A separate leaf-symbol view attributes 27.9% to
memory planning/schema/snapshot validation, 18.4% to allocation/deallocation,
10.4% to resource bookkeeping/locks and 7.3% to byte copying/zeroing. The two
views overlap and must not be added together. Inclusive function costs also
overlap: indexed assignment occurs under many of those same planning calls.

Seidel's indexed_assign_matrix_selection is on 32.2% of execution stacks;
Deriche's indexed_assign_snapshot is on 28.5%. plan_call_memory appears on
23.1% and 28.0%, respectively. In Gramschmidt, transpose_snapshot appears on
13.4% of stacks, including its admission/planning; sum_snapshot appears on
2.2%. Nussinov additionally spends substantial time moving and validating
private frames. These observations prioritize extending the shared admitted
typed matrix update path and avoiding repeated generic call planning. Raw
arithmetic and fused comprehension changes have not yet been established as
the largest remaining costs. Inlining, unresolved JIT symbols and sampling
uncertainty limit precision; unresolved JIT self samples are not a measurement
of all numerical arithmetic.

The collector storage change passes 23 matrix, all 30 PolyBench and 95 selected
Program integration tests, plus seven focused collector/row-block units. A
seven-sample Gramschmidt probe improved only 1.018x against the round-01 binary;
that is a training probe, not a frozen aggregate result. No new holdout timing
was performed for this collector-only candidate. At that point the latest
qualified holdout score was 1610.274x C. The frozen round below includes both
collector storage and the subsequent scalar matrix update change.

Recompute classifications using a Rust symbol filter (one name per line):
python3 scripts/analyze-polybench-samples.py --demangler /path/to/rustfilt --output /tmp/profile-analysis.json hosts/gpu/tests/fixtures/polybench/profiles/round-02/*.sample.txt.gz
The recorded filter used rustc-demangle 0.1.28 with alternate Display formatting
to omit symbol hashes. The analyzer tests verify execution-only denominators,
disjoint categories, inclusive/self separation and malformed count rejection.

kernel             split   recorded x    replay x       Mech/C x      C fresh x
2mm                opt          0.940       0.987          85.46          62.65
3mm                opt          0.910       0.975          37.57          16.94
adi                holdout      1.124       1.122       18651.47       18147.49
atax               opt          0.919       1.047          80.60          28.87
bicg               opt          0.895       0.996          95.56          29.50
cholesky           opt          4.031       4.017        5369.23        4328.15
correlation        opt          1.010       1.001         312.73         184.64
covariance         opt          0.943       0.991          50.66          34.86
deriche            opt          3.803       3.774       11103.47        8440.70
doitgen            opt          1.049       0.926          22.30          11.47
durbin             opt          1.687       1.676        2309.92        1192.70
fdtd-2d            holdout      1.056       1.048        9431.99        6652.65
floyd-warshall     holdout      1.022       1.028        4540.17        4468.64
gemm               opt          1.014       1.016          36.59          23.14
gemver             holdout      1.023       1.001         260.84         116.47
gesummv            holdout      0.995       1.024         108.70          33.99
gramschmidt        opt          1.026       1.023       26878.28       19748.64
heat-3d            holdout      1.066       1.052        3822.54        3509.24
jacobi-1d          holdout      2.233       2.226        4088.34         727.59
jacobi-2d          holdout      1.048       1.049        8975.12        7259.19
lu                 holdout      1.993       1.987       10156.59        9402.30
ludcmp             holdout      1.990       1.986        9585.97        7805.44
mvt                opt          0.992       1.002         140.01          63.42
nussinov           opt         26.496      26.459        4357.94        4316.46
seidel-2d          opt          7.583       7.556        6618.34        6506.65
symm               holdout      1.006       0.996         209.89         128.84
syr2k              holdout      1.002       0.997         217.41         159.81
syrk               holdout      1.006       1.008         234.72         167.83
trisolv            holdout      1.865       1.857        1863.09         473.51
trmm               holdout      1.003       1.008         205.65         117.86

round-01-qualification.json records 8 core unit checks, 23 engine unit checks,
one route restoration check, 22 matrix integration tests, all 30 PolyBench
original/decoded cases (29,482 C live-outs each), and 78 selected scalar, buffer
and compression regressions. This includes allocation guarantees, original
work-prefix denials, alias safety, restored metadata and fresh recovery.
The remaining 439 Program tests were filtered out. The three timing-runner
unit tests also pass. This receipt covers MINI CPU execution; larger datasets
and GPU/SIMD selection still need qualification.

Second frozen compiler round (IR358): scalar matrix updates

round-02-frozen.json pins compiler ea3977e8f1cf3aa085424b854816bbb6e81d85db.
The shared admitted matrix replacement path now handles all seven native
scalar kinds and independent row/column addressing. It reuses canonical
backing storage only with exclusive same-account ownership and retains the
copy path for aliases. Exact bits, error ordering, finite accounting and
original unsigned native-buffer guarantees are preserved. No benchmark source,
input, C binary or numerical tolerance changed.

The holdout primary score is 1529.299x C, compared with round 01's 1610.274x C:
a 1.053x improvement in the score. The goal strictly below 1.0x C remains open;
all thirty kernel medians are slower than C. Summed holdout Mech medians are
1.562742s, versus 1.634173s in round 01. Optimization summed medians fell from
1.339377s to 0.456340s (2.935x); that large training gain generalizes only
modestly to the primary holdout score.

The candidate was frozen before holdout timing. Both partition receipts retain
900 sequential observations, five per actor (candidate, original compiler, C)
per kernel/fresh comparison. Full finite warm-up live-outs pass for each
observation; timed checked calls succeed. These timing runs were separate from
profiling. The fixed split and earlier holdout results were already visible;
this is not a blind external validation.

partition       recorded x    replay x    Mech/C x    C fresh x
optimization         2.037       2.317      346.02       198.18
holdout              1.310       1.306     1529.30       920.89
all                  1.633       1.740      727.44       427.21

Seven training cases regress over 5% against recorded historical timings.
Against the contemporaneous original compiler replay, 3mm is 1.067x slower
and covariance 1.074x slower; the cause is not established. Neither comparison
has a holdout regression above 5%. Both reference comparisons are retained.

Post-update profiles in round-02-post-update-profile.json retain 15,040
execution samples across three optimization programs. Compressed raw stacks
are under profiles/round-02-after-updates. The sampling procedure and execution
boundary match the earlier profiles. The new analyzer also recognizes generic
native update bodies; its exact hash is recorded. Recompute with the preceding
analyzer command, changing the input glob to that directory.

case             library calls   visit admission   frame transport   collection
seidel-2d                0.0%             31.4%             27.7%         0.0%
nussinov                 0.0%             27.0%             44.3%         0.0%
gramschmidt             42.8%             22.6%              0.0%        14.4%

Ordinary canonical library-call overhead has disappeared from the sampled
Seidel/Nussinov update paths. Admission plus frame transport now accounts for
59.1% and 71.2%, respectively; typed matrix scalar reads add 16.1% and 7.2%.
Gramschmidt still spends 42.8% in ordinary library calls and 14.4% in collection
bookkeeping. These observations prioritize using existing native loop-region
machinery to batch visits/frame transport, and extending native numerical
comprehension fusion. These are next targets, not claimed implementations.
Categories are disjoint; leaf and inclusive function views overlap them.
Inlining, unresolved JIT symbols and sampling uncertainty limit precision.

round-02-qualification.json records 25 core checks, 24 matrix integration
checks, all 30 original/decoded PolyBench programs (29,482 finite C live-outs
per artifact), 102 selected Program regressions, nine route/lifetime/collector
checks and eight Python checks. Coverage includes all seven scalar kinds,
both addressing modes, exclusive pointer reuse, immutable aliases, exact F64
bits, restored metadata, Object emission and one-unit-short finite denials.
The remaining 415 Program tests were filtered out. Full compression application
and AOT qualification, larger datasets and GPU/SIMD execution are not covered.

kernel             split   recorded x    replay x       Mech/C x      C fresh x
2mm                opt          0.759       0.962          76.54          59.04
3mm                opt          0.688       0.937          44.47          27.07
adi                holdout      1.157       1.152       18501.20       17626.25
atax               opt          0.714       1.087         107.07          31.16
bicg               opt          0.915       1.026          82.06          32.22
cholesky           opt          6.799       6.965        3183.12        2231.34
correlation        opt          0.968       1.003         293.21         142.19
covariance         opt          0.764       0.931          60.85          33.14
deriche            opt         18.164      18.478        2273.74        1742.83
doitgen            opt          0.770       1.045          27.73          13.69
durbin             opt          1.674       1.721        2070.48         977.56
fdtd-2d            holdout      1.077       1.075        9521.00        6714.60
floyd-warshall     holdout      1.045       1.049        4535.99        4406.31
gemm               opt          1.023       1.022          34.27          22.69
gemver             holdout      1.025       0.998         227.83         100.24
gesummv            holdout      1.008       1.020         120.78          31.22
gramschmidt        opt          1.030       1.056       25054.64       15228.98
heat-3d            holdout      1.085       1.069        3745.56        3416.66
jacobi-1d          holdout      2.236       2.244        3737.10         767.91
jacobi-2d          holdout      1.061       1.065        8915.81        7264.04
lu                 holdout      2.317       2.315        8924.88        7580.93
ludcmp             holdout      2.335       2.331        8435.50        6598.90
mvt                opt          0.828       1.005         136.86          62.82
nussinov           opt         35.314      35.627        3309.76        2407.83
seidel-2d          opt         34.831      35.570        1415.24        1389.97
symm               holdout      1.016       1.009         209.85         134.82
syr2k              holdout      1.017       1.011         207.86         167.55
syrk               holdout      1.013       1.019         245.54         175.72
trisolv            holdout      2.878       2.870        1254.12         351.70
trmm               holdout      1.014       1.006         210.94         133.71

Third frozen compiler round (IR359): typed native comprehension loops

round-03-frozen.json pins compiler e68d404257b794b4daaea33c4dd7b6310b3896d3.
The existing native scalar map path now supports all seven scalar kinds,
prepared U64 range cursors, immutable captured matrix reads and nested leaf
comprehensions. Source validation and matrix geometry are hoisted once per
loan; scalar operations and checked matrix loads emit directly through the
maintained Cranelift machinery. Each source stays pinned to its current arena
handle and budget account. Failed arithmetic or bounds attempts discard their
private output and replay the original checked path. Existing unsigned-only
raw-buffer APIs retain their narrower contract; typed map stores use a separate
exact-kind sink, with a canonical Bool guard before each store.

Complete maps allocate exact output capacity and avoid a possible final
shrink-copy. Both read/write logical copy floors remain admitted. The filtered
fallback still funds its growing bank and final copy, including exact
one-unit-short denial, prior-output preservation and recovery. The instruction
audit now includes the numerical map header debit already present in both
visit tables. Native work policy is 128. No Mech port, input, C binary, timing
boundary or tolerance changed.

The holdout primary score is 1468.557x C, versus round 02's 1529.299x C:
a 1.041x improvement in the score. Summed holdout Mech medians are 1.539413s,
versus 1.562742s. The paired original compiler speedup is 1.369x; all thirty
kernel medians are still slower than C. The goal strictly below 1.0x remains
open. These score comparisons span separate timing rounds; paired replays and
raw C observations are retained to expose timing variation.

partition       recorded x    replay x    Mech/C x    C fresh x
optimization         2.488       2.433      318.84       189.07
holdout              1.372       1.369     1468.56       893.55
all                  1.847       1.825      684.28       411.03

There are no >5% historical regressions in this round. Against the paired
original compiler replay, Bicg is 1.059x slower; the cause is unproven. No
holdout program exceeds a 5% regression in either comparison. The fixed split
and all earlier holdout timings were already visible. The compiler was frozen
before holdout timing, and the two receipts retain all 900 sequential
observations with full finite warm-up validation and successful timed calls.

Before freezing, a separate seven-sample training probe replayed round 02
against this compiler on all fifteen optimization programs. Its equal-weight
speedup is 1.059x with no regression above 5%; Cholesky improves 1.477x,
Durbin 1.214x, Deriche 1.124x and Gramschmidt 1.070x. This is not a C comparison
or a holdout aggregate. Gramschmidt source visits fall from 648,403 to 569,803;
Cholesky falls from 114,632 to 29,352. Native loop work remains prepaid.
round-03-training-probe.json preserves the raw samples, reports and hashes.

round-03-qualification.json records 17 focused native-map/collector/route
checks, 24 matrix integration checks, all 30 original/decoded PolyBench
programs and 102 selected scalar/buffer/compression regressions. Actual
Gramschmidt and Cholesky native loop selection is observed in original,
decoded and restored executions and compared to complete finite C live-outs.
The other 1416 GPU units and 415 Program integration tests were filtered out;
this is not full application/AOT or GPU/SIMD qualification.

The remaining source gaps include conditional comprehension bodies and full
numeric regions over tagged machine state. The existing numeric Tail proof
requires untagged array fields and restricts its private mutable arrays to
unsigned storage. Extending those shared proofs and emissions is the next
compiler work; benchmark sources remain unchanged.

kernel             split   recorded x    replay x       Mech/C x      C fresh x
2mm                opt          1.006       0.993          96.25          51.70
3mm                opt          1.020       1.007          35.06          17.16
adi                holdout      1.155       1.145       18234.00       17735.88
atax               opt          1.000       1.095          71.73          26.76
bicg               opt          0.966       0.944          91.05          29.43
cholesky           opt         10.186      10.131        2412.99        1841.58
correlation        opt          1.068       0.997         309.99         161.70
covariance         opt          1.001       0.987          49.47          35.18
deriche            opt         20.809      20.707        2072.51        1667.91
doitgen            opt          1.148       0.980          19.84          10.47
durbin             opt          2.053       2.038        1898.51        1032.41
fdtd-2d            holdout      1.069       1.059        9644.01        6805.72
floyd-warshall     holdout      1.040       1.045        4651.41        4428.55
gemm               opt          1.089       1.038          34.39          21.81
gemver             holdout      1.016       0.999         245.13         105.01
gesummv            holdout      0.991       1.015         109.11          29.58
gramschmidt        opt          1.117       1.114       24688.77       16651.70
heat-3d            holdout      1.082       1.079        3933.86        3511.91
jacobi-1d          holdout      2.232       2.225        3743.27         912.31
jacobi-2d          holdout      1.058       1.060        8991.34        7390.95
lu                 holdout      2.641       2.630        7586.95        6919.60
ludcmp             holdout      2.689       2.691        6894.56        6154.63
mvt                opt          1.060       1.002         139.97          65.40
nussinov           opt         35.612      35.592        3350.11        3235.35
seidel-2d          opt         35.544      35.357        1411.46        1390.95
symm               holdout      1.023       1.016         217.95         130.59
syr2k              holdout      1.008       1.002         217.38         166.37
syrk               holdout      1.006       1.001         247.32         183.85
trisolv            holdout      4.633       4.621         843.51         223.58
trmm               holdout      1.019       1.019         212.82         123.61

Fourth frozen compiler round (IR360): conditional native comprehension loops

The compiler now admits complete acyclic scalar branch bodies and phi copies
inside exact typed native maps. Original CFG edges preserve source order and
inactive reads/arithmetic stay inactive. Language literal equality uses the
same maintained emitter as ordinary branches, including signed zeros and NaNs.
All seven scalar kinds, prepared ranges, immutable captured matrix reads and
current outer scalar/matrix phi captures use the existing map loan and output
banks. One Push must dominate the successful latch; filters keep the ordinary
route. Failed attempts discard private partial output and replay the source.
The full conditional body is prepaid under work policy129. No Mech port,
input, C executable, tolerance or timing boundary changed.

Both sets use the same metric: the equal-weight geometric mean of median
Mech/C kernel runtime, lower is better. The strict target below 1.0x applies
to the fixed fifteen-program holdout. History retains both sets:

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274

Round 04 aggregate results:
partition       recorded x    replay x    Mech/C x    C fresh x    sum Mech s
optimization         2.487       2.671      301.20       166.68      0.387935
holdout              2.407       2.436      724.27       469.38      0.804653
all                  2.447       2.551      467.06       279.70      1.192588

Compiler 19cd8a6bcb5ba504b6ac22c059c3e33a66d6ee5c was frozen before holdout
timing; driver SHA256 is d000b7b77f102db3d0a68e9d2d738e8767e9cbde364bcd0585ad2da3725b765d.
The strict below-1.0x goal remains open. Raw C observations and paired
original replays are retained because scores across rounds include timing
variation. All 900 sequential observations validate their process's untimed
warm-up against all finite C live-outs; timed checked calls must succeed.
The split and previous holdout timings were already exposed.

Regressions above 5% against the recorded and paired original compilers:
optimization 3mm: 1.059x slower than recorded original.
optimization covariance: 1.313x slower than recorded original.
optimization doitgen: 1.299x slower than recorded original.
optimization gemm: 1.227x slower than recorded original.
optimization mvt: 1.225x slower than recorded original.
holdout gemver: 1.055x slower than recorded original.
holdout gesummv: 1.091x slower than recorded original.

Before freezing, seven samples per actor on all fifteen optimization programs
replayed the round03 compiler against this candidate. This separate training
probe is not a C or holdout comparison:
{"count": 15, "geomean_speedup_from_round_03_replay": 1.1171130340897801, "regressions_over_five_percent": 1, "sum_candidate_median_seconds": 0.386654248, "sum_reference_median_seconds": 0.432214834}
correlation: 1.479x speedup vs round03 replay.
durbin: 1.750x speedup vs round03 replay.
gramschmidt: 1.794x speedup vs round03 replay.
mvt: 0.939x speedup vs round03 replay.

MVT is 1.065x slower in that probe; its source visits and admitted work are
unchanged, and the cause is unproven. All cases are retained. Gramschmidt
source visits fall from 569,803 to 46,693 and Durbin from 16,164 to 1,422.

round-04-qualification.json records 21 focused native/collector/route checks,
24 matrix checks, all 30 original/decoded PolyBench programs and 102 selected
scalar/buffer/compression Program regressions. Actual conditional Gramschmidt
native selection is observed in original, decoded and restored execution and
compared with full finite C outputs. Other 1416 GPU units and 415 Program
integration checks were filtered. Larger datasets, GPU/SIMD execution and full
compression application/AOT qualification remain unqualified.

Post-round-04 profiles cover 99.01% of summed optimization
kernel medians in 5 programs, with 20671 execution-stack samples.
No holdout program was profiled. Parsing, JIT, input import, cache flushing
and serialization are excluded by stack. Profiled times do not enter scores.
round-04-training-profile.json includes raw compressed sample hashes, exact
commands, finite warm-up validation and disjoint action attribution.
 28.34% Private frame transport (weighted by unprofiled case medians)
 23.49% Source visits and admission (weighted by unprofiled case medians)
 16.13% Canonical library calls (weighted by unprofiled case medians)
 10.52% Other helper dispatch (weighted by unprofiled case medians)
  9.34% Typed matrix scalar reads (weighted by unprofiled case medians)
  4.26% Other selected library dispatch (weighted by unprofiled case medians)
  3.50% Selected matrix updates (weighted by unprofiled case medians)
  1.99% Loop-local retirement (weighted by unprofiled case medians)
  1.57% Comprehension collection bookkeeping (weighted by unprofiled case medians)
  0.85% Other execution and generated code (weighted by unprofiled case medians)

Remaining compiler gaps include full native numeric regions over tagged
state and mutable Bool/I64/F64 loop arrays. Existing numeric Tail proofs
require untagged array leaves and restrict private mutable arrays to unsigned
storage. The benchmark sources retain their idiomatic state machines.

kernel             split   recorded x    replay x       Mech/C x      C fresh x
2mm                opt          0.967       1.002          92.91          60.14
3mm                opt          0.944       0.966          38.47          17.71
adi                holdout      1.947       1.950       10426.37       10193.06
atax               opt          0.997       0.989          76.78          31.72
bicg               opt          0.967       0.984          90.94          30.86
cholesky           opt         10.182      10.137        2253.74        1668.12
correlation        opt          1.384       1.470         206.77          98.07
covariance         opt          0.762       0.975          58.51          41.23
deriche            opt         22.032      22.466        1851.78        1558.11
doitgen            opt          0.770       1.018          27.40          13.12
durbin             opt          3.409       3.474        1076.30         431.45
fdtd-2d            holdout      2.453       2.499        3229.89        2753.96
floyd-warshall     holdout      5.250       5.290         882.93         855.83
gemm               opt          0.815       0.959          42.23          23.01
gemver             holdout      0.948       0.975         262.59         107.50
gesummv            holdout      0.916       1.005          39.34          32.92
gramschmidt        opt          1.913       1.940       13632.77        7482.81
heat-3d            holdout      8.966       8.895         454.28         414.03
jacobi-1d          holdout      5.320       5.304        1570.62         289.27
jacobi-2d          holdout      3.363       3.405        2491.94        1886.17
lu                 holdout      2.651       2.659        7944.00        7089.15
ludcmp             holdout      2.711       2.702        6634.92        5782.61
mvt                opt          0.817       0.994         150.57          45.85
nussinov           opt         35.119      35.337        3387.93        3196.50
seidel-2d          opt         35.296      35.284        1396.06        1360.28
symm               holdout      1.387       1.387         152.17          92.46
syr2k              holdout      1.497       1.522         116.63         104.54
syrk               holdout      1.585       1.595         134.27         104.50
trisolv            holdout      4.651       4.645         840.30         203.14
trmm               holdout      1.368       1.384         137.32          73.20

Typed scalar numeric phase prerequisite (IR361)

After the frozen round04 candidate, the shared numeric Tail proof and runtime
now admit linear Bool/I64/F64 reads and replacements alongside all four
unsigned kinds. Leaf, tree and phase derivations use exact selected scalar
factories and the existing current-root/version proofs. Cranelift emits the
same scalar-word loads, arithmetic and stores; Bool stores additionally check
canonical 0/1 before writing Rust bool storage. The maintained scalar-only
entry and primitive instruction vocabulary remain unchanged.

PrivateNumericMatrix shares the existing pre-admitted publication and same-
account ownership machinery with PrivateUnsignedMatrix. A selected original
first-change replacement owns bounds/error/copy ordering. Frozen aliases force
a backing copy on the first change; later native stores use that private
allocation. No-op writes retain their identity. All fallible setup and
publication admission precede mutable access; finish allocates no new owner.
The old unsigned append, preparation and initialized-prefix APIs keep their
narrow contract. The private bank accounts for the actual owner variant type;
tight entry/resource caps keep the checked ordinary route. Work policy is 130.

The new tests observe native read/write execution for all seven scalar kinds
in single and nested loops, original/decoded artifacts and restored execution.
Selected cell sites have no per-iteration library callbacks. They cover exact
signed/float bits, rectangular column-major selection, aliases, no-op stores,
partial-write bounds failures, finite denials, atomic outputs and recovery.
core-tests.log: passes [23], filtered [173].
typed-fourth.log: passes [2], filtered [1437].
numeric-regressions.log: passes [293], filtered [1146].
matrix-polybench.log: passes [24, 30], filtered [0, 0].
program-regressions.log: passes [102], filtered [415].

The broadened numeric/phase run exposed an obsolete policy127 assertion.
Both version pins now name 130 and retain the established exact work bills.
The initial failure and the passing final logs are hashed in
typed-numeric-phases-qualification.json.

This is a compiler prerequisite, not a new frozen timing round. The latest
measured geometric-mean Mech/C kernel multipliers remain:
optimization: 301.197x C.
holdout: 724.274x C.

No timing samples or history entry were added for this prerequisite. The
fixed source ports, C binaries, data, split, tolerances and timing boundaries
remain unchanged. Full numeric tagged carriers and independent two-axis loop
selectors remain compiler gaps. The strict below 1.0x holdout goal remains open.

Independent numeric cell phase prerequisite (IR362)

Shared native numeric leaf/tree/phase proofs now retain two independent
selectors in explicit CellRead/CellWrite operations. Entry validates the actual
current scalar matrix view; generated code checks row bounds, then column
bounds, before computing a physical row-major address. Linear-index CSE and
window proofs remain confined to their original coordinate system. Scalar
liveness, current-version invalidation, mutation summaries, speculative capture
barriers and cold-owner reconciliation all include cell effects.

First-change transport adds a separately admitted column word only for a
proof containing a cell store. The selected original replacement receives
both axes and retains its bounds/resource/COW checks. The unsigned cell-private
bridge shares that implementation and preserves its narrower owner, so a
cell write can precede unsigned appends without granting Bool/I64/F64 spare-
cell authority. The existing linear transport and Huffman work bills stay
unchanged. Typed IR is 362 and native work policy is 131.

All seven scalar kinds execute actual native cell loops with original/decoded
artifacts, restored descriptors and Object emission. Tests cover distinct
columns, nested loops, zero and oversized axes, empty dimensions, inactive
reads, partial writes followed by failed reads, older immutable versions,
exact bits, aliases, no-op/COW behavior and finite recovery. Selected cell
sites have no per-iteration library dispatch. A mixed unsigned cell-write/
append test retains initialized prefixes, capacity changes and final shapes.
focused-second.log: passes [4], filtered [1439].
numeric-regressions.log: passes [297], filtered [1146].
matrix-polybench.log: passes [24, 30], filtered [0, 0].
program-regressions.log: passes [102], filtered [415].

numeric-cell-phases-qualification.json records the exact source/driver and
log hashes. Core source files did not change; the prior 23 Core ownership
checks remain recorded in typed-numeric-phases-qualification.json. No port,
input, C executable, tolerance, split or timing boundary changed.

This is a compiler prerequisite, not a new frozen timing round. The latest
measured geometric-mean Mech/C kernel multipliers remain:
optimization: 301.197x C.
holdout: 724.274x C.

No new timing samples or history entry were added. Full managed-array tagged
state remains the principal compiler gap before these regions can consume
the benchmark state machines. The strict below 1.0x holdout goal remains open.

Fifth frozen compiler round (IR363): guarded native numeric FSM phases

The compiler projects one independently guarded enum variant into the existing
numeric phase proof. Original private tag/payload homes remain disjoint. Entry
checks the actual current tag; every retained backedge proves the same tag,
array identity and current functional versions. The original selected EnumPack
factory separately certifies its constant ordinal and identity payload axes.
Entry retains the full logical enum/payload and array output envelopes.

Cranelift emits scalar tag tests, parallel payload copies, native arithmetic
and typed matrix loads/stores. Unsupported transitions finish the loan before
their original action. Exact current scalar/owner exports clear inactive
variants and publish the valid tag last. An ordinary different-tag constructor
can rearm native entry; a refused hot run does not retry every iteration.
Static native operation count selects a variant, with schema-order ties. No
benchmark name, measured runtime or holdout observation selects it.

The compiler source is 6887ed78c8741a311c7648d050b773897ec420f0, work policy132.
No benchmark port, input, C executable, tolerance or timing boundary changed.
302 numeric/phase, 24 matrix, all 30 PolyBench and 102 selected Program checks
pass. Five new tagged-state checks cover all seven scalar kinds, actual native
entry, ordinary cold start/reentry, refusal without repeated attempts, exact
source errors, older versions, aliases, finite admission and retry.

Both sets retain the same equal-weight geometric mean of median Mech/C kernel
runtime, lower is better; the strict below-1.0x goal applies to the holdout.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668

Round 05 aggregate results:
optimization: kernel 159.693062454x C; fresh 100.886669584x C; paired-original speedup 4.687483x; sum medians 0.137106001s.
holdout: kernel 814.668095179x C; fresh 488.787497303x C; paired-original speedup 2.441323x; sum medians 0.796756834s.
all: kernel 360.689399626x C; fresh 222.063375497x C; paired-original speedup 3.382848x; sum medians 0.933862835s.

The seven-sample-per-actor optimization probe replays round04 across all fifteen
programs: 1.760489x aggregate speedup. Seidel improves
559.783827x and Nussinov 8.701504x. Seidel source visits fall from
1,668,655 to 26; Nussinov from 942,817 to 123,753. The 2mm probe is
1.052877x slower, with an identical execution report; cause unproven.
This probe is neither a C nor a holdout comparison.

All 900 sequential process observations validate full finite warm-up outputs;
timed checked calls must succeed. They do not separately flatten every timed
output. Raw regressions and both original-compiler references remain in the
qualification receipt. The split and previous holdout results were exposed.
No holdout was profiled and no compiler change followed this round's freeze.
Only one flat tuple-payload state per Tail is selected; immutable managed
captures, nested payloads and multi-variant native loans remain compiler work.
CPU MINI F64 is qualified. Larger datasets, GPU/SIMD selection, full compression
application/AOT qualification and an external application are unqualified.
The strict goal remains open until the holdout metric is below 1.0x.

Holdout interpretation: geometric-mean Mech runtimes are 1.018895x faster
than round04, C runtimes are 1.146059x faster, and the unchanged original
Mech replay is 1.016618x faster. Thus the normalized holdout score worsens
1.124806x despite nearly unchanged paired-original compiler speedup. This
round does not demonstrate a holdout compiler improvement.

The five largest optimization medians cover 98.313% of summed
optimization runtime, with 16,566 canonical-execution stack samples.
No holdout was profiled. Profiles exclude parsing, JIT, input import, cache
flushing and serialization by stack; profiled timings never enter scores.
Canonical library calls: 45.547% of weighted profiled time.
Private frame transport: 17.345% of weighted profiled time.
Source visits and admission: 11.991% of weighted profiled time.
Other helper dispatch: 8.754% of weighted profiled time.
Selected matrix updates: 5.256% of weighted profiled time.
Comprehension collection bookkeeping: 4.248% of weighted profiled time.
Typed matrix scalar reads: 2.447% of weighted profiled time.
Other selected library dispatch: 2.436% of weighted profiled time.
Loop-local retirement: 1.330% of weighted profiled time.
Other execution and generated code: 0.646% of weighted profiled time.

Sixth frozen compiler round (IR364): current numeric cache headroom

Prepared numeric calls previously needed to prove optional-cache headroom
against the entire temporary-memory ceiling. A ceiling equal to the global
account cannot coexist with already-live arguments, so those calls repeatedly
rebuilt bindings and memory plans. The compiler/runtime now derives a shallow
current-call coexistence bound for maintained F64 transpose, sums, broadcasts,
matrix multiplication, square root and power. This selects existing caches.

The allocation-free preview reads same-account retained bytes/nodes, atomic
axis dimensions and packed lengths. It bounds legal result populations,
canonical draft/finalization workspaces, prior outputs and numeric staging.
Overflow, recursive dimension expressions and unknown geometry decline optional
metadata. The bound is never a kernel permit or a replacement for actual facts.
Current schema/shape/policy/account checks, maintained kernel admission, finite
work and atomic publication still run. Values and results are never cached.

Four new tests cover eleven numeric call forms, real cache retention, fresh
content/shapes, empty axes, foreign work accounts, finite call limits and
retry. Cold/warm byte/node pressure preserves an otherwise viable ordinary
call; a separate growing-payload threshold proves warm cache eviction.
306 numeric/phase, 24 matrix, all 30 PolyBench and 102 selected Program tests
pass. The preceding unsigned and scalar native paths retain their checks.

The source is fe60ce70f840357c9eaee32dc751f0e96213b17f, work policy133.
The compiler and driver were frozen before holdout timing. No Mech port, input,
C executable, tolerance or timing boundary changed. Both sets retain the same
equal-weight geometric mean of median Mech/C kernel runtime, lower is better.
The strict below-1.0x goal applies to the holdout and remains open.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804

Round 06 aggregate results:
optimization: kernel 142.999400197x C; fresh 87.220594167x C; paired-original speedup 5.566147x; sum medians 0.096089540s.
holdout: kernel 440.803690020x C; fresh 263.876744119x C; paired-original speedup 4.517397x; sum medians 0.344056625s.
all: kernel 251.067049366x C; fresh 151.708557468x C; paired-original speedup 5.014429x; sum medians 0.440146165s.

Relative to frozen round05, both aggregate compiler times improve.
optimization: Mech 0.866085512x runtime; original compiler 1.028432449x; C 0.967191804x; normalized Mech/C score 0.895464073x.
holdout: Mech 0.540833902x runtime; original compiler 1.000753100x; C 0.999538195x; normalized Mech/C score 0.541083777x.
The 1.849x holdout compiler gain accompanies C and original-compiler
aggregates within 0.1% of their round05 values. Individual regressions above
5% versus round05 are retained, even though neither original-baseline
comparison has a regression above 5%:
optimization cholesky: 1.058761883x previous Mech runtime.
optimization nussinov: 1.089676455x previous Mech runtime.
optimization seidel-2d: 1.114255547x previous Mech runtime.
holdout trisolv: 1.054233676x previous Mech runtime.

The seven-sample-per-actor optimization probe replays round05 across all fifteen
programs: 1.161430x aggregate speedup. Gramschmidt improves
3.039326x, Correlation 1.685092x, and Durbin 1.409842x.
Cholesky is 1.061786x slower; work increases from 49,468,185,460
to 50,349,314,868, with visits unchanged at 16,365. Cause unproven.
This probe is neither a C nor a holdout comparison. All cases are retained.

All 900 sequential observations validate full finite warm-up outputs; timed
checked calls must succeed. Timed outputs are not independently flattened.
Raw regressions and both original-compiler references remain in the receipts.
The split and previous holdout results were exposed. No holdout profiling or
compiler changes after this round's freeze were used. Numeric kernels and Core
code are unchanged; this round fills an optional metadata reuse gap.
Larger datasets, GPU/SIMD selection, full compression application/AOT and an
external application remain unqualified.

The five largest optimization medians cover 97.959% of summed
optimization runtime, with 15,881 canonical-execution stack samples.
Parsing, JIT, input import, cache flushing and serialization are excluded by
stack; profiled times do not enter scores. No holdout was profiled.
Canonical library calls: 24.139% of weighted profiled time.
Private frame transport: 23.742% of weighted profiled time.
Source visits and admission: 17.517% of weighted profiled time.
Other helper dispatch: 12.165% of weighted profiled time.
Selected matrix updates: 7.533% of weighted profiled time.
Comprehension collection bookkeeping: 5.378% of weighted profiled time.
Typed matrix scalar reads: 3.737% of weighted profiled time.
Other selected library dispatch: 2.869% of weighted profiled time.
Loop-local retirement: 1.869% of weighted profiled time.
Other execution and generated code: 1.051% of weighted profiled time.

Seventh frozen compiler round (IR365): immutable numeric phase captures

Flat numeric phases can now borrow direct immutable numeric matrices defined
in a strict lexical ancestor, including inputs to guarded FSM variants. The
proof uses the same Array descriptor and invocation-current view machinery as
native leaves. Captures have distinct immutable functional roots; writing or
appending to them declines the phase. Aliased carrier writes still use the
maintained first-change copy-on-write path. Current source facts, exact kinds,
same-account owners, finite setup and visit funding, selector checks, cold
exports and atomic publication remain required. No pointer or permit is cached.

Newly eligible capture phases attempt the existing complete-tree proof first.
A proved complete tree keeps its append bounds, owner handoff and projected
fuel optimizations. Guarded states or incomplete trees use the new phase when
the stronger complete proof cannot be established. Nine initial selection
assertions exposed this priority regression; production selection was fixed,
and the established tree tests were preserved.

The first optimization-only replay improved Deriche 9.888x but slowed
Nussinov 8.874x: source visits rose from 123,753 to 1,157,019. Newly eligible
cold-prefix reads made the Cell variant displace the recurrent Split variant.
The final static selector counts only numeric operations that can reach the
native latch without crossing a cold transition. Reverse CFG reachability
changes profitability only, never runtime guards or ownership. A separate
three-variant FSM test prevents a larger cold prefix from displacing repeated
native numeric work. The initial probe and its source identity are retained
in round-07-initial-training-probe.json; no holdout was timed for that trial.

Three new tests exercise original/decoded artifacts, all seven exact scalar
kinds, linear and row/column reads, actual native entry without per-iteration
library callbacks, mutation followed by reads from aliased immutable inputs,
fresh invocation contents, zero trips, exact source errors, finite limits,
atomic failure, old outputs, cleanup and retry. All 309 numeric/phase,
24 matrix, 30 PolyBench and 102 selected Program checks pass (465 total).

Compiler source 0c37ee2e170769c3c187bda09deab98326722e8b, work policy134.
The source and driver were frozen before holdout timing. The ports, inputs,
C executables, comparison tolerances and timing boundary did not change.
The strict below-1.0x C holdout target remains the objective. Both fixed sets
use equal-weight geometric means of median Mech/C kernel times, lower better.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181

Round 07 aggregate results:
optimization: kernel 123.864753583x C; fresh 75.091706562x C; paired-original speedup 6.314985x; sum medians 0.052412834s.
holdout: kernel 456.181240661x C; fresh 265.263690608x C; paired-original speedup 4.480578x; sum medians 0.345367749s.
all: kernel 237.707334686x C; fresh 141.135053111x C; paired-original speedup 5.319284x; sum medians 0.397780583s.

Optimization probe work and visit reports:
deriche captured-phase: work 21266550755, visits 9760, comparison work 411144.
deriche round-06: work 771162365599, visits 317984, comparison work 410632.
nussinov captured-phase: work 109827897956, visits 123753, comparison work 37614.
nussinov round-06: work 109827897956, visits 123753, comparison work 37614.

Inter-round runtime comparisons against frozen round06:
optimization: Mech 0.859107971x; original compiler 0.974687489x; C 0.991823106x; normalized Mech/C score 0.866190721x.
holdout: Mech 1.007603722x; original compiler 0.999391183x; C 0.973638105x; normalized Mech/C score 1.034885258x.

This round does not demonstrate a holdout compiler gain. Mech holdout
runtime is 1.007604x its round06 value, C is 0.973638x and the preserved
original compiler is 0.999391x. Paired-original holdout speedup falls from
4.517397x to 4.480578x, and the normalized Mech/C score worsens 1.034885x.
These observations do not establish a cause for individual timing shifts.
The below-1.0x C holdout target remains unmet.

Cases more than 5% slower than frozen round06:
optimization correlation: 1.052971255x previous Mech runtime.
optimization gemm: 1.066473400x previous Mech runtime.
optimization mvt: 1.093214258x previous Mech runtime.

The optimization-only seven-sample replay against round06 improves 1.162422x geometrically; this probe is neither a C nor a holdout comparison.
3mm: 0.919452x training-probe speedup.
deriche: 10.073435x training-probe speedup.
doitgen: 0.950091x training-probe speedup.

All 900 sequential observations compare complete finite warm-up outputs;
timed checked calls must succeed. Timed outputs are not independently
flattened. All cases, regressions, paired originals and raw observations are
retained. The split and earlier holdout results were exposed. No holdout was
profiled, and no compiler change followed this round's freeze.

The five largest optimization medians cover 96.234% of
summed optimization runtime, with 14,374 canonical-execution stack samples.
Parsing, JIT, input import, cache flush and serialization are excluded by
stack. Profiled runtimes do not enter scores.
Canonical library calls: 44.667% of weighted profiled time.
Other helper dispatch: 14.255% of weighted profiled time.
Source visits and admission: 12.261% of weighted profiled time.
Comprehension collection bookkeeping: 10.810% of weighted profiled time.
Private frame transport: 10.294% of weighted profiled time.
Typed matrix scalar reads: 2.360% of weighted profiled time.
Other execution and generated code: 1.642% of weighted profiled time.
Loop-local retirement: 1.505% of weighted profiled time.
Other selected library dispatch: 1.421% of weighted profiled time.
Selected matrix updates: 0.786% of weighted profiled time.

Core, Engine and runtime numeric kernels are unchanged in this round.
Phase captures of managed ancestor frames, nested tagged payloads and multiple
variants remain compiler gaps. Larger datasets, GPU/SIMD selection, full
compression application/AOT and an external application remain unqualified.

Eighth frozen compiler round (IR367): packed numeric output and geometry reuse

Maintained numeric leaves now use the existing Core typed matrix constructors
for plain supported outputs. This avoids a complete intermediate draft output
bank in transpose, sum, matrix binary operations, F64 matmul and floating unary
operations. All fourteen existing numeric kinds are covered. Original cell
order, arithmetic, reduction seeds, checked errors, conservative preflight,
exact schema checks, equality-sensitive writes and atomic publication remain.
Unsupported schemas keep their established draft path. Core is unchanged.

Prepared shallow F64 numeric calls retain at most 64 exact geometry bindings
and their existing turn-plan caches. Current schemas, complete geometry,
factory identity, policy and same ledger remain checked; current payload facts,
plans and finite admission still run. Every slot owns prepaid byte/node
metadata. No values, result, pointer or permit is cached. Nonnumeric families
retain one-slot behavior. Current pressure retires optional histories.

An established 32MiB MatrixAdd regression caught insufficient coexistence
headroom: cached execution succeeded while an ordinary call alongside history
failed. The corrected proof includes all retained metadata, ordinary binder
and invocation workspace, and a complete future plan-cache envelope. Existing
test ceilings and exact Huffman work equations were preserved.

Four new resident numeric tests and three original/decoded history tests cover
exact kind/bit behavior, seeds/overflow, finite caps, schema fallback, malformed
outputs, fresh payloads, actual binding and plan reuse, the 64-slot cap,
foreign identity/account/policy rejection, pressure eviction, atomic failure,
cleanup and retry. All 194 Engine, 309 native numeric/phase, 24 matrix,
30 PolyBench and 102 selected Program tests pass (659 total).

Compiler source cec3d5d8eb386be4d91d7daf251b0249bf10ec48, work policy136.
The candidate source and binary were frozen before holdout measurement.
The fixed ports, inputs, C binaries, tolerances and timing boundary are intact.
Both scores below are equal-weight geometric means of median Mech/C kernel
times; lower is better. The primary goal is strict holdout <1.0x C.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181
round-08-packed-numeric-output                111.804        435.641

Round 08 aggregate results:
optimization: kernel 111.804266149x C; fresh 71.308884088x C; paired-original speedup 6.765599x; sum medians 0.043942873s.
holdout: kernel 435.641482835x C; fresh 270.075994332x C; paired-original speedup 4.621156x; sum medians 0.329353334s.
all: kernel 220.695664417x C; fresh 138.776142671x C; paired-original speedup 5.591502x; sum medians 0.373296207s.

Inter-round runtime comparisons against frozen round07:
optimization: Mech 0.935879770x; original compiler 1.002660723x; C 1.036834470x; normalized Mech/C score 0.902631805x.
holdout: Mech 0.966908314x; original compiler 0.997245187x; C 1.012496403x; normalized Mech/C score 0.954974567x.

Cases more than 5% slower than frozen round07:
optimization bicg: 1.065462986x previous Mech runtime.
optimization durbin: 1.087921118x previous Mech runtime.
holdout trisolv: 1.131392053x previous Mech runtime.

Against original compilers, optimization Atax is 1.060582412x the paired
original runtime and BiCG is 1.051967387x the recorded original runtime.
No holdout case exceeds 5% slower than either original reference.

Publication-only optimization probe: 1.022957x speedup; no holdout measured. Final optimization-only seven-sample replay: 1.073831x speedup against round07. Probes are not C or holdout aggregate qualifications.
cholesky: 2.010183x final training-probe speedup.
durbin: 0.905951x final training-probe speedup.

All 900 sequential observations validate complete finite warm-up live-outs;
timed checked calls must succeed. Timed outputs are not independently
flattened. All cases, regressions, paired originals and raw observations are
retained. The split and earlier holdout results were exposed. No holdout was
profiled and no compiler change followed this round's freeze.

The five largest optimization medians cover 95.693% of
summed optimization runtime; 13,968 canonical execution stack samples
exclude parsing, JIT, input import, cache flush and serialization by stack.
Profiled times do not enter aggregate scores.
Canonical library calls: 34.553% of weighted profiled time.
Other helper dispatch: 16.471% of weighted profiled time.
Source visits and admission: 14.662% of weighted profiled time.
Comprehension collection bookkeeping: 12.507% of weighted profiled time.
Private frame transport: 12.143% of weighted profiled time.
Typed matrix scalar reads: 3.277% of weighted profiled time.
Other execution and generated code: 2.476% of weighted profiled time.
Loop-local retirement: 1.652% of weighted profiled time.
Other selected library dispatch: 1.513% of weighted profiled time.
Selected matrix updates: 0.747% of weighted profiled time.

This qualification covers CPU MINI F64 and selected regression suites.
Larger datasets, GPU/SIMD choice, full compression application/AOT and an
external application remain unqualified. Numeric geometry history covers
only already-supported shallow F64 templates. Comprehension/reduction fusion,
managed-frame captures and multi-variant phase loans remain compiler gaps.

Ninth frozen compiler round (IR368): native rectangular comprehensions

One enclosing horizontal-block matrix comprehension can now reuse its exact
inner scalar native DAG and fill one final bank. The inner U64 range must have
immutable constant endpoints. The inner row and singleton transpose must each
have one consumer; complete source CFG, capture/read and escape inventories
exclude intervening effects. The source compiler selects structure and exact
factories, never benchmark names or timing data. Existing Cranelift scalar
emission, checked branches/indices, current immutable owners and synchronous
output loans are reused. Original row-block permutation, shape construction
and atomic publication remain. A failed native attempt discards partial output
and follows the original checked source path.

The same loan checks exact source and output counts separately, with current
handles/storage, matrix owners, epoch, account, width, capacity, progress and
complete initialization. Both selected factories, range cardinality and the
intermediate shapes remain checked. Conservative finite bounds cover ordinary
draft/planning/finalization metadata, output, nodes, cloned bytes, physical
coexistence, comparison admission and fallback work. A finite test caught a
missing comparison bound for the elided transpose; production now checks it
before entry and retains ordinary rejection under insufficient allowance. No per-cell helper or generic dispatch remains.
The existing resident transpose kernel is now reachable for all seven native
scalar kinds, with exact matching input/output element types. Core is unchanged.

Five new original/decoded tests cover real native entry, Objects/restored
images, all seven kinds and F64 bit patterns, branches/phi words, shapes, empty
sources, fresh payloads, immutable aliases, partial bounds/arithmetic errors,
nine finite limit dimensions, changed loans/source counts, denied interleaving,
old outputs, cleanup and retry. Dynamic/ragged inner ranges remain ordinary.
The existing Gram coverage assertion now recognizes the exact same body/steps/
reads inside a completed rectangular parent as well as direct inner entry.

Resource checks reflect real optimized work/storage. An existing exact inner
bank retains three unsigned cells instead of ordinary capacity four, so a later
transpose may report 272 rather than 288 required clone bytes; tests preserve
origin, phase, dimension, limit and rejection, with nonincreasing demand.
Native execution may fit a compute cap that ordinary source-call preparation
exhausts; that success must match the complete full-budget output oracle.
Arithmetic/bounds errors still match exactly. Existing finite tests, Huffman
work equations and scalar entry remain maintained. All 194 Engine, 314 native
numeric/phase, 24 matrix, 30 PolyBench and 102 Program checks pass (664 total).
The 194 Engine results precede GPU test-expectation corrections only; Engine
source was unchanged by those corrections and the later GPU admission fix.

Compiler source 24135d2a5aef38c56ebc651d91d8a851b0442fde; work policy 137.
The source and driver were frozen before holdout timing. Both fixed sets
use the equal-weight geometric mean of median Mech/C kernel runtime; lower
is better. The primary goal remains strict holdout <1.0x C.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181
round-08-packed-numeric-output                111.804        435.641
round-09-rectangular-maps                     104.876        278.733

Round 09 aggregate results:
optimization: kernel 104.875920472x C; fresh 63.638018940x C; paired-original speedup 7.426036x; sum medians 0.031275085s.
holdout: kernel 278.733261294x C; fresh 160.100032543x C; paired-original speedup 7.475046x; sum medians 0.162169290s.
all: kernel 170.974873430x C; fresh 100.937846734x C; paired-original speedup 7.450501x; sum medians 0.193444375s.

Inter-round comparisons against frozen round 08:
optimization: Mech 0.903306976x; original compiler 0.991484953x; C 0.962981522x; normalized Mech/C score 0.938031473x.
holdout: Mech 0.617172571x; original compiler 0.998320137x; C 0.964599534x; normalized Mech/C score 0.639822589x.

Measured holdout Mech runtime improves 1.620292x, with the preserved original
at 0.998320x and C at 0.964600x previous runtime. The normalized holdout Mech/C
score improves by 1.562933x. The strict below 1.0x C goal remains unmet.

Gram optimization-probe counters (conservative budget charges and visits):
rectangular-map: work 58128242741, comparison work 4860, visits 11593.
round-08: work 39435454217, comparison work 514260, visits 46693.
Work counters are conservative budget charges; elapsed kernel times define the performance metric.

Cases more than 5% slower than frozen round 08:
None.

Cases more than 5% slower than an original compiler:
None.

The optimization-only seven-sample replay improves 1.098150x geometrically versus round 08; this is neither a C nor a holdout score.
doitgen: 0.939702x training-probe speedup.
gramschmidt: 3.995828x training-probe speedup.

All 900 sequential observations validate complete finite warm-up outputs;
timed checked calls must succeed. Timed outputs are not independently flattened.
All cases, regressions, paired originals and raw observations are retained.
No compiler source changed after freezing. The split and previous holdout
results were exposed; no holdout was profiled.

The five largest optimization medians cover 93.980% of
summed optimization runtime with 12,848 canonical-execution stack samples.
Parsing, JIT, input import, cache flush and serialization are excluded by stack.
Profile times do not enter scores; separate profiles do not establish causes
for individual unprofiled timing shifts.
Canonical library calls: 30.376% of weighted profiled time.
Source visits and admission: 17.530% of weighted profiled time.
Private frame transport: 16.940% of weighted profiled time.
Other helper dispatch: 16.497% of weighted profiled time.
Comprehension collection bookkeeping: 6.255% of weighted profiled time.
Typed matrix scalar reads: 4.211% of weighted profiled time.
Other execution and generated code: 3.165% of weighted profiled time.
Loop-local retirement: 2.021% of weighted profiled time.
Other selected library dispatch: 1.880% of weighted profiled time.
Selected matrix updates: 1.125% of weighted profiled time.

The qualification remains CPU MINI F64 plus selected regression suites.
Larger datasets, GPU/SIMD choice, full compression application/AOT and an
external application remain unqualified. General comprehension/reduction
fusion, managed frame captures and multi-variant phase loans remain compiler
gaps. This rectangle proof currently requires constant inner U64 endpoints,
one native scalar body and its sole consuming singleton transpose.

Tenth frozen compiler round (IR369): complete numeric FSM phases

Closed FSMs with exact compatible flat prefixes can retain one numeric loan
across multiple tags, including direct matrix payload states such as Done(table).
Every managed field appears in every tag. Exact tag flow guards each payload
read; existing source provenance and functional-version proofs establish every
array identity. All selected factories and current views remain checked. The
same Cranelift word/scalar emitter, finite phase schedule, original disjoint
owner bank and first-change bridge are reused. Current-tag entry imports only
active physical scalar homes; private scalar suffixes are zeroed when absent.
Dirty owners freeze back into entry homes before active payload synchronization,
inactive cleanup and checked tag publication. No benchmark port was rewritten.

A general source-dataflow profitability rule retains a constant-tag proof when
it already covers every recurring state. Broader phases may cover additional
numeric work or recurring scalar transition states. This follows an initial
optimization-only replay that slowed Seidel despite reducing callbacks; that
replay and all regressions remain in round-10-initial-training-probe.json.
No holdout evidence selected this refinement. An all-kind parity test also
caught an entry-bank pointer/scalar collision during development; the importer
now reads current physical homes directly, and that test remains.

Eight new native tests plus one Engine projection test cover all seven kinds
and F64 bits, original/decoded artifacts, Objects and restored images, actual
whole-phase execution, errors after private changes, nine finite dimensions,
aliases, distinct atomic old outputs, exact metadata forgery rejection, older
versions/foreign roots, sticky refusal, cleanup/retry and profitability. The
existing fixed-variant fallback fixtures keep incompatible prefix orders to
retain their original coverage. Both exact Huffman equations remain unchanged.
All195 Engine,322 native numeric/phase,24 matrix,30 PolyBench and102 selected
Program checks pass (673 total). This is selected coverage, not the full suite.
The Engine log predates GPU-only profitability refinement; Engine was unchanged.

Source b08f2ff9cf543fd63e10e0b2c6c05970b13dd9b9; work policy138; executable 60810bf8be6643d9a50d92e5606c194f4e834a5e661f29ab14c7019670539ea2.
Both fixed sets use equal-weight geometric means of median Mech/C kernel
runtime; lower is better. The primary target remains strict holdout <1.0x C.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181
round-08-packed-numeric-output                111.804        435.641
round-09-rectangular-maps                     104.876        278.733
round10-prefix-fsms                            76.829        269.227

optimization: kernel 76.828717920x C; fresh 46.734406865x C; paired-original speedup 9.723806x; sum medians 0.020363041s.

holdout: kernel 269.226652328x C; fresh 165.669210437x C; paired-original speedup 7.376814x; sum medians 0.163487544s.

all: kernel 143.820508024x C; fresh 87.991205729x C; paired-original speedup 8.469399x; sum medians 0.183850585s.

Inter-round comparisons against frozen round09:
optimization: Mech 0.769752494x; original 1.007930005x; C 1.050759449x; Mech/C score 0.732567758x.
holdout: Mech 1.012086162x; original 0.998786045x; C 1.047823736x; Mech/C score 0.965893525x.

Cases more than 5% slower than frozen round09:
optimization doitgen: 1.062588235x frozen round09 Mech runtime.
optimization durbin: 1.051079321x frozen round09 Mech runtime.

Cases more than 5% slower than original compiler:
None.

Final optimization-only seven-sample replay: 1.335857x geometric speedup versus round09. This is neither a C nor a holdout score.
nussinov: 73.739274x training replay speedup.

Key replay counters (conservative budget charges, not CPU time):
nussinov prefix-phase: work 4301681268; comparison work 37823; visits 8.
nussinov round-09: work 109827897956; comparison work 37614; visits 123753.
seidel-2d round-09: work 2364180484; comparison work 29662; visits 26.
seidel-2d prefix-phase: work 2364180484; comparison work 29662; visits 26.
doitgen round-09: work 10960690; comparison work 0; visits 4.
doitgen prefix-phase: work 10960690; comparison work 0; visits 4.

All 900 balanced observations validate complete finite untimed warm-ups; timed checked calls must succeed. Timed outputs are not independently flattened. All cases, raw samples and regressions remain. Five largest optimization cases cover 92.361% of summed medians and provide 9,702 canonical-execution samples; sampling follows aggregate timing and excludes parsing/JIT/import/serialization stacks.
Canonical library calls: 49.567% of weighted profiled execution.
Other helper dispatch: 12.916% of weighted profiled execution.
Comprehension collection bookkeeping: 10.795% of weighted profiled execution.
Source visits and admission: 8.641% of weighted profiled execution.
Private frame transport: 7.331% of weighted profiled execution.
Other execution and generated code: 3.559% of weighted profiled execution.
Selected matrix updates: 2.050% of weighted profiled execution.
Typed matrix scalar reads: 1.965% of weighted profiled execution.
Loop-local retirement: 1.758% of weighted profiled execution.
Other selected library dispatch: 1.417% of weighted profiled execution.

No compiler source changed after freeze. Earlier holdout exposure remains
recorded; this is not a blind experiment, and no holdout was profiled. Scope is
CPU MINI F64 plus selected tests. Nested tagged payloads, absent managed fields,
partial union phases, other result recipes, reduction/comprehension fusion,
managed captures, larger datasets and GPU/SIMD selection remain future work.

Round10 interpretation: optimization Mech runtime improves1.299x geometrically.
Holdout Mech runtime is1.012086x its prior value (about1.2% slower); the preserved
original is0.998786x. C measured1.047824x slower, lowering the normalized holdout
score. This round has not demonstrated a holdout runtime improvement. The
strict below1.0x C goal remains open.

Eleventh frozen compiler round (IR370): ordered F64 map reductions

An ordinary F64 scalar comprehension solely consumed by SumColumns and literal
scalar selector1 now folds in the existing native numeric-map DAG. It preserves
positive-zero initialization, source iteration order, branches, current scalar
captures and immutable numeric matrix/range views. No reassociation or FMA is
introduced. The actual intermediate bank is empty; complete original producer,
sum and selector shape/type/output/resource bounds remain admitted, including
the singleton result of an empty producer. Failed checked attempts discard the
private fold and replay the original CFG. Original ports remain unchanged.

The complete primitive result parks only across the exact original finish path
and epoch under the same account. Normal continuation visits perform admission
and retirement before a native post-action join stores the original scalar
local and continues the original source control. Unsupported escapes, integer
folds, different axes/selectors, rectangles and conflicting scalar groups keep
the maintained route. Shared Cranelift words/scalars and resident factory/type
authority remain in use. Whole-FSM reduction composition is still a gap.

Six new focused tests plus actual Cholesky/Gram-Schmidt selection assertions
cover original/decoded/Object/restored images, F64 bits/order/empty inputs,
nested conditional/range maps, partial source failures, nine finite dimensions,
distinct atomic prior outputs, immutable aliases, current loans/pending epochs,
interleaving refusal, metadata forgery, cleanup/retry and ordinary unsupported
cases. Final 195 Engine, 328 native, 24 matrix, 30 PolyBench and 102 selected Program
checks pass (679 total). Both exact Huffman work bills remain unchanged.

Source c72146bbcb1d86c71dd72e27e6efca759a5c2b6d; policy139; executable 1ff3b8915a991596ee86296390518900aa416b472b5347553b759d4f345dbb0b.
Both fixed 15-program sets use equal-weight geometric means of median Mech/C kernel
runtime; lower is better. The strict holdout target remains <1.0x C.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181
round-08-packed-numeric-output                111.804        435.641
round-09-rectangular-maps                     104.876        278.733
round10-prefix-fsms                            76.829        269.227
round11-map-reductions                         69.994        241.699

optimization: kernel 69.993848087x C; fresh 42.421142936x C; paired-original speedup 11.317598x; sum medians 0.013919080s.

holdout: kernel 241.699065666x C; fresh 146.970640262x C; paired-original speedup 8.376101x; sum medians 0.148482875s.

all: kernel 130.067089170x C; fresh 78.959879293x C; paired-original speedup 9.736393x; sum medians 0.162401955s.

Inter-round comparisons against frozen round10:
optimization: Mech 0.865174325x; original 1.006981766x; C 0.949658234x; Mech/C score 0.911037565x.
holdout: Mech 0.887305625x; original 1.007502816x; C 0.988362625x; Mech/C score 0.897753115x.
Both actual Mech geomean runtimes improved this round. The reference movements
above remain visible; separate frozen measurements do not establish the cause
of individual shifts. No case or observation was removed.

Cases more than 5% slower than frozen round10:
optimization mvt: 1.056524036x frozen round10 Mech runtime.

Cases more than 5% slower than original compiler:
None.

Optimization-only seven-sample replay: 1.156784x geometric speedup versus round10. This is neither a C nor a holdout score.
cholesky: 2.285539x training replay speedup.
doitgen: 0.926929x training replay speedup.
durbin: 3.115591x training replay speedup.
gramschmidt: 1.395128x training replay speedup.
Doitgen replay is more than 5% slower with identical execution reports. Its
timing cause is not established. This unchanged candidate was then frozen; no
holdout timing/profile was used for selection. All replay samples remain.

Key counters (conservative budget charges, not CPU time):
cholesky round-10: work 43979337780; comparison work 2422; visits 16365.
cholesky map-reduction: work 3765218215204; comparison work 1618691702; visits 16365.
gramschmidt map-reduction: work 2266403799207; comparison work 960181500; visits 11593.
gramschmidt round-10: work 58128242741; comparison work 4860; visits 11593.
durbin map-reduction: work 187956553291; comparison work 80962831; visits 1422.
durbin round-10: work 2202020662; comparison work 12559; visits 1422.
doitgen round-10: work 10960690; comparison work 0; visits 4.
doitgen map-reduction: work 10960690; comparison work 0; visits 4.
Conservative reduction admission charges can grow while elapsed time falls.
Finite caps that cannot admit the complete bound retain ordinary execution.

All 900 balanced observations validate complete finite untimed warm-ups; timed checked calls must succeed. Timed outputs are not independently flattened. Five largest optimization cases cover 88.774% of summed medians and provide 7,933 canonical-execution samples. Sampling follows aggregate timing; parsing/JIT/import/serialization stacks are excluded.
Canonical library calls: 34.318% of weighted profiled execution.
Other helper dispatch: 22.724% of weighted profiled execution.
Source visits and admission: 12.742% of weighted profiled execution.
Comprehension collection bookkeeping: 11.711% of weighted profiled execution.
Private frame transport: 8.625% of weighted profiled execution.
Other execution and generated code: 3.630% of weighted profiled execution.
Selected matrix updates: 2.639% of weighted profiled execution.
Typed matrix scalar reads: 1.401% of weighted profiled execution.
Other selected library dispatch: 1.110% of weighted profiled execution.
Loop-local retirement: 1.101% of weighted profiled execution.

No compiler source changed after freeze. All previous receipts remain. The
exposed fixed split is not a blind experiment; no holdout was profiled. Scope
remains CPU MINI F64 plus selected tests. Larger datasets, GPU/SIMD selection,
full compression application/AOT and a fresh external application are unqualified.

Twelfth frozen compiler round (IR371): direct transposed scalar maps

A scalar row comprehension with one transpose consumer can publish its existing
typed bank directly in the selected column shape. All seven native scalar kinds
keep their exact bits, including F64 NaN payloads, infinities and signed zero.
Empty source geometry remains 0x1 after transpose. The same Cranelift map DAG,
word producer or ordinary checked producer supplies cells. No benchmark port
changed. Exact selected factories, producer/target shapes and both original
logical constructor/call limits remain authoritative.

A separately funded finish helper admits complete original work, comparison,
cloned, output, temporary/node and physical coexistence bounds plus original
finish/call replay credit. It then uses the normal Core typed constructor once.
The tracked result stays private across exact continuation admission/retirement
and publishes after the original transpose visit, so repeated loop generations
cannot retire a prematurely installed owner. The shared post-action join keeps
normal downstream managed transport and source control. Unsupported escaping
rows, shaped blocks, other scalar families or scalar groups retain their route.

All 195 Engine, 334 native numeric/phase, 24 matrix, 30 PolyBench, 102 selected
Program and one explicit helper-protocol checks pass (686 selected checks).
Six new tests cover original/decoded/Object/restored images, all kinds/bits,
empty/2D inputs, real activation checkpoints, word/native producers, nine finite dimensions, distinct atomic
prior outputs, aliases, nested iterations, inactive/partial bad reads, malformed
helpers/loans, table-proof forgery, ordinary routes, cleanup and retry. Actual
Deriche, Gram-Schmidt and Durbin source checks require the new publication path.
Both exact Huffman work equations remain unchanged under policy140.

Source b7a3610472f83f4ccda82d193235f71bf167bb97; policy140; executable 2d51ba37ab564b1863e60f808692ffd9257bcdf80bc0bd49da3481fecbf60846.
Both fixed 15-program sets use equal-weight geometric means of median Mech/C
kernel runtime; lower is better. The holdout target is strict <1.0x C.

frozen compiler                         optimization x      holdout x
baseline                                      783.258       2020.085
round-01-native-matrix-frames                 441.860       1610.274
round-02-native-scalar-matrix-updates         346.025       1529.299
round-03-native-maps                          318.844       1468.557
round-04-conditional-maps                     301.197        724.274
round-05-guarded-tagged-numeric               159.693        814.668
round-06-numeric-call-cache                   142.999        440.804
round-07-captured-numeric-phases              123.865        456.181
round-08-packed-numeric-output                111.804        435.641
round-09-rectangular-maps                     104.876        278.733
round10-prefix-fsms                            76.829        269.227
round11-map-reductions                         69.994        241.699
round12-map-transposes                         64.359        214.241

optimization: kernel 64.359122131x C; fresh 38.917585879x C; paired-original speedup 12.089934x; sum medians 0.011976041s.

holdout: kernel 214.241267758x C; fresh 123.277372626x C; paired-original speedup 9.655118x; sum medians 0.144621125s.

all: kernel 117.423932472x C; fresh 69.265126406x C; paired-original speedup 10.804154x; sum medians 0.156597166s.

Inter-round comparisons against frozen round11:
optimization: Mech 0.921729047x; original 0.984629664x; C 1.002427640x; Mech/C score 0.919496840x.
holdout: Mech 0.866572563x; original 0.998896867x; C 0.977635080x; Mech/C score 0.886396756x.
These are separate frozen measurements; reference movements remain visible.
The cause of individual timing shifts is not established. Every observation,
case, prior receipt and measured regression remains.

Cases more than 5% slower than frozen round11:
None.

Cases more than 5% slower than original compiler:
None.

Optimization-only seven-sample replay: 1.071889x geometric speedup versus round11. This is neither a C nor a holdout score.
correlation: 1.201191x training replay speedup.
deriche: 1.470511x training replay speedup.
durbin: 1.565900x training replay speedup.
The candidate was selected and frozen before any holdout timing. Complete
conservative bounds can increase work units while elapsed time falls; finite
caps that cannot admit them retain ordinary execution.

Key counters (budget charges, not CPU time):
deriche map-transpose: work 1167942385891; comparison work 64233632264; visits 9760.
deriche round-11: work 21266945251; comparison work 411144; visits 9760.
gramschmidt map-transpose: work 2333576279265; comparison work 4723210230; visits 11593.
gramschmidt round-11: work 2266403799207; comparison work 960181500; visits 11593.
durbin map-transpose: work 275253174808; comparison work 4970900767; visits 1422.
durbin round-11: work 187956553291; comparison work 80962831; visits 1422.
cholesky round-11: work 3765218215204; comparison work 1618691702; visits 16365.
cholesky map-transpose: work 3765218215204; comparison work 1618691702; visits 16365.

All 900 balanced observations validate complete finite untimed warm-ups; timed checked calls must succeed. Timed outputs are not independently flattened. The five largest optimization cases cover 87.721% of summed medians and provide 7,380 canonical-execution samples. Sampling follows aggregate timing; parsing/JIT/import/serialization stacks are excluded.
Other helper dispatch: 27.158% of weighted profiled execution.
Canonical library calls: 23.314% of weighted profiled execution.
Source visits and admission: 13.553% of weighted profiled execution.
Comprehension collection bookkeeping: 11.266% of weighted profiled execution.
Private frame transport: 10.057% of weighted profiled execution.
Other execution and generated code: 7.440% of weighted profiled execution.
Selected matrix updates: 3.093% of weighted profiled execution.
Other selected library dispatch: 1.611% of weighted profiled execution.
Typed matrix scalar reads: 1.375% of weighted profiled execution.
Loop-local retirement: 1.134% of weighted profiled execution.

No compiler source changed after freeze. The exposed fixed split is not a
blind experiment, and no holdout was profiled. Scope remains CPU MINI F64 plus
selected tests; broader datasets, GPU/SIMD selection, full compression/AOT and
a fresh external application remain unqualified.

Round 13: direct rectangular native publication
Source 5cbf06b8000f37a10cabf1aa74a752cf29216ff3; IR372/work141; executable b027872f169df308ad1093ad188bbede894535ae50886f6819164bb812985447.
The same native scalar DAG now stores rectangular cells directly at their final
row-major locations while preserving the exact column-then-row source order.
A same-footprint three-state row geometry marks only complete native banks.
The original source work, temporary/node checks and physical reservations remain;
the cycle-mark allocation and swap permutation are skipped. Ordinary producers
and failed native attempts retain the original builder/order/CFG.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 60.500560x C; 12.371485x faster than recorded start; 12.107126x faster than paired original; summed medians 0.011805626s; raw runtime multiplier vs round12 0.999667x.
holdout: 196.253849x C; 10.131471x faster than recorded start; 9.942332x faster than paired original; summed medians 0.144079581s; raw runtime multiplier vs round12 0.956371x.

Optimization-only seven-sample round12 replay: 1.011703x speedup; 0 >5% regressions.
Final checks: 338 native/phase/ordinary block, 24 matrix, 30 PolyBench, 102
selected Program and one protocol (495 newly run); unchanged Engine/Core/stdlib/
syntax trees retain the passing195-check Engine log. Both exact Huffman bills
remain unchanged. Initial policy-pin failures are retained by log hash.
All900 balanced finite observations, 29482 live-outs per artifact, both sets,
all regressions, fourteen-entry history and five raw compressed optimization
profiles remain. Profile samples 7258; coverage of
training summed medians 87.562430%.
No source changed after freeze and no benchmark port was rewritten. The split
and past holdout results were exposed; no holdout was profiled. The strict
holdout goal remains open unless the recorded score is below1.0x C.

Round 14: ordered source reductions inside native phases
Source e065bf86a67ad7c74c40d3264ef5940de74fb9db; IR373/work142; executable 4b754b8924ed3eb2ec97e7f79edf71a46423c88861b417f58897de5de897cffa.
The original range comprehension, ordered F64 sum and scalar extraction now
share the parent numeric phase loan and checked scalar DAG. Both builder/range
setup orders retain exact cold replay before either source operation. Current
array versions, source shapes, elided constructor/clone/output/temporary/node
bounds, physical coexistence probes and unspent replay credit remain checked.
No port changed and no arithmetic was reassociated. Both Cholesky source folds
select the composed path without per-fold loans or per-cell helpers.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 48.362912x C; 16.232673x faster than recorded start; 15.745911x faster than paired original; summed medians 0.008501627s; raw runtime multiplier vs round13 0.762135x.
holdout: 181.021054x C; 11.348599x faster than recorded start; 11.259767x faster than paired original; summed medians 0.143865794s; raw runtime multiplier vs round13 0.892751x.

Optimization-only seven-sample round13 replay: 1.326803x speedup; 1 >5% regressions. The original Doitgen flag remains; its fixed six-pair/31-sample follow-up is 0.995699x speedup with unchanged work reports.
Final selected checks:342 native/phase/ordinary block,24 matrix,30 PolyBench,
102 Program,195 freshly rerun resident Engine, one protocol, one additional
inclusive-range endpoint test (695 total). Both exact Huffman bills remain.
All900 balanced finite observations,29482 live-outs per artifact, both sets,
all regressions and fifteen-entry history remain. Five optimization-only raw
profiles retain 5296 canonical execution samples and cover
84.926885% of training summed medians.
No compiler source changed after freeze and no benchmark port was rewritten.
The split and past holdout results were exposed; no holdout was profiled.
The strict holdout goal remains open unless its score is below1.0x C.

Round 15: selected range cursors for matrix block comprehensions
Source 6b84591c2f3e435552af969fe435f3c1937a3d35; IR374/work143; executable 2e71eb27bb4c808a4a20c09ef759fc37573434ee407f0be5ddef504eac1380fa.
The existing exact unit-U64 selected range source now feeds matrix block
comprehensions. Sole use, setup order, current endpoints, range and physical
representation errors, finite admission and block row/shape checks remain.
Existing direct native rectangle execution accepts the new cursor sources.
No benchmark port or scalar arithmetic order changed.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 47.201219x C; 16.778477x faster than recorded start; 16.446279x faster than paired original; summed medians 0.007184208s; raw runtime multiplier vs round14 0.967470x.
holdout: 133.611342x C; 15.090981x faster than recorded start; 14.872726x faster than paired original; summed medians 0.119366334s; raw runtime multiplier vs round14 0.752012x.

Optimization-only seven-sample round14 replay: 1.043806x speedup; 1 >5% regressions retained.
Final selected checks: 712; both exact Huffman bills remain.
All 900 balanced finite observations, 29482 live-outs per artifact, both sets,
all regressions and sixteen-entry history remain. Five optimization-only raw
profiles retain 4718 canonical execution samples and cover
81.957552% of training summed medians.
No compiler source changed after freeze and no benchmark port was rewritten.
The split and prior holdout results were exposed; no holdout was profiled.
The strict holdout goal remains open unless its score is below 1.0x C.

Round 16: call-scoped typed numeric operand views
Source 52a80673f627d6bfff778382203dc8db2b98bd59; IR375/work144; executable de22a56242d8b162a907314f1a4a93a6450997f3a3cf2ac0d2502908f55dcf20.
The shared resident/JIT numeric operand loans exact immutable Core primitive
slices once per admitted call. Broadcasting, transpose, unary operations and
ordered sums keep original arithmetic, shape checks, complete resource prices,
publication and canonical fallback. No port or arithmetic order changed.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 44.385131x C; 17.465133x faster than recorded start; 17.425091x faster than paired original; summed medians 0.006872043s; raw runtime multiplier vs round15 0.960684x.
holdout: 129.648057x C; 15.724632x faster than recorded start; 15.691207x faster than paired original; summed medians 0.117851793s; raw runtime multiplier vs round15 0.959703x.

Optimization-only seven-sample round15 replay: 1.063600x speedup; 0 >5% regressions retained.
Final selected checks: 715; both exact Huffman bills remain.
All 900 balanced finite observations, 29482 live-outs per artifact, both sets,
all regressions and seventeen-entry history remain. Five optimization-only raw
profiles retain 4636 canonical execution samples and cover
82.327541% of training summed medians.
No compiler source changed after freeze and no benchmark port was rewritten.
The split and prior holdout results were exposed; no holdout was profiled.
The strict holdout goal remains open unless its score is below 1.0x C.
