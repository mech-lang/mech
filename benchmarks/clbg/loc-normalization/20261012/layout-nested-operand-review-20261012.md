# Bounded nested layout-language value operands

This additive working repair implements the existing approved rule that a
complete expression-only conditional remains one composition, including when
it is a parenthesized operand of a call or operator. It introduces no new
LOC policy and changes no benchmark source, timing, manifest or frozen job.
Twelve fixture methods are drafted; only syntax compilation has run.

The prior `_scalar` guard saw any `if` token and required the whole expression
to start with a conditional. That leaves valid `checksum + (if ... then ...
else ...)` and `size * (if ... then ... else if ... then ... else ...)`
ownership gated. The working repair recursively proves only a complete
parenthesized operand, at bounded depth, before proving the unchanged outer
composition. Separate bindings, assignments, imperative bodies, incomplete
conditionals and wrapped data stay outside this scalar proof.

Exact source evidence, copied from the frozen job15 identities:

| Identity | Source relative to `benchmarks/clbg/` | Source region |
|---|---|---|
| `a8795e9c1a8df0f5bb7b5e9200c0870b10a4f16b46245260143aff8e84f19cee:roc` | `zig-odin-roc-elm-comparison/20261010/sources/fannkuchredux-roc-idiomatic.roc` | line36 checksum plus parenthesized conditional |
| `6a1f36b2ac6e9bffea518fedb4c73bf82e786181b38a0910b02da0007cb3a523:elm` | `zig-odin-roc-elm-comparison/20261010/sources/elm/Fannkuchredux.elm` | line41 checksum plus parenthesized conditional |
| `23109dab7cdae1d776960de561bb798672467653cf353af8f49deb3260e595f9:elm` | `zig-odin-roc-elm-comparison/20261010/sources/elm/Fasta.elm` | lines40/59 stage-size multiplier |
| `4fcf77c033e959328809011b76e7f905611ce59ef44e56ea9597ebb60550d8d2:fsharp` | `remaining-languages-comparison/20261011/sources/fsharp/regexredux-fsharpcore-6.fs` | lines15–19 computation with already proved pure local recursive definition |
| `a355c5804474d2ecd074a8030af671538bc8f9a9fc6e0d4d0c68efce3c194f67:fsharp` | `remaining-languages-comparison/20261011/sources/fsharp/spectralnorm-fsharpcore-4.fs` | line51 array computation with independently proved local conditional binding |

These are motivating source regions, not newly verified source hashes or
admitted counts. An actual future whole-pool pass must establish which gates
clear and reconcile exact source/producer/provenance pins.

The outer nested-value diagnostic now records the exact control-introducer
offsets. It can be discharged only when every such token is inside a strict
child `composed-definition` proof already emitted by the same grammar. A
callback/computation region alone is insufficient and cannot remove this
gate: its independent local definitions may still lack value-versus-control
ownership. No child executable body or diagnostics is suppressed.

Roc spectral-norm and k-nucleotide callbacks therefore remain gated at this
stage. Their bodies need a later general local-definition pass after callable
contexts become executable; no source-specific exception is applied. The
existing pure anonymous conditional callback gate also remains.

The new stage is separate from immutable job18 and has not been queued,
imported, lexed, normalized, hashed, tested or admitted to a chart. Original
policy/audit/source/count/timing records remain immutable.
