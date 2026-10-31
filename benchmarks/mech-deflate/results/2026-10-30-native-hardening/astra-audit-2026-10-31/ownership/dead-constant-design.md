# R56 unused constant construction audit

This is a static investigation, not a production edit, build or timing result.
The executable descriptor is pinned to SHA-256
`1b4dbdd2fb0c1f9f51ae7e9dd0bc931bbc2804e6cb949653461d6348c31615c9`.
Run `python3 /private/tmp/mech-r56-owner-audit-20261031/dead_constants.py` to
reconstruct the inventory from the retained R56 descriptor.

## Finding

The native plan contains 500 constants. Exactly 309 have no original semantic
read anywhere: 248 U8, 51 U32, and 10 Index literals. The zero-use subset is
confirmed across all 41,039 blocks, 450 loops, 122 collection source definitions,
outputs and effects. All 1,438 prepared-call source ConstantId references were
also checked; none names a candidate. Every constant has root immutable scope,
no candidate appears in a lexical retirement row, and this program has no
activation nodes, observations, effects or scalar-aggregate constants.

The R56 census records 500 registration allocations of 104 bytes and 500 import
wrappers of 112 bytes in Prepared::new_with_budget. Its unconditional loop
imports all 500 constants. Thus zero-use constants account for a *predicted*
618 requests and 66,744 requested bytes. No production removal has been measured.
A new admission-claim representation may add overhead; do not call 618 a result.

This is a refinement of the brief's known 1,000-request constant family, not an
additional disjoint family. It is stronger than assuming scalar constants never
need canonical owners: zero semantic uses exclude all canonical consumers.

## Existing proof and all indirect consumers

- `src/engine/src/resident/general/native_match_lowering.rs:1285` imports the
  complete canonical constant pool into the native plan, even literals left
  behind by earlier constant folding. Keep this pool and its identities intact.
- `src/engine/src/native_artifact/fusion_semantics.rs:271` implements the existing
  exhaustive `visit_role_reads`. `NativeProgramFacts::semantic_use_counts()` at
  `program_facts.rs:216` exposes that inventory. Reuse it; do not create a second
  graph traversal or a benchmark-specific literal list.
- The visitor covers library and primitive operands, ordinary copies, collection
  pushes, structural sources AND StringEqual peers, hidden PrepareU64Range
  operands, all branch operands, collection semantic sources, loop seeds/next/
  returned values, outputs and effects. Virtual selected consumers are expanded
  before visiting, so ordinary fallback cannot silently lose an input.
- Prepared templates retain original source provenance but execution receives
  actual inputs through NativePreparedLibraryCall::invoke_current. The template
  source scan additionally excludes all 309 candidates, even if this setup-only
  provenance were conservatively treated as a runtime demand.
- `native_collection_calls.rs:233` can read a constant record selector at
  preparation, but no candidate is an Id and the broader source scan has no
  overlap. The prepared template and canonical pool must remain available.
- Runtime map alphabet/domain inference reads the immutable plan constants
  directly (`runtime/maps/key_domain/mod.rs:127,164`), not their imported arena
  owner. Candidates are atomic numeric/Index, never String/Map.
- Ordinary scalar constant initialization and worker startup still read the
  descriptor constants and write native words. Preserve these stores initially;
  they do not need the imported arena wrapper.
- No candidate can be a tail/carrier, collection source, capture, output or
  effect without being counted in the existing semantic inventory. Derived
  buffer-frame/numeric/parallel metadata must remain derived from those same
  reads; do not grant a new alternate execution surface.
- `prepare/retirement/mod.rs:125` omits root-scope values from lexical cleanup;
  candidates currently remain live in the arena until invocation teardown.
  Their logical claims must therefore have that same lifetime, including failure.
- Image loading must retain every canonical constant, canonical validation,
  constant_bytes check, and identity hash. Recompute a zero-use mask from the
  independently loaded plan; never deserialize an elision permission.

## Preserving admission without unnecessary canonical wrappers

