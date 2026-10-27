# Retaining packed maps through private read-only uses

Status: implemented and measured at `78007ae6f` (IR49/ABI26/work65).
Fourteen focused tests, 319 GPU native tests and one catalog test pass; three
existing manual tests remain excluded. The design below records the proof and
accounting obligations as well as the measured scope.

The preceding implementation kept a source-written String/U64 map in packed
storage only while a proved closed Tail executed. On its canonical path,
`program_map_closed_runtime::end_closed_map` calls `PackedMap::finish`, which
decodes every key into `OwnedStringMap`. At the original Tail result copy,
`Binding::finish` publishes a sorted canonical `Value`. Even a subsequent
membership test and scalar lookup require this complete conversion.

The new representation retains the **entire immutable packed map** through
proved private reads. It changes no source traversal, branches, key set, counts
or updates. It recognizes semantic operations and dataflow, never workload
names, particular queries or nucleotide alphabets.

## Initial proof boundary

Start from an existing proved packed map owner and its Tail result. Internal
source functions are already expanded into the native plan, so this does not
require a new inter-function ABI or cross-thread map transport.

Derive a separate `FrozenMapUses` proof from the complete semantic action and
use inventory:

- The result has the declared non-parameterized String/U64 map schema. Its
  original result-copy site dominates every selected use; normal current-visit
  and lexical-scope validation remains mandatory.
- Follow only scalar copies and canonical-value aliases within the same parent
  iteration. Every producer of a selected alias must derive from this owner.
  A join with a foreign map, ambiguous producer or incomplete use inventory
  rejects the candidate. Branches may select reads; they do not move them.
- All consuming operations are the selected `MapContainsKey` or `MapKeyAccess`
  templates with the map in its exact operand position. Keys may be arbitrary
  runtime Strings. Their producers must already be available at the original
  read sites; no speculative key computation or lookup is introduced.
- Reject the entire candidate if any alias reaches mutation, map enumeration,
  equality, display, a constructor/collection payload, another loop's seed or
  capture, a transport result, an activation boundary or any unknown operation.
  The initial implementation does not support nested-scope aliases.
- Prove retirement at the original parent iteration's retirement boundary.
  Repeat entries get new generations. A missing runtime admission selects the
  existing canonical route at the original Tail exit.

This is an all-uses decision before execution. It must not freeze a partial
alias closure and discover an unsupported use later. Whole-map observers keep
the existing eager canonical path, including its ordering and failure frontier.
Lazy materialization at arbitrary later observers is outside the initial scope.

In the currently measured source plan, the query histogram's result feeds only
contains/get in the same worker, then the worker yields a formatted String.
The frequency histogram feeds keys/values and therefore keeps canonical
publication. These are consequences of the use proof, not compiler special cases.

## Ownership and complete-map validation

At a successful original exit, transition the packed owner into a worker-local
`FrozenPackedMap`. End generated mutation authority and preserve all slots,
controls, width, domain/decode metadata and shared memory reservations. No key
or count is discarded, including entries that are never queried.

A private session lane owns the table. Generated aliases hold checked opaque
tokens tied to invocation, owner and generation; no pointer becomes a source
`Value`, and no fabricated canonical value enters `ValueArena`. Selected copy
and read hooks bypass canonical transport only after checking the complete proof.
Ordinary helpers must reject private-only sites. Successful reads return normal
bool/u64 values through the existing typed output mechanism.

Freeze preserves the original boundary's validation before any later source
action: frame/owner identity, funded versus consumed traversals, update count,
table/control bounds, key-domain validity, occupied count and declared schema
and cardinality. It also needs **uniqueness and probe reachability**. The current
conversion detects duplicates by checking the size after import into a String
map. A replacement witness must be equally strong: for example, a bounded full
probe of every occupied key must resolve to that exact slot, and the occupied
count must equal the reported length. Its work must be prepaid and bounded.
Checking fingerprints and counts alone is insufficient.

