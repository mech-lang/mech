# Native compression: current status

Raw observations, full stack censuses, diagnostic code and rejected experiments
remain outside the source tree. Source keeps this status, one maintained harness
and the [compact evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R95

The canonical shape-storage change removes **628 allocation requests** and
**69,825.5 retired instructions** from complete pinned level-12 compression of
eight bytes. Elapsed is effectively flat: **8.792392×→8.783390× Roc's time**,
a 0.102381% lower matched median with two faster and two slower pairs. This is a
verified allocation/instruction improvement, not an established elapsed speedup.
The target remains 1×; larger-input throughput is unestablished.

| Metric | R94, freshly measured | R95 |
|---|---:|---:|
| Elapsed |2.5234165ms|2.520833ms|
| Elapsed / fresh 0.287ms reference |8.792392×|8.783390×|
| Instructions, separate 32-call cohort |27,866,086.5|27,796,261 (−0.250575%)|
| Cycles, same separate cohort |7,839,994|7,832,119 (−0.100447%)|
| Requests including reallocations |5,781|5,153 (−10.863172%)|
| Requested allocator bytes |98,008,140|97,997,834 (−10,306B)|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Generated compressor text |15,641,476B|unchanged|
| Entry stack |143,424B|unchanged|

Four prepared pairs contain 3,968 primary/3,984 total observations. Two pairs and
33 descriptive blocks improve; two pairs and 31 blocks worsen. All contrary
measurements remain. No significance claim. Compare matched cohorts rather than
ratios from different rounds: the reference clock and baseline also vary.
Elapsed, hardware, heap, full stack tracing, small allocation controls and samples
are independent cohorts, never multiplied. Requested capacity is not copying or
live memory. All 162 reallocations and the large-allocation histogram stay equal.
Hardware-cohort RSS rises 32,768B; footprint changes −64B. Neither establishes a
memory-footprint win; managed allocator peak is unchanged.

Base/prior published head: `6c2db2f3a613cff6f010c5f234286fed5a2d0cc6`.
Measured/retained compiler: **`67a7c88e745092264a5a735933dd95cf48fc5446`**.
Baseline compiler: `c35698030a4e89f6d4548af1b17762ec737cd427`.
Separate CI: `64810afee6076fda397706bc68e573e08b7d1ed9`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) remains draft pending green
CI and clean review. Publication CI is cancelled by instruction, not counted as
passing. All earlier retained optimizations, including R94 exit-store narrowing,
R92 unread-constant ownership and the checked-read retry correction, remain.
The slower R93 suffix alternative remains reverted with all evidence archived.

## What the patch changes

The canonical dimension solver previously allocated lower-bound parameters,
copied those values into another buffer, collected borrowed dimension/extent
pairs into a vector, and allocated Boolean scratch for fixed parameters. It now
consumes the unpublished parameter buffer and uses a clonable borrowed iterator.
Small flag tapes use 16 stack slots; larger tapes use the same general heap
solver. This is a storage cutoff, not a dimension or program limit: 17- and
65-parameter cases execute and retain the general fallback.

There is one canonical dimension solver. Its lower-bound validation still
precedes arity/witness errors; uniqueness, shared parameters, min/max ambiguity,
products, overflow and final schema/body validation retain their original order.
The component collector and its structural validation order remain intact.
Only unpublished storage moves; existing shapes, clones and caller values remain
unchanged. There is no new permission cache or compiler/source special case.

Fresh complete censuses reconcile every request, byte and reallocation with the
separate heap counters. Shape allocation families are:

| Family | Before | After |
|---|---:|---:|
| Initial parameter owner, retained for final shape |178|178|
| Copied final parameter owner |178|0|
| Witness-pair vectors |272|0|
| Fixed-parameter heap scratch |178|0|
| **Total** |**806**|**178**|

The exact reduction is 628 requests and 10,306 requested bytes. After adding back
those size/kind bins, the remaining histogram is identical. Full raw stacks are
retained; no exact cross-binary join of every other stack family is claimed.
Before/after traces have 1,239/1,155 symbolized PCs and 3,592/3,162 stack groups.
All stacks are complete within the checked 48-frame diagnostic bound.

