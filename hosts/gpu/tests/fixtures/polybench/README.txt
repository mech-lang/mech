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

Round 17: nested ordered reductions inside scalar maps and rectangles
Source 8974a661a4e0987cfd6ae24bb572dc4284bd478e; IR376/work145; executable 81edc385496470335e202e43feeee56447c840a1e1ef19cf154fb21323231ed4.
Exact child range/map/sum/extraction summaries share one parent native loan.
The checked phase fold/map emitter uses current parent SSA words and immutable
views in a private stack frame. Complete source work/constructor bounds,
physical coexistence and unspent whole-parent replay credit remain admitted.
Independent images rebuild nonserialized funds from source/calls/visit prices.
No port, source arithmetic order, reassociation or FMA changed.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 41.513488x C; 19.308929x faster than recorded start; 18.862150x faster than paired original; summed medians 0.005633250s; raw runtime multiplier vs round16 0.904511x.
holdout: 91.534885x C; 22.400509x faster than recorded start; 22.144092x faster than paired original; summed medians 0.024106209s; raw runtime multiplier vs round16 0.701977x.

Optimization-only seven-sample round16 replay: 1.056739x speedup; 1 >5% regressions retained.
Final selected checks: 722; both exact Huffman bills remain.
All 900 balanced finite observations, 29482 live-outs per artifact, both sets,
all regressions and eighteen-entry history remain. Five optimization-only raw
profiles retain 3710 canonical execution samples and cover
78.899836% of training summed medians.
No compiler source changed after freeze and no benchmark port was rewritten.
The split and prior holdout results were exposed; no holdout was profiled.
The strict holdout goal remains open unless its score is below 1.0x C.

Round 18: direct typed F64 broadcast arithmetic
Source ec96f1d0ec97a8554c07b30c7b7bda94d98c3a16; IR377/work146; executable 33f88161e373bbb6fbfef8be97abee305fd7ce4297c80c42b1d88eea09d12b6a.
Current exact Core F64 banks feed maintained elementwise add/subtract/multiply/divide directly, with one operation selection and no per-cell generic drafts. Scalar, row, column and exact broadcasting retain original geometry, complete finite admission and strict atomic publication. Other schemas and dense transport retain their paths. No port or arithmetic order changed.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 39.771610x C; 19.907344x faster than recorded start; 19.569783x faster than paired original; summed medians 0.005521582s; raw runtime multiplier vs round17 0.969940x.
holdout: 89.689184x C; 23.293593x faster than recorded start; 23.170984x faster than paired original; summed medians 0.023861167s; raw runtime multiplier vs round17 0.961660x.

Optimization-only seven-sample round17 replay: 1.061343x speedup; 0 >5% regressions retained.
Final selected checks: 725; both exact Huffman bills remain.
All 900 balanced finite observations, 29482 live-outs per artifact, both sets, all regressions and nineteen-entry history remain. Five optimization-only raw profiles retain 3606 canonical execution samples and cover 79.651828% of training summed medians.
No compiler source changed after freeze. The strict holdout goal remains open unless its score is below 1.0x C.
Final >5% raw runtime regressions vs round17: optimization durbin (+6.77%), optimization gramschmidt (+7.42%), holdout trisolv (+5.35%).

Round 19: inline current leaf shape evaluation
Source 3294d36340510bdbd0c7ec071ef78ce96468389f; IR378/work147; executable ca2afeebc733b113f4a3f1a19e934ce6faaa26aea8139969f8b0a02f2f8fc7ed.
Constant and Parameter dimensions share a thin inlinable evaluator across current resolution, shape validation and recursive children. The complete original compound evaluator, checked errors and order remain. No evaluated dimension, Value or permission is cached. Source prices, ABI42, arithmetic order and all benchmark ports remain.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 39.050848x C; 19.447819x faster than recorded start; 19.809039x faster than paired original; summed medians 0.005530333s; raw runtime multiplier vs round18 1.023629x.
holdout: 84.281767x C; 23.539471x faster than recorded start; 23.543110x faster than paired original; summed medians 0.023708749s; raw runtime multiplier vs round18 0.989555x.

Optimization-only seven-sample round18 replay: 1.022580x speedup; 1 >5% flags retained.
Final selected checks: 750; both exact Huffman bills remain.
All 900 balanced finite observations, 29482 live-outs per artifact, both sets, all regressions and twenty-entry history remain. Five optimization-only raw profiles retain 3487 canonical execution samples and cover 78.917020% of training summed medians.
The earlier callback-composition probe was effectively flat and its code was fully reverted; its complete training observations remain in round-19-callback-experiment.json. No holdout was measured for that experiment.
No compiler source changed after freeze. The strict holdout goal remains open unless its score is below 1.0x C.
Final >5% raw runtime regressions vs round18: optimization atax (+8.62%), optimization bicg (+5.12%), optimization covariance (+8.94%).
Fixed all15 follow-up: 1.010864x; 0 >5% regressions. All initial flags remain.

Round 20: complete vector maps inside checked native phases
Source 9f6ed6c5e789f363a5194b604f95db4ad4fe6ad8; IR379/work148/ABI42; executable a108da7bcf12caf550cc0ff5bcd402e5fba7c2bbae00020141b834d530c0586c.
A separate checked MatrixMap summary composes full fixed scalar vector comprehensions and optional transpose with existing numeric phases. Destination identity/current versions and exclusive private ownership are proved. Reused map DAG emission writes admitted stack scratch before a typed commit. Existing reductions, cold source paths and complete finite construction prices remain.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 33.211571x C; 23.561205x faster than recorded start; 23.103262x faster than paired original; summed medians 0.005186502s; raw Mech speedup vs round19 1.211509x; C runtime multiplier 0.970542x.
holdout: 86.621162x C; 23.874005x faster than recorded start; 23.505592x faster than paired original; summed medians 0.023762666s; raw Mech speedup vs round19 1.014212x; C runtime multiplier 0.959359x.

Optimization-only seven-sample round19 replay: 1.209682x speedup; no >5% regressions. Source and binary then frozen before both final sets.
Correctness: 530 unique selected checks; runtime-only and codegen feature builds. The broad numeric run had one obsolete host-transpose route assertion; its corrected test and all eleven map checks passed with complete C comparisons. Original failure retained in qualification evidence.
All 900 balanced finite observations, 29482 warm-up live-outs per artifact and twenty-one history entries remain. Five optimization-only profiles retain 3548 canonical execution samples and cover 81.544208% of training summed medians.
Final >5% raw runtime regressions vs round19: none.
Full vector maps require an already private destination with an existing scalar write producer. Rectangles/nested maps, dynamic counts and first-map bank construction remain compiler gaps. No ports or compiler source changed after freeze. The strict holdout goal remains open unless below 1.0x C.

Round 21: phase-owned rectangular and nested map outputs
Source 3789f44d051a5749daf5c5cfb2e8cd3b81bdbf88; IR380/work149/ABI42; executable 7908d2e7daf98566039cf417674f021c2c88658f72366943741e917209f5d10b.
Complete fixed rectangular/nested comprehensions now compose into native phases with disjoint admitted Core private banks, current-version proofs, scratch-before-commit and exact original cold/terminal publication. Existing scalar first-change roots and ordered folds remain.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 35.081028x C; 21.160349x faster than recorded start; 23.140359x faster than paired original; summed medians 0.005224209s; raw Mech speedup vs round20 0.898101x; C runtime multiplier 1.054124x; reference-adjusted Mech speedup 1.001606x.
holdout: 116.813414x C; 16.941709x faster than recorded start; 17.020435x faster than paired original; summed medians 0.094824043s; raw Mech speedup vs round20 0.709630x; C runtime multiplier 1.044959x; reference-adjusted Mech speedup 0.724102x.

Training-only seven-sample replay vs frozen20: 1.083661x speedup; 1 regression over 5%. Source and binary then frozen before both final sets.
Correctness: 542 selected checks; runtime-only and codegen builds pass. Exact historical Huffman equations add only the new admitted 8-byte optional field at 8 work units/byte (64 units). Development failures and their corrections remain documented in qualification evidence.
All 900 balanced observations, 29482 finite warm-up live-outs per artifact and 22 history entries remain. Five training-only profiles retain 2774 canonical samples and cover 77.286820% of training summed medians.
Final raw runtime regressions over 5% vs round20: optimization 2mm (+31.20%), optimization 3mm (+43.15%), optimization atax (+46.73%), optimization bicg (+42.34%), optimization cholesky (+36.15%), optimization correlation (+21.00%), optimization covariance (+36.81%), optimization doitgen (+5.94%), optimization durbin (+5.93%), optimization gemm (+14.87%), holdout floyd-warshall (+34.00%), holdout gesummv (+7.33%), holdout lu (+896.24%), holdout ludcmp (+873.81%), holdout syrk (+5.78%), holdout trisolv (+43.72%), holdout trmm (+42.38%).
No ports or compiler source changed after freeze. The strict holdout goal remains open unless below 1.0x C.

All thirty kernels, descending current Mech/C kernel ratio (largest first).
Each C ratio uses its round's C measurement; raw Mech speedups are separate.
kernel             set            start xC          now xC
lu                 holdout            20,830.73        3,257.48
ludcmp             holdout            20,723.72        3,201.45
jacobi-2d          holdout             9,612.12          457.60
jacobi-1d          holdout             7,168.14          400.79
adi                holdout            21,631.38          194.12
gemver             holdout               260.73          163.16
gesummv            holdout               112.38          113.39
mvt                optimization          138.96          107.16
bicg               optimization           87.94           97.73
deriche            optimization       43,560.74           95.94
atax               optimization           79.13           93.25
2mm                optimization           77.03           82.96
correlation        optimization          327.94           80.52
gramschmidt        optimization       27,896.34           69.70
trmm               holdout               219.61           50.92
symm               holdout               218.10           45.36
covariance         optimization           50.46           42.01
syr2k              holdout               224.54           38.29
syrk               holdout               245.10           37.61
3mm                optimization           38.12           36.49
trisolv            holdout             3,606.16           32.77
fdtd-2d            holdout            10,500.55           31.87
heat-3d            holdout             4,178.38           27.43
gemm               optimization           36.05           26.62
durbin             optimization        3,899.39           22.88
doitgen            optimization           22.47           19.84
floyd-warshall     holdout             4,841.31           15.72
cholesky           optimization       23,662.23           10.80
nussinov           optimization      119,638.18            5.82
seidel-2d          optimization       50,168.86            2.95

The formal round fails the holdout generalization check. LU/LUDCMP regress about 10x in raw runtime; no compiler change was made after freeze. The seven-sample training probe and final measurements are separate receipts.

Round22: reached summary charges and matrix geometry
Source 6a6b5f47078edd5f67b5857fed70655f9f21326c; IR382/work150/ABI42; executable 2f1fe3109ec55521ab7bffb48b8f69f5dc6391e6afdcdbaac4a7c3ace77a1ee9.
Complete map/fold bounds now debit only at reached source operations, with separate attempt counters and unchanged full replay/physical admission. Exact source/error/cold/first-change/terminal boundaries reconcile all counts. Shared guarded vector/matrix addressing and fixed geometry/count constants preserve ordered scalar arithmetic.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 31.633182x C; 24.636747x faster than recorded start; 24.129555x faster than paired original; summed medians 0.004915169s; raw Mech speedup vs21 1.164288x; C runtime multiplier 0.952509x; reference-adjusted Mech speedup 1.042748x.
holdout: 68.476876x C; 30.717878x faster than recorded start; 30.656826x faster than paired original; summed medians 0.017118792s; raw Mech speedup vs21 1.813151x; C runtime multiplier 0.940838x; reference-adjusted Mech speedup 1.801178x.

Training-only seven-sample replay vs21: 1.006015x; one>5% regression. This is near neutral, not a broad gain claim. Source and binary then frozen before both complete sets.
Correctness:543 selected checks and both feature builds pass; both exact historical Huffman work equations remain unchanged. Development failures and their corrections remain in qualification evidence.
All900 balanced observations,29482 finite warm-up live-outs per artifact and23 history entries remain. Five training-only profiles retain 3092 canonical samples and cover 80.546569% of the training sum.
Raw runtime regressions over5% vs21: none.
No ports, limits or compiler source changed after freeze. The strict holdout goal remains open unless below1.0x C.

All thirty kernels, descending current Mech/C kernel ratio. Each C ratio uses its round's C measurement; raw Mech speedups are separate.
kernel             set            start xC          now xC
jacobi-2d          holdout             9,612.12          463.90
jacobi-1d          holdout             7,168.14          428.92
adi                holdout            21,631.38          193.04
gemver             holdout               260.73          181.78
gesummv            holdout               112.38          123.70
mvt                optimization          138.96          113.26
deriche            optimization       43,560.74           97.90
ludcmp             holdout            20,723.72           88.74
bicg               optimization           87.94           77.84
atax               optimization           79.13           76.48
correlation        optimization          327.94           74.59
gramschmidt        optimization       27,896.34           71.24
2mm                optimization           77.03           61.90
lu                 holdout            20,830.73           61.08
symm               holdout               218.10           45.32
syr2k              holdout               224.54           45.16
syrk               holdout               245.10           41.51
trmm               holdout               219.61           38.03
3mm                optimization           38.12           35.32
covariance         optimization           50.46           33.34
trisolv            holdout             3,606.16           26.71
fdtd-2d            holdout            10,500.55           26.58
heat-3d            holdout             4,178.38           24.31
gemm               optimization           36.05           22.90
durbin             optimization        3,899.39           22.25
doitgen            optimization           22.47           18.21
floyd-warshall     holdout             4,841.31           13.07
cholesky           optimization       23,662.23            8.44
nussinov           optimization      119,638.18            5.25
seidel-2d          optimization       50,168.86            3.02

Performance acceptance rule (adopted after round22)

performance-history.json retains every measured run, including failed runs.
performance-acceptance.json separately records the accepted comparator under
the new rule. Earlier results are classified retrospectively; this does not
change their original selection criteria or any measurement.