Retire the frozen lane and all alias generations at the original parent-scope
boundary, on failure, and during unwinding. Retirement must not happen at the
closed Tail's inner-latch boundary. Geometry includes the frozen owner's actual
shared claims until its last permitted use. No worker publishes a packed map to
another thread; it yields its ordinary completed result after private retirement.

## Arbitrary queries and observable behavior

At each original read site, check descriptor identity, invocation/account,
current scope and visit, selected template, map token generation, and admitted
String key schema/ownership. Keep the original source diagnostic origin.

Encode the actual query only after checking **every byte's domain membership**
and exact width. A different length, non-ASCII byte or character outside the
proved domain cannot equal any retained key and means absent; masking or
truncating such a query into a valid key is incorrect. The initial packed owner
already declines sources outside its supported ASCII proof. Empty maps/keys and
zero-valued hits must retain their normal semantics.

An encoded candidate is a hit only after bounded probing and full packed-key
equality. Contains returns false on absence. A direct missing get produces the
maintained operation's missing-key error at its original site; a guarded get
executes only when its original branch selects it. Do not substitute a generic
descriptor error for a source lookup failure. Multiple distinct runtime queries
must remain valid. Optional cached lookup reuse requires immutable owner and
exact query identity; it is not necessary for the first implementation.

## Finite accounting and error order

Freeze and each selected read use one bounded call scope under the existing
shared invocation ledger. They receive no new allowance. Charge all table
validation, key scanning/encoding, probe work, typed output construction,
metadata and retirement. Bound complete collision chains. Do not restart a call
allowance for each occupied slot or query byte. Actual byte/node reservations
must remain atomic and belong to the same capped account, including overlap if
canonical fallback constructs a new owner.

Keep source-level schema/cardinality and logical output-element/output-byte/
clone-size checks at the original result-copy boundary (`MAP_OWNER_END`), after
that source visit's debit. Complete packed-storage validation remains at the
earlier closed-tail exit. Factor the relevant demand formulas
from `OwnedStringMap::publication_plan`; private representation must not bypass
those declared limits or postpone a boundary failure until a later query. Error
origins and the order of actual source operations stay unchanged. Never evaluate
an untaken read merely to check its per-call allowance.

Removing physical key decoding, copying and sorting changes implementation work
and allocation demands. Exact old sort-comparison debit sequences and their
resource-exhaustion prefixes cannot simultaneously be preserved while that work
is removed. The new representation therefore needs an explicit work-policy
identity and conservative, input-dependent pricing for its actual work. It does
not increase limits, refund spent work or disable checks. This distinction must
remain explicit in validation and performance reports. If identical old physical
conversion failure prefixes are required, the conversion must remain; a late
fallback after intervening source actions is not an equivalent substitute.

Derive and revalidate the new proof for original, decoded and restored plans;
include its facts in descriptor identity. Advance IR/work versions, and ABI only
if the compiled entry layout changes. Do not trust serialized proof fields alone.

## Validation and expected scope

Required checks include arbitrary present/missing runtime queries, non-ASCII and
out-of-domain queries, different and zero widths, zero values, duplicate-key or
unreachable-slot corruption, aliases across branches, stale/cross-session tokens,
wrong schemas and read sites, rejected escapes, full canonical fallback and key
ordering. Exercise exact/one-short per-call and shared limits, failures before
later source errors, unselected branches, output retirement, original/decoded/
restored execution and concurrent-worker cleanup. Execution receipts must still
verify all tables and all source updates, plus actual frozen-path selection.

This targets conversion and publication after counting. In the earlier SIMD stage's
exact-code full-input capture, 84.42% of observed worker samples have generated
program code as their innermost frame; publication appears in 4.94% inclusively.
The prefix tail captures two remaining workers and includes publication in
56.29%, work debits in 54.59% and waiting in 41.30%. Categories overlap and are
not wall-time shares. They support this as a possible prefix-tail improvement,
not a prediction of recovered time. The later 16-byte narrow-slot representation
is an independent change affecting every table probe/store; its historical
measurements are retained with the subsequent stages.

