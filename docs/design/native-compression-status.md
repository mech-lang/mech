# Native compression: current status

Maintained code/tests and compact hashes stay in source. Raw observations, failed
experiments, profiles and replay scripts are in the [external evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R81

Complete unchanged **eight-byte, level-12** compression improves
**11.077177× → 10.141253× Roc**, or **8.449124% less elapsed time**. The target
remains 1×; larger-input throughput and parity are unestablished.

Round base: `54badcdaf0a2c4094fa2127c69e25fb8bd1260ad`.
Production: `2f9bf24de`; actual optimized source:
`c29854f6f1f404658c39bcb024296b01ec80a71d`.
Failing witness: `f03293515`; current-result recipe contract: `d2c85a6af`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft. Hosted CI is
cancelled by instruction, without a passing-CI or merge-readiness claim.

| Metric | Fresh reference | Matched R80 | R81 |
|---|---:|---:|---:|
| Primary elapsed |0.291000 ms|3.2234585 ms /11.077177×|**2.9511045 ms /10.141253×**|
| Instructions, initial separate cohort |1,364,372|35,886,029.5|**31,824,002 (−11.319245%)**|
| Cycles, initial separate cohort |931,415|10,200,563.5|**11,112,392 (adverse)**|
| Allocation requests incl. reallocations |95|6,451|**6,399**|
| Requested bytes |9,021,923|98,091,456|**98,074,788**|
| Incremental allocator peak usable bytes |9,257,328|20,481,304|**20,473,824**|
| Generated text bytes |—|14,660,740|14,874,444|
| Generated entry stack bytes |—|124,048|133,504|

Four prepared pairs contain 3,968 primary /3,984 total observations. All pairs and
all 64 descriptive blocks improve. No significance claim; blocks are not
independent experiments. Reference clocks change between rounds, so use this
round's matched multiples. Separate counters, sampling and heap cohorts remain
separate. The initial adverse cycle cohort stays visible; two predeclared
opposite-order hardware pairs improve cycles by 7.67% and 8.18% and reproduce
roughly 11.3% fewer instructions. The cause of the first cycle result is unresolved.

## What changed

Managed unsigned-array projections now remain immutable source aliases within
native parent pieces. Existing provenance, availability, liveness and numeric
proofs resolve current owners and exact exits. Views are acquired afresh for each
piece and end before ordinary child execution. Real escapes retain ordinary
canonical installation; internal projections need no temporary arena value.
A pure current-slot/type/account/generation preview funds possible puts; it stores
no authority. Existing `arena.put`, frame synchronization and retirement remain
responsible for publication and cleanup.

Four isolated per-call censuses agree. Host callbacks fall **14,460 → 10,759**;
ordinary managed projections **2,176 → 499**; canonical arena puts **3,039 → 2,178**.
The 1,677 moved projections reconcile as **861 omitted intermediate puts +816
required alias exports**. Native pieces merge from 566 to 379; actual entries fall
from 1,255 to 806. These are execution counts, distinct from 534 static selected
projection sites. R79 multiple-child composition and R80 exclusive first admission
remain retained. No new executor, ledger or loan across child execution.

Removing canonical pins also permits the existing checked update path to reuse
more backing: replacement copies fall **226 → 174**, requested backing bytes
**2,205,788 → 2,189,120**. This accounts for all 52 fewer requests and 16,668 fewer
requested bytes. Other size/kind counts are unchanged. Large histograms still
contain five tree, one cache and 35 path requests; reallocations remain 162.
Requested capacity is not initialized/copied volume.

Typed IR /work policy /private ABI /MBC are **134 /96 /38 /14**; frozen scalar ABI
remains **1**. Explicit policy 96 prices private alias handling and real escapes.
Actual compute changes **67,457,686,430 → 45,563,430,475**; comparisons **23,063 →
23,066**. These are resource units, not hardware instructions. The whole-Huffman
pricing golden changes explicitly from policy 95's 91,452,918 to policy 96's
88,767,602. Finite allowances and literal compression outputs are unchanged.

Fresh CPU samples contain 1,308 baseline /1,304 candidate /1,516 reference exclusive
weights. Frame-action ancestry falls 227→190; generated-entry exclusive samples
154→185, and contrary thread-local ancestry 27→40 remains. Categories overlap;
these are not per-function instruction counts or exact timer fractions. Code and
stack growth remain measured tradeoffs, alongside the instruction/time reduction.

## Fidelity and qualification

The [25-component compressor assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
runs as one compiled computation. Measured source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithms, tables, allocation sites, tree 262144 U16/cache 1500513 U32/path 305257 U64,
depth 300/nice 258/near-EOF checks, input aliases, pinned allocator, output capacity 13
clear/rebind and evaluate_into-only timer remain unchanged. Search/skip stay
separate; skip has no cache argument or recording path. Raw output is
`[75,76,74,78,73,77,75,207,0,0]`; independent inflation agrees and final managed
bytes/nodes are zero. Physical public-boundary copies remain.

Only the native backend was rebuilt against the unchanged coherent R80 graph.
Fresh passing selections overlap: runtime 335; work/memory/values 33; native CI 41;
component 236 with unchanged literal expectations; connected 8; path 1; linked
original/decoded 16; scalar 12 (maximum 1.046644805 versus unchanged 1.05, equal pure
scalar allocations and zero loop slope); frame 8; workflow Python 132; architecture/
type/memory 15. Runtime-only GPU cfg compiles warnings-as-errors against pinned
feature-rich dependencies. Warning policy and unsafe allowlist stay unchanged.
Unchanged Core/Budget/planner/Engine integration receipts remain inherited.

The baseline witness rejects ordinary projection boundaries. Initial policy and
recipe assertions are updated explicitly for private alias representation; raw
failures remain archived. A busy-process profiling preflight ran no sample; a
later clean run succeeded. An accounting build hit actual OS disk exhaustion;
removing 463 obsolete executables recovered about 49 GB of free space, preserving
current inputs/libraries/artifacts and all source/evidence, then the same build
passed. No required local failure remains. Hosted CI, giant complete knucleotide,
large/full-corpus compression and a fresh minimal/no_std graph are skipped.

External packet manifests preserve every version; numerical replay verifies the
current reports without compiling or compressing. No Git history rewrite.
Next: broaden immutable owner transport for nonnumeric managed fields while
keeping numeric-view authorization separate, then investigate precise live-word
synchronization. Ordinary debit tuning remains closed without new mechanism
evidence. A source variant does not replace the pinned baseline.