A candidate passes the performance gate only when BOTH optimization and holdout
Mech/C geometric means are strictly lower than the last accepted run. Ties or
mixed results do not replace that comparator. A run that worsens both scores
versus the immediately preceding measurement is explicitly a failed run.
Correctness and frozen-source qualification remain separate prerequisites.
The fixed aggregate gate uses Mech/C; raw Mech speedup, original-reference drift
and C drift remain separately reported. It does not establish statistical
significance for small changes.

round  optimization xC  holdout xC  outcome / performance gate
19              39.05       84.28  accepted comparator before rounds20/21
20              33.21       86.62  mixed; not promoted
21              35.08      116.81  both regressed vs20; failed, not promoted
22              31.63       68.48  both improve vs19; accepted

Run after every complete two-set summary, retaining failed runs as well:
python3 scripts/record-polybench-acceptance.py
Before reporting a new performance improvement, require the gate to pass:
python3 scripts/record-polybench-acceptance.py --check --require-latest-accepted

The command preserves prior decisions, detects edited measurements and stale
ledgers, and returns failure if the latest candidate was not accepted. It never
rewrites timing receipts or history. Continue selecting candidates from training
profiles only, freeze before either formal set, and use the holdout aggregate
only for acceptance. The previously exposed split is not blind validation.

Round23: shared checked map and cell bounds
Source 68ccea3f81bd2aebcaa88231253bb523784e241e; IR383/work150/ABI42; executable d36717332070821c123e21e3d90f80ed7e6ee6ad7bb734b11bac6569570a7ba1.
Maps and state-machine cells share checked native addressing. One-axis map reads combine zero/upper bounds; two-axis reads share one pure rejection branch. Original write ordering, full source prices, physical/fuel admission and exact cold errors remain.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 30.769365x C; 25.335174x faster than recorded start; 24.664952x faster than paired original; summed medians 0.004756417s; raw Mech speedup vs22 1.028349x; original-reference runtime multiplier 0.994009x; C runtime multiplier 0.999733x; reference-adjusted Mech speedup 1.022188x.
holdout: 63.724306x C; 31.772123x faster than recorded start; 31.449850x faster than paired original; summed medians 0.016566336s; raw Mech speedup vs22 1.034320x; original-reference runtime multiplier 0.991828x; C runtime multiplier 1.038924x; reference-adjusted Mech speedup 1.025868x.

Training-only seven-sample replay vs22: 1.023462x; 2 regressions over5%. Source/binary frozen after selection and before either formal set.
A proposed two-axis private-first write ordering failed its training probe at0.953301x speedup. A reversed-order14-sample training ablation isolated the regression; original write ordering was restored before new qualification/selection. Neither rejected probe used the holdout. Every raw training observation remains.
Correctness:546 selected checks and both feature builds pass. All900 formal observations,29482 finite warm-up live-outs per artifact and24 history entries remain. Five optimization-only profiles retain 2998 canonical execution samples and cover 79.983168% of training summed medians.
Performance gate: accepted; accepted history index23. Both scores must improve against the last accepted run.
Raw runtime regressions over5% vs22: none.
No ports, limits or compiler source changed after freeze. Holdout aggregate informs acceptance/rejection; optimization selection/tuning uses the optimization pile only. The strict goal remains open unless holdout is below1.0x C.

All thirty kernels, descending current Mech/C kernel ratio. Each C ratio uses its own round C; raw speedups remain separate.
kernel             set            start xC          now xC
jacobi-2d          holdout             9,612.12          449.19
jacobi-1d          holdout             7,168.14          387.74
adi                holdout            21,631.38          201.98
gemver             holdout               260.73          153.56
mvt                optimization          138.96          109.05
gesummv            holdout               112.38           99.55
deriche            optimization       43,560.74           96.16
ludcmp             holdout            20,723.72           83.61
correlation        optimization          327.94           73.83
2mm                optimization           77.03           72.45
bicg               optimization           87.94           72.36
atax               optimization           79.13           65.59
gramschmidt        optimization       27,896.34           55.28
lu                 holdout            20,830.73           52.51
symm               holdout               218.10           46.08
syr2k              holdout               224.54           44.87
syrk               holdout               245.10           39.44
trmm               holdout               219.61           36.85
covariance         optimization           50.46           32.44
3mm                optimization           38.12           31.72
fdtd-2d            holdout            10,500.55           26.27
heat-3d            holdout             4,178.38           24.53
trisolv            holdout             3,606.16           24.29
gemm               optimization           36.05           23.72
durbin             optimization        3,899.39           22.51
doitgen            optimization           22.47           18.04
floyd-warshall     holdout             4,841.31           10.45
cholesky           optimization       23,662.23            8.28
nussinov           optimization      119,638.18            6.07
seidel-2d          optimization       50,168.86            2.86

Round24: typed ordered F64 reductions
Source f1f701bcb14b7c4d76737177cd2dec46d68bb2ae; IR383/work150/ABI42; executable 7d2a90bdbb3b0d556cc7161ff7528695f956055eb2af6937eb5ff6ebaef02d93.
Current exact F64 banks feed the maintained packed constructor after the complete original preflight. Four independent accumulators retain each sum positive-zero seed and original term order. Input NaNs, other kinds and unmatched backing retain ordinary numeric_sum. No source prices, fuel, memory admission, aliases, ports or arithmetic rules change.

Equal-weight geometric means of median kernel runtimes (lower Mech/C is better):
optimization: 30.724979x C; 25.291690x faster than recorded start; 24.885445x faster than paired original; summed medians 0.004800958s; raw Mech speedup vs23 0.998284x; original-reference runtime multiplier 1.010674x; C runtime multiplier 1.003166x; reference-adjusted speedup 1.008940x.
holdout: 64.047151x C; 31.703905x faster than recorded start; 31.466331x faster than paired original; summed medians 0.016547085s; raw Mech speedup vs23 0.997853x; original-reference runtime multiplier 1.002677x; C runtime multiplier 0.997100x; reference-adjusted speedup 1.000524x.

Training-only seven-sample replay vs23: 1.010768x; 0 regressions over5%. Every exact execution report matches accepted23. Selected source/executable frozen before either formal set.
An unguarded development prototype failed one exact NaN payload check (12 passed). Its full log and reconstructible source patch remain in round-24-development-failure.json. No performance was measured for it. Current input NaNs now use the original reduction; all IEEE bits remain checked.
559 selected checks (546 native and13 resident numeric) and both feature builds pass. All900 formal observations,29482 finite warm-up live-outs per artifact and25 history entries remain. Five optimization-only profiles contain 3022 canonical execution samples and cover 80.218344% of training summed medians.
Performance gate: not_promoted; accepted history index23. Both aggregate scores must improve against the last accepted run.
Raw runtime regressions over5% vs23: optimization gramschmidt (+6.62%), holdout syrk (+7.19%).
Selection and tuning use only the optimization pile. Holdout aggregate informs accept/reject after freeze; this previously exposed split is not blind validation. The below1.0x holdout goal remains open unless measured achieved.

All thirty kernels, descending current Mech/C kernel ratio. Each C ratio uses its own round C measurement; raw speedups remain separate.
kernel             set            start xC          now xC
jacobi-2d          holdout             9,612.12          454.88
jacobi-1d          holdout             7,168.14          396.78
adi                holdout            21,631.38          189.52
gemver             holdout               260.73          171.31
gesummv            holdout               112.38          112.98
mvt                optimization          138.96          111.60
deriche            optimization       43,560.74           96.88
ludcmp             holdout            20,723.72           80.95
bicg               optimization           87.94           72.33
correlation        optimization          327.94           70.74
atax               optimization           79.13           66.38
2mm                optimization           77.03           62.26
gramschmidt        optimization       27,896.34           58.26
lu                 holdout            20,830.73           56.11
syr2k              holdout               224.54           44.88
symm               holdout               218.10           41.72
syrk               holdout               245.10           41.32
trmm               holdout               219.61           37.56
3mm                optimization           38.12           34.47
covariance         optimization           50.46           32.20
fdtd-2d            holdout            10,500.55           25.88
durbin             optimization        3,899.39           23.94
heat-3d            holdout             4,178.38           23.82
gemm               optimization           36.05           23.05
trisolv            holdout             3,606.16           22.41
doitgen            optimization           22.47           18.18
floyd-warshall     holdout             4,841.31           10.37
cholesky           optimization       23,662.23            7.75
nussinov           optimization      119,638.18            6.28
seidel-2d          optimization       50,168.86            2.84

Round24 promotion decision and source restoration

Round24 is mixed and not promoted. Accepted23 remains optimization30.769365x C and holdout63.724306x C. After the frozen candidate/profile audit passed, all4 Rust changes were restored exactly from a9d3efc15e. Complete compiler fingerprint is again 0aa2b72813db1aa39632cd3c4e219cc03c64c08880211ccac52c1b70101812c2, matching accepted23. The rejected source f1f701bcb14b7c4d76737177cd2dec46d68bb2ae,559-check qualification,900 observations, development failure and training profiles remain. round-24-restoration.json links the exact decision and byte restoration; no holdout per-case tuning was performed.

Round 25: native elementwise arithmetic

Source-selected F64 matrix arithmetic now uses checked typed loans, shared scalar emission, independent SIMD lanes and packed output. Complete source bills and fallback/replay admission remain. Whole enclosing matrix-valued comprehension/child-FSM fusion remains open.

Both frozen scores improve against accepted23; round25 is accepted. Lower Mech/C is better.

| Set | Accepted23 x C | Round25 x C | Raw speedup vs23 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 30.77x | 27.80x | 1.114x | 28.21x |
| Holdout | 63.72x | 56.60x | 1.127x | 35.80x |

553 selected checks and both feature builds pass, including all 29,482 finite C live-outs in original/decoded artifacts and Object emission. IR384/work151/ABI43. The unchanged root frame and scalar prefix remain; the arithmetic loan is 72 bytes. Four optimization cases and zero holdout cases regress by>5% in raw runtime against23; none are excluded.

Original-reference runtime multipliers vs23 are 1.00950x/1.00737x; reference-adjusted speedups 1.12412x/1.13516x; C multipliers 0.99408x/0.99911x (optimization/holdout). Summed medians are 4.326ms/16.533ms.

Training-only selection was 1.19384x vs23, before freeze. Holdout aggregate informed acceptance; no holdout per-case tuning. Five optimization-only profiles contain 2826 execution samples and cover 81.12% of training summed medians. Failed development sources/logs remain in `round-25-development-failure.json`.

The goal remains unmet: holdout 56.60x C, targeting strictly below 1.0x C. PR1001 remains draft. All 900 observations and 26 append-only history entries remain.

| Kernel | Set | Accepted23 x C | Round25 x C | Starting x C |
|---|---|---:|---:|---:|
| jacobi-2d | Holdout | 449.19x | 449.53x | 9612.12x |
| jacobi-1d | Holdout | 387.74x | 432.71x | 7168.14x |
| adi | Holdout | 201.98x | 191.32x | 21631.38x |
| mvt | Opt | 109.05x | 110.22x | 138.96x |
| gemver | Holdout | 153.56x | 100.37x | 260.73x |
| deriche | Opt | 96.16x | 87.68x | 43560.74x |
| ludcmp | Holdout | 83.61x | 81.62x | 20723.72x |
| bicg | Opt | 72.36x | 74.89x | 87.94x |
| atax | Opt | 65.59x | 67.30x | 79.13x |
| gesummv | Holdout | 99.55x | 67.27x | 112.38x |
| gramschmidt | Opt | 55.28x | 59.66x | 27896.34x |
| correlation | Opt | 73.83x | 52.43x | 327.94x |
| lu | Holdout | 52.51x | 52.32x | 20830.73x |
| 2mm | Opt | 72.45x | 42.06x | 77.03x |
| 3mm | Opt | 31.72x | 34.33x | 38.12x |
| trmm | Holdout | 36.85x | 34.13x | 219.61x |
| syr2k | Holdout | 44.87x | 33.11x | 224.54x |
| symm | Holdout | 46.08x | 32.73x | 218.10x |
| syrk | Holdout | 39.44x | 31.32x | 245.10x |
| trisolv | Holdout | 24.29x | 25.55x | 3606.16x |
| fdtd-2d | Holdout | 26.27x | 25.38x | 10500.55x |
| heat-3d | Holdout | 24.53x | 23.95x | 4178.38x |
| covariance | Opt | 32.44x | 23.74x | 50.46x |
| durbin | Opt | 22.51x | 22.79x | 3899.39x |
| doitgen | Opt | 18.04x | 18.20x | 22.47x |
| gemm | Opt | 23.72x | 14.89x | 36.05x |
| floyd-warshall | Holdout | 10.45x | 10.16x | 4841.31x |
| cholesky | Opt | 8.28x | 8.45x | 23662.23x |
| nussinov | Opt | 6.07x | 5.96x | 119638.18x |
| seidel-2d | Opt | 2.86x | 2.81x | 50168.86x |

[Qualification and controls](round-25-qualification.json) · [Frozen source](round-25-frozen.json) · [Acceptance](round-25-performance-acceptance.json) · [Optimization-only profile](round-25-training-profile.json)

Round26: ordered native matrix products

F64 matrix products now use source-selected Cranelift loops over exact current typed loans, sharing resident geometry and scalar arithmetic. Independent output-column SIMD lanes preserve each cell’s +0 identity and term order, including odd tails and empty inner dimensions. Full source costs, physical checks, replay and atomic publication remain.

Performance gate: accepted; classification: improved_both. Compare against accepted25, with no excluded cases.

| Set | Accepted25 x C | Round26 x C | Raw speedup vs25 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 27.80x | 18.08x | 1.534x | 43.28x |
| Holdout | 56.60x | 46.09x | 1.246x | 44.61x |

