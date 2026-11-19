# Early codebook construction pilot

The accepted 18 November review changes the main line of attack: construct
bounded units before root SSA, instead of constructing a large root and
extracting its regions later. The complete objective stays 1x libdeflate code
and fresh source-through-final-link time, retaining all corresponding peer wins,
at least 81 Roc and 62 C wins, and ultimately more C wins.

## Before-edit hypothesis

1. Measured pool: the current selected compiler reproduces the 63,804 B object,
   complete metadata and bytecode while collecting diagnostics. Its root has
   16,157 laid-out blocks and 42,255 instructions before cleanup, with 831,378
   allocated SSA value slots before compaction and 122,162 afterward. These are
   root-only snapshots, not cumulative totals across all bodies. The qualified
   late codebook boundary maps from native blocks 4069:4954 to canonical source
   sites 4119:5031; discovery finds 885 reachable sites and 469 definitions.
2. Work to remove: assign that region an owner before native emission. Compile
   its operations once with local word homes/SSA and typed captures, and emit a
   coarse call in the controller. Do not construct its internal root blocks or
   SSA homes. Global canonical bytecode may stay unchanged. Sum all units so
   moving work into a helper cannot masquerade as removing it.
3. Expected effect: fewer total intermediate instructions/blocks/SSA slots,
   smaller maximum unit, lower complete build time/RSS, and a better opportunity
   to narrow live state and emitted code. No savings are forecast from profile
   fractions or the root's share alone. The whole encoder must include helper
   bodies, packets, stubs, extra spills and alignment in its byte accounting.
4. Correctness boundary: independently admit the typed source region, including
   its one entry, one normal exit, all failures and continuation behavior.
   Preserve constants, captured/current views, mutable partial work, ownership,
   collection cursor state, error order/origins, cleanup and finite admission.
   The 44-input-word/eight-output-word discovery interface is conservative and
   is not an ABI or compiler proof. Constant elision and word-level liveness
   still need the actual typed plan; hidden state is not covered by this list.
   Keep hot matcher/scorer loops free of new fine-grained calls.
5. Acceptance and stopping: first instrument total construction across root plus
   every helper, including instruction/block counts, cumulative allocated SSA
   and peak live values, maximum unit, CFG/dominance builds, full validations,
   emitted text, complete link-inclusive build time and RSS. Then implement
   bounded emission. Reject a claimed architecture win that only shrinks root
   measurements while duplicating work elsewhere. Compiler-only exact-worker
   changes may reuse its qualification; any changed worker must pass both full
   orders, all individual control wins and 81/62 floors, outputs/aliases,
   allocation failures/recoveries, and the unchanged 5,280 B stack ceiling.

The current implementation has not performed early emission. This discovery
script deliberately reads debug records only; production admission must use the
canonical typed plan and existing proof vocabulary. It must not parse debug
strings, trust a recorded source-site list, or rewrite the Mech program.

Sealed complete-input parser execution is the independent semantic workstream:
fuse one measured high-frequency grammar family while retaining logical work
accounting, events, identities, diagnostics, suspension and exact fallback. It
has not started in this experiment. Conservative prepared-helper IR reuse is a
bounded follow-up, not a replacement for early construction. Do not reopen the
failed retirement-transition cache or turn register/layout tuning into the
main campaign.
