# Direct declarator integration review

This is a static, read-only review of the working proposal and integration
surfaces. No module was imported, no lexer/parser was invoked, no fixture or
source audit ran, and no queue, source, report, timing, core module, frozen job,
or chart was modified. This note is the only new artifact. The active frozen
job 18 does not contain this proposal. The 44 methods in
`test_c_direct_declarators.py` are unexecuted assertions, not a pass receipt.

## Result

The helper can provide a bounded independent-binding count after a positive
type-prefix proof. It must be integrated through an analysis envelope, rather
than replacing every existing `identify_decl` call. Immediate unrestricted
integration is unsafe because the new lexical mask provenance remains
unqualified and unconsumed, and the type environment does not inventory
arbitrary value bindings. Whole declaration statement ownership is required before
multiline forms can be admitted by a counter that currently scans physical
rows.

An initial useful integration can admit single-row, positively typed direct
forms with numeric/call composition or the approved bare-name invariant
profile. It must retain literal overlap, unknown expression leaves,
conditional/macro/type shadow, multiline statement, executable argument and
unresolved ownership gates. A source that retains any gate is not qualified
for chart admission.

## The helper's actual contract

`analyze_direct_decl` requires `(end, spelling, evidence)` for the stripped,
semicolon-less segment and returns `declaration`, `matched`, `diagnostics`
and `evidence`. Only a non-null declaration without diagnostics is a binding
proof. Every part of a packed declaration is checked. An unsupported part
prevents the entire declaration result; accepted rows do not leak as a
partial count.

The current API explicitly permits ordinary fallback only if `matched` is
false **and** diagnostics are empty. Delimiter/type/input diagnostics can
occur before `matched` becomes true. A caller that checks only `matched`, or
uses `identify_direct_decl(...) is None` to choose ordinary fallback, loses
this distinction and is unsafe.

Direct rows use `initialized=False` because that existing field means a
written top-level `=`. They contribute one to the existing `uninitialized`
term; `direct_initialized`, `initialization` and `direct_role` record their
actual bounded initialization/prototype profiles. Preserve this interpretation:
`T a(0),b=c=0;` has one direct name plus two written assignment owners, giving
three units under the existing additive counter. Changing direct rows to
`initialized=True` would undercount that mixed statement.

The helper creates neither executable header/closer events nor masks. Its
object/prototype ambiguity is a claim that both bounded profiles have one
name and zero argument/formal units. It is not a proof that an ambiguous name
is a data member or a general value operand.

## Exact safe counter seam

Keep the existing ordinary `identify_decl` resolver available for its current
field, formal and type-shadow consumers. Add an explicit C++ declaration
analysis API or a normalizer-local adapter with this control flow:

1. Prove independent declaration context from the exact effective contexts.
   Parameters, call arguments, control headers, data groups and already owned
   executable headers must not be sent as independent declaration statements.
2. Preserve the input string and absolute beginning offset. Calculate the
   leading whitespace once, then obtain `_type_prefix(code, environment,
   offset + leading)` on the unchanged stripped, semicolon-less code.
3. Require the complete positive type proof. A possible template spelling,
   uppercase name, raw declaration regex or absence of a value shadow is not
   a type proof. The working standard-template contract may supply the proof
   after its own source/order/namespace/shadow and macro ownership gates.
4. Pass the original segment and original offset, with that exact prefix, to
   `analyze_direct_decl`. Pass the unchanged `_declarators` callback only for
   its non-direct parts. Register returned diagnostics and evidence at their
   already absolute offsets, without rebasing them a second time.
5. If diagnostics exist, keep the source review-gated and do not invoke any
   ordinary or heuristic declaration fallback for this segment. A provisional
   physical count may remain in a report only while explicitly gated.
6. If a declaration exists, use its `declaration_bindings` and `uninitialized`
   in the existing assignment counter. Do not suppress ordinary RHS assignment
   tokens, replace them with one expression event, or change header/close
   ownership.
7. If no declaration exists and the envelope has no diagnostics and is not
   matched, use the unchanged ordinary resolver and then the current possible
   declaration review path.

The existing physical-row counter is at `normalizer.py`'s independent body
segment loop. Its generic `_declaration_bindings` result is computed before
the positive C/C++ resolver. A diagnosed direct form must therefore clear or
ignore that provisional heuristic binding result; otherwise the rejected
form can still acquire units from the earlier regex.

## Literal provenance: new unexecuted metadata and conservative first stage

