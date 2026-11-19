# Typed source ownership and local-domain preview

The compiler now derives this plan directly from the typed canonical program
before native SSA. It does not parse the diagnostic source-graph strings or
trust the old recorded site list. Source boundaries, constant eligibility,
local value maps, whole hidden-state scopes and initialization are checked
again on the current program. **Early native emission is not implemented.**

For the codebook pilot at canonical sites **4119:5031**:

| Typed preview | Result |
| --- | ---: |
| Owned original operations | 885 |
| Invariant constants seeded locally | 20 |
| Input words | 22 |
| Normal output words | 6 |
| Helper local value words | 1,578 |
| Helper cursor/builder words | 20 |
| Root local value words | 13,371 |
| Root cursor/builder words | 230 |
| Root original operations | 7,193 |
| Synthetic coarse root calls | 1 |

The helper owns all references to iterations 44–52 and collections 15–16.
Each reachable original operation has one owner. Dense local value IDs and word maps
retain explicit canonical IDs and source origins. The existing all-path
initializer accepts the exact proposed inputs/outputs. Only reads preceding
an action belong in its incoming set; a result read by that same action's
terminator is available locally. Action reads of an old value remain inputs
even when that action overwrites the same home. Constants with any source
definition are captured as current values rather than seeded as invariant.

The original 44-input/eight-output-word discovery interface was conservative.
Typed invariant-constant elision and correct incoming-read liveness reduce it
to 22/six. This is still a plan, not an admitted native ABI. In particular,
retirement may need owner words absent from source uses/defs. Ownership,
partial-write/failure transport, finite admission, unit CFG lowering, frame
checks and actual-worker qualification remain to be implemented. See the
parent `lowering-contract.md` for preserving one shared source ownership model
without adding language-level call effects.

The snapshot's `emission_authorized` flag is false. The actual root is still
constructed normally, so these planned smaller domains are **not** a claim
of fewer SSA allocations, smaller emitted code, or a compiler-time gain.
The diagnostic execution preserves the exact selected object, metadata and
bytecode. Current all-unit measured counts remain in the neighboring census.

Seven focused tests check conditional and failure paths, old-value capture,
closed cycles, sparse canonical IDs, constant mutation, overlapping regions,
outside entries, split hidden state, malformed boundaries and word overflow.
`partition.json.gz`, `receipt.json.gz`, `source.patch.gz`, `compile.log.gz` and
`tests.log` preserve the snapshot, exact source patch and validation.

Build the benchmark compiler with `--features early-region-planning` to
include the experimental preview. To request it, use the selected native
recipe and set `MECH_COMPACT_EARLY_REGION_PREVIEW=4119:5031` plus
`MECH_COMPACT_EARLY_REGION_REPORT=/absolute/path/to/partition.json`.
This adds diagnostic planning/proof work; its elapsed time is not a build
optimization measurement. Ordinary builds do not execute the preview.