Ten separate allocation controls cover changing values at 0/1/16/17/65 parameters.
Zero-parameter shape requests fall 1→0; one/16-parameter cases 4→1; larger cases
4→2, retaining their heap scratch. Exclusive frame controls save three requests
per invocation; aliased controls remain equal. No control allowance is raised.

## Binary and resource contracts

The complete compressor Object, descriptor and metadata are byte-identical to
R94. Object SHA-256 is
`f6759d4c388e059790ab8490c4d304e8ca51c402c1f11e6c0ebd8428c30ef124`;
text SHA-256 is
`b51241ae4b045d08b8fdf733e8284b3d725d7ff71ac4ef8bcea75e69a29fb8a8`.
The improvement is in host construction surrounding that generated computation.
Fresh profiles cover 695 Mech calls per subject and 20,000 reference calls.
CPU sample weights are not retired-instruction shares or exact timer fractions.

IR/work/private ABI/MBC remain **140/99/38/14**, scalar ABI1. Existing conservative
finite prices remain unchanged. Full logical work 42,449,879,781 and exact Huffman
work 88,531,834 stay equal; neither is a hardware instruction count. Source
algorithms, tables, allocation sites, allocator and timing boundaries remain
pinned. No expected output or component allowance changes.

Native binaries remain local after the earlier automatic upload rejection.
Independent numerical replay retrieves source/build/hash receipts, descriptors
and raw observations; it does not independently retrieve Object bytes.

## Qualification and failures

The complete Core-dependent graph was rebuilt: twelve project libraries, identical
selected operations/features, frozen third-party dependencies and optimized
warnings-as-errors flags. Two independent build jobs took 361.432s in total.
No Cargo-wide feature expansion was used.

Passing overlapping selections: **376 Core units**, **102 tests in ten actual
Rust consumer targets**, **10 canonical shape contracts**, a fresh locked/offline
**no_std Core check**, runtime355, work/memory33, native CI64, image1, unchanged
components236, connected8, path1, linked40, scalar12, frame8, workflow132 and
architecture/type/memory15. The scalar gate remains 1.05; maximum observed ratio
is 1.041001, with equal pure-scalar allocations and zero loop allocation slope.
Contrary scalar timings remain. The new CI step inventories and executes all ten
shape contracts. Local runs use the retained selected feature profile; no fresh
all-features dependency/test graph or complete no_std test-suite claim.

Tests cover zero/full-U64/changing extents, both sides of the inline cutoff,
compound/shared/min/max witnesses, errors and retry, unchanged clones, actual
parameter-buffer ownership transfer, and lower-bound failure before arity errors.
Existing original/decoded/restored and actual linked/AOT cases retain input
aliases, published storage, cleanup and recovery. Final managed bytes/nodes are
zero.

Two preparation mistakes are resolved and retained externally. The first
error-order fixture had unused parameters removed by canonical normalization;
actual body references preserve the intended overflow. A Rust-consumer driver
initially put its source path in a codegen-option position; corrected argument
selection runs all 102 tests. No solver semantics or allowances were weakened.
No required current local failure remains. Hosted green CI, giant complete
knucleotide, large/full-corpus timing and independent Object retrieval are skipped.

## Fidelity and next evidence

The unchanged source SHA-256 is
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Level12 retains the 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64
path capacity, depth300, nice258 and original near-EOF rules. Search/skip remain
separate. Mech compresses eight bytes itself and returns
`[75,76,74,78,73,77,75,207,0,0]`, checked by independent inflate. Input aliases,
output capacity13 clear/rebind and `evaluate_into` timing stay pinned. Physical
public-output backing still changes; physical output reuse is unproved.

The smaller allocation count did not materially change elapsed time. The next
bounded investigation is a calibrated instruction breakdown of setup, generated
execution, callback classes and cleanup. Current samples are insufficient to
assign the whole-call instruction excess to individual helpers. Use that ledger
to choose the next structural runtime change. Private scratch/source-local
transport, child-seed placement, alias proof and outlining remain separately
measured candidates; completed optimizations are not proposed as new work.