555 selected checks and both feature builds pass, including29482 finite C live-outs in original/decoded artifacts and Object emission. IR385/work152/ABI44. Root frame144bytes; separate matrix arithmetic loan80bytes.

Original-reference runtime multipliers vs25: 0.99911x/0.99106x. Reference-adjusted gains: 1.53291x/1.23478x. C multipliers: 1.00213x/0.98560x. Raw regressions>5% vs25: 0/0 (optimization/holdout). All remain.

Training-only seven-sample selection: 1.67345x. Summed medians: 3.918ms/15.892ms. Holdout target remains strictly below1.0x C.

| Kernel | Set | Accepted25 x C | Round26 x C | Starting x C |
|---|---|---:|---:|---:|---:|
| jacobi-2d | Holdout | 449.53x | 450.51x | 9612.12x |
| jacobi-1d | Holdout | 432.71x | 431.56x | 7168.14x |
| adi | Holdout | 191.32x | 192.41x | 21631.38x |
| deriche | Opt | 87.68x | 86.63x | 43560.74x |
| ludcmp | Holdout | 81.62x | 78.78x | 20723.72x |
| bicg | Opt | 74.89x | 62.47x | 87.94x |
| mvt | Opt | 110.22x | 59.01x | 138.96x |
| gramschmidt | Opt | 59.66x | 56.81x | 27896.34x |
| gemver | Holdout | 100.37x | 56.53x | 260.73x |
| lu | Holdout | 52.32x | 56.10x | 20830.73x |
| atax | Opt | 67.30x | 50.78x | 79.13x |
| correlation | Opt | 52.43x | 47.84x | 327.94x |
| syrk | Holdout | 31.32x | 28.14x | 245.10x |
| fdtd-2d | Holdout | 25.38x | 26.54x | 10500.55x |
| syr2k | Holdout | 33.11x | 26.52x | 224.54x |
| heat-3d | Holdout | 23.95x | 24.53x | 4178.38x |
| durbin | Opt | 22.79x | 23.92x | 3899.39x |
| trmm | Holdout | 34.13x | 23.43x | 219.61x |
| trisolv | Holdout | 25.55x | 22.85x | 3606.16x |
| symm | Holdout | 32.73x | 22.40x | 218.10x |
| covariance | Opt | 23.74x | 18.58x | 50.46x |
| gesummv | Holdout | 67.27x | 15.99x | 112.38x |
| 2mm | Opt | 42.06x | 11.52x | 77.03x |
| floyd-warshall | Holdout | 10.16x | 10.34x | 4841.31x |
| 3mm | Opt | 34.33x | 8.55x | 38.12x |
| cholesky | Opt | 8.45x | 7.73x | 23662.23x |
| nussinov | Opt | 5.96x | 6.08x | 119638.18x |
| doitgen | Opt | 18.20x | 5.41x | 22.47x |
| gemm | Opt | 14.89x | 5.15x | 36.05x |
| seidel-2d | Opt | 2.81x | 2.85x | 50168.86x |

[Qualification](round-26-qualification.json) · [Frozen source](round-26-frozen.json) · [Acceptance](round-26-performance-acceptance.json) · [Optimization-only profile](round-26-training-profile.json)

Round27: complete fixed-array FSM results

Complete fixed-array numeric FSMs retain one native phase through terminal construction/extraction and direct array return. At equal recurring numeric coverage, reuse the proved complete-prefix path; growing arrays retain their existing selection. Whole enclosing matrix-valued generator fusion remains open.

Performance gate: accepted; outcome: improved_both. All cases remain.

| Set | Accepted26 x C | Round27 x C | Raw speedup vs26 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 18.08x | 17.52x | 1.045x | 45.23x |
| Holdout | 46.09x | 44.53x | 1.007x | 44.94x |

556 selected checks and both feature builds pass, including29482 finite C live-outs in original/decoded artifacts and Object emission. IR386/work153/ABI44. Original full-prefix source prices/funding, errors, aliases and finite admission remain.

Original-reference runtime multipliers vs26: 0.99225x/0.99779x. Reference-adjusted gains: 1.03686x/1.00519x. C multipliers: 0.98753x/1.02743x. Raw regressions>5% vs26: 0/0 (optimization/holdout). All remain.

Training-only seven-sample replay: 1.00233x overall; one>5% regression retained. Counter changes reflect removed managed boundaries, with all other counters matching. The four diagnosed child proofs have no cold exits; whole enclosing composition remains open. Summed medians: 3.515ms/15.948ms. Holdout goal remains strictly below1.0x C.

| Kernel | Set | Accepted26 x C | Round27 x C | Starting x C |
|---|---|---:|---:|---:|---:|
| jacobi-2d | Holdout | 450.51x | 471.95x | 9612.12x |
| jacobi-1d | Holdout | 431.56x | 394.25x | 7168.14x |
| adi | Holdout | 192.41x | 193.76x | 21631.38x |
| ludcmp | Holdout | 78.78x | 84.69x | 20723.72x |
| deriche | Opt | 86.63x | 73.67x | 43560.74x |
| mvt | Opt | 59.01x | 67.49x | 138.96x |
| gemver | Holdout | 56.53x | 57.17x | 260.73x |
| gramschmidt | Opt | 56.81x | 55.67x | 27896.34x |
| bicg | Opt | 62.47x | 53.15x | 87.94x |
| lu | Holdout | 56.10x | 52.82x | 20830.73x |
| correlation | Opt | 47.84x | 49.16x | 327.94x |
| atax | Opt | 50.78x | 44.82x | 79.13x |
| syrk | Holdout | 28.14x | 28.69x | 245.10x |
| fdtd-2d | Holdout | 26.54x | 26.09x | 10500.55x |
| syr2k | Holdout | 26.52x | 24.56x | 224.54x |
| heat-3d | Holdout | 24.53x | 24.19x | 4178.38x |
| trisolv | Holdout | 22.85x | 23.44x | 3606.16x |
| durbin | Opt | 23.92x | 23.13x | 3899.39x |
| symm | Holdout | 22.40x | 22.28x | 218.10x |
| trmm | Holdout | 23.43x | 20.81x | 219.61x |
| covariance | Opt | 18.58x | 18.58x | 50.46x |
| gesummv | Holdout | 15.99x | 11.64x | 112.38x |
| 2mm | Opt | 11.52x | 11.60x | 77.03x |
| floyd-warshall | Holdout | 10.34x | 10.37x | 4841.31x |
| cholesky | Opt | 7.73x | 7.73x | 23662.23x |
| 3mm | Opt | 8.55x | 7.54x | 38.12x |
| nussinov | Opt | 6.08x | 6.52x | 119638.18x |
| gemm | Opt | 5.15x | 5.15x | 36.05x |
| doitgen | Opt | 5.41x | 4.86x | 22.47x |
| seidel-2d | Opt | 2.85x | 2.92x | 50168.86x |

[Qualification](round-27-qualification.json) · [Frozen source](round-27-frozen.json) · [Acceptance](round-27-performance-acceptance.json) · [Optimization-only profile](round-27-training-profile.json)

Round28: bounded matrix generators

One ordinary native generator combines fixed F64 matrix comprehensions, bounded FSMs and scalar maps. Source proofs, native scalar/array/frame machinery, source prices, immutable owner pins, finite admission, checked failure replay and atomic publication are shared. Column/row outer broadcasting is supported by the maintained type schemes.

Performance gate: not_promoted; outcome: mixed_or_tied. Optimization improved, holdout regressed. Accepted compiler remains round27; this candidate is retained for further training-only compiler work.

| Set | Accepted27 x C | Round28 x C | Raw speedup vs27 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 17.52x | 15.98x | 1.087x | 49.18x |
| Holdout | 44.53x | 45.76x | 0.979x | 43.99x |

575 selected checks and both feature builds pass, including all30 original/decoded C-liveout programs and Object emission. Actual full Deriche generator execution also passes under the unchanged benchmark caps, matches4096 finite C outputs and has no successful body callbacks. Resource refusal, aliases, source errors, malformed loans, NaN payloads and retry cleanup remain covered. IR387/work154/ABI45.

Original-reference runtime multipliers vs27: 1.01296x/1.00174x. Reference-adjusted gains: 1.10129x/0.98062x. C multipliers: 1.00839x/0.99417x. Raw regressions>5% vs27: 4/2 (optimization/holdout). All remain.

Optimization-only seven-sample replay: 1.10308x overall; Deriche 5.81406x; two>5% regressions retained. All unrelated execution counters match. Original source work/comparison accounting increases conservatively. Formal summed medians: 1.788ms/16.266ms. The holdout goal remains strictly below1.0x C.

Training-only profiles cover78.3% of the current optimization sum and1553 canonical execution samples. Deriche is now about50.7% generated execution and47.8% canonical library calls, with transpose prominent. Correlation remains46.4% canonical library calls. Future source selection will use optimization evidence only.

| Kernel | Set | Accepted27 x C | Round28 x C | Starting x C |
|---|---|---:|---:|---:|---:|
| jacobi-2d | Holdout | 471.95x | 466.21x | 9612.12x |
| jacobi-1d | Holdout | 394.25x | 401.11x | 7168.14x |
| adi | Holdout | 193.76x | 196.67x | 21631.38x |
| ludcmp | Holdout | 84.69x | 91.34x | 20723.72x |
| mvt | Opt | 67.49x | 68.53x | 138.96x |
| bicg | Opt | 53.15x | 58.08x | 87.94x |
| lu | Holdout | 52.82x | 56.58x | 20830.73x |
| gramschmidt | Opt | 55.67x | 55.48x | 27896.34x |
| gemver | Holdout | 57.17x | 49.75x | 260.73x |
| atax | Opt | 44.82x | 48.94x | 79.13x |
| correlation | Opt | 49.16x | 48.34x | 327.94x |
| syrk | Holdout | 28.69x | 27.27x | 245.10x |
| fdtd-2d | Holdout | 26.09x | 26.14x | 10500.55x |
| syr2k | Holdout | 24.56x | 25.97x | 224.54x |
| trisolv | Holdout | 23.44x | 25.22x | 3606.16x |
| heat-3d | Holdout | 24.19x | 24.06x | 4178.38x |
| durbin | Opt | 23.13x | 22.70x | 3899.39x |
| symm | Holdout | 22.28x | 22.31x | 218.10x |
| trmm | Holdout | 20.81x | 21.85x | 219.61x |
| covariance | Opt | 18.58x | 17.92x | 50.46x |
| gesummv | Holdout | 11.64x | 15.04x | 112.38x |
| deriche | Opt | 73.67x | 13.47x | 43560.74x |
| 2mm | Opt | 11.60x | 11.81x | 77.03x |
| floyd-warshall | Holdout | 10.37x | 10.37x | 4841.31x |
| 3mm | Opt | 7.54x | 9.15x | 38.12x |
| cholesky | Opt | 7.73x | 7.84x | 23662.23x |
| nussinov | Opt | 6.52x | 6.20x | 119638.18x |
| doitgen | Opt | 4.86x | 5.01x | 22.47x |
| gemm | Opt | 5.15x | 4.94x | 36.05x |
| seidel-2d | Opt | 2.92x | 3.04x | 50168.86x |

[Qualification](round-28-qualification.json) · [Frozen source](round-28-frozen.json) · [Acceptance](round-28-performance-acceptance.json) · [Optimization-only profile](round-28-training-profile.json)

Round29: typed native matrix transpose

Sealed F64 rank2 transposes now use the maintained numeric-operation loan and resident shape/source-budget machinery. Ordinary calls and generator banks share exact-bit Cranelift transpose lowering. Dynamic and literal axes remain checked; complete source costs, replay, pinned owners, finite admission and atomic publication remain.

Performance gate: accepted; outcome: improved_both. Accepted compiler after this run: round29.

| Set | Accepted27 x C | Round29 x C | Raw speedup vs27 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 17.52x | 14.24x | 1.228x | 55.52x |
| Holdout | 44.53x | 41.53x | 1.090x | 49.00x |

576 selected checks and both standalone feature checks pass, including all30 original/decoded C-liveout programs and Object emission. Fixed/dynamic empty, vector and rectangular shapes preserve exact F64 signedzero, infinity and quiet/signaling NaN payload bits. Existing finite caps, managed memory/node refusal, forged loans, source errors, alias preservation, cleanup and retry remain covered. IR388/work155/ABI46.

Original-reference runtime multipliers vs27: 1.00460x/1.00344x. Reference-adjusted gains: 1.23322x/1.09424x. C multipliers: 1.00253x/0.98323x. Raw regressions>5% vs27: 4/2 (optimization/holdout). All remain.

The prospective optimization-only A/B rule rejected shared numeric-loan ownership at0.98737x vs transpose-only; that source was restored exactly before freeze. Selected transpose-only raw training speedup vs accepted27: 1.32837x. All execution counters match between the two transpose variants. Formal summed medians: 1.620ms/15.606ms. The holdout goal remains strictly below1.0x C.

