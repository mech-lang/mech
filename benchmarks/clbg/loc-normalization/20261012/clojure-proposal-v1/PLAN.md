# Clojure canonical LOC proposal: source plan only

Owner: Pony/LOC agent, coordinating authored forms with the Crystal/Clojure
source owner. Target Clojure1.12.6, with the author's exact dependency/runtime
receipts still pending. No fixture, reader, hash, normalization or queue has run.
All actual operations require the common timing mutex and an immutable reviewed
packet. This plan admits no count or chart endpoint.

The existing `lisp-racket-comparison/20261010/loc_final.py` contributes the
reader-aware approach: preserved source offsets, explicit form trees, body and
binding sequence ownership, literal arguments distinct from executable bodies.
It is not a Clojure parser and must not be aliased. Its generalized `list` node
loses list/vector/map identity, Common Lisp character/reader forms differ, and
its macro body table does not establish Clojure ownership. Existing qualified
`foreign_core.py` accounting may be reused after the new reader supplies actual
owners, but no C or generic braced classifier is a language grammar substitute.

Primary grammar evidence:

- Tagged1.12.6 [LispReader.java](https://raw.githubusercontent.com/clojure/clojure/clojure-1.12.6/src/jvm/clojure/lang/LispReader.java).
- Official [reader reference](https://clojure.org/reference/reader).
- Official [special forms](https://clojure.org/reference/special_forms).

The Clojure-owned reader will preserve list/vector/map/set, exact literal spans,
quoted forms, char escapes including delimiter characters, string versus regex,
commas as whitespace, line comments, `#_` recursive next-form discard and metadata
with type hints attached to the subsequent owner. Delimiter kind mismatch,
unterminated literals, unknown dispatch, malformed metadata/discard and reader
conditionals must diagnose. The parser must not execute user readers or reader
evaluation. Syntax quote/unquote and tagged literals require an explicit bounded
review or remain gated. Reader-discard is not the same as the `comment` macro.

After all20 scored namespaces/support files are authored, freeze an explicit
inventory of the selected source forms. Candidate owners include `ns` semantic
imports; `def`/`defonce`; `defn`/`fn` including optional doc/metadata and each
arity; `let`/`loop` independent initialized binding pairs; `do`, `when`, branch
bodies and `try`/`catch`/`finally`; `recur`; `set!`; `dotimes`/`doseq`; selected
`deftype`/`defrecord`/`reify` method bodies and Java interoperability calls. Use
actual tagged core macro definitions to establish macro body positions. Unknown
macros, generated forms and unusual destructuring owners must diagnose rather
than silently count argument/data vectors as code or hide executable callbacks.
Every authored helper, declaration, class/type field and ABI binding remains in
the source bundle. Generated Java bytecode/classes are runtime/build evidence,
not replacement source LOC; vendor Clojure/JVM libraries remain excluded.

Meaningful planned fixtures cover packed/expanded independent top-level forms,
two `let` initialized bindings, three `do` body effects, multi-arity function
bodies, nested callback `fn` bodies, `reify` methods, Java field assignment and
quoted data. Structural argument/vector/map/set commas must not inflate counts.
Negative fixtures put executable `do`/`fn`/`set!` inside presumed literal/argument
owners, use mismatched delimiters, malformed #_, unknown tagged readers and
reader conditionals. Reader comments/strings/regex/character `\;` and `\)` must
keep exact positions. Type hints must neither hide initialized bindings nor
become extra executable expressions.

Qualification order: source inventory/manual semantic review; bounded parser and
fixtures authored; immutable snapshot with exact shared dependencies; root
packet review; locked fixture execution plus before/after dependency reconciliation;
actual original-source counting only after fixture success and source-owner
review. Never rewrite a benchmark to evade unsupported grammar. Preserve failed
receipts and create additive revisions. Native work/timing proofs are separate
requirements and cannot be replaced by a reader fixture or schema assertion.

## Actual source inventory received

The author has now supplied original
`clojure-comparison/20261012/sources/clbg/{binarytrees,fannkuchredux,nbody,spectralnorm,mandelbrot,pidigits,fasta,revcomp,knucleotide,regexredux}/{compact,optimized}.clj`
and `support/clbg/support.clj`, covering all twenty scored namespaces.
Read-only review of support, optimized trees, compact K and compact Pi confirms
the concrete forms: `ns` with `:require`, `:import`, `:gen-class`; `def`, `defn`,
variadic `-main`; `let`, `loop`, `recur`, `when`, `when-not`, `if`, `if-let`, `cond`,
`case`, `doseq`, `dotimes`, `for`, `do`; Java constructors/member/static calls; primitive and reference type hints including `^objects`,
`^BigInteger`, `^String`, `^StringBuilder`, `^HashMap`; vector destructuring; regex
reader literals and `#(...)` anonymous functions. Support has explicit `fn` and
`reify Callable` with a `call` method, plus `try`/`finally`. These callback and
method bodies are executable owners even when nested inside `.submit` arguments.
No custom macros, tagged readers, `deftype` or authored `.java` are currently used.
Binding metadata belongs to the following binding, never a second binding.
Packed six-value loop bindings and four independent Pi division-related let
bindings must survive normalized counting. Main and exact authored support are
both counted per profile; generated class inventory is provenance only.
