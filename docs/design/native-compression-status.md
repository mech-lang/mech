# Native compression: current status

Maintained code/tests and compact hashes stay in source. Raw observations,
profiles, failed attempts and replay scripts are in the [external evidence
index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R87

Complete pinned level-12 compression of `abcdefgh` improves **9.807058× →
9.638890× Roc**, or **1.714769% less elapsed**, directly measured against R85.
The target remains 1×; larger-input throughput is unestablished.

Prior published head: `fc3887ddced43a84a0e58776064a108d9dbc483e`.
R87 base: `f72db5372a6179d48855ca3c80825af111b042bc`; production/actual optimized
source: **`c2b334feed4326a66ce1f69ca4bc99b0cbe41bb2`**.
Separate CI coverage commit: `74a7c4936`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft. Hosted CI is
cancelled by instruction, without a passing-CI or merge-readiness claim.

| Metric | R85 | R87 |
|---|---:|---:|
| Matched elapsed |2.853854 ms|**2.804917 ms**|
| Elapsed / fresh 0.291 ms reference |9.807058×|**9.638890×**|
| Instructions, separate 32-call cohort |31,054,859.5|**30,534,424.5 (−1.675857%)**|
| Cycles, same separate cohort |9,045,530|**8,849,412 (−2.168121%)**|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,788|
| Incremental peak live allocator bytes |20,473,824|20,473,824|
| Generated text bytes |14,920,564|identical|
| Entry stack bytes |133,984|identical|

Four prepared pairs contain 3,968 primary/3,984 total observations. All four pairs
improve; 60 blocks improve/four are adverse. Blocks are descriptive, not independent
experiments; no significance claim. Do not compare fresh reference-clock ratios
as a source regression or pool/multiply independent cohorts. Separate reference
hardware medians are 1,364,372 instructions/932,702.5 cycles. Hardware, heap, CPU
samples and diagnostics remain separate. Counter-instrumented elapsed is excluded
from primary timing. Whole-executable RSS/physical-footprint medians are slightly
adverse (+16,384/+114,688 bytes); no memory improvement is claimed.

## What changed

Native-piece entry already checks every current source's schema, account and
admitted capacity. It now retains their largest capacity (including the existing
scalar bound). Maximum capacity times output-field count is a conservative
sufficient bound. When it fits the current byte cap, repeated field queries are
unnecessary. A too-large or overflowing product retains the previous per-field
bound and exact-footprint fallback. An overly conservative bound never rejects a
valid tight cap. No owner changes between source checks and the output preview.

Current source availability, output arity, temporary/fuel admission, error order,
original conservative charges and final publication remain dynamic. This is no
new permission cache, executor, unsafe, allocation site, allocator, ABI, limit or
pricing rule. NoLimits still sets every limit to its representation maximum.

Four isolated calls agree: **2,159** successful coarse proofs bypass **8,116**
field visits, with no per-field/exact fallback on this pinned call. All **2,596**
current-source checks remain. Instrumented observations are excluded from
performance acceptance. Existing original/decoded/restored tight-cap and finite
failure/retry tests pass through the retained fallback. Two new tests establish
that heterogeneous/repeated/empty fields cannot gain false permission; overflow,
maxima and the minimum footprint decline safely. CI inventories and executes both
new tests, with its equality guard updated from 45 to 47.

R79 multi-child composition, R80 exclusive first admission, R81/R82 owner aliases,
R83 current-admission proofs and R85 bulk zeroing remain retained. R87 reuses their
exact Object/image/metadata (original emission `4fe3ca57b`), with current
independently rederived image proofs. IR/work/private ABI/MBC **135/96/38/14**;
frozen scalar ABI1. Work **45,853,669,270**/comparisons **23,004** are unchanged.
No output golden, finite allowance, warning or architecture policy changes.

## What the instruction evidence says

R86's rotating samples cover all 10,712 reached callback positions four times
across 256 diagnostic calls. No-op calibration is 5,027 instructions. Estimated
group totals include ordinary library calls 7.21M, source-step admission 3.56M,
and native-piece entry/exit 5.28M. Library projections, record packs and indexed
replacements each account for about 2M. Source-step admission covers ordinary
operations, not assertions specifically. These sums of calibrated medians across
calls are estimates, not one exact instruction ledger.

A separate 256-call diagnostic covers all 810 entry/810 exit boundaries four
times, with 5,047-instruction internal-marker calibration. Estimated output
byte-cap preview is 704K instructions (R87's target), reached retirements 645K,
stale-output/retirement availability 544K, and current-source checks 428K. Small
corrected observations remain unclamped. All diagnostic calls verify output,
inflation, aliases, capacity and zero final managed memory. R86 changes no
production code. Its three reports replay from independently retrieved packets.

Fresh R87 serial profiles contain 695 calls per subject and 1,470/1,346 exclusive
sample weights. Entry-body weights are 203/181; begin_numeric_segment 59/37.
Frame-action ancestry is 212/219, an overlapping union whose share increases.
Samples are not retired-instruction counts or exact timer fractions. There is no
fresh reference CPU sample in R87; fresh reference timing/counters are separate.

Next inspect remaining projection/record-pack and retirement boundaries. Use the
measured current operations to define a minimal acceptance witness before further
compiler expansion. The weighted SSA inventory is 22,032 words, not a machine-store
count. Do not forecast large gains from spill loops alone. Ordinary debit tuning
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
allocation histograms and reallocations162 are unchanged. Requested capacity is
not copied/initialized bytes or live memory. No fresh output per repetition or
zero-filled path matrix is substituted.

Fresh passing selections overlap: runtime **341**; work/memory/values33; native CI
inventory+execution **47**; image1; components236 with unchanged goldens/allowances;
connected8; path1; linked/AOT original/decoded16; scalar12 (maximum **1.016350483**
against unchanged1.05, equal pure-scalar allocations/zero loop slope); frame8;
workflow Python132; architecture/type/memory15. Runtime-only GPU cfg compiles
warnings-as-errors against the unchanged coherent feature-rich dependencies.
Core/Budget/Engine integration remains inherited; no dependency/math rebuild is
claimed. Unsafe allowlist and architecture/catalog checks remain unchanged.

An isolated diagnostic generator initially matched both outer/nested guards with
an unanchored selector, before any build or measurement. The anchored correction,
failed driver/staged copy and successful observations are preserved. No production
change was required. No required local failure remains. Hosted CI, giant complete
knucleotide, large/full-corpus timing and a fresh minimal/no_std graph are skipped.
Historical adverse/rejected rounds remain archived. Source keeps one status,
compact index and maintained harness; no history rewrite.

Independent retrieval verifies R86's five packets/829 file versions and R87's
20 packets/1,638 file versions. Their three/four numerical reports reproduce
byte-identically without compiling or compressing.