| Kernel | Set | Accepted27 x C | Round29 x C | Starting x C | Round29 Mech us | Round29 C us |
|---|---|---:|---:|---:|---:|---:|
| jacobi-2d | Holdout | 471.95x | 451.95x | 9612.12x | 3502.583 | 7.750 |
| jacobi-1d | Holdout | 394.25x | 397.02x | 7168.14x | 181.917 | 0.458 |
| adi | Holdout | 193.76x | 192.91x | 21631.38x | 8439.917 | 43.750 |
| ludcmp | Holdout | 84.69x | 84.41x | 20723.72x | 1041.084 | 12.333 |
| gramschmidt | Opt | 55.67x | 60.40x | 27896.34x | 208.916 | 3.459 |
| lu | Holdout | 52.82x | 59.10x | 20830.73x | 659.917 | 11.167 |
| mvt | Opt | 67.49x | 58.27x | 138.96x | 75.292 | 1.292 |
| bicg | Opt | 53.15x | 47.12x | 87.94x | 74.625 | 1.584 |
| correlation | Opt | 49.16x | 42.99x | 327.94x | 184.500 | 4.292 |
| fdtd-2d | Holdout | 26.09x | 26.55x | 10500.55x | 182.541 | 6.875 |
| heat-3d | Holdout | 24.19x | 24.81x | 4178.38x | 433.167 | 17.458 |
| durbin | Opt | 23.13x | 24.26x | 3899.39x | 30.334 | 1.250 |
| trisolv | Holdout | 23.44x | 23.82x | 3606.16x | 25.791 | 1.083 |
| trmm | Holdout | 20.81x | 23.37x | 219.61x | 77.875 | 3.333 |
| symm | Holdout | 22.28x | 21.60x | 218.10x | 81.916 | 3.792 |
| syrk | Holdout | 28.69x | 20.96x | 245.10x | 117.042 | 5.583 |
| syr2k | Holdout | 24.56x | 18.65x | 224.54x | 128.250 | 6.875 |
| gemver | Holdout | 57.17x | 18.51x | 260.73x | 39.333 | 2.125 |
| gesummv | Holdout | 11.64x | 18.37x | 112.38x | 18.375 | 1.000 |
| covariance | Opt | 18.58x | 15.49x | 50.46x | 69.708 | 4.500 |
| atax | Opt | 44.82x | 14.92x | 79.13x | 18.042 | 1.209 |
| 2mm | Opt | 11.60x | 13.68x | 77.03x | 22.792 | 1.667 |
| deriche | Opt | 73.67x | 10.78x | 43560.74x | 316.542 | 29.375 |
| floyd-warshall | Holdout | 10.37x | 10.38x | 4841.31x | 676.042 | 65.125 |
| 3mm | Opt | 7.54x | 8.29x | 38.12x | 22.792 | 2.750 |
| cholesky | Opt | 7.73x | 7.91x | 23662.23x | 56.375 | 7.125 |
| nussinov | Opt | 6.52x | 6.35x | 119638.18x | 185.208 | 29.167 |
| doitgen | Opt | 4.86x | 5.83x | 22.47x | 18.459 | 3.167 |
| gemm | Opt | 5.15x | 4.75x | 36.05x | 20.208 | 4.250 |
| seidel-2d | Opt | 2.92x | 2.94x | 50168.86x | 316.417 | 107.667 |

[Qualification](round-29-qualification.json) · [Frozen source](round-29-frozen.json) · [Acceptance](round-29-performance-acceptance.json) · [Training selection](round-29-training-selection.json)

Round30: ordered native matrix sums

Sealed F64 row/column reductions share resident geometry, full source costs/replay, checked numeric loans and native scalar emission. Every lane starts at positive zero and adds terms in source order. SIMD lanes compute independent outputs; empty reduced axes return typed zero. Current owner pins, initialized banks, finite admission, span/frame checks, ordinary fallback and atomic publication remain.

Performance gate: not_promoted; outcome: mixed_or_tied. Accepted compiler after this run: round29.

| Set | Accepted29 x C | Round30 x C | Raw speedup vs29 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 14.24x | 13.04x | 1.098x | 60.98x |
| Holdout | 41.53x | 42.29x | 0.997x | 48.87x |

579 selected checks and both standalone features pass, including all30 original/decoded C-liveout programs and Object emission. New reductions cover original/decoded/restored fixed/dynamic shapes, empty axes, odd tails, signedzero, cancellation, input/generated NaNs and infinities. Existing finite limits, physical byte refusal, forged loans, aliases, interleaving, cleanup and retry include both axes. IR389/work156/ABI47.

Optimization-only seven-sample replay measured 1.10250x vs accepted29, with 0 raw regressions>5%. All unrelated counters match; every work/comparison/visit/transport delta remains. Profiling accepted29 selected this family using1380 execution samples covering74.8% of optimization medians; no holdout profile or per-case diagnosis.

Original-reference runtime multipliers vs29: 1.00596x/1.00241x. Reference-adjusted gains: 1.10476x/0.99975x. C multipliers: 0.99425x/0.98473x. Raw regressions>5% vs29: 1/1 (optimization/holdout), all retained.

Formal summed medians: 1.503ms/15.684ms. The goal remains strictly below1.0x C on the fixed15 holdout.

| Kernel | Set | Accepted29 x C | Round30 x C | Starting x C | Round30 Mech us | Round30 C us |
|---|---|---:|---:|---:|---:|---:|
| jacobi-2d | Holdout | 451.95x | 461.17x | 9612.12x | 3555.167 | 7.709 |
| jacobi-1d | Holdout | 397.02x | 437.90x | 7168.14x | 182.708 | 0.417 |
| adi | Holdout | 192.91x | 188.95x | 21631.38x | 8463.083 | 44.791 |
| ludcmp | Holdout | 84.41x | 84.42x | 20723.72x | 1041.166 | 12.333 |
| gramschmidt | Opt | 60.40x | 59.51x | 27896.34x | 205.791 | 3.458 |
| lu | Holdout | 59.10x | 54.58x | 20830.73x | 657.166 | 12.041 |
| mvt | Opt | 58.27x | 52.91x | 138.96x | 70.542 | 1.333 |
| bicg | Opt | 47.12x | 49.92x | 87.94x | 74.875 | 1.500 |
| correlation | Opt | 42.99x | 34.45x | 327.94x | 146.417 | 4.250 |
| fdtd-2d | Holdout | 26.55x | 26.65x | 10500.55x | 183.208 | 6.875 |
| heat-3d | Holdout | 24.81x | 25.08x | 4178.38x | 432.625 | 17.250 |
| trmm | Holdout | 23.37x | 24.45x | 219.61x | 77.417 | 3.166 |
| trisolv | Holdout | 23.82x | 23.85x | 3606.16x | 27.833 | 1.167 |
| durbin | Opt | 24.26x | 22.64x | 3899.39x | 29.250 | 1.292 |
| symm | Holdout | 21.60x | 22.41x | 218.10x | 83.125 | 3.709 |
| syrk | Holdout | 20.96x | 21.55x | 245.10x | 117.625 | 5.458 |
| gemver | Holdout | 18.51x | 20.33x | 260.73x | 37.292 | 1.834 |
| syr2k | Holdout | 18.65x | 20.13x | 224.54x | 131.666 | 6.542 |
| gesummv | Holdout | 18.37x | 16.87x | 112.38x | 17.584 | 1.042 |
| atax | Opt | 14.92x | 15.47x | 79.13x | 19.334 | 1.250 |
| 2mm | Opt | 13.68x | 11.84x | 77.03x | 21.209 | 1.791 |
| floyd-warshall | Holdout | 10.38x | 10.83x | 4841.31x | 676.500 | 62.458 |
| deriche | Opt | 10.78x | 9.98x | 43560.74x | 289.541 | 29.000 |
| cholesky | Opt | 7.91x | 8.07x | 23662.23x | 55.791 | 6.917 |
| 3mm | Opt | 8.29x | 8.03x | 38.12x | 20.416 | 2.542 |
| covariance | Opt | 15.49x | 7.09x | 50.46x | 31.625 | 4.458 |
| nussinov | Opt | 6.35x | 6.25x | 119638.18x | 183.750 | 29.416 |
| doitgen | Opt | 5.83x | 5.47x | 22.47x | 16.875 | 3.083 |
| gemm | Opt | 4.75x | 5.04x | 36.05x | 21.000 | 4.167 |
| seidel-2d | Opt | 2.94x | 3.01x | 50168.86x | 316.791 | 105.333 |

[Qualification](round-30-qualification.json) · [Frozen source](round-30-frozen.json) · [Acceptance](round-30-performance-acceptance.json) · [Optimization-only profile](round-30-training-profile.json)

Round31: inverse scalar bitcasts and native profile attribution

Lossless scalar bitcast round trips alias the existing SSA source when flags and original/result types match. Live inner uses, every FP operation, memory access, branch, source error/debit, observation and publication remain. Vectors and mismatched flags remain unchanged. Existing ordered matrix sum source from30 remains included.

Performance gate: accepted; outcome: improved_both. Accepted compiler after this run: round31.

| Set | Accepted29 x C | Round31 x C | Raw speedup vs29 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 14.24x | 11.44x | 1.191x | 66.14x |
| Holdout | 41.53x | 39.32x | 1.044x | 51.16x |

582 selected checks and both standalone features pass, including all30 original/decoded C-liveout programs and Object emission. New direct JIT witnesses preserve arbitrary F32/F64 bits, signaling/quiet NaNs, signedzero, live word stores, unfused multiply/add and reached error branches. IR390/work156/ABI47.

Optimization-only seven-sample replay measured 1.19992x vs accepted29 and 1.07209x vs base30, with 2 raw regressions>5% vs29. All execution counters match base30 exactly. No case or observation was dropped.

Profiling only the five largest round30 optimization medians covered 75.99% of summed optimization time: 1306 canonical execution samples and 970 resolved JIT self samples. Same-process maps attribute real native instructions; all136 sampled hot-site instructions and surrounding68-byte windows matched actual frozen JIT bytes to real checked Objects. Current Object emission removes84 inverse pairs across six training programs. Profiles, debugger snapshots and Object inspection never enter timing scores.

Original-reference runtime multipliers vs29: 0.99889x/0.98743x. Reference-adjusted gains: 1.18984x/1.03096x. C multipliers: 1.04497x/1.01181x. Raw regressions>5% vs29: 0/0 (optimization/holdout), all retained.

Formal summed medians: 1.280ms/15.617ms. Goal remains strictly below1.0x C on the fixed15 holdout.

| Kernel | Set | Accepted29 x C | Round31 x C | Starting x C | Round31 Mech us | Round31 C us |
|---|---|---:|---:|---:|---:|---:|
| jacobi-2d | Holdout | 451.95x | 458.98x | 9612.12x | 3538.250 | 7.709 |
| jacobi-1d | Holdout | 397.02x | 388.01x | 7168.14x | 177.791 | 0.458 |
| adi | Holdout | 192.91x | 191.08x | 21631.38x | 8550.917 | 44.750 |
| ludcmp | Holdout | 84.41x | 85.37x | 20723.72x | 1092.125 | 12.792 |
| gramschmidt | Opt | 60.40x | 54.48x | 27896.34x | 197.500 | 3.625 |
| lu | Holdout | 59.10x | 53.43x | 20830.73x | 641.084 | 12.000 |
| mvt | Opt | 58.27x | 44.97x | 138.96x | 73.083 | 1.625 |
| bicg | Opt | 47.12x | 42.71x | 87.94x | 71.208 | 1.667 |
| correlation | Opt | 42.99x | 32.72x | 327.94x | 147.208 | 4.500 |
| fdtd-2d | Holdout | 26.55x | 25.45x | 10500.55x | 174.958 | 6.875 |
| trisolv | Holdout | 23.82x | 22.67x | 3606.16x | 25.500 | 1.125 |
| trmm | Holdout | 23.37x | 21.52x | 219.61x | 75.333 | 3.500 |
| syrk | Holdout | 20.96x | 21.20x | 245.10x | 111.292 | 5.250 |
| durbin | Opt | 24.26x | 21.03x | 3899.39x | 28.917 | 1.375 |
| symm | Holdout | 21.60x | 20.30x | 218.10x | 77.834 | 3.834 |
| heat-3d | Holdout | 24.81x | 19.11x | 4178.38x | 341.667 | 17.875 |
| gesummv | Holdout | 18.37x | 18.49x | 112.38x | 17.708 | 0.958 |
| syr2k | Holdout | 18.65x | 17.94x | 224.54x | 123.333 | 6.875 |
| gemver | Holdout | 18.51x | 16.42x | 260.73x | 35.583 | 2.167 |
| atax | Opt | 14.92x | 13.31x | 79.13x | 17.750 | 1.333 |
| 2mm | Opt | 13.68x | 11.63x | 77.03x | 20.833 | 1.792 |
| floyd-warshall | Holdout | 10.38x | 9.61x | 4841.31x | 633.958 | 66.000 |
| 3mm | Opt | 8.29x | 8.65x | 38.12x | 22.000 | 2.542 |
| cholesky | Opt | 7.91x | 7.79x | 23662.23x | 54.541 | 7.000 |
| deriche | Opt | 10.78x | 7.04x | 43560.74x | 206.583 | 29.333 |
| covariance | Opt | 15.49x | 6.34x | 50.46x | 29.083 | 4.584 |
| nussinov | Opt | 6.35x | 5.83x | 119638.18x | 172.709 | 29.625 |
| gemm | Opt | 4.75x | 4.57x | 36.05x | 20.583 | 4.500 |
| doitgen | Opt | 5.83x | 4.42x | 22.47x | 14.375 | 3.250 |
| seidel-2d | Opt | 2.94x | 1.89x | 50168.86x | 203.708 | 107.750 |

[Qualification](round-31-qualification.json) · [Frozen source](round-31-frozen.json) · [Acceptance](round-31-performance-acceptance.json) · [Optimization-only native profiles](round-31-training-profile.json)

Round32: shared SIMD numeric bank copies

Complete checked matrix maps and existing numeric generator prefix copies share one exact-bit emitter.128-bit transfers run in64-byte chunks with exact vector/scalar tails. Partial overlaps retain original left-to-right scalar-unit order. Source computations all succeed before destination stores; current owners, initialized storage, finite admission, source prices/replay, actual fixed-template funding and atomic publication remain.

Performance gate: not_promoted; outcome: failed_both_regressed. Accepted compiler after this run: round31.

| Set | Accepted31 x C | Round32 x C | Raw speedup vs31 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 11.44x | 12.13x | 0.991x | 65.55x |
| Holdout | 39.32x | 39.49x | 0.993x | 50.81x |

585 selected checks and both standalone feature checks pass, including all30 original/decoded C-liveout programs and Object emission. Direct speed-optimized JIT tests preserve arbitrary bits, all scalar widths, unaligned and odd tails, both partial overlaps and self copy. Independent inaccessible guard pages enforce exact reads/writes and zero-length null spans. IR391/work156/ABI47.

