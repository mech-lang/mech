# Native compression: current status

This is the current-status owner. [Experiment index and retrieval](../../benchmarks/mech-deflate/EXPERIMENTS.md)
link immutable external packets; source retains maintained code, tests and compact hashes.

## Current measured checkpoint: R80

Retain exclusive empty first admission. Complete pinned **eight-byte, level-12**
compression improves **11.514359× → 11.464732× Roc**, **0.430999% less elapsed time**.
The preceding R79 multiple-child composition improvement is retained (9.794019%
less matched elapsed and10.234005% fewer instructions in its own cohort).
Larger-input throughput and parity remain unestablished.

Round base: `d68e5f0d97c7950b11aa2a798f805f9a56cc0096`.
Production and actual optimized source: `e4d444b5fcd33a69c05837ad1cafbd4c50b9ae97`.
New regression witness: `134c0132d`; test reuse/CI inventory: `e8b671df3`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft; hosted CI is
cancelled by instruction and is not counted as passing.

| Metric | Fresh reference | Matched R79 | R80 |
|---|---:|---:|---:|
| Primary elapsed |0.280000ms|3.2240205ms /11.514359×|**3.2101250ms /11.464732×**|
| Retired instructions, separate cohort |1,364,372|36,055,951.5|**35,879,460.5 (−0.489492%)**|
| CPU cycles, separate cohort |918,008|10,261,671.5|**10,192,981.5 (−0.669384%)**|
| Allocation requests including reallocations |95|6,791|**6,451 (−340)**|
| Requested bytes |9,021,923|98,113,216|**98,091,456 (−21,760)**|
| Incremental allocator peak usable bytes |9,257,328|20,482,648|**20,481,304 (−1,344)**|
| Generated text bytes |—|14,660,740|14,660,740|
| Generated entry stack bytes |—|124,048|124,048|

Four independently prepared pairs contain3,968 timed/3,984 total observations.
All pairs and48/64 descriptive blocks improve;16 adverse blocks remain. No
statistical-significance claim; blocks are not independent experiments. Hardware,
heap and CPU samples are separate cohorts. Today’s0.280ms reference differs from
R79’s0.289ms; comparing the old11.207469× directly with11.464732× would mix clocks.

The complete symbolized allocation census reconciles all requests. Exactly340
64-byte platform-lock allocations under ordinary snapshot first admission vanish;
all other request-size/kind counts are unchanged. Reallocations stay162. Large
requests remain five tree, one cache and35 path allocations. Requested capacity
is not initialized/copied bytes. Post-call RSS319.52→319.55MB includes retained
compiler/catalog/image storage and is not an isolated compression peak.

## Architectural changes retained

R80 proves wrapper **and** data exclusivity with `Arc::get_mut`, excluding strong
aliases and weak observers, and checks the current registry is empty. It uses the
existing exclusive accessor, then reacquires actual exclusivity before publication.
The same node-before-byte admission logic serves both paths. Registration
construction, footprint, schema/shape, current funding, error order and recovery
remain authoritative. Shared/weak/registered/foreign cases retain the ordinary
locked recheck. No unsafe code, cached authority, new executor or ledger.

R79 inventories multiple ordinary child boundaries and proves acyclic native
pieces between them using existing frame, provenance, liveness, admission and
numeric proofs. Child initialize/exit, mutation/publication/recovery stay ordinary;
current owners/views are reacquired for each piece. One usize marker checks exact
nested ancestry. No loan or prepaid fuel survives child execution.

R79’s diagnostic calls count17,689→14,460 callbacks and500→1,255 executed pieces;
static emitted pieces67→566. Managed projections remain2,176: the next structural
target. R79 grew generated text11,753,624→14,660,740 and entry stack107,280→124,048;
that tradeoff remains explicit. R80 Object/descriptor/metadata are byte-identical.
TypedIR/work/privateABI/MBC stay**133/95/38/14**, frozen scalarABI**1**. Fresh R80
compute**67,457,686,430** and comparisons**23,063** are unchanged. No prices or
allowances increase.

Fresh CPU samples contain1,307 baseline/1,321 candidate/1,518 reference exclusive
weights. Registration ancestry33→31 and contrary frame ancestry211→240 are retained.
Groups overlap; these are samples, not per-function instructions or exact timer
fractions. Hardware counters provide the separate whole-call instruction result.
The retained Mech worker is173.04MB, including compiler/catalog/image code, versus
the283KB reference harness; these are not equivalent standalone artifact sizes.
Raw Mach-O section reports and source/binary pins are externally retained.

## Computation, qualification and remaining scope

Mech performs compression as one compiled computation through the maintained
[25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs).
Pinned measured source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithms/tables, allocation sites, tree262144U16/cache1500513U32/path305257U64,
depth300/nice258/near-EOF checks, input aliases, pinned allocator, output capacity13
clear/rebind and evaluate_into-only timer remain unchanged. Search/skip stay
separate, with no cache/recording path in skip. Raw bytes are
`[75,76,74,78,73,77,75,207,0,0]`; independent inflation agrees and final managed
bytes/nodes are zero. Physical public-boundary copies remain.

All twelve Core-dependent project libraries were rebuilt coherently with retained
selected-operation release features, not the entire math catalogue. Fresh passing
selections overlap: Core373; ownership inventory/execution11; GPU runtime334;
work/memory/values32; component236 with unchanged expectations; connected8; path1;
native CI selection40; linked original/decoded16; scalar12 (max1.021116707 versus
unchanged1.05, equal pure-scalar allocation and zero loop slope); frame8; workflow
Python132; architecture/type/memory15. Current runtime-only GPU cfg compiles with
warnings-as-errors against the coherent feature-rich dependencies. Allowlist and
warning policy remain unchanged.

The initial baseline witness fails with two shared gates instead of zero; all
current tests pass. The export runner’s obsolete output-path error was resolved
before timing; all original artifact hashes were verified unchanged. No required
local failure remains. Hosted CI, giant complete knucleotide unit, large/full-corpus
compression and fresh minimal/no_std graph are skipped. Separate unchanged
Budget/planner/Engine integration receipts remain inherited. Historical large-input
and scalar results do not qualify this revision or erase adverse evidence.

Historical evidence through R78 and all current raw packets are external, with
verified manifests and numerical replay. No Git history rewrite. Next: managed
projections within native pieces, then precise live-state synchronization as
separate measured changes. Ordinary debit-function tuning stays closed without
new mechanism evidence. A separately named source variant may be tried later.