When this review began, `lexical.mask_source` returned `cleaned`, diagnostics,
lexical adjustments and semantic imports, but discarded the spans of masked
string tokens. The audit parent has now added C-family-only
`masked_literal_spans` to the mutable module, with eight named unexecuted
fixtures in `test_c_literal_spans.py`. I read that actual addition as source;
it records checked `Literal.String` mask regions with exact `source_text`,
bounds, `token_kinds` and `provenance`, merging contiguous string-token
fragments. Include filenames retain their separate `semantic_imports` owners.
No lexical fixture or source replay has executed for this addition, and the
direct helper does not yet consume it. Frozen job 18 remains unchanged.

A legal identifier spelled `S` or `SSS` cannot be distinguished from a literal
mask by looking at cleaned spelling. Use the actual new interval inventory,
never a mask-letter heuristic.

Passing `literal_spans=()` when literals are present is not acceptable. For
example a masked argument `SSS` can enter the approved unknown bare-name
invariance path; it can also collide with an actual type or value name. The
helper's literal overlap protections work only if ownership was supplied.

The new mutable lexical addition follows the needed provenance design during
the existing checked C/C++ token pass, without another lexer invocation. Its
qualification and later consumer must retain these constraints:

- Record the exact start/end, original `source_text` and token category for
  every successfully checked `Literal.String` span that is actually masked.
  Offsets are Python string character offsets, as specified by `lexical.py`.
- Merge only directly contiguous string-token spans into one literal mask
  region, retaining original characters, width, whitespace and newline
  positions. A fragmented escape must not become a separate expression term.
- Keep include filename owners separate. They are non-executable include
  provenance and must not become ordinary expression literal owners.
- Record ambiguous prefixed/raw/adjacent literal ownership as unresolved.
  Do not infer a complete C++ literal merely from a run of mask letters or
  invent a parser/lexer ownership property absent from the actual token pass.
- A token coverage mismatch, overlapping spans or source mismatch remains a
  lexical gate. No unmatched token receives a successful literal proof.

For the first stage, any positively typed direct-form statement overlapping
one of these exact literal mask regions can receive a
`cpp-direct-literal-data-owner-review` gate before ordinary fallback. This
requires no literal expression admission or source rewrite and preserves
every existing literal/data row. The overlap test is on recorded intervals,
never on whether the cleaned text contains `S`.

Later admission may use the helper's exact single-row literal contract:
`kind='literal'`, absolute nonoverlapping bounds, original same-width
`source_text`, exact mask correspondence and no newline. A contiguous region
is provenance of masking; complete literal expression ownership still needs
the bounded lexical/syntax proof. Adjacent literals separated by whitespace,
prefixes and raw literals can remain gated until that proof exists. Do not
split a wrapped literal into admitted single-row atoms.

## Type and value role callbacks

The existing `_resolve(environment, name, offset)` proves visible types from
positive aliases/imported/source aggregate owners and checks recorded
shadows. It does not return the direct helper's `role` field. A type adapter
may return a copied, positively resolved plain type entry with `role='type'`,
while retaining all conditional/dependent/macro/unresolved/shadowed flags.
A bare class-template contract without a proved specialization is not a type
ID. Rejected ownership must return an explicitly rejected proof, rather than
`None`, where the invariant bare-name path would otherwise conceal the gate.

`environment['shadows']` is filtered to names colliding with type owners.
It is not a general inventory of variables/formals and not evidence that
every unresolved name is a value. Never return `role='value'` simply because
`_resolve` returned `None`.

A general future `value_bindings` inventory needs source declaration/formal
owners, exact written name bounds, visibility start, enclosing scope end,
conditional status and macro status. A source-position lookup must select the
actual visible owner and reject unknown or competing scope ownership. It can
reuse the bounded ordinary declarator, function definition formal and loop
header grammar, but must inventory all written names rather than the current
alias-collision subset. Constructor/lambda/formal scopes and unbraced nested
loops require their own lifetime proof; a nearby spelling or enclosing brace
alone is insufficient. Such an inventory contributes no LOC units.

Until this inventory exists, use the helper's explicit invariant bare-name
profile only where approved and keep unresolved compound leaves gated. The
actual spectral-norm `vector<double> u(N), v(N);` can use the invariant `N`
profile without inventing a value owner. Its `v(u.size())` is a bounded
callee-agnostic composition. In actual Mandelbrot, `C(2*(double)x/n - 1.5,
2*(double)y/n - 1.0)` needs source value proofs for `x`, `y` and `n`; the
current type-shadow inventory cannot supply them. The source writes `n` in
the main body and `x`/`y` in nested loop headers, including an unbraced outer
loop. Those declarations are the general inventory's useful test case.

Active macro ownership also needs a consumer-level token gate. The helper's
callee-agnostic call path deliberately does not resolve the called name,
and builtin type roles are recognized internally. Consequently rejected
`role_resolver` results alone cannot block a macro affecting those tokens.
Check actual active macro evidence at the written type/callee/operand token
positions before admission; absence of a `macro` flag on a previously
collected type entry is not proof that no macro is active at its use.