The optimization-only probe measured 1.00681x against accepted31, retaining 2 regressions>5%. All execution counters match exactly. Current real Objects emit54 SIMD memory instructions in the profiled copy function versus0 before.

Accepted31 optimization profiles used 1099 canonical execution samples, including 806 resolved JIT self samples, covering 72.47% of summed optimization medians. All130 sampled hot-site instructions and surrounding68-byte windows matched actual checked JIT bytes to real Objects. No profiles, debugger snapshots or Object-inspection times enter scores.

Original-reference runtime multipliers vs31: 0.99699x/1.00928x. Reference-adjusted gains: 0.98815x/1.00224x. C multipliers: 0.95138x/1.00250x. Raw regressions>5% vs31: 3/3 (optimization/holdout), all retained.

Formal summed medians: 1.283ms/15.198ms. Goal remains strictly below1.0x C on the fixed15 holdout.

| Kernel | Set | Accepted31 x C | Round32 x C | Starting x C | Round32 Mech us | Round32 C us |
|---|---|---:|---:|---:|---:|---:|
| jacobi-2d | Holdout | 458.98x | 451.17x | 9612.12x | 3478.042 | 7.709 |
| jacobi-1d | Holdout | 388.01x | 400.61x | 7168.14x | 183.375 | 0.458 |
| adi | Holdout | 191.08x | 183.27x | 21631.38x | 8224.208 | 44.875 |
| ludcmp | Holdout | 85.37x | 83.79x | 20723.72x | 1029.833 | 12.291 |
| mvt | Opt | 44.97x | 53.40x | 138.96x | 69.000 | 1.292 |
| lu | Holdout | 53.43x | 53.09x | 20830.73x | 634.916 | 11.959 |
| gramschmidt | Opt | 54.48x | 52.34x | 27896.34x | 189.750 | 3.625 |
| bicg | Opt | 42.71x | 46.97x | 87.94x | 72.375 | 1.541 |
| correlation | Opt | 32.72x | 34.40x | 327.94x | 146.167 | 4.250 |
| fdtd-2d | Holdout | 25.45x | 24.05x | 10500.55x | 165.334 | 6.875 |
| symm | Holdout | 20.30x | 23.05x | 218.10x | 82.625 | 3.584 |
| trmm | Holdout | 21.52x | 22.58x | 219.61x | 77.125 | 3.416 |
| durbin | Opt | 21.03x | 22.38x | 3899.39x | 28.916 | 1.292 |
| heat-3d | Holdout | 19.11x | 21.07x | 4178.38x | 359.875 | 17.083 |
| trisolv | Holdout | 22.67x | 20.92x | 3606.16x | 25.292 | 1.209 |
| syrk | Holdout | 21.20x | 20.34x | 245.10x | 114.417 | 5.625 |
| syr2k | Holdout | 17.94x | 19.10x | 224.54x | 129.708 | 6.792 |
| gemver | Holdout | 16.42x | 18.67x | 260.73x | 37.333 | 2.000 |
| gesummv | Holdout | 18.49x | 15.12x | 112.38x | 17.625 | 1.166 |
| atax | Opt | 13.31x | 14.90x | 79.13x | 18.625 | 1.250 |
| 2mm | Opt | 11.63x | 12.20x | 77.03x | 20.833 | 1.708 |
| floyd-warshall | Holdout | 9.61x | 9.95x | 4841.31x | 638.334 | 64.125 |
| 3mm | Opt | 8.65x | 8.15x | 38.12x | 20.708 | 2.542 |
| cholesky | Opt | 7.79x | 8.05x | 23662.23x | 54.000 | 6.708 |
| deriche | Opt | 7.04x | 7.03x | 43560.74x | 204.250 | 29.041 |
| covariance | Opt | 6.34x | 6.52x | 50.46x | 28.792 | 4.417 |
| nussinov | Opt | 5.83x | 6.25x | 119638.18x | 187.458 | 30.000 |
| gemm | Opt | 4.57x | 5.37x | 36.05x | 22.583 | 4.208 |
| doitgen | Opt | 4.42x | 5.16x | 22.47x | 15.708 | 3.042 |
| seidel-2d | Opt | 1.89x | 1.89x | 50168.86x | 204.208 | 108.000 |

[Qualification](round-32-qualification.json) · [Frozen source](round-32-frozen.json) · [Acceptance](round-32-performance-acceptance.json) · [Optimization-only profiles](round-32-training-profile.json)

Round33: direct source-ordered rectangular loops

Rectangular comprehensions visit complete columns with direct column/row counters, retaining the exact scalar body, term and error order, checked accesses and row-major storage. One-row rectangles retain one loop. Genuine current loans, ownership, initialized banks, finite admission, full source costs/replay, actual fixed-template funding and atomic publication remain. Rejected32 SIMD-copy source is excluded.

Performance gate: not_promoted; outcome vs previous measured run: improved_both. Promotion compares accepted31. Accepted compiler after this run: round31.

| Set | Accepted31 x C | Round33 x C | Raw speedup vs31 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 11.44x | 11.81x | 1.002x | 66.26x |
| Holdout | 39.32x | 38.82x | 1.003x | 51.31x |

585 selected checks and both standalone feature checks pass, including all30 original/decoded C-liveout programs and Object emission. New production-emitter JIT tests cover all exact scalar kinds/widths, NaN payloads/signedzero, one/two-row shapes, range/value and reordered/empty sources, column-first failure and malformed frames. Simple two-axis fixtures emit no Udiv/Urem. IR392/work156/ABI47.

The optimization-only probe measured 1.00727x against accepted31, retaining 2 regressions>5%. Every execution counter matches exactly. Gram-Schmidt measured1.19228x; its actual Object function has three fewer integer division/multiply-high instructions,26 to23.

Accepted31 optimization profiles used 1099 canonical samples and 806 resolved JIT self samples, covering 72.47% of summed optimization medians. All130 sampled hot-site instructions/windows matched actual checked JIT bytes to real Objects. No diagnostic times enter performance scores.

Original-reference runtime multipliers vs31: 1.00329x/1.00120x. Reference-adjusted gains: 1.00511x/1.00403x. C multipliers: 0.96635x/1.00994x. Raw regressions>5% vs31: 2/4 (optimization/holdout), all retained.

Formal summed medians: 1.268ms/15.300ms. Goal remains strictly below1.0x C on fixed15 holdout.

| Kernel | Set | Accepted31 x C | Round33 x C | Starting x C | Round33 Mech us | Round33 C us |
|---|---|---:|---:|---:|---:|---:|
| jacobi-2d | Holdout | 458.98x | 462.11x | 9612.12x | 3581.333 | 7.750 |
| jacobi-1d | Holdout | 388.01x | 452.29x | 7168.14x | 188.500 | 0.417 |
| adi | Holdout | 191.08x | 186.00x | 21631.38x | 8323.333 | 44.750 |
| ludcmp | Holdout | 85.37x | 85.68x | 20723.72x | 1056.667 | 12.333 |
| lu | Holdout | 53.43x | 55.07x | 20830.73x | 635.584 | 11.541 |
| mvt | Opt | 44.97x | 51.04x | 138.96x | 70.167 | 1.375 |
| gramschmidt | Opt | 54.48x | 49.63x | 27896.34x | 175.791 | 3.542 |
| bicg | Opt | 42.71x | 42.15x | 87.94x | 68.500 | 1.625 |
| correlation | Opt | 32.72x | 37.11x | 327.94x | 157.708 | 4.250 |
| trisolv | Holdout | 22.67x | 24.40x | 3606.16x | 26.417 | 1.083 |
| symm | Holdout | 20.30x | 23.51x | 218.10x | 87.166 | 3.708 |
| durbin | Opt | 21.03x | 22.39x | 3899.39x | 28.916 | 1.292 |
| gemver | Holdout | 16.42x | 22.19x | 260.73x | 38.833 | 1.750 |
| fdtd-2d | Holdout | 25.45x | 22.01x | 10500.55x | 152.250 | 6.917 |
| trmm | Holdout | 21.52x | 21.75x | 219.61x | 75.208 | 3.458 |
| syrk | Holdout | 21.20x | 20.50x | 245.10x | 111.916 | 5.459 |
| syr2k | Holdout | 17.94x | 19.96x | 224.54x | 131.417 | 6.583 |
| heat-3d | Holdout | 19.11x | 18.44x | 4178.38x | 328.083 | 17.792 |
| atax | Opt | 13.31x | 12.85x | 79.13x | 17.125 | 1.333 |
| 2mm | Opt | 11.63x | 12.46x | 77.03x | 21.291 | 1.709 |
| gesummv | Holdout | 18.49x | 9.70x | 112.38x | 17.375 | 1.792 |
| floyd-warshall | Holdout | 9.61x | 8.37x | 4841.31x | 546.084 | 65.209 |
| 3mm | Opt | 8.65x | 7.98x | 38.12x | 21.292 | 2.667 |
| cholesky | Opt | 7.79x | 7.44x | 23662.23x | 53.333 | 7.167 |
| deriche | Opt | 7.04x | 7.26x | 43560.74x | 208.792 | 28.750 |
| covariance | Opt | 6.34x | 6.68x | 50.46x | 30.042 | 4.500 |
| nussinov | Opt | 5.83x | 6.33x | 119638.18x | 172.792 | 27.292 |
| doitgen | Opt | 4.42x | 5.04x | 22.47x | 15.958 | 3.167 |
| gemm | Opt | 4.57x | 4.90x | 36.05x | 20.833 | 4.250 |
| seidel-2d | Opt | 1.89x | 1.90x | 50168.86x | 205.542 | 107.958 |

[Qualification](round-33-qualification.json) · [Frozen source](round-33-frozen.json) · [Acceptance](round-33-performance-acceptance.json) · [Optimization-only profiles](round-33-training-profile.json)


Training-only experiment: F64 SSA homes (IR394), not promoted

Known F64 numeric payloads used floating SSA homes with exact raw-word boundaries,
checked guarded/prefix bindings, maintained scalar emission and existing bitcast
cleanup. All585 selected checks and both feature checks passed. The single
seven-sample optimization15 probe measured0.997353x versus accepted31, with three
regressions above5%; it was rejected by the prospective rule before holdout.
Accepted31 remains11.436358x C optimization and39.315695x C holdout. No formal
measurement or history entry was added. Complete cases, correctness logs, both
development failures, checked Object code and source commit remain in
training-experiments/f64-ssa-homes.json and qualification/experiments/f64-ssa-homes/.


Training-only experiment: ordered F64 fold accumulators (IR395), not promoted

The shared comprehension fold used an F64 SSA accumulator with positive-zero
seed, unchanged source term order and the existing raw-word completion frame.
All584 selected checks and both feature checks passed. Checked Gram-Schmidt
code lost two integer/F64 transfers and six instructions, but the single
seven-sample optimization15 probe measured0.989228x versus accepted31, with
two regressions above5%. It was rejected before holdout under the preset rule.
Accepted31 remains11.436358x C optimization and39.315695x C holdout. No formal
measurement or history entry was added. All cases, correctness logs, actual
emitter witnesses, checked Objects and source commit remain in
training-experiments/f64-fold-accumulators.json and
qualification/experiments/f64-fold-accumulators/.


Training-only experiment: guarded direct phase maps (IR396), not promoted

Checked totality, range and alias proofs allowed complete same-cell/disjoint
maps to write existing exclusive banks without the completion copy. All585
selected checks and both features passed. Actual JIT pages matched checked
Objects at30 sampled sites and across the entire profiled numeric function.
Only Gram-Schmidt changed native code: the single optimization15 probe measured
1.04468x there and0.973791x overall, with three regressions above5%. The trial
was rejected before holdout. Accepted31 remains11.436358x C optimization and
39.315695x C holdout. All measurements, failures, profiles and source remain in
training-experiments/direct-phase-maps.json and
qualification/experiments/direct-phase-maps/.

Round34: cache canonical native type keys

Immutable native types retain their canonical schema key instead of allocating canonical bytes and hashing the same schema during execution. The definition and cache are private, derived at source/image construction and absent from serialized images. No runtime field, allocation, pointer, helper, admission, source fee or arithmetic change is introduced. Compiled host metadata adds32 bytes per type.

Performance gate: not_promoted; accepted compiler after this run: round31. Strict comparison is against accepted31.

| Set | Accepted31 x C | Round34 x C | Raw speedup vs31 | Raw speedup since start |
|---|---:|---:|---:|---:|
| Optimization | 11.44x | 11.47x | 1.00858x | 66.71x |
| Holdout | 39.32x | 38.72x | 1.00612x | 51.48x |

608 selected checks and both feature checks pass. Type/image checks include fixed and parameterized managed matrices, all scalar kinds, same-ID different definitions, and rejection of caller-supplied derived keys; maintained negative nominal/constrained type checks remain. All30 original/decoded finite C-liveout programs and Object emission pass. IR397/work156/ABI47.

The one prospective optimization-only probe measured 1.02682x vs accepted31, retaining all 2 regressions>5%. Every execution counter matches exactly. All15 optimization Object function disassemblies are identical to accepted31.

Original-reference runtime multipliers vs31: 1.01083x/1.01920x. Reference-adjusted gains: 1.01950x/1.02544x. C multipliers: 0.98848x/1.00913x. Raw regressions>5% vs31: 3/2, all retained.

Formal sums of Mech medians: 1.301ms/15.315ms. All900 observations,29482 finite C-liveouts, and35 append-only history entries are retained. Source and driver were committed/frozen before both formal sets. Goal remains strictly below1.0x C on fixed15 holdout.

[Qualification](round-34-qualification.json) · [Frozen source](round-34-frozen.json) · [Acceptance](round-34-performance-acceptance.json) · [Optimization-only profile](training-experiments/root-operator-profile.json)

