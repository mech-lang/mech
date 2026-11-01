# Native compression: current status

Maintained code/tests and compact hashes stay in source. Raw observations,
profiles, failed attempts and replay scripts are in the [external evidence
index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R85

Complete pinned level-12 compression of `abcdefgh` improves **10.090325× →
9.980769× Roc**, or **1.085752% less elapsed**, directly measured against retained
R83. The target remains 1×; larger-input throughput is unestablished.

Prior published checkpoint: `ff4cf633c153d559a7a3f5288b34f656f94a1cfe`;
actual R83 source: `c0b5999f434650df9ff7fef6dc2c9f4c40d4fc42`.
R85 base: `24f1019c83c1776ff64a4fb4ef288d644f696b2e`; actual optimized/production
source: **`490ede99581fd233edd4c05d14f74641c9ae45c9`**.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft. Hosted CI is
cancelled by instruction, without a passing-CI or merge-readiness claim.

| Metric | Retained R83 | R85 |
|---|---:|---:|
| Direct elapsed |2.885833 ms|2.854500 ms|
| Direct elapsed / fresh 0.286 ms reference |10.090325×|**9.980769×**|
| Instructions, separate 32-call cohort |31,053,282.5|31,053,485 (+202.5; effectively flat)|
| Cycles, same separate cohort |9,156,399.5|**8,947,483.5 (−2.281639%)**|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,788|
| Incremental peak live allocator bytes |20,473,824|20,473,824|
| Generated text bytes |14,920,564|identical|
| Entry stack bytes |133,984|identical|

Four prepared pairs contain 3,968 primary/3,984 total observations. All pairs
improve; 54 blocks improve and 10 are adverse. Blocks are descriptive, not
independent experiments. No significance claim. A separate predeclared R84→R85
comparison improves1.236842% (10.018517×→9.894604×, fresh 0.288 ms reference),
all four pairs improve andone block is adverse. Never pool/multiply independent
cohorts or compare reference-clock ratios as a source regression. This is an
elapsed/cycle improvement, not a retired-instruction reduction. Hardware, heap,
CPU sampling and diagnostics remain separate. Requested capacity is not copied
volume; reference allocation count 95 is observed, not a semantic minimum.

## What changed

`CollectedValues::fill_unsigned` keeps its empty/exact-capacity and kind/word
checks in the same order. U8 is unchanged. U16/U32/U64 use a distinct
`resize_with(count, || 0)` initializer only for zero; nonzero keeps ordinary
`resize(count, value)`. Actual ARM64 shows all three zero helpers call `bzero`.
There is no new unsafe, allocator, allocation site, executor, ledger, ABI or limit.
The earlier `resize(count, 0)` attempt (R84/10f5bbbe9) was outlined into shared
runtime-word vector loops. Its mixed 0.168704% elapsed decrease comes with 2,056.5
extra instructions and 28 adverse blocks; it is an attributed first attempt,
not the independently retained endpoint. All raw observations remain available.

Four isolated calls agree:101 uniform constructors. All84 wider calls are zero:
one U16 call initializes 262,144 elements/524,288 bytes; 63 U32 calls initialize
1,510,232 elements/6,040,928 bytes; 20 U64 calls initialize1,456 elements/11,648
bytes. **6,576,864 actual initialized wider bytes** use bulk zeroing, including
the real tree/cache. U8's17 calls/16 zero calls and 2,223 zero bytes are unchanged.
These are initialized bytes, not reserved capacity or copied volume. Fresh samples
actually show `bzero` beneath U16/U32 `resize_with`/fill; no per-function instruction
count is inferred from sample weights.

R79 multi-child composition, R80 exclusive first admission, R81/R82 owner aliases
and R83 current-admission byte-cap proofs remain retained. R85 reuses their exact
Object/image/metadata (original emission 4fe3ca57b), with current independently
rederived image proofs. IR/work/private ABI/MBC **135/96/38/14**; frozen scalar ABI1.
Work 45,853,669,270 and comparisons 23,004 are unchanged. NoLimits remains all
representation maxima. No pricing rule, source output golden or allowance changes.

## What the new instruction evidence says

An isolated R84 GPU copy records eight fixed markers per complete call. Seven
adjacent windows are disjoint; each raw interval includes approximately 5,029
instructions of calibrated marker cost. Eight repetitions agree on boundaries.
Raw medians: whole call 31.09M; preparation 1.13M; executing native turn plus
callbacks 29.43M (~94.65%); publication/cleanup 0.49M. Small activation/import windows
are dominated by probe cost. These are instruction windows, not elapsed fractions
or a per-function ledger; individual-window medians need not sum to median total.
All instrumentation is excluded from primary/hardware/heap acceptance.

R86 now supplies the finer diagnostic: rotating samples cover all 10,712 reached
callback positions four times across 256 calls. No-op calibration is 5,027
instructions. Estimated group totals include ordinary library calls 7.21M,
source-step admission 3.56M, and native-piece entry/exit 5.28M. Library projections,
record packs and indexed replacements each account for about 2M. Source-step
admission covers ordinary operations, not assertions specifically. These sums of
calibrated medians across calls are estimates, not one exact instruction ledger.

A separate 256-call cohort covers all 810 entry/810 exit boundaries four times,
with 5,047-instruction internal-marker calibration. Output byte-cap preview costs
an estimated 704K instructions, reached retirements 645K, stale-output/retirement
availability 544K, and current source checks 428K. Instrumented observations are
excluded from performance acceptance. All calls retain output, inflation, alias,
capacity and zero-final-managed-memory checks. R86 changes no production code or
performance result; its reports replay from external packets.

Next: a conservative maximum-current-source-capacity times field-count proof to
avoid repeated byte-cap field walks when that bound fits; retain existing checks
when it does not. There are 810 reached pieces and 22,032 weighted SSA words, but
the latter is not a machine-store count. Measure the boundary work before
forecasting a large SSA synchronization gain. A large constant-admission project
is not justified solely by allocation count. Ordinary debit primitive tuning
stays closed without new mechanism evidence.

## Fidelity and qualification

The [25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithm/tables/source allocation sites/depth 300/nice 258/near-EOF checks,
tree 262144 U16/cache 1500513 U32/path 305257 U64, input aliases, pinned allocator,
capacity 13 clear/rebind and evaluate_into-only timer remain unchanged. Search and
skip stay separate; skip has no cache/recording path. Raw bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree; final managed
bytes/nodes 0. Physical public-output backing still changes. Heap totals, large
allocation histograms and reallocations 162 are unchanged. No zero-filled path
matrix or fresh output per repetition is substituted.

Fresh passing selections overlap: runtime 339; work/memory/values 33; native CI
inventory+execution 45; image 1; components 236 with unchanged goldens/allowances;
connected 8; path 1; linked/AOT original/decoded16; scalar 12 (maximum 1.005873967
versus unchanged1.05, equal pure-scalar allocations/zero loop slope); frame 8;
workflow Python 132; architecture/type/memory 15. Runtime-only GPU cfg compiles
warnings-as-errors against the unchanged coherent feature-rich dependencies.
Core/Budget/Engine integration remains inherited. Warning policy, unsafe allowlist
and architecture/catalog checks are unchanged.

R84's first marker-driver generator assumed exact main formatting and was resumed
after correction without rebuilding its completed GPU. An optional profile guard
was duplicated; initial replay failed after three outputs, then the corrected
script reproduced all four. Original scripts/partial outputs remain preserved.
An addendum corrects inherited R83 timing-scope prose: its pinned C0 source/counters
prove current Some(MAX) caps; raw observations and timing boundaries never change.
No required local failure remains. Hosted CI, giant complete knucleotide,
large/full-corpus timing and a fresh minimal/no_std graph are skipped.

Raw evidence and numerical replay stay external. Source keeps this current
status, compact index and one maintained harness; no history rewrite.
