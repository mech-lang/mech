# Direct scalar homes and shared checked returns

The compact native emitter can use a value directly after proving that its
physical home has exactly one definition, that definition dominates every
reachable read, no read precedes the definition, and the home is outside input,
initial-value and owner-bearing lanes. Selected nonowner geometry fields use
the same finalized-graph proof. Failed proofs reject the optional compilation.
Unselected homes continue through the existing SSA variable path.

Checked continuations can also retain shared cold returns in the caller while
outlining a copy. Each admitted return contains only local integer constants
and preserves its original status and relative error origin. External edges
continue to reach the retained caller block.

The changes are adopted from the source-pinned #993 experiments. The local
source build exactly reproduces the published **67,648-byte encoder object and
metadata**. The linked worker uses the same frozen M3 runtime and allocator as
the existing local control; it differs from the separately linked upstream
worker. The original Mech program and canonical bytecode are unchanged.
[Upstream source pins](upstream-source-pins.json) ·
[Local source inventory](local-source.json) · [Artifact replay](artifact-replay.json)

The linked worker passes 162 exact-output and independent-inflation calls,
input/output alias checks, checked failures and cleanup. Measured peak stack is
4,752 bytes, below the unchanged 5,280-byte ceiling. Focused tests cover valid
dominating definitions and refusal for one-arm definitions, repeated writes,
early reads, unused homes and unreachable reads, with direct lowering both on
and off. With these new options disabled, the earlier 67,656-byte object and
metadata are byte-identical. [Correctness record](correctness.json) ·
[SSA tests](ssa-word-tests.txt)

Runtime qualification is separate from artifact reproduction. No new complete
M3 qualification is claimed here. The target remains 81/81 against Roc and at
least 61/81 against libdeflate level 12 in both orders.

Build the compiler from this branch, then run
`reproduce.py COMPILER OUTPUT_DIRECTORY`. The recipe includes all inherited
flags, including direct entry literals, and verifies the source, profiles,
object, metadata and canonical bytecode hashes. [Reproduction recipe](recipe.json)
