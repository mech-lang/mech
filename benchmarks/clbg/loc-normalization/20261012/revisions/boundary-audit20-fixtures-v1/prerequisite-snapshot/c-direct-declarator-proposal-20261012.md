# Bounded C++ direct declarators: static proposal

This is an unhooked proposal. Only `c_direct_declarators.py`, the unexecuted
`test_c_direct_declarators.py`, and this note are new. No normalizer, type
environment, queue, benchmark, frozen input, template ownership file, or
integration patch was edited. No parser was imported or invoked, no lexer or
normalizer ran, and no fixtures, build, timing, hash, or Git command ran.
Python source compilation is the only validation; it checks syntax,
not fixture behavior or a LOC result.

## Source evidence

The actual upstream files were read as text:

- `benchmarks/clbg/spectralnorm/upstream/spectralnorm-gpp-1.cpp` writes
  `vector<double> u(N), v(N);` and separately
  `vector<double> v(u.size());`. Its explicit `<vector>` include and
  `using namespace std;` ownership are the template agent's separate work.
- `benchmarks/clbg/mandelbrot/upstream/mandelbrot-gpp-3.cpp` writes
  `std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);`.
  The two inner commas are argument separators. The outer comma separates
  two independent declared names. Its `<complex>` ownership remains separate.
- The existing `c_type_environment._declarators` recognizes named
  pointer/reference/array shapes and ordinary `=` initializers, then rejects
  a direct `(...)` tail. This proposal does not replace or edit that API.

The proposed binding result for each packed target is two names. Their pure
written value composition and formals contribute zero extra declaration
units. This is a static grammar policy claim, not a measured source LOC total.

## API contract

`analyze_direct_decl(segment, *, offset, type_prefix, context='declaration',
role_resolver=None, type_id_resolver=None, legacy_declarators=None,
literal_spans=())` returns an envelope containing `declaration`, `diagnostics`,
`evidence`, `matched`, and `proposal_only=True`. `declaration` is either the
current count-compatible dictionary or `None`. `identify_direct_decl` returns
only that dictionary or `None`; a diagnostics envelope cannot accidentally
enter the existing counter as a truthy declaration.

The supplied `(end, spelling, evidence)` type prefix refers to the stripped,
semicolon-less code, exactly as in the current declaration API. It must come
from the caller's positive type proof. The helper does not collect/import or
guess named types and does not use capitalization as evidence. Ordinary and
imported-template proofs may both be supplied. Original segment/source
offsets are retained in every name and argument record. Explicit conditional,
dependent, macro, unresolved, and shadowed proof flags remain gates.

An optional `role_resolver(name, absolute_offset)` returns a source proof with
`role='value'` or `role='type'`. An optional
`type_id_resolver(text, absolute_offset)` returns `(end, evidence)` for a
source-proven type-id prefix; its end is relative to that unchanged text.
This provides a route for template cast/formal types without importing the
template proposal or promoting a spelling. Rejected callback proofs cannot
fall back to an unresolved bare-name alternative.

At least one named direct suffix must be present. The caller may use the
existing ordinary fallback only when `matched` is false **and diagnostics
are empty**. Every diagnosed result remains a gate, including unbalanced
input. All packed declarators fail together if one part fails. No partially
accepted binding count is returned.

Non-direct parts delegate individually to the unchanged existing
`_declarators`; their names, ordinary initializer strings, and offset API are
preserved. Ordinary RHS assignments are not suppressed or replaced. Written
work in delegated RHS call-argument scopes is checked and remains gated.
RHS executable callbacks require separate header/body/closer ownership.

## Independent-unit policy and ambiguity

Each admitted object or named function prototype contributes one independent
declaration binding. Argument and formal syntax is structural and contributes
zero additional units. Empty `f()` is a prototype. A complete bounded formal
list or value-expression list owns the entire direct suffix; trailing arrays,
functions, cv/noexcept/attributes, and other unhandled tails remain gates.

`f(U())` and `f(U(value))` can match both bounded prototype/formal and object
composition profiles. The helper records both profiles and their invariant
one-name/zero-argument unit result. It does not resolve most-vexing parsing
into a new executable owner. `f((U()))` has the grouped value profile only.
A missing resolver for a single bare argument is recorded as a possible
bare-value/unnamed-type ambiguity because both profiles have the same units
and gates. An unresolved name embedded in other expression grammar, an
unknown multiword formal, or an explicit rejected owner proof remains gated.

