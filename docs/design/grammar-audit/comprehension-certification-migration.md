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
pattern. S4 currently executes scalar Bool/Index/F64 comprehension patterns and
yields, with live generator/filter/join, normalization, work-limit, and failed
publication recovery regressions. Structured pattern execution and FSM lowering
remain unfinished. The FSM witness retains its required executable outcome;
the completion gate continues to fail until that implementation exists.
