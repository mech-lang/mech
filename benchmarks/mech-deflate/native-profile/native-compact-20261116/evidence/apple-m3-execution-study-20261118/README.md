# DEFLATE execution on Apple M1 and M3

The unchanged Roc executable gains more than the unchanged Mech executable
from the M1/macOS 15.6.1 environment to the M3/macOS 26.2 environment. A matched
counter diagnostic locates this difference in CPU execution cost. It does not
identify a specific cache, branch-prediction or pipeline mechanism, and does not
separate hardware changes from operating-system changes.

The eight completed M3 optimization screens below did not establish a replacement
for the qualified 71,192-byte encoder. No experimental compiler change is promoted
by this study. The complete M3 qualification remains the
[previous reproduction](../apple-m3-reproduction-20261117/README.md): 71/81 and
69/81 against Roc, and 57/81 and 58/81 against libdeflate 1.26 level 12, for the
source-rebuilt worker in forward and reverse order respectively. These are
different campaigns from this counter diagnostic.

## Matched binary counters

Both machines execute the exact same archived Mech worker (`ddefe57e371b…`) and
Roc worker (`ed1ab9175cd1…`), with the same input and expected output. The encoder
object is `96696eeed083…`. Each actor has two warmup commands followed by twelve
alternating, approximately 200 ms counter blocks. Only one actor does CPU work
at a time. M3 acquisition finishes before M1 acquisition begins.

For the natural-text 65,536-byte input:

| Whole-loop diagnostic, per compression | Mech M1 | Mech M3 | Roc M1 | Roc M3 |
| --- | ---: | ---: | ---: | ---: |
| Elapsed seconds | 0.0105496 | 0.00794057 | 0.0108550 | 0.00767040 |
| Time / contemporaneous Roc time | 0.972× | 1.035× | 1.000× | 1.000× |
| Retired instructions | 124.12 M | 124.24 M | 116.19 M | 115.97 M |
| CPU cycles | 33.80 M | 32.20 M | 34.78 M | 31.11 M |
| Cycles per instruction | 0.2723 | 0.2592 | 0.2993 | 0.2682 |
| Effective clock, GHz | 3.204 | 4.056 | 3.204 | 4.056 |

Mech's cycle count falls **4.72%**, while Roc's falls **10.56%**. Instruction
counts change by about 0.10% and -0.19%, respectively. Both receive effectively
the same 1.266× clock increase. Their observed loop speedups are 1.329× and
1.415×. Thus a recompile or an algorithm doing less work does not explain Roc's
extra gain in this diagnostic. The aggregate counter evidence supports a larger
execution-efficiency improvement for Roc's existing code.

The counter interval includes the worker's entire warmup command: compression,
independent inflation/output validation, loop overhead and pipe handling. It is
**not** the narrower native-call interval used by the published qualification,
and these diagnostic times must not replace qualification timings or win counts.
The twelve terminal native-call checks are additional correctness observations,
not a timing cohort. CPU counters come from `proc_pid_rusage` version 4; CPU time
uses the machine's recorded Mach timebase. The diagnostic records the complete
before/after values and per-block call counts.

[All diagnostic values](counter-comparison.csv) ·
[Plans, identities and interpretation](analysis.json) ·
[Diagnostic driver](cross-machine-diagnostic.py)

## Compression phases and emitted code

Five-second stack captures put nearly all of the 64 KiB compression time in
two phases: match search and optimal-path scoring. The corresponding symbols are:

| Phase | Mech | Roc |
| --- | --- | --- |
| Match search | `mech_compact_numeric_475_vector` | `roc__p82cb10df89b60108bc0589036ee8d0c3.fast` |
| Optimal-path scoring | `mech_compact_numeric_3009_no_growth_composed` | `roc__p251d42c9b11261f4b6c38feff48bfd2d.fast` |

