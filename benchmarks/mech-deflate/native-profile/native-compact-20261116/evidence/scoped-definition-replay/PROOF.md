# Scoped definition replay

The STATEMENT → VARIABLE_DEFINE family at PR #993 commit dae46c686703560230150a93f36c44c4978ffdc7 took about 643 ms in selected replay on the matched Apple M1 parser. This prototype is limited to that invocation on sealed one-piece one-shot input. The patch uses the same journal implementation. [Pinned family profile](/private/tmp/mech-best-choice-profile.Y0SGCETr/evidence/best-choice-replay/README.md)

## Selection and ownership

The document continuation tries the existing alternatives in order. A strict increase in consumed length replaces the winner. Halted and committed outcomes and choice rewinds keep their original order. The recorder retains event data when the variable definition becomes the winning trial. A later longer winner clears it. Recording and applying both decline while another trial is observing the raw scanner, preserving the first-origin witness.

The recipe copies at most 24,000 live events from 8,192 consumed bytes and uses up to 65,536 node or token ordinals. It owns references to the live cached nodes in that slice. The recipe holds no journal page snapshots and expires at the selection boundary.

## Admission and application

- The trial and replay share the start checkpoint, sealed consume and context bounds, recovery policy and prior recovery work. They remain in the same parser poll. The selected expression has positive extent and one closed variable-definition cache root. The trial has unchanged diagnostics, diagnostic allocator, recovery statistics and enclosing marker/rule/nesting depths.
- Both grants cover the original logical work and conservative stale-cache and terminal prediction headroom. The recorded fuel charges fit the live fuel. The historical event and marker peaks already include trial transitions. Their sum plus the resource error envelope must fit the event limit.
- The raw base continuation observes immediately before claiming a Unicode scanner. A successful lexical memo exposes the virtual first raw access at the same logical point. The summary either leaves the tail untouched or requires a fresh first origin in both executions and restores the outgoing tail. Literal and negative lookups use immutable source and bounds. Their ordinary fallback produces the original result and work. The successful lexical memo already has fresh-origin, fuel and resource guards.
- Both boundaries have drained cache work. Grant headroom keeps the stale cache routing consistent. The recipe requires live local identity-bearing starts and cached publications, with ordered token, finish and tombstone events. Cached children must be the identical recorded publications. Preflight remaps their ordinals onto the next live node and token ID ranges, preserving holes from discarded work. It prepares the new nodes outside the parser.
- Application appends the fresh journal recipe, including the original cached ranges. It updates cursor, covered prefix, allocator cursors, grammar and child work, fuel, logical rewinds and accumulated cache work. The trial already registered the transient resource peaks. Actual one-shot journal allocations, mutations and discards remain physical counters.

The ordinary replay runs when any check fails. Nonfinal or segmented input, tight grants, recovery and halted/committed selection retain their existing paths and diagnostic origins.

## Differential and ablation

The private tests compare enabled and disabled execution at the same entry. They hash the raw tape, each cache publication and its immediate child identities, the final tree, syntax and restart indexes and diagnostics. They compare allocator cursors, fuel, rewinds, cache work, grants and visible state at each poll, and resource peaks. The matrix covers exact-fit and smaller allowances, irregular grants, event/fuel/nesting limits, single-scalar UTF-8 pieces, nonfinal input and recovery. The active scanner test verifies refusal before replacing the witness.

The capture-only build retains the bounded winner and checks the live admission conditions. It then runs the ordinary replay before identity remapping. Timing uses ordinary optimized builds with validation observers absent. Pre-review source and receipts identify the earlier result separately.