Round34 was not promoted because optimization11.4712817243x C is worse than accepted31 11.4363579771x, despite holdout38.7231493345x improving from39.3156954892x. Both scores must strictly improve. Exact accepted31 compiler source and driver are restored; all608 checks,900 observations and all regressions remain. No remote push. Next training-only target is closed matrix-expression regions using existing resident/JIT kernels and current-bank machinery; see training-experiments/root-operator-regions-plan.json.

Matrix-expression implementation stage: source-derived regions cover8 optimization programs,37 operators and25 nonescaping intermediates;37 logical banks can use26 strictly disjoint storage slots. The source inventory is compiler-only and region execution remains selected:false. Core now exposes and shares exact prospective ownership costs for fresh plain-F64 matrices without making intermediate snapshots merely to price them.94 snapshot checks,6 region checks,24 matrix checks and all30 numerical correctness cases pass, along with both standalone feature builds. No new timings, formal history entry or promotion; accepted31 remains11.4363579771x/39.3156954892x C. Runtime lending and complete-region kernel emission are the next required work. See training-experiments/matrix-region-inventory.json and training-experiments/matrix-region-ownership.json.

Round35: complete matrix-expression loans (not promoted)

General root matrix-expression DAGs use one current checked loan and the existing ordered scalar/SIMD kernels. Private intermediates share disjoint storage; escaping values keep canonical publication. Shared Core F64 owner pricing and a singleton/live-matrix broadcast type gap are filled. All697 selected checks pass, including30 original/decoded finite C-liveout cases and both feature builds. IR398/work156/ABI47.

The one prospective optimization15 probe measured1.03552x raw geomean speedup against accepted31, with one regression>5% retained and all execution counters identical. The exact compiler and driver were committed/frozen before BOTH formal sets.

| Set | Accepted31 x C | Round35 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 12.02065x | 0.96734x | 0.97068x |
| Holdout | 39.31570x | 40.93923x | 0.96381x | 0.97439x |

Both sets regress. No tolerance, retries or exclusions are applied. All900 observations,29482 finite C-liveouts,36 append-only history entries, failed source and raw training/formal evidence remain. Exact accepted31 compiler source and driver are restored, and no remote push is made. The below1.0x C holdout goal remains unmet. Subsequent profiling and source choices remain optimization-only.

Round36: native finite-result guards and checked matrix-region replay (not promoted)

General root matrix-expression regions use shared ordered scalar/SIMD kernels and per-result finite checks. A nonfinite private result releases its exact loan and replays unchanged source before any logical settlement. Canonical F64 ownership pricing and singleton/live-matrix typing are shared prerequisites. All700 broad checks and28 focused checks pass, including30 finite C-liveout programs and both feature builds. IR399/work156/ABI47.

The one prospective optimization15 probe measured1.07776x raw geomean speedup versus accepted31, with no regressions over5% and all execution counters identical. Exact source and driver were committed/frozen before BOTH formal sets.

| Set | Accepted31 x C | Round36 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 13.94767x | 0.76584x | 0.89660x |
| Holdout | 39.31570x | 41.57038x | 0.97499x | 0.99461x |

Both scores regress. All900 observations,29482 finite liveouts,37 append-only history entries, failed source and raw evidence remain. Accepted31 compiler source and driver are restored. No push is made. The below1.0x C holdout goal remains unmet. Profiling now covers all15 accepted optimization programs, prioritized by Mech/C; no holdout per-case diagnosis was used. Investigate the training-probe/formal sampling difference and the largest optimization-set native loop next.

Round37: carried rectangular comprehension coordinates (not promoted)

The shared Cranelift map emitter carries outer-column and inner-row coordinates for multi-row rectangular comprehensions, preserving source column-then-row order, exact checked scalar arithmetic, current loans, private output mapping and source bills. Standalone maps, phase maps and nested reductions share this lowering. Empty/one-row cases retain existing paths. No source-port changes, benchmark-name checks, FP reassociation or FMA. All685 broad checks,109 focused checks and both feature checks pass, including all30 finite C-liveout programs. IR400/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process samples per actor, interleaving accepted31 and candidate. It measures1.006893x raw geomean speedup and1.135240x on Gram-Schmidt, retaining both>5% regressions (BiCG and covariance) and all210 observations. Every execution counter matches. The actual hot Gram-Schmidt function loses3 high-multiplies,3 shifts and7 multiply-subtracts from coordinate reconstruction; other14 training programs have unchanged generated code. Source/driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round37 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 12.10729x | 0.98270x | 0.99723x |
| Holdout | 39.31570x | 39.45198x | 1.01649x | 1.03755x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,38 append-only history entries, all regressions, failed development sources and raw evidence remain. No holdout per-case tuning was used. The below1.0x C holdout goal remains unmet.

[Qualification](round-37-qualification.json) · [Frozen source](round-37-frozen.json) · [Acceptance](round-37-performance-acceptance.json) · [Optimization-only profiles](training-experiments/optimization-margin-profile.json)

Round38: versioned readonly map bounds (not promoted)

The shared Cranelift map emitter computes pure current range/capture bounds predicates once, then selects a bounds-proved or unchanged checked body inside the same native loop. False hypotheses never cause earlier source replay or evaluate inactive reads. Source evaluation order, lazy bounds/error checks, native admission, output banks, arithmetic and exact accounting remain. Standalone maps, phase maps and nested reductions share the pass. No source-port changes, benchmark-name checks, FP reassociation or FMA. All686 broad checks,110 focused checks and both feature checks pass, including all30 finite C-liveout programs. IR403/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process samples per actor, interleaving accepted31 and candidate. Raw geomean speedup is1.011909x; Gram-Schmidt is1.070034x; all210 observations and0 regressions>5% remain. Visits match and source work/comparison bills do not decrease. Actual checked Object inspection preserves both native bodies and demonstrates an available bounds-proved CFG path with fewer per-read comparisons. Static path counts do not establish input feasibility or measured performance. Source/driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round38 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 11.86032x | 0.98606x | 0.99917x |
| Holdout | 39.31570x | 39.86498x | 0.97935x | 0.99873x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,39 append-only history entries, all regressions, failed development sources and raw evidence remain. No holdout per-case tuning was used. The below1.0x C holdout goal remains unmet.

[Qualification](round-38-qualification.json) · [Frozen source](round-38-frozen.json) · [Acceptance](round-38-performance-acceptance.json) · [Optimization-only profiles](training-experiments/optimization-margin-profile.json)

Round39: canonical numeric composite construction (not promoted)

The compiler derives a numeric tuple/record strategy from the actual resident CompositePack factory. It reuses Core CompositeSnapshotConstructor and the complete original-source construction/publication/replay envelope. Current admitted child schemas, keys and shapes determine the result; no input shapes or owners are cached. All finite caps, caller coexistence, aliases, atomic publication, exact visits and work/comparison charges remain. Existing private-frame and scalar routes retain priority. No source-port changes or benchmark-name checks. All686 broad checks,19 focused checks and both feature checks pass, including all30 finite C-liveout programs. IR404/work156/ABI47.

The one prospective v2 optimization15 probe uses seven fresh-process samples per actor in alternating accepted31/candidate order. Raw geomean speedup is1.150103x; MVT is2.699389x and BiCG is2.528068x. All210 observations and2 regressions>5% remain. Visits match and source work/comparison bills do not decrease. Before timing, a debugger census on the exact qualified driver confirms one numeric composite call, zero generic canonical calls and unchanged native arithmetic callbacks in both MVT/BiCG; all160 C liveouts pass. The census has zero timing samples. Source and driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round39 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 10.88235x | 1.12822x | 1.12780x |
| Holdout | 39.31570x | 41.26878x | 0.98834x | 1.00595x |

The strict two-set gate rejects this run; exact accepted31 source and driver are restored. All900 formal observations,29482 finite liveouts,40 append-only history entries, all regressions and development failures remain. Holdout use is aggregate acceptance only. The below1.0x C holdout goal remains unmet.

[Qualification](round-39-qualification.json) · [Frozen source](round-39-frozen.json) · [Acceptance](round-39-performance-acceptance.json) · [Optimization profiles](training-experiments/optimization-margin-profile.json) · [Constructor profile](training-experiments/composite-pack-profile.json)

Round40: native map range facts and private copies (not promoted)

The compiler recovers each source-literal range fact independently, checks current frame metadata, then retains known start/cardinality and the checked total element count in native SSA. Dynamic ends preserve dynamic counts while known starts remain constants. The complete map still succeeds in private scratch before a byte-preserving packed copy initializes the existing result bank, with exact tails. No source-port change, benchmark-name checks, FP arithmetic changes, new frame fields, weaker admission or source work reduction. All686 broad checks,122 focused checks and both feature configurations pass, including all30 finite C-liveout programs. IR405/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process observations per actor in alternating accepted31/candidate order. Raw geomean speedup is1.015342x; Gram-Schmidt is1.225011x. All210 observations and3 regressions>5% remain. Visits match and source work/comparison charges do not decrease. Before timing, actual checked optimization Objects show packed private copies and source-literal large cell bounds retained in the profiled native function, without new FMA. Static loop counts do not establish runtime cost or feasibility. Source and driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round40 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 12.17672x | 0.97907x | 0.99370x |
| Holdout | 39.31570x | 39.59330x | 1.00206x | 1.01801x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,41 append-only history entries, all regressions and development stages remain. No holdout per-case diagnosis, profiling or tuning. The below1.0x C holdout goal remains unmet.

[Qualification](round-40-qualification.json) · [Frozen source](round-40-frozen.json) · [Acceptance](round-40-performance-acceptance.json) · [Optimization-only native loop profile](training-experiments/native-map-memory-profile.json)

Round41: native SSA map composition (not promoted)

Native-to-native calls pass current range geometry, typed views and captures directly to the maintained checked map DAG and return ordered folds in SSA. The host/runtime Frame adapter retains complete current-loan validation. Whole-map replacements still finish in the same private scratch bank before the same non-failing scalar copy to the exclusive result bank. No source-port change, benchmark-name checks, FP arithmetic change, runtime ABI change, weaker admission, ownership relaxation or source work reduction. All685 broad checks,121 focused checks and both feature configurations pass, including all30 finite C-liveout programs. IR406/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process observations per actor in alternating accepted31/candidate order. Raw geomean speedup is1.009134x; Gram-Schmidt is1.011145x. All210 observations and3 regressions>5% remain. Visits match and source work/comparison charges do not decrease. Before timing, actual checked optimization Objects satisfy the preregistered requirement for fewer native instructions, a smaller stack and fewer stack memory operations in the profiled function, without new FMA. Static counts do not establish runtime cost or feasibility. Source and driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round41 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 12.03883x | 0.98107x | 0.98068x |
| Holdout | 39.31570x | 40.63008x | 0.98303x | 1.00066x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,42 append-only history entries, all regressions and development stages remain. No holdout per-case diagnosis, profiling or tuning. The fixed below1.0x C holdout goal is assessed from the accepted scores, with a separate completion audit required before claiming success.

[Qualification](round-41-qualification.json) · [Frozen source](round-41-frozen.json) · [Acceptance](round-41-performance-acceptance.json) · [Optimization-only native loop profile](training-experiments/native-map-memory-profile.json)

Round42: native total private maps (not promoted)

Source CFG totality, literal guarded range/capture facts, checked fixed geometry and pointwise target dependence authorize direct writes only into fresh private banks. All non-target views have disjoint complete spans. Pure same-cell identity tails retain the existing bits only when the current view is already that private bank. Every uncertain or fallible map retains complete scratch evaluation and copy. No source-port change, benchmark-name checks, FP arithmetic change, runtime ABI change, weaker admission, ownership relaxation or reduced source bills. All687 broad checks,123 focused checks and both feature configurations pass, including all30 finite C-liveout programs. IR407/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process observations per actor in alternating accepted31/candidate order. Raw geomean speedup is1.016381x; Gram-Schmidt is1.099101x. All210 observations and1 regressions>5% remain. Visits match and source work/comparison charges do not decrease. Before timing, actual checked optimization Objects must satisfy the preregistered removal of all three hot large whole-map copy loops and smaller combined/function stacks, without new FMA. Static counts do not establish runtime cost or feasibility. Source and driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round42 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 11.89518x | 0.99485x | 0.99301x |
| Holdout | 39.31570x | 40.45042x | 0.98173x | 0.99411x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,43 append-only history entries, all regressions and development stages remain. No holdout per-case diagnosis, profiling or tuning. The fixed below1.0x C holdout goal is assessed from the accepted scores, with a separate completion audit required before claiming success.

[Qualification](round-42-qualification.json) · [Frozen source](round-42-frozen.json) · [Acceptance](round-42-performance-acceptance.json) · [Optimization-only native loop profile](training-experiments/native-map-memory-profile.json)

Round43: native guarded map domains (not promoted)

Source CFG totality and current geometry authorize private bounds witnesses that suppress unreachable per-read axis guards. Pure row-invariant integer/Boolean control prefixes authorize retaining the rest of an identity column only in an already initialized current private bank. Inner-, memory- and FP-dependent prefixes retain normal iteration. Full direct-bank preflight, alias separation, fallible-map scratch evaluation, FP order/bits and all original source charges remain. No source-port change, benchmark-name checks, FP arithmetic change, runtime ABI change, weaker admission, ownership relaxation or reduced source bills. All689 broad checks,125 focused checks and both feature configurations pass, including all30 finite C-liveout programs. IR408/work156/ABI47.

The single prospective v2 optimization15 probe uses seven fresh-process observations per actor in alternating accepted31/candidate order. Raw geomean speedup is1.012463x; Gram-Schmidt is1.341187x. All210 observations and3 regressions>5% remain. Visits match and source work/comparison charges do not decrease. Before timing, actual checked optimization Objects must satisfy the preregistered removal of all three hot large whole-map copy loops, a smaller stack and fewer conditional branches than round42, without new FMA. Static counts do not establish runtime cost or feasibility. Source and driver were committed/frozen before both formal sets.

