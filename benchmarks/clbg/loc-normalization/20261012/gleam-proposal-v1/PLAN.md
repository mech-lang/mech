# Gleam canonical LOC proposal: source plan only

Owner: Pony/LOC agent, coordinating actual authored forms with the Nim/Gleam
source owner. Target Gleam1.19.0, stdlib1.0.5 and measured OTP29.1.1 pending actual
compiler/runtime/source receipts. No fixture, hash, normalization or queue has
run. All actual operations require the common timing mutex and a reviewed,
immutable packet. This plan admits no count or chart endpoint.

Existing `erlang-comparison/20261010/support/loc_probe.erl` uses Erlang AST clause
and block sequence ownership. That is the appropriate model for separately
counted, authored Erlang FFI glue. Emitted `.erl`, abstract forms and `.beam`
cannot substitute for authored Gleam LOC: they may change names, split generated
helpers and carry generated rather than original source positions. Gleam needs
its own tagged grammar and source-owner proof. Shared neutral `foreign_core.py`
accounting may be reused, but not a C-like brace parser or language alias.

Primary pinned grammar evidence:

- Tagged1.19.0 [parser](https://raw.githubusercontent.com/gleam-lang/gleam/v1.19.0/compiler-core/src/parse.rs).
- Tagged1.19.0 [lexer](https://raw.githubusercontent.com/gleam-lang/gleam/v1.19.0/compiler-core/src/parse/lexer.rs).

The original-source reader must distinguish import/type/fn/const/annotation
headers, parameters and labeled argument lists, tuples `#(...)`, lists, generic
types, records/custom constructors, case subjects/patterns/guards/arms and
executable blocks. Gleam statement sequences contain expressions, `let`, `use`
and `assert`; line breaks alone do not define their boundaries. Ordinary `=` is
let binding syntax, and `->` can be a return type or case arm. Neither is a
generic C assignment/control rule. Root's canonical owner policy must be applied
to the actual statement AST or a bounded language-owned parser that reproduces
these distinctions. Unsupported forms must diagnose, never fall through to
physical LOC or silence hidden callback/block bodies.

Concrete authored support already includes `pub type Pattern`, generic opaque
`Vector(value)`, `@external(erlang, module, function)`, typed function prototypes,
`Result(#(Int,Int),Nil)`, List(value), BitArray and qualified external byte/vector
services. Planned benchmark forms include recursive functions, `let`, `case`,
custom Tree Leaf/Node constructors with fresh general references, lists/tuples,
Dict(Int,Int) and explicitly counted generic Erlang IO/regex/process/vector glue.
Each external annotation plus declaration must have a reviewed semantic owner;
all actual scored `.gleam` and authored `.erl` support belongs in each relevant
source bundle. General stdlib/OTP code and generated modules are excluded.

Planned fixture pairs pack/expand two independent let bindings, two function
effects, nested case arm bodies and callback fn bodies. Constructor fields,
labeled arguments, record fields, tuple/list items, case pattern alternatives,
guards and type parameter lists remain structural. Real anonymous function/block
bodies are counted even inside calls or initializers. Positive external/type
fixtures come from the actual support files. Negative fixtures cover unknown
annotations/target attributes, malformed arms, `use` callback ownership not yet
implemented, unsupported bit-array segment modifiers, invalid separators,
unterminated comments/strings and executable blocks hidden inside presumed data.
Gleam comments/literals must preserve positions under the exact tagged lexer;
do not add C nested comments or character syntax by analogy.

Qualification order: actual20-source/support inventory; manual semantic owners;
bounded parser or exact tagged AST extraction tool and meaningful fixtures;
reviewed immutable snapshot with exact dependencies; locked fixture execution;
separate locked original Gleam + authored Erlang counts and complete bundle
reconciliation. Existing Erlang probe reuse also requires its exact runtime/tool
pins, fixtures and no generated module substitution. Never rewrite a benchmark
to evade grammar; preserve failures in additive revisions. Source LOC alone does
not qualify allocation, worker/map ownership, both Pi divisions or full timings.

## Actual20-profile inventory received

The author has supplied `gleam-comparison/20261012/profiles.json` containing all
twenty exact main/helper bundles, with original
`sources/{10 benchmarks}/{compact,optimized}.gleam`. Selected forms include fn,
let, let assert, case arms (some packed), anonymous closures, custom Leaf/Node/Body
constructors and record updates, tuple projections, List spread, Dict, Int/Float
operators, string consts and BitArray utf8/bytes segments. The selected utf8/bytes
segments require positive fixtures and explicit tagged-expression ownership;
they cannot be gated wholesale or rewritten away. More complex unimplemented
segment modifiers still diagnose. Opaque external types/generic signatures and
@external declarations are counted in original support `.gleam`; actual compiled
Erlang glue now has distinct `*_ffi.erl` module names plus `clbg_runner.erl`.
The conflicting-name initial source draft is preserved under
`drafts/initial-ffi-module-names` and is not a compiled profile source substitute.
Generated abstract forms/BEAM and vendor stdlib remain build/runtime evidence.

The author's first preserved compiler attempt passed compact tree cases, then
rejected parenthesized arithmetic grouping in optimized trees. The additive
`revisions/private-v2/sources` proposal replaces exactly six grouping pairs with
legal `{ expression }` blocks across optimized trees/fannkuch/Mandelbrot and both
spectralnorm forms; the other fifteen main sources and all support remain
unchanged. Original grouping expression blocks require explicit ownership:
proved scalar arithmetic grouping stays one expression, while a block containing
independent let/effect/callback/control owners counts those actual bodies. Braces
alone cannot justify either executable-block expansion or a data waiver. Include
actual selected-source positive pairs and hidden-owner negatives; never rewrite
a benchmark to satisfy the LOC grammar. The new sources are still an unsealed,
uncompiled proposal and supply no admitted count.
