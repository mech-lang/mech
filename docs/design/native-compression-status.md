# Native compression: current status

Maintained code/tests and compact hashes stay in source. Raw observations,
profiles, failed attempts and replay scripts are in the [external evidence
index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R83

Complete pinned level-12 compression of `abcdefgh` improves **10.144898× →
9.969036× Roc**, or **1.733503% less elapsed time**, directly measured against
retained R81. The target remains 1×. Larger-input throughput is unestablished.

Published R81 checkpoint: `84e1375e53375c11209f9933852fac9ee98eb2b7`;
actual R81 optimized source: `c29854f6f1f404658c39bcb024296b01ec80a71d`.
R82 production: `4fe3ca57b17b4bcab90948f8357f8fd8fd9d11da`.
R83 base: `3831952d49a2f2d762f33e87dadc8f29349fc9d9`; final production and
actual optimized source: **`c0b5999f434650df9ff7fef6dc2c9f4c40d4fc42`**.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft. Hosted CI is
cancelled by instruction, without a passing-CI or merge-readiness claim.

| Metric | Retained R81 | Combined R82/R83 |
|---|---:|---:|
| Direct primary elapsed |2.9420205 ms|2.8910205 ms|
| Direct elapsed / fresh 0.290 ms reference |10.144898×|**9.969036×**|
| Instructions, separate 32-call cohort |31,824,928.5|**31,050,590.5 (−2.433118%)**|
| Cycles, same separate cohort |9,356,502.5|**9,219,308 (−1.466301%)**|
| Requests including reallocations |6,399|6,399|
| Requested bytes |98,074,788|98,074,788|
| Incremental peak live allocator bytes |20,473,824|20,473,824|
| Generated text bytes |14,874,444|14,920,564 (+46,120)|
| Entry stack bytes |133,504|133,984 (+480)|

Four prepared pairs contain 3,968 primary/3,984 total observations. All pairs
improve; 60 blocks improve and 4 are adverse. Blocks within a pair are descriptive,
not independent experiments. No significance claim. Hardware, heap and CPU
sampling remain separate; do not infer per-function instructions from samples or
copied bytes from requested capacity. Reference allocation count 95 is observed,
not a semantic minimum.

A separate predeclared R82→R83 comparison improves2.091294%
(10.009425×→9.800099×, fresh 0.294 ms reference); all 64 blocks improve. R82 alone
was 0.280294% adverse elapsed (10.327484×→10.356432×, fresh 0.285 ms reference),
with 0.320138% fewer instructions and unchanged heap totals. That contrary result
remains archived. R82 is retained with R83 for broader correct owner transport and
the independent instruction improvement; it is not claimed as an elapsed win.
Do not multiply these ratios or compare reference clocks as a source regression.

## What changed

R79 multi-child composition, R80 exclusive first admission and R81 unsigned-array
aliases remain retained. R82 separates immutable owner transport from numeric
view permission. Existing exact provenance, availability, liveness, retirement
and exit proofs now forward matrices of records through native parent pieces.
Every child remains an ordinary boundary with freshly acquired owners/views.
Library managed projections and composite spans remain ordinary. Numeric views
still require their unsigned factory proof. No new executor, ledger or loan
crossing a child.

Actual R81→R82 callbacks fall10,759→10,712; managed projections 499→478. Every
additional 21 projection escapes canonically: exports 816→837, arena puts 2,178
unchanged. No omitted canonical intermediate is claimed. Static pieces 379→384;
actual entries 806→810.

R83 proves current output byte caps using the conservative capacity already
recorded for each fully admitted immutable owner. Core derives it from exact
retained footprint plus ownership/schema headers and ties it to the current
account, shape and schema owner. Current schema keys stay checked. A sufficient
checked sum avoids exact traversal; refused, overflowing or tighter bounds take
the original exact walk and check. No permission is cached or limit changed.

Four isolated calls agree: **4,109 exact walks→0**, **2,159 byte caps proved**,
**2,596 current source key checks retained**. Other execution counts match R82.
A positive4,096-byte-cap witness lies below cached admission but above exact
footprint; original/decoded/restored execution still succeeds via exact fallback.

The initial untimed 5a00d5768 guard incorrectly assumed NoLimits used None. The
counter showed all 4,109 calls have a cap: NoLimits uses Some(u64::MAX). Corrected
c0b5999f4 retains that mode and uses the existing conservative witness instead.
All untimed commands/receipts remain separately preserved.

R83 reuses byte-identical R82 Object/image/metadata emitted by4fe3ca57b, with
current independently rederived image proofs and new runtime. IR/work/private
ABI/MBC **135/96/38/14**; frozen scalar ABI1. Pricing rule96 is unchanged. Actual
work 45,853,669,270/comparisons 23,004 match R82; R81 used 45,563,430,475/23,066
under its previous selection. These are resource units, not instructions.

## Fidelity and qualification

The [25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithms, tables, source allocation sites, depth 300/nice 258/near-EOF limits,
tree 262144 U16/cache 1500513 U32/path 305257 U64, input aliases, pinned allocator,
capacity 13 clear/rebind and evaluate_into-only timer remain unchanged. Search and
skip stay separate; skip has no cache or recording path. Raw bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree; final managed
bytes/nodes 0. Physical public-output backing still changes. Large allocation
histograms, overall requested bytes and incremental allocator peak are unchanged.

Fresh passing selections overlap: runtime 339; work/memory/values 33; native CI
inventory+execution 45; components 236 with unchanged goldens/allowances; connected 8;
path 1; linked/AOT original/decoded16; scalar 12 (maximum 1.014254266 versus
unchanged1.05, equal pure-scalar allocations/zero loop allocation slope); frame 8;
workflow Python 132; architecture/type/memory 15; image/factory 1. Runtime-only GPU
cfg compiles warnings-as-errors against pinned feature-rich dependencies. Core/
Budget/Engine integration receipts remain inherited; their sources/libraries are
unchanged. Warning policy, unsafe allowlist and architecture/catalog checks stay.

The minimal baseline test fails the intended forwarding-selection assertion.
The first fixture escaped a composite and lacked a private parent even on the
candidate; corrected source isolates the real record carrier. An initial retry
test compared against the first publication instead of the immediately prior one;
corrected tests pass. One evidence upload hit the restricted network context
after all linked cases passed; the exact packet was resumed and verified. Raw
failed attempts and adverse observations remain archived. No required local
failure remains. Hosted CI, giant complete knucleotide, large/full-corpus timing
and a fresh minimal/no_std dependency graph are skipped.

Fresh R83 CPU samples contain 1,469 baseline/1,309 candidate/1,529 reference
exclusive weights. Frame-action ancestry212/1,469→199/1,309 has a contrary share
increase 14.43%→15.20%; samples are not instruction or timer partitions. Uniform
unsigned fill remains visible 52/1,309 and already uses Vec::resize; inspect actual
emitted zero-fill behavior before changing it. Next, measure coarse instruction
windows for setup/execution/cleanup and inspect precise live-word synchronization.
Ordinary debit tuning remains closed without new mechanism evidence.

Raw evidence and numerical replay stay external. Source retains this current
status, compact index and one maintained harness. No Git history rewrite.