| Set | Accepted31 x C | Round43 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 12.37882x | 0.98841x | 0.98821x |
| Holdout | 39.31570x | 40.55975x | 0.97931x | 0.99185x |

The strict two-set gate rejects this run; exact accepted31 compiler source and driver are restored. All900 formal observations,29482 finite liveouts,44 append-only history entries, all regressions and development stages remain. No holdout per-case diagnosis, profiling or tuning. The fixed below1.0x C holdout goal is assessed from the accepted scores, with a separate completion audit required before claiming success.

[Qualification](round-43-qualification.json) · [Frozen source](round-43-frozen.json) · [Acceptance](round-43-performance-acceptance.json) · [Optimization-only native loop profile](training-experiments/native-map-memory-profile.json)

Round44: prepared shared composite binding (accepted)

Core prepares immutable flat tuple/record schema relations once. Each binding validates fresh current input shapes in their own parameter arenas, checks constants/repeated axes, validates the full output shape and retains exact child identities. Resident CompositePack and the compiler-selected canonical numeric route reuse the ordinary constructor with this binder. Cached normalization costs preserve the full original source work, comparison, clone, publication and finite/coexistence/replay envelopes. Unsupported or denied attempts retain the ordinary path. No source-port, FP arithmetic, ABI, ownership or accounting relaxation. All703 broad checks,30 focused checks and both feature configurations pass, including all30 finite C-liveout programs. IR409/work156/ABI47. Every development attempt remains.

The single prospective v2 optimization15 probe retains all210 fresh-process observations. Raw geomean speedup is1.190997x; MVT3.404506x, BiCG3.080285x. All1 regressions>5% remain; visits match and source work/comparison bills do not decrease. Untimed route census confirms one numeric pack callback, zero generic callbacks and unchanged arithmetic callback counts in each profiled case, checking all160 finite C liveouts. Source/driver were committed and frozen before both formal sets.

| Set | Accepted31 x C | Round44 x C | Raw speedup vs31 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 11.43636x | 10.24618x | 1.13838x | 1.16126x |
| Holdout | 39.31570x | 38.46048x | 0.98713x | 1.00858x |

Both primary scores strictly improve; round44 is accepted. All900 formal observations,29482 finite liveouts,45 append-only history entries and all regressions remain. Holdout is the aggregate gate only; no per-case diagnosis, profiling or tuning. Below1.0x C holdout remains the unchanged objective, requiring a separate completion audit.

[Qualification](round-44-qualification.json) · [Frozen source](round-44-frozen.json) · [Acceptance](round-44-performance-acceptance.json) · [Optimization-only constructor profile](training-experiments/composite-pack-profile.json)

Accepted44 optimization-only margin profiles
--------------------------------------------

All15 optimization programs were freshly sampled with the exact accepted44
compiler and driver, in descending Mech/C margin order. Every process exited
successfully; all finite C liveouts and accepted44 execution reports match.
Diagnostic samples and Object export add no performance observations or history
entries. Gram-Schmidt has90.83% generated-numeric-function self samples.
Correlation has substantial helper/collection/transpose/publication overhead.
Its emitted Object has no rectangular map because the inner range ends at a
current immutable input, while the current rectangle proof accepts only literal
inner bounds. Next work is the general current-bound rectangular-map gap, with
unchanged source factories, full finite admission/replay and original fallback.
No holdout per-case diagnosis or profiling. All101 raw evidence archives have
verified compressed and uncompressed hashes.

[Profile evidence](training-experiments/accepted44-margin-profile.json)

Current-bound rectangular maps: qualified, not performance-selected
-----------------------------------------------------------------

The compiler now derives current immutable U64 inner-range endpoints before an
enclosing rectangular map, transports the exact original scalar operands,
certifies current cardinality and shapes with the original selected factories,
and reuses the existing Cranelift scalar body and row-major private map bank.
Fixed scratch phases explicitly decline current bounds. Typed IR410/work156/
ABI47. All709 broad checks and91 focused checks pass, including all30 finite
C-liveout cases, both features, fresh endpoints and all scalar bits, empty and
invalid ranges, finite denial/replay, loan/proof tampering, cleanup and retry.

The untimed actual correlation census replaces56 inner-map callbacks with2
rectangle prepare/finish callbacks and removes28 inner transpose callbacks.
All784 finite C liveouts match. Physical visits fall392 to28, work99926852068
to36175107508, comparison work5082102222 to1496518178. Other execution counters
match. Therefore the precommitted v2 counter guard fails BEFORE any timing
probe. No performance sample or formal history entry was added. Accepted44
remains the performance baseline (10.24618x optimization /38.46048x holdout).

A proposed v3 gate permits only independently source-proved fusion with actual
callback/counter evidence and full source admission/replay/finite guarantees.
It is not adopted. The fixed ports, split, primary metric, all timing details
and strict two-set acceptance gate stay unchanged. All development evidence,
including two debugger selector failures before process launch, is preserved.

[Qualified source/evidence](training-experiments/current-bound-rectangles-development.json)
[Selection gate proposal](training-experiments/current-bound-rectangles-selection-proposal.json)

Rectangular source fees and complete restored-corpus qualification
----------------------------------------------------------------

The initial untimed rectangle checkpoint had a real shared-comparison funding
defect. A regression spent the genuine invocation account at the rectangular
header: the ordinary path rejected, but the native rectangle completed. That
checkpoint was never benchmarked or performance-accepted. The initial proposal
to allow lower fee counters is superseded.

The corrected compiler shares the ordinary transpose funding formulas. Every
elided row retains its original compute, clone, comparison, temporary/output/
node and replay prices. The current shared comparison ledger must fund both
the native attempt and ordinary replay; the source fee is paid before factory
metadata/shape work. Work policy157 records this correction; typed IR410 and
helper ABI47 remain. No visits are fabricated or padded.

The actual untimed correlation census now reports:

                         accepted44         corrected candidate
work                     99926852068         107839595060
comparison work           5082102222           5082102222
physical visits                 392                  28
inner transpose callbacks        28                   0

All784 finite C outputs match. Original billing guards pass; only the v2
identical-visit guard fails. The revised proposed gate permits independently
source-proved physical-visit reductions while retaining nondecreasing source
work/comparison fees and every fixed benchmark/acceptance requirement. It is
NOT adopted and no candidate performance sample has been collected.

All712 broad checks and102 focused checks pass, including both features, exact
compression work totals and all30 original/decoded C integration cases. The
coverage audit also added restored-helper checks for all30 original/decoded
forms:60 forms and58964 finite C liveouts, preserved aliases and zero retained
bytes/nodes after cleanup. The prior restored-helper C tests covered selected
programs; the full-corpus gap is now closed. All149 audit archives verify both
compressed and raw hashes, including every failure and superseded checkpoint.
Accepted44 remains10.24618x C optimization /38.46048x C holdout; history45.

[Corrected qualification/audit](training-experiments/current-bound-rectangles-shared-comparison.json)
[Proposed physical-visit guard](training-experiments/current-bound-rectangles-selection-proposal.json)

Prospective training guard v3
-----------------------------

The identical-visit condition was an assistant-added v2 guard, not part of the
fixed user benchmark protocol. Asking for separate permission to revise that
condition unnecessarily blocked the authorized general compiler optimization
work. V3 is adopted before any candidate timing, with source-rederived private
fusion proofs and actual callback evidence required for physical-visit
reductions. Original work/comparison fees must remain nondecreasing and every
other execution/transport counter must match. No fee exception is allowed.
All ports, inputs, C flags/binaries, finite limits, tolerances, fixed15/15 split,
cache/timing boundaries, one prospective210-observation probe,900-observation
formal evaluation and strict improvement of BOTH scores over accepted44 stay
fixed. No prior result is reclassified and no failed fingerprint is retried.
The earlier approval question is not a prerequisite; silence is not treated as
approval. Complete the running accepted44 profiles before any census or trial.

[Prospective v3 policy](training-experiments/current-bound-rectangles-training-policy-v3.json)

Round45: current-bound rectangles with complete source fees (accepted)

Immutable current inner bounds are certified by the genuine original range/transpose factories; the existing Cranelift body fills one row-major private bank. Shared ordinary transpose funding retains all original row fees on the same comparison account, with full finite/physical/replay/current-owner/atomicity checks. Fixed phases decline dynamic cardinality. All712 broad/102 focused checks and both features pass; restored helpers additionally validate60 original/decoded forms and58964 finite C liveouts. IR410/work157/ABI47.

The single prospective v3 optimization probe retains all210 fresh-process observations. Raw GM speedup versus accepted44 is1.033376x; correlation1.872073x. All1 regressions>5% remain. The untimed all15 gate proves the only physical-visit reduction, preserves all source work/comparison bills and matches every other counter. The exact source/driver were committed/pushed/frozen before both formal sets.

| Set | Accepted44 x C | Round45 x C | Raw speedup vs44 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 10.24618x | 9.78419x | 1.05001x | 1.02774x |
| Holdout | 38.46048x | 25.86145x | 1.52216x | 1.49809x |

Both primary scores strictly improve; round45 is accepted. All900 observations,29482 finite liveouts,46 append-only history entries and every regression remain. Holdout is the aggregate gate only. Below1.0x C holdout is unchanged and requires a separate completion audit.

[Qualification](round-45-qualification.json) · [Frozen source](round-45-frozen.json) · [Acceptance](round-45-performance-acceptance.json) · [Source-fee audit](training-experiments/current-bound-rectangles-shared-comparison.json)

Round46: reuse existing validated native type slots (not promoted)

Numeric input validation and output retirement reuse the exact type keys already bound to their source value/transport indexes. Complete current schema ID/key, shape, account, finite/cold/replay, ownership and atomicity checks remain. No new retained cache, wire field, allocation or instruction change. All 713 broad/12 focused checks pass, including key/schema/index substitution negatives, 60 original/decoded restored forms and 58,964 finite C liveouts. Both features pass. IR411/work157/ABI47.

One prospective probe retained all 210 fresh-process observations: raw GM speedup versus accepted45 1.071221x; all 0 regressions over5% remain. The untimed all15 gate verifies exact counters and native instructions. Selected source/driver were committed/pushed/frozen before both formal sets.

| Set | Accepted45 x C | Round46 x C | Raw speedup vs45 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 9.78419x | 11.13864x | 0.82799x | 0.95082x |
| Holdout | 25.86145x | 24.97576x | 1.05150x | 1.06661x |

The strict two-set gate rejects this run; exact accepted45 source and driver are restored. All 900 observations, 29,482 finite liveouts, 47 append-only history entries and every regression remain. Holdout is the aggregate gate only; the below1.0x C goal requires a separate completion audit.

[Qualification](round-46-qualification.json) · [Freeze](round-46-frozen.json) · [Acceptance](round-46-performance-acceptance.json) · [Development](training-experiments/prepared-type-slots-development.json)

Fresh accepted45 Gram-Schmidt profile: 3829 canonical execution samples; 3436 generated-function self samples (89.74%). Same-process JIT capture verifies all 17,212 bytes / 4,303 decoded instructions and binds sampled offsets to actual executed code. C liveouts and accepted45 counters match; diagnostic times are excluded from scores. All 24 raw/analysis archives verify. No holdout diagnosis; formal history remains47 and accepted scores remain9.78419x /25.86145x C. See training-experiments/accepted45-gramschmidt-profile.json.

Round47: source-proved conditional matrix maps (not promoted)

The compiler independently derives row/column identity complements from actual source CFGs and evaluates selected coordinates through the maintained checked Cranelift map body. Complete private scratch starts from the exact immutable identity source. Publication occurs only after all checked actions succeed; selected stores require the destination to equal that base bank. Prior completed maps, aliases, exact scalar bits, finite source fees and replay boundaries remain checked. No benchmark-specific rule or source-port rewrite. IR412/work157/ABI47.

All 720 broad/eight focused groups pass, including all30 original/decoded/restored C cases (60 restored forms and 58,964 finite liveouts), both features and every scalar width. All nine development attempts remain archived. The untimed optimization15 gate matches all 17,002 C liveouts and every execution counter; actual Objects show precisely three masks in the profiled Gram numeric_5, with no new FMA.

One prospective probe retained all 210 fresh-process observations: raw GM speedup versus accepted45 1.049149x; all 0 regressions over5% remain. Selected source and driver were committed/pushed/frozen before both formal sets.

| Set | Accepted45 x C | Round47 x C | Raw speedup vs45 | Reference-adjusted speedup |
|---|---:|---:|---:|---:|
| Optimization | 9.78419x | 9.57594x | 1.05427x | 1.05211x |
| Holdout | 25.86145x | 27.50652x | 0.90496x | 0.93400x |

The strict two-set gate rejects this run; exact accepted45 source and driver are restored. All 900 observations, 29,482 finite liveouts, 48 append-only history entries and every regression remain. Holdout is the aggregate acceptance gate only; the below1.0x C goal requires a separate completion audit.

[Qualification](round-47-qualification.json) · [Freeze](round-47-frozen.json) · [Acceptance](round-47-performance-acceptance.json) · [Development](training-experiments/masked-matrix-maps-development.json)

User-directed selection correction after round47

Round47 is accepted from its optimization improvement: 9.784192694066512x C ->9.575943659594117x C (1.0542683121826288x raw Mech speedup). Holdout is reported independently: 25.86144806723818x C ->27.506517683644883x C. Its regression does not select, veto, retune or roll back compiler changes. Exact qualified47 source/driver are restored without new observations. The earlier two-set decision remains in the immutable round47 receipts and archived prior global ledger; an explicit amendment changes promotion from history47 onward. Historical decisions through46, all900 observations, all29,482 finite C outputs, the48-entry append-only measurement history, fixed benchmark protocol and below1.0x C holdout goal remain.

[Optimization acceptance](round-47-optimization-acceptance.json) · [Policy amendment](performance-selection-policy-v4.json)

