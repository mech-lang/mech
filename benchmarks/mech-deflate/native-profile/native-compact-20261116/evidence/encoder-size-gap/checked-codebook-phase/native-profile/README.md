# Current native construction profile

Three diagnostic stack samples follow the current 63,804 B source recipe from
encoding through process exit, so they include native construction, emission and
teardown. The compiler is frozen at `4e1d5afe…`; every run reproduces the exact
C5 encoder object, complete metadata and canonical bytecode.

The collector runs while the test compiler is active. These samples are not
latency measurements and their fractions are not exclusive phase timings. Idle
and wait stacks are separated. The classifier attributes each remaining stack
sample once, prioritizing validation/graphs/SSA/backend activity inside broader
native callers. Emission-entry frames can serve phases as well as the root.

Of 2,758 non-wait samples, 23.0% fall under cleanup rewrites, 20.7% validation,
12.0% guard construction, 9.2% SSA construction, 7.0% CFG/dominance and 7.5%
machine emission/register allocation. See `summary.json` for all categories,
per-trial counts and native owners. These identify work to price; they do not
justify removing validation or assigning those percentages of build latency.

`raw-native-samples.tar.gz` contains the raw reports, sampler/compiler logs and
as-run tools. A preliminary one-second window omitted the end of native emission;
this packet uses the corrected complete interval and makes no claim from the
truncated samples.