The optional `type_id_resolver` needs a separate bounded type-ID adapter.
Directly returning `_type_prefix`'s first element will leave trailing
pointer/reference/cv syntax unconsumed because `_type_prefix` proves only the
base prefix. Either let the helper's own plain type-ID grammar use the type
role adapter, or extend a positively proved base/template prefix through
the supported type-ID suffix grammar at exact offsets. Unsupported suffixes
must remain gates.

## Shadow and member ownership must remain separate

`cpp_initializers._member_declarations` currently trusts every declarator
returned by its resolver as a direct nonstatic data member. Globally replacing
that resolver with the new direct helper would admit `int m();` as a row and
could make a constructor initializer `m(0)` appear to initialize a data field.
The helper explicitly labels that row `prototype`; its count-invariant
object/prototype rows are not data-member certification. Preserve the current
member resolver, or require separate positive direct data-member ownership
before admitting any new row to the constructor member inventory.

Formal/control consumers likewise must retain their structural context.
The independent statement helper must not reinterpret a function formal
declarator as an independently counted binding.

For ordinary declaration shadows, direct names can hide source/imported
type names even when the direct statement's count remains gated. A failed
argument role proof must not leave a later type use falsely unshadowed.
Add source-keyed blocked shadow ownership for that case rather than inventing
a value binding. For successful rows, object versus prototype declaration
points and lifetime need their grammar-specific proof; the helper's equal
LOC count does not settle that semantic detail. Its prototype formal names
must never leak into the surrounding body scope.

Do not create competing shadow records through both the new ordinary path
and `collect_named_type_shadows` without reconciling their exact owner and
offset provenance. Revalidation of source aliases after final shadows must
continue to run on their original RHS offsets.

## Whole statements and preservation of wrapped rows

The helper accepts a supplied full statement such as `T a(0),\n b(1);`, but
the normalizer currently calls declaration resolution per physical row.
Running the helper only there can omit later names, create a gate on the
trailing comma of the first row, or admit an incorrect physical fallback on
subsequent rows. Wrapped type prefixes such as `std::vector<\nint\n> a,b;`
are a related known gap in the working standard-template stage.

A statement owner must span the whole semicolon-terminated declaration with
balanced grammar and exact source contexts. It must exclude owned
formal/control/header/alias/data spans and distinguish declaration delimiters
from executable bodies. Until that owner exists, retain a source-keyed gate
for a potential/proven direct declaration crossing rows; do not claim whole
source qualification from the helper's packed/expanded unit fixtures.

Keep the helper's existing gate for every multiline argument/formal group.
Do not mask an admitted multiline declaration as a single canonical event
without separately preserving literal/data wrapping. The current counter's
standalone literal/data row fallback is part of the approved policy, not
noise to be removed by a new direct-declaration span.

## Required locked integration fixtures

These are future requirements, not executed results:

- Exact template/header ownership plus actual spectral-norm packed/expanded
  names; actual Mandelbrot should stay role-gated until the new source value
  inventory proves its nested-loop/body variables.
- `T a(0),b=c=0;` and mixed pointer/reference/array/direct forms preserve the
  existing additive assignment count, exact leading/part whitespace and name
  offsets.
- A diagnosed matched direct form, and a diagnosed pre-match malformed
  delimiter form, cannot use ordinary or earlier heuristic fallback.
- Literal arguments gate by exact mask provenance while identically spelled
  source identifiers remain unaffected. Include literals stay separate;
  fragmented escape tokens are contiguous provenance; multiline/prefixed/raw/
  adjacent unresolved literals retain gates and written rows.
- Unknown compound expression leaves cannot acquire value role from absence
  of a type; known source values respect declaration order, nested expiry,
  formal/control ownership, conditional and macro evidence.
- A prototype cannot enter the constructor data-member inventory; a
  prototype formal cannot become a body value owner or independent LOC unit.
- Direct declaration names colliding with aliases/imported template names
  block later type proofs even when their arguments are review-gated;
  otherwise valid outer ownership resumes after proved scope expiry.
- Cross-row direct declarations and wrapped template prefixes remain gated
  until full statement ownership, while ordinary wrapped literal/data rows
  keep their existing physical-unit treatment.

No executable proposal patch is supplied: the safe seam needs qualification
of the new lexical provenance and an envelope consumer before a core hook can
be concrete. Applying a narrow hook without those prerequisites would make a
successful simple fixture conceal unsupported source cases. The next frozen
qualification should include the 44 helper methods plus separately named
integration fixtures and all prerequisite modules, under the common timing
mutex. Only its terminal receipt and source review can support admission.