Do NOT simply skip the import loop or remove constants from the plan. The dead
subset alone has a 56,566,776-byte sum of original setup envelopes. Those are
conservative admission envelopes, not physical allocated bytes or simultaneous
retained capacity. Exact original funding and work order must survive.

A bounded implementation would separate Core's current-account immutable
snapshot admission from publication of its import wrapper, and retain an opaque
admission claim for a proven unobserved constant. The actual constant stays
owned by the descriptor. A claim is not a Value, cannot be supplied to a selected
call, and grants no execution or source-use permission.

For each candidate in original order, it must:
1. Charge the unchanged `bound * 8 + 128` setup debit.
2. Reserve the same original bound before deriving/using any current claim.
3. Use Core's existing immutable ownership witness and same-account physical
   ticket adjustment. Keep schema/shape validity and all arithmetic errors.
4. Preserve registration/wrapper allocation-test gates at their original
   positions if the existing fault-injection contract is retained.
5. Retain the same snapshot node witness, then split the same admitted bytes
   from the reservation, preserving node-before-split order.
6. Replay arena insertion checks and work: ledger, 32-unit prefix, exact slot
   schema/key, current account, next generation, `required_bytes * 4 + 32`
   retirement debit. Keep the generation effect, but expose no usable Value.
7. Hold the claim until the same invocation teardown point. A failed setup drops
   all preceding claims; success/failure/retry must return to previous census.

The distinction between `ManagedMemoryBudget::retained_nodes()` and
`retained_snapshot_nodes()` is essential. Ordinary `reserve_nodes` only changes
the combined/private census. Caller geometry reads retained_snapshot_nodes, so
replacing canonical claims with ordinary private reservations would undercount
current callers and is invalid. The first implementation should use a Core-owned
claim with the original SnapshotNodeCharge behavior, not a host-side guessed sum.

Several sequential claims could eventually merge their same-account capacity
and node ownership to avoid a new per-constant allocation. That is optional;
first prove semantic/resource equivalence with bounded admitted storage, count
its new allocations, and report the net result. Do not cache prior-invocation
permission or move admission outside evaluate_into.

## Alias/registration visibility is the remaining design hazard

A zero-use constant's *import registration* can still be observed if another
reachable value shares its frozen data. Skipping wrapper construction must not
turn later same-account alias reuse into a new import with a different finite
admission result. A blanket zero-use skip is therefore not implementation-ready.

For this exact descriptor, no two constant data serializations are equal, hence
no dead constant can share frozen data with another constant. Both inputs are
packed U8 matrices; dead constants are atomic U8/U32/Index. There is no Dynamic
input or resource/observation surface that can smuggle a nested scalar root into
this computation. Image decoding also finalizes constants independently.

A production proof must either conservatively establish equivalent isolation
for the selected zero-use constants (cover root rebindings and nested Dynamic
values, not just IDs), or decline for aliases. Arbitrary externally held clones
and concurrent same-account import observers deserve explicit treatment; a
static use count alone does not establish their absence. This is the main reason
to keep the first experiment bounded and reviewed before generalizing it.

## Acceptance

- A minimal source with an interned unused literal must show the zero-use claim
  and request delta. A source returning that literal must keep a canonical owner.
- Cold/unselected uses, nested/recursive captures, hidden range operands,
  structural peers, ordinary fallback, direct outputs and effects must prevent
  incorrect omission. A use in an unselected branch is still a real static use.
- Same-storage constants and rebindings, reachable aliases, foreign-account and
  nested-Dynamic fixtures must prove decline or unchanged import behavior.
- Original and decoded artifacts must derive identical eligibility; corrupt or
  mismatched images must not confer permission.
- At each old byte, node and work admission boundary, compare exact failure,
  spent prefix, unchanged publication, and cleanup. Include import allocation
  failure countdowns, generation overflow and success/failure/retry.
- Complete eight-byte compression must retain exact bytes, inflation, input
  aliases, output capacity13, work60260219686/comparison47612 and final zero
  managed bytes/nodes. Separately measure requests, requested bytes, instructions,
  cycles and matched elapsed; allocation savings do not predict elapsed savings.
