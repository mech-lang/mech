# Shared immutable program facts

Read packing and branch analysis query the same native plan definitions and
predecessors. Both now borrow one local facts owner. Its lazy definition
and graph inventories can serve both queries. The owner is dropped when the
two derivations finish. Only immutable program descriptions cross the
derivations. The emitted bytecode, complete metadata and encoder Object
match the fixed reference in all runs.
[Source diff](source.patch.gz) · [Rebuild](rebuild.json)

| Apple M1, prebuilt compiler and fixed support | Separate owners | Shared owner |
| --- | ---: | ---: |
| Source through final benchmark link | 3.75364 s | 3.73605 s |
| Native preparation through object | 1.46692 s | 1.47173 s |

Eight alternating pairs give 0.995785× complete time and 0.998903×
native time. Complete time includes independent parser and link variation.
The native result is effectively flat. Retain the narrow reuse and measure
the number and cost of lazy inventory materializations separately before
extending its lifetime. The initial study verifies byte-identical outputs
and all 162 exact reference worker checks.
[Plan](plan.json) · [Raw trials](matched.json.gz) · [Fixed worker](worker.json)
