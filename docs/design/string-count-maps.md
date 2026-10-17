# String-count maps

The ordinary library operations `map/histogram`, `map/keys` and `map/values`
use canonical map schemas and values. `+> map` admits qualified calls such as
`map/histogram(words)`. `+> map/*` imports unqualified calls such as
`histogram(words)`.

`map/histogram` accepts a rank-two String matrix and counts complete, exact
Strings. It returns a `Map<String,U64>` with checked counts and dynamic
cardinality. Unicode text is preserved; no normalization, alphabet encoding,
packed-window representation or numeric conversion is applied.

The initial `map/keys` and `map/values` overloads accept `Map<String,U64>`.
Both traverse the same canonical UTF-8 key order and return a row matrix of
shape 1×N. An empty map produces 1×0 matrices. Counts remain exact U64 values,
including those above the exact integer range of F64.

The operations have maintained source schemes and pure FullWrite contracts,
managed runtime factories, separate resident artifact binders and native-link
installers under the `map_ops` feature. Memory and sort work are admitted
before owned result construction; a complete candidate is validated before
publication. This ordinary library surface does not establish Cranelift
lowering of an enclosing Mech graph.
