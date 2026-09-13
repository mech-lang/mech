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
Comprehension composite bindings/yields, computed pattern blocks, composition with
nested comprehensions, and the FSM resident continuation owner remain unfinished.
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
typed revision-six artifact rather than an ordinary source operation. Revision 6
adds the FSM body variant; revision 5 remains the closed nested-match graph grammar.

## FSM graph revision

Graph payload revision 6 closes the representation introduced by typed FSM bodies.
The 51 artifact-bearing semantic witnesses were regenerated with the revision-six
producer; their semantic hashes changed because the complete artifact bytecode is
part of each witness fingerprint. The source programs, dispositions, required
outcomes, and non-graph artifact sections are unchanged. Revision 5 remains rejected
without a compatibility reader, including for externally supplied FSM artifacts.
The committed 20-fixture bytecode-v1 corpus was also regenerated and checked across
five fresh producer processes. Seventeen compiler-produced, graph-bearing fixtures
changed at identical byte lengths; the three constructed scalar, matrix, and
composite fixtures remain unchanged.

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