The existing counter uses
`max(1, written_assignments + uninitialized, declaration_bindings)`.
Its `initialized` field means a top-level `=` initializer, rather than every
C++ initialization form. A direct name therefore has `initialized=False` and
adds one to `uninitialized`; `direct_initialized`, `initialization`, and
`direct_role` retain its direct/prototype profile. In
`T a(0),b=c=0;`, the direct name adds one alongside the two ordinary written
assignment owners, preserving three independent bindings. The fixture with
`T a(0),*p,&r=value,arr[2],b=c=0;` preserves five declared names and the
existing additive six-unit counter expression. The proposal does not count
or replace those written assignments itself.

## Bounded argument grammar and gates

The helper reads balanced scalar, call, member, index, proven C/C++ cast,
unary/binary operator, and ternary composition. Call spelling is agnostic;
there is no constructor/function allowlist. This is syntactic composition,
not a claim that a called function has no runtime effects. Every unresolved
value leaf needs a supplied role proof unless it qualifies for the single
bare-name count-invariance case described above.

Written assignments, compound assignments, alternative-token assignments,
updates, comma/semicolon sequences inside grouping or indexing, callback
bodies, control/declaration forms, braced data arguments, unsupported formal
callback declarators, template-callee argument roles, malformed scalar
spellings, and unknown argument/formal roles remain explicit review gates.
Defaults such as `f(int n=0)` remain gated even though a future proven formal
profile might classify them structurally.

Every multiline argument/formal group is gated in this phase, including
scalar formatting. This matches the existing constructor member helper's
conservative row rule. Packed names on separate declaration rows can still
retain their independent name count when each argument group is single-row.
No source row, literal/data wrapper, or written closer is collapsed.

Single-row literals require caller-supplied exact absolute `literal_spans`
with kind `literal`, original `source_text`, nonoverlapping bounds, and exact
`S`/whitespace mask ownership. The helper creates no mask. Literal ownership
takes precedence over the same spelling in name/type resolver results, so
an owned `SSS` token cannot become a type prefix, formal/cast type, binding,
member, qualified callee, or literal-backed callee name. Data spans and
wrapped literal spans remain gated. The current lexer does not expose this
complete literal metadata contract, so it is an explicit future integration
requirement rather than an automatic pathway.

## Static fixtures and validation boundary

The fixture source includes normal unittest discovery with individually named
fixture methods. Its imports and methods have not executed. A future worker
must import/discover it under the common timing boundary; the wrapper then
loads `analyze_direct_decl`, `identify_direct_decl`, and the unchanged existing
`_declarators`. The supplied-callback
`exercise_direct_declarator_fixtures` entry point remains available for the
additional API/offset/ownership/additivity assertions.
Fixtures include the exact measured-candidate declaration spellings,
meaningful packed/expanded declarations, prototypes and most-vexing forms,
source offsets, legacy additive initialization, callee-agnostic calls,
literal/type ownership conflicts, rejected proof flags, and flat-cast
recursion bounds. They also specify assignment/update/sequence/callback/
control/multiline/template-role/tail gates. These assertions have not run.

Body, function/control header, executable closer, canonical-event suppression,
type-shadow collection, imported-type ownership, and benchmark admission
remain outside this proposal. The approved policy continues to require one
unit for each function/control header and every written executable close;
this helper emits neither header nor closer events.

The API/count questions were sent before final code. The audit parent
confirmed the analyze envelope, bare-name invariant ambiguity, gated formal
defaults, gated **all** multiline argument groups, and the legacy additive
field interpretation. A separate static reviewer identified resolver-flag,
literal/type precedence, nested-formal, scalar, and recursion-bound failure
paths; repairs and adversarial fixtures are part of the unexecuted proposal.
The final review also caught qualified delegated lambda headers and template
commas that could otherwise masquerade as two comparison arguments. Capture
bodies are gated without parsing/suppressing their executable owners. A
possible template callee with comma-bearing angle syntax is explicitly gated
before argument splitting, even when every identifier has value evidence;
known C++ cast type frames defer to the supplied type-id proof instead.
The audit parent requested the ordinary unittest wrapper because callback-only
fixtures would otherwise silently run zero tests under frozen discovery.