The 64 KiB captures suggest match search benefits less in Mech. For example,
Mech's hot child-selection sequence at matcher offsets 1696/2440 contains a
stack store/reload of traversal state and separate index scaling/address
calculations. Roc's corresponding sequence at offsets 3912/4352 carries that
state in registers and uses scaled halfword loads/stores. These are concrete
code-generation differences and optimization targets. Their responsibility for
the cross-machine cycle delta has **not** been established by a controlled
ablation. The stack captures do not measure branch misses, cache misses or stall
events. Fixed 1 ms sampling can also alias short periodic loops; sampled phase
fractions are not exact phase timers.

The 1 KiB random-input Mech capture exposes another limit: buffer clearing and
mapping/unmapping take a substantial share of its short call. Its final M1
capture/closure has possible overlap with a new compiler process and Xprotect
activity visible in the final process snapshot. That small-case capture is
retained but is not used to assign the physical cause of relative scaling.

## M3 optimization screens

Each complete screen compares a source-built experimental worker with the
source-built qualified worker and the unchanged Roc/libdeflate peers. The normal
screen has 15 selected cases, including the weak 1 KiB cases and natural text
from 4 KiB through 64 KiB. The first screen has 27 cases. Each order uses 96
balanced actor permutations and the unchanged qualification checker.

| Mechanism | Evidence directory | Disposition |
| --- | --- | --- |
| NEON horizontal minimum and reverse-lane simplification | `neon-reductions` | No required recovery established |
| Bottom-tested vector loop with NEON simplifications | `latch-hoist-neon` | Mixed small-case changes; no retained gain |
| Bottom-tested vector loop alone | `latch-only` | No retained gain |
| Earlier vector entry at four candidates | `vector-4` | Regresses natural-text cases |
| Earlier vector entry at eight candidates | `vector-8` | Regresses natural-text cases |
| Earlier vector entry at twelve candidates | `vector-12` | No retained gain |
| Map smaller zero-filled halfword arrays | `halfword-mapping` | Large 1 KiB regressions |
| Reuse current unsigned comparisons in native phases | `phase-words-profiled` | Fewer comparisons, larger matcher, no retained gain |

All **11,842,560 timed calls** in the eight complete screens pass raw-observation
replay, including output identity, independent inflation, aliases, cleanup, actor
identity and order balance. The full per-case seconds and paired multiples of
Roc, libdeflate and Mech control are in [screen-timings.csv](screen-timings.csv).
[Screen summaries](screens.json) · [Replay result](raw-replay.json)

The broader comparison-reuse version increases peak stack to 5,296 bytes,
exceeding the predeclared 5,280-byte ceiling, and is rejected independently of
timing. The phase-only version preserves the ceiling and passes its linked
failure probes. A separate unit harness checks 224 unsigned-comparison cases.

A deferred-winner experiment evaluates the pure item expression once after an
ordered minimum, instead of at every candidate. It passes 162 complete output
checks, checked allocation failures and the 5,280-byte stack bound. Independent
generated controls have not yet selected that compact reduction, so no oracle
pass is claimed. A partial timing acquisition was explicitly excluded after a
separate M1 timing campaign was observed. There is no accepted runtime result
for this experiment. Its patch is retained as research, outside the qualified
compiler. Disabling it reproduces the exact qualified object and locally built
worker, with all 162 checks passing.
[Experiment status](deferred-item-status.json) ·
[Patch](compiler-wave5-current.patch) · [Disabled build](disabled-build.json)

## Evidence and reproduction

The archive listed in [evidence.json](evidence.json) contains the raw observations,
plans, all unsuccessful trials, build receipts, source patches, linked proof
workers, stack captures and counter records. Three redundant compiler-helper
executables and Python bytecode caches are omitted; their sources and build
commands are included. Their failed validation attempts are retained and labeled.
The frozen peer executables, canonical inputs and source-rebuilt reference
are available in the preceding M3 reproduction archive.

After extracting this study and that foundation archive, `replay-experiments.py`
rechecks each completed raw-data campaign using the unchanged qualification
verification code and canonical fixture contracts. Acquisition paths in the
frozen plans are provenance; rerunning acquisition needs relocated paths, fresh
output directories and the shared campaign lock. Hashes, raw data and excluded
attempts must be kept when reproducing or extending the study.