Initial transpose/product-view probe (not selected)

All728 broad/eight focused checks, both features, all30 C cases and the untimed optimization15 fee/callback/Object gate passed. The single210-observation probe measured0.9957168176603722x raw GM speedup versus accepted47, with four regressions over5%. All four physically transformed optimization programs improved, but the complete optimization aggregate did not meet the preregistered selection rule. All observations remain; the fingerprint is ineligible for an exact retry. No holdout or formal timing ran. Exact accepted47 compiler is restored; measurement history remains48 entries and accepted scores remain9.575943659594117x optimization /27.506517683644883x holdout.

[Unselected observations](training-experiments/transpose-product-views-unselected-probe.json) · [Qualification/restoration](training-experiments/transpose-product-views-unselected.json)

Hoisted product geometry and original loan storage (not selected)

All728 broad/eight focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 fee/callback/Object gate passed. Actual MVT Object innermost dot-product loops have zero integer divisions, versus two static division sites in the initial transpose-view candidate. Original Pending fields match accepted47; no floating-point order or timing-boundary change. These static reductions do not establish a runtime gain.

The single210-observation optimization-only probe measured0.9883820880980985x raw GM speedup versus accepted47, with 2 regressions over5%. The complete optimization aggregate fails the prospective selection rule; all observations remain and the fingerprint is ineligible for an exact retry. No holdout or formal timing ran. Exact accepted47 source is restored; history remains48 entries and accepted scores remain9.575943659594117x optimization /27.506517683644883x holdout (reported independently).

[Unselected observations](training-experiments/hoisted-transpose-product-views-unselected-probe.json) · [Qualification/restoration](training-experiments/hoisted-transpose-product-views-unselected.json) · [All30 source line counts](training-experiments/source-line-counts.json)

Accepted48: ordered product epilogues

General source-closed ordered F64 product epilogues remove real product/scaling/addition intermediate buffers and passes while retaining source arithmetic order, resident geometry and resource obligations. Actual fused Objects and honest visit reductions were verified in 2mm/GEMM/MVT; all729 broad/eight focused checks and both features passed.

The single210-observation optimization probe selected at1.0093501643084763x raw GM speedup. One frozen900-observation formal evaluation matched29482 finite C liveouts and appended history49 without altering earlier entries. The v4 optimization-only gate accepts optimization9.401799375716326x C; holdout24.666190246754944x C is independently reported. Raw optimization Mech runtime was essentially flat (0.9985868065558229x speedup); measured C runtime rose1.0199638482497508x, contributing to the ratio gain. Holdout raw Mech speedup is1.1407256700285937x. No source tuning or rollback uses holdout. The below1.0x C holdout goal remains unmet.

[Qualification](round-48-qualification.json) · [Performance audit](round-48-audit.json) · [Progress](round-48-progress.json) · [Accepted evidence](training-experiments/ordered-product-epilogues-accepted.json) · [Static telemetry](round-48-static-telemetry.json)

Accepted48 auxiliary optimization telemetry

The instrumented external host allocator passed known-allocation/reallocation/live-peak calibration. Eight untimed checked calls for every optimization program matched frozen counters and153018 finite cell comparisons, with managed cleanup to baseline. Requested-heap counts were identical across all8 calls per case. Peak additional live requested heap ranges7072..242656B; Deriche1094 requests/451163B requested per call, correlation608/116052B, MVT114/24133B, Gram–Schmidt90/67101B. OS process peaks include compiler/catalog/input/diagnostic storage and are kept separate from checked-call requested heap.

Uninstrumented warm-residence CPU samples resolve actual owned PID JIT maps; fresh Objects for all5 sampled optimization programs exactly match the frozen accepted48 Objects. Generated self time is87.4514% Gram–Schmidt,53.6851% Durbin,32.8063% MVT,30.0745% correlation and63.3362% Deriche. These samples do not reproduce cold-cache benchmark timing and do not enter selection or performance history. Hardware retired-instruction/cache counters were unavailable. No holdout diagnosis was performed.

[Full15-program telemetry](round-48-runtime-telemetry.json) · [CPU sampling](round-48-cpu-telemetry.json) · [Reproducible probe sources/raw evidence](training-experiments/accepted48-telemetry.json)

Selected scratch initialization: formal evaluation

General guarded same-bank selected publication skips redundant complete scratch initialization; different-bank complete publication retains it. Partial scratch grants no full-output completion certificate. All734 broad/13 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history50 without altering earlier entries. Formal optimization9.4153094154070995x C, independent holdout25.3710300143922964x C. V4 optimization-only decision: not_promoted. Raw runtime and C/reference drift controls remain in round-49-audit.json. All observations and regressions are preserved, no retries/exclusions. Exact accepted48 compiler restored solely because optimization failed its strict formal score gate.

[Qualification](round-49-qualification.json) · [Audit](round-49-audit.json) · [Progress](round-49-progress.json) · [Complete experiment](training-experiments/selected-scratch-initialization-formal.json)

Compound conditional domains: formal evaluation

General source-proved compound conditional domains skip only total identity buckets, retaining source order and unknown/fallible branches. Same-bank selected publication skips redundant scratch initialization; different-bank complete publication retains it. Partial scratch grants no full-output completion certificate. All740 broad/19 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history51 without altering earlier entries. Formal optimization9.2435020206874565x C, independent holdout25.1484878192208328x C. V4 optimization-only decision: accepted. Raw runtime and C/reference drift controls remain in round-50-audit.json. All observations and regressions are preserved, no retries/exclusions. Compiler promoted to accepted50.

[Qualification](round-50-qualification.json) · [Audit](round-50-audit.json) · [Progress](round-50-progress.json) · [Complete experiment](training-experiments/compound-conditional-domains-formal.json)

Accepted50 optimization-only live-code profile

Current accepted50 Gram-Schmidt/Durbin/MVT were sampled in owned warm checked-call processes and their named executable JIT ranges captured from those same PIDs after sampling. Every sampled instruction/window matches its genuine Object; both numeric functions and the complete MVT function are byte-identical. Small root call-relocation differences are isolated. All warmup/final C values and frozen execution counters match. One diagnostic async-wait failure and its successful same-process exit/code/sample were preserved; only remaining cases ran after its fix. No official timing samples or history entries were added.

Generated self-time is82.92% Gram-Schmidt,50.83% Durbin,33.86% MVT. Residence includes output reset/drop and is not the exact kernel timer. The verified MVT transpose reconstructs coordinates with two integer divisions per copied cell; the next proposed general experiment replaces that reconstruction with induction, retaining traversal/admission/buffers/fees/ownership/publication. No runtime win is inferred from static operations.

[CPU telemetry](round-50-cpu-telemetry.json) · [Full profile/code evidence](training-experiments/accepted50-optimization-profile.json)

Native transpose induction (not selected)

All743 broad/13 focused checks, both features, all30 C cases and restored images, and the untimed optimization15 C/source-fee/unchanged-visit/Object gate passed. Carried coordinates remove per-cell hardware division from the original native transpose copy loop. Every cell is still copied in the same order with the same buffers, checks and source fees. Static removal does not establish a runtime gain.

The single210-observation optimization-only probe measured0.9928689728233402x raw GM speedup versus accepted50, with 1 regressions over5%. It fails the prospective selection rule. All observations remain; no exact fingerprint retry and no holdout/formal timing ran. Exact accepted50 source is restored. Accepted scores remain9.243502020687457x optimization /25.148487819220833x holdout, independently reported.

[Unselected observations](training-experiments/native-transpose-induction-unselected-probe.json) · [Qualification/restoration](training-experiments/native-transpose-induction-unselected.json)

Accepted50 correlation CPU profile

Correlation, the next highest optimization margin beyond the three already sampled, shows31.40% generated,17.78% allocation/free and15.66% planning/schema/validation self time in a warm diagnostic loop. All13820 native bytes match the same sampled PID and accepted Object. The dominant0x2ce0 site (13.58% of samples) is the two-column ordered product k loop. This motivates grouping independent output rows to share right loads/control and expose independent accumulators, also for one-column products. Host preparation and materialized elementwise-to-sum intermediates remain separate opportunities. No official performance samples/history entries were added; no holdout profiling.

[CPU evidence](round-50-correlation-cpu-telemetry.json) · [Full profile and code evidence](training-experiments/accepted50-correlation-profile.json)

Ordered product row pairs: formal evaluation

General ordered products pair independent output rows, sharing right loads and k control while preserving each cell +0/k arithmetic, source fees, current owners, bounds, full buffers, existing epilogues and atomic completion. All744 broad/19 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history52 without altering earlier entries. Formal optimization9.0969086510528872x C, independent holdout24.6009906555209206x C. V4 optimization-only decision: accepted. Raw runtime and C/reference drift controls remain in round-51-audit.json. All observations and regressions are preserved, no retries/exclusions. Compiler promoted to accepted51.

[Qualification](round-51-qualification.json) · [Audit](round-51-audit.json) · [Progress](round-51-progress.json) · [Complete experiment](training-experiments/ordered-product-row-pairs-formal.json)

Immutable map read reuse (not selected)

All748 broad/4 focused checks, both features, all30 C cases and restored images, and the untimed optimization15 C/source-fee/unchanged-visit/Object gate passed. Identical later immutable matrix reads reuse a successful checked word under source dominance and stable selectors. Full buffers, original failures, checks and source fees remain; static removal does not establish a runtime gain.

The single210-observation optimization-only probe measured0.9906874989625698x raw GM speedup versus accepted51, with 2 regressions over5%. It fails the prospective selection rule. All observations remain; no exact fingerprint retry and no holdout/formal timing ran. Exact accepted51 source is restored. Accepted scores remain9.096908651052887x optimization /24.60099065552092x holdout, independently reported.

[Unselected observations](training-experiments/immutable-map-read-reuse-unselected-probe.json) · [Qualification/restoration](training-experiments/immutable-map-read-reuse-unselected.json)

Ordered reduction producers: formal evaluation

General source-closed pointwise DAGs feed original ordered sum terms without producer materialization. Original errors, arithmetic, source fees, current owners, bounds, full buffers and atomic completion remain. All752 broad/8 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history53 without altering earlier entries. Formal optimization8.9645700557118566x C, independent holdout24.5203097801984526x C. V4 optimization-only decision: accepted. Raw runtime and C/reference drift controls remain in round-52-audit.json. All observations and regressions are preserved, no retries/exclusions. Compiler promoted to accepted52.

[Qualification](round-52-qualification.json) · [Audit](round-52-audit.json) · [Progress](round-52-progress.json) · [Complete experiment](training-experiments/ordered-reduction-producers-formal.json)

Accepted52 correlation CPU profile

One owned warm checked-call process was sampled and its entire14620-byte JIT range captured from that same PID after sampling. All bytes match the accepted52 Object; warmup/final finite C outputs and frozen counters match. Generated self-time28.53%, allocation/free19.29%, planning/schema/validation16.62%. Complete PC attribution places11.91% in the two-row product k region,1.13% in the new producer/sum region and5.10% in an ordinary broadcast loop. No official timing sample or history entry was added.

The next proposed source-derived experiment shares RHS loads/k control across four independent ordered output rows, retaining all existing guards/tails/fees/ownership/publication and exact per-cell arithmetic order. These profiles are warm diagnostics and no runtime/generalization gain is inferred.

[CPU telemetry](round-52-correlation-cpu-telemetry.json) · [Full same-process profile/code evidence](training-experiments/accepted52-correlation-profile.json)

Four independent product rows: formal evaluation

General F64 products group four independent output rows around one RHS load/k loop, with original per-cell arithmetic order and2/1-row tails. Original errors, arithmetic, source fees, current owners, bounds, full buffers and atomic completion remain. All756 broad/4 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history53 without altering earlier entries. Formal optimization8.8663442023512840x C, independent holdout24.8165403467171153x C. V4 optimization-only decision: accepted. Raw runtime and C/reference drift controls remain in round-53-audit.json. All observations and regressions are preserved, no retries/exclusions. Compiler promoted to accepted53.

[Qualification](round-53-qualification.json) · [Audit](round-53-audit.json) · [Progress](round-53-progress.json) · [Complete experiment](training-experiments/ordered-product-four-rows-formal.json)

Accepted53 phase profiles

Owned Gram-Schmidt/Durbin warm checked-call samples attribute83.45%/53.16% self time to generated code. Both numeric functions match their genuine Objects byte-for-byte; complete caller code matches except expected BL relocations independently bound to captured same-PID targets. C liveouts and frozen counters match. Hot ordered sum loops round-trip their accumulator between integer and FP registers every term. The next proposed general pass keeps proved F64 reduction accumulators typed in SSA while retaining raw-word ABI boundaries and every source operation/error/fee. Rejected41 still used an I64 fold; this is a different physical computation. No benchmark timing observation/history entry was added or speedup inferred.

[CPU telemetry](round-53-phase-cpu-telemetry.json) · [Profile and complete code binding](training-experiments/accepted53-phase-profile.json)

Typed ordered map folds: formal evaluation

Source-proved ordered F64 map sums remain in F64 SSA until the original raw-word frame boundary, including standalone, phase and nested folds. Original errors, arithmetic, source fees, current owners, bounds, full buffers and atomic completion remain. All758 broad/4 focused checks, both features, all30 original/decoded/restored C cases and the untimed optimization15 exact-fee/counter/Object gate passed.

One210-observation optimization-only probe selected; one frozen900-observation formal run matched29482 finite C liveouts and appended history53 without altering earlier entries. Formal optimization8.3658618933018118x C, independent holdout24.5993830037455226x C. V4 optimization-only decision: accepted. Raw runtime and C/reference drift controls remain in round-54-audit.json. All observations and regressions are preserved, no retries/exclusions. Compiler promoted to accepted53.

[Qualification](round-54-qualification.json) · [Audit](round-54-audit.json) · [Progress](round-54-progress.json) · [Complete experiment](training-experiments/typed-map-folds-formal.json)
