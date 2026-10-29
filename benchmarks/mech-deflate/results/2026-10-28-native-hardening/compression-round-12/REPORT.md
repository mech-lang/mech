# Round 12: private unsigned indexed-write leaf recurrences

Corrected compiler `c0bef396bbc88cdd8dd94037053866b182b7a68d`: the final fresh
pair is **345.80× → 250.94× Roc**, 0.099590208 → 0.072269500 s, reference
0.000288 s. This is **27.43% less elapsed / 1.3780× faster Mech**. Exact bytes,
independent inflation, reserve13, false physical-backing reuse and final cleanup
checks pass. The separate 56-repetition profile is excluded from primary samples.
`final-primary/` retains this cohort; the earlier cohort below remains separate.

The provisional broader run executed126 tests:125 passed and the unchanged
private-allocation test failed. The initial native boundary cloned its own
carrier pin and always requested a copy, doubling private payload allocation.
`c0bef396b` transfers that pin into the original consuming store instead. Core's
exclusive same-account check remains authoritative; external aliases and other
fields still force a copy. The original allocation test plus all11 new cases
pass (12 tests), with no expected values or allowances changed. The private U32
case now allocates its initial buffer once across0/1/8 updates; the exact large
minus small delta is6000900 bytes in each case. Broad/fuel/linked qualification
and the corrected compiler's protected scalar cohort are pending at this commit.

One unit-profile build was deliberately interrupted to fix this regression first.
Switching back rebuilt the unhashed selected math library; the corrective default
build took3m58s. The timing overlap guard rejected an attempt while freshness
verification was still active, before taking any samples. That attempt's log is
retained; measurement restarted only after the build finished and library hashes
were verified. See `corrected-timing-inventory.json` for all receipts.

Compiler `ed75406f3893e05070542dc523634251947a7fdc`, base
`efa2ff81741362394553d5c7eae7d9dfc39766c4`. The core fixed-shape handoff is
`02fb501b3935d16f75b1eda4fbe049bdf1051465`; native selection/emission/runtime
and focused regressions are `ed75406f3`. PR parent remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The unchanged eight-byte level-12 pair improves from **344.70× to 252.69× Roc**
(Roc = 1×), **26.69% less elapsed / 1.3641× faster Mech**. This is a small-case
observation, not large-input throughput or parity. The fresh reference median is
0.000289 s from 20,000 observations; Mech medians are 0.099616958 s before and
0.073028666 s after (three primary observations each). All five observations on
each side preserve exact raw bytes, independent inflation and final zero bytes /
nodes. The separate 55-repetition sampled process contributes no primary samples.

Qualification at this first timing checkpoint: 87 snapshot tests pass. The new
focused cases pass across the final two receipts (ten cases, then the corrected
real-buffer measurement). Broader component, actual finite-fuel, linked/AOT and
protected scalar controls are still pending; this is not the final acceptance
packet. The historical round-10 decoded U64 ratio 1.0526036 against 1.05 remains an
unresolved observation and must not be erased by a later cohort.

The source, canonical bytecode, algorithm, tables, sizes, limits and allocation
sites are unchanged, including the real 262144 U16 tree / 1500513 U32 cache,
depth 300, nice length 258, original near-EOF limits and six deliberate Huffman
initializers. The same mimalloc 2.1.7 object is pinned. The timer encloses only
`evaluate_into`; reserve13, clear/rebind, JIT, validation and drop keep their
existing boundaries. Physical output backing changes on **both** sides at the
immutable public argument boundary (`same_backing=false` on every call); those
copies remain timed. Capacity retention is not physical output reuse.

## Representation and authority

The existing canonical Program numeric-leaf proof now admits selected plain
unsigned indexed replacements and one numeric-array result. A forward must-analysis
checks current functional versions on every acyclic source path: stale content
reads, stale returns and discarded recursive updates decline the class. Static
root provenance alone cannot grant in-place mutation. Exact frame recipes,
selected factories, schemas and geometry still select it; names and table sizes
do not. Shared immutable inputs and separate fields with actual aliases remain
valid. A suspended ordinary parent cannot run while its leaf holds native views.

The first **changed** write settles native fuel, calls the original selected
replacement with current operands, and obtains one private copy. The existing
exclusive core handoff retains that backing in a new, unregistered fixed-shape
owner with its future canonical wrapper/registration already admitted. Subsequent
successful iterations use typed loads/stores and SSA scalar state. Unchanged
writes keep the original identity. No general per-element runtime dispatcher was
substituted. On fuel exit, current cells freeze without allocation/admission and
current scalar fields are restored. A pre-admitted arena ticket binds the eventual
result to its exact arena generation, type, account and retained bound.

The cold first-change boundary reopens the same **post-materialization** account;
the partially executed traversal has already been paid and is not charged or
funded twice. Exceptional paths close/discard ownership before the original typed
error returns. IR117/work policy92 identifies these proofs and prices, including
store comparisons and prepaid publication. Canonical bytecode remains revision14;
ABI35 is unchanged. Restored layouts are independently rederived from the plan.

## Observations and limitations

The actual unchanged Huffman counter at 256 symbols drops from 5629 requests /
827362 bytes / 13846 ordinary visits to **280 / 44906 / 6**. Its after counts stay
flat from 1 through 256 symbols, for original and decoded artifacts. The generic
increment loop drops from 4090 requests / 275146 bytes / 4628 visits to
**276 / 38610 / 6**, also flat for positive iteration counts. These System/probe
component observations are separate from primary mimalloc timing.

Real-buffer typed-store witnesses request **561530 bytes** for the U16 tree and
**6039294 bytes** for the U32 cache, with **276 requests / 6 visits** each. One and
eight updates agree exactly for both artifact forms. These witnesses use typed
constant replacements, isolating stores from the separate narrow-arithmetic
factory capability; the compressor source was not changed.

The after profile contains 1351 sampled stacks. Inclusive overlapping ancestry:
helper996, buffer-frame action454, frame-word admission329, frame-input
admission327, visit231, selected library call193, selected unsigned replacement69,
selected unsigned read30. They are neither additive buckets nor predicted gains.
Ordinary mixed array/scalar frames remain a demonstrated cost; returning arbitrary
mixed carriers, growing producers and nested native regions are outside this slice.

## Attempt disposition

All failed attempts are retained. The initial core check missed a `SequenceView`
import; the first native check mapped a memory error as a snapshot error. Both
introduced compilation mistakes were corrected before release. The first core
test command addressed a nonmember package through a consumer workspace and was
rejected; the second selected zero tests. Neither is passing evidence. The corrected
command executed eight ownership tests, followed by all 87 snapshot tests.

The first new native cohort passed eight cases and failed three fixture assumptions:
two requested narrow additions lacking the existing scalar semantic witness; the
store-width fixtures now use typed constants. The exact-budget fixture initially
omitted prior-output retirement by taking an unwarmed receipt; it now uses the
actual warmed receipt without increasing any allowance. The next cohort passed ten
cases and exposed one cold metadata allocation difference in the new real-buffer
measurement (261 requests versus 256). Warming both measurements removes that
one-time cache difference; the final real-buffer case passes with identical counts.
Existing component expected values and allowances are unchanged. The first static
run passed architecture/catalog/inventory/unsafe checks and rejected formatting;
formatting is corrected, with the final full static run pending.

See `timing-inventory.json` for retained files and exact hashes. Release build:
16m49s, selected five math operations; the clean freshness build was entirely
fresh. Executables remain private with recorded hashes. Clearing the inactive
37 GB debug cache recovered about35 GB; the successful core test executable and
all receipts were retained. No cancelled large-input campaign was restarted.