Evidence: [SIMD benchmark and profile records](https://github.com/mech-lang/benchmarks/tree/c1532d3bf/experiments/mech/native-owner-aot-v1).

## Private read-only owner implementation

The first implementation accepts only complete maps whose original result and
all copy aliases stay in the same immutable parent visit. Its consumers are
ordinary `MapContainsKey` and `MapKeyAccess` calls with String queries. Any map
output, enumeration, mutation, capture, join of different producers, or virtual
consumer declines the representation before execution. The serialized layout is
included in the artifact identity and rederived from the same plan when loading
an image.

The original closed-tail exit checks every occupied control, complete packed
key, probe reachability, uniqueness, cardinality, update count, and funded
traversal count. It retains the whole table and all of its existing byte/node
claims. No requested-key set is supplied. The original result-copy step checks
the schema and logical output/clone limits, then publishes an invocation-unique
private token. No canonical Map handle is fabricated. Original source copies
carry that token, and each original read checks the exact source site, parent
visit, token, and same-account owner before accepting the query.

String queries validate byte length and membership before encoding, including
both packed words. Absence and a stored zero are distinct. A missing key access
returns the original library `InvalidInput` error at the original source origin.
Parent latch/exit retirement releases the owner; root and worker teardown cover
completion and every failure. Worker receipts carry only the original scalar or
String output, never a map token or table.

Work policy 65 charges complete validation and actual bounded probe groups.
Allocation-free validation uses the existing bounded exact-debit batch: it
reserves no speculative worker credits and cannot renew per-call limits. The
canonical import, key allocation, and publication sort are physically absent,
so their former physical work charges are absent as well. Mutation-body charges,
source updates, source control flow, and finite logical result limits remain.
Actual execution reports expose `native_frozen_map_regions` and
`native_frozen_map_reads`; candidate metadata by itself does not claim selection.

## Measured scope

Six rotated full-input cohorts retain all observations. The 125-million-base
median is 1.536 seconds (2.76 times Rust), versus 1.927 seconds for the guarded
parent (3.46 times Rust) and 0.556 seconds for Rust. This is 20.3% less elapsed
time. A slow Rust observation in the first three cohorts prompted three further
balanced cohorts; it remains in the six-cohort aggregate.

Three alternating one-million-base prefix observations give 0.0968 seconds
(4.64 times Rust), versus 0.492 seconds for the parent (23.58 times Rust) and
0.0209 seconds for Rust: 80.3% less elapsed time. One warmup per executable is
excluded only in this prefix protocol. Inputs, source, bytecode and finite
launch limits match the parent; work policy65 explicitly prices the changed
physical representation. Complete secured processes execute sequentially.

Every candidate execution verifies five actual frozen maps, ten actual reads,
seven selected guarded SSA loops, a peak of seven workers, complete output and
all 6,999,961 prefix or 874,999,961 full-input updates. The two frequency maps
still publish canonically. The frontend elides the measured source's alias,
so no private-copy-hook execution is claimed.

The full-input profile has identical complete machine code to the timed binary.
Of 6,117 worker observations, 90.76% end in generated counting code. Library
debits occur in 3.63% and waits in 4.69% inclusively. Publication is not observed
in this capture. These overlapping categories are not elapsed-time shares.
Both completed prefix attempts have empty call graphs and provide no hotspot
observations. Remaining targets are counting-loop state, repeated checks and
stack traffic; no gain is assigned to unmeasured proposals.

Evidence: [timings, exact-code profiles, disassembly and retained validation attempts](https://github.com/mech-lang/benchmarks/tree/5ddd042f3ef15a07f76529ed9100fe56b77d14d6/experiments/mech/native-owner-aot-v1),
[focused and complete native validation](https://github.com/mech-lang/benchmarks/tree/5ddd042f3ef15a07f76529ed9100fe56b77d14d6/experiments/mech/native-owner-dispatch-v1).
