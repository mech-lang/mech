# Typed comprehension certification migration

The semantic fingerprint domain is `canonical-source-program-v2`. Source maps
no longer contain executable comprehension qualifiers. Typed qualifiers,
patterns, local operations, and yields are included through graph payload
revision 4 in the artifact bytecode. The outer bytecode format remains v1.

The migration checked all 42 previously artifact-ready semantic witnesses.
For 41 witnesses, the source program, ordinary contracts, diagnostic anchors,
slot shapes, and every artifact section remained identical after normalizing
only the graph revision and the removed empty diagnostic qualifier list.
Their new fingerprints reflect the fingerprint/graph format changes.

The remaining existing witness, `pattern` (`[1 | 42 <- xs]`), now contains an
executable generator with an equality pattern and a separate yield instead of
an ordinary comprehension operation. Eight additional witnesses now reach
validated artifacts: matrix/set comprehensions and the array, array-item,
array-token, atom-struct, tuple, and tuple-struct patterns. Their 50 resulting
fingerprints were recorded only after this comparison and typed-control review.

These artifact witnesses do not establish resident support for every structured
pattern. S4 executes borrowed tuple, array, and tagged-pattern projections with
primitive Bool/Index/F64 bindings and yields. Live matrix/set inputs, repeated
bindings, generator/filter/join behavior, normalization, work admission, and failed
publication recovery have execution regressions. Compound match results and nested
match bodies/guards now use the same ordinary construction and memory providers.
S8 executes composite comprehension bindings and yields, including retained
tuple values and whole-tuple bindings, and composes nested matches and
comprehensions through one recursive control body with owned element-shape
witnesses. Computed patterns lower their pure lexical operations before the
owning generator, so each outer binding gets one ordered evaluation before
candidate filtering. The FSM resident continuation owner remains unfinished.
FSM pipes now retain machine identity, named and positional argument bindings,
ordered stage kinds, and recursively typed stage values in the artifact itself.
Source-map strings are not an FSM execution input. The `~>` commit, resume,
capture, cancellation, and fairness contract is recorded in
`docs/design/specification.mec`.

## Nested-match graph revision

Graph payload revision 5 gives each match-local operation a typed body, permitting
recursive match declarations without an ordinary-call contract on control nodes.
The 50 existing artifact-ready semantic witnesses were captured with both
producers and compared section by section. For every witness, the graph revision
changed from 4 to 5 and every other section payload remained byte-identical. Only
those 50 semantic hashes changed; sources, dispositions and required outcomes did
not. Positive compound-match, nested-match and exact table-join behavior remains
in the completion suite. The required FSM executable witness now reaches a
typed revision-seven artifact rather than an ordinary source operation. Revision 7
retains the FSM body variant introduced in revision 6; revision 5 remains the closed
nested-match graph grammar.

## FSM graph revision

Graph payload revision 7 retains typed FSM bodies and adds explicit
structural-pattern parameter sources.
The 51 artifact-bearing semantic witnesses were regenerated with the revision-seven
producer; their semantic hashes changed because the complete artifact bytecode is
part of each witness fingerprint. The source programs, dispositions, required
outcomes, and non-graph artifact sections are unchanged. Revision 5 remains rejected
without a compatibility reader, including for externally supplied FSM artifacts.
The committed 20-fixture bytecode-v1 corpus was also regenerated and checked across
five fresh producer processes. Seventeen compiler-produced, graph-bearing fixtures
changed at identical byte lengths; the three constructed scalar, matrix, and
composite fixtures remain unchanged.

## Composed-control graph revision

Graph payload revision 8 gives comprehension declarations canonical block IDs and
uses the same recursive operation body in match blocks and comprehension steps.
Nested control contract references retain canonical preorder, and mixed match and
comprehension depth and population share the artifact admission limits. Matrix
comprehensions embed the yielded element's dimension parameters before their own
cardinality parameter and reject inconsistent element shapes before publication.
Revision 7 remains rejected without a compatibility reader.

## Nominal-pattern graph revision

Graph payload revision 9 adds one canonical nominal-enum pattern to the shared
structural grammar. It carries the declared variant ordinal and either the
variant's recursive payload pattern or no payload. The artifact validator checks
that the ordinal and payload agree with the scrutinee's exact nominal enum
schema. Revision 8 remains rejected without a compatibility reader.

## Pattern-function graph revision

Graph payload revision 10 gives every match declaration an explicit partial
dispatch bit. Ordinary expressions keep exhaustive admission. Pattern-function
calls may publish a partial declaration and fail execution when no ordered arm
matches, without a synthetic fallback value. Revision 9 remains rejected without
a compatibility reader.

## Complete syntax evidence

Clean-tree fingerprints use canonical-clean-tree-v2: an explicit traversal
records every node/token kind name, range, flag value, token text, and child order.
Typed-access fingerprints use canonical-typed-access-v2, with canonical kind
names and separately encoded primitive fields instead of Debug formatting.
The migration changes only the 80 clean-tree and 80 typed-access cells. Accepted,
rejected, and recovery sources, recovery fingerprints, semantic fingerprints,
and required outcomes are unchanged. Regressions reject duplicate, missing, and
unknown certification rules and detect flag-only tree changes even when the
parser's structural hash is unchanged.

## Recovery evidence wording migration

Recovery fingerprints use `canonical-recovery-v3`. Diagnostic messages, label
messages, and fix titles are presentation wording and are excluded. Canonical
diagnostic code, phase, severity, rule, context, ranges, expected/found syntax,
related diagnostic indices, recovery actions, tags, label ranges, fix
applicability, edit ranges, and replacement text remain evidence. Wording
invariance and structural-sensitivity regressions enforce that boundary. Only the
80 recovery-hash cells change in this migration.

## Artifact and execution completion boundary

`phase-2i-semantic-completion.tsv` records artifact readiness separately from
behavior demonstrated on a named target, intentional target unavailability, and
unfinished implementation. The typed FSM artifact is accepted S5 evidence, while
the resident FSM continuation target is intentionally unavailable and remains an
S4 obligation required for S6. The gate is false until every required row records
`behavior-demonstrated`. FSM remains an executable, fail-closed construct; it is
neither structural syntax nor an expected user error.
