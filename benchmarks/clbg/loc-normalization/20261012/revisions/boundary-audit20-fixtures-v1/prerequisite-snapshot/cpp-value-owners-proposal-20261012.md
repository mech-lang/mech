# Bounded C++ source value inventory proposal

New unhooked artifacts: `cpp_value_owners.py`, `test_cpp_value_owners.py`,
and this note. No core, queue, source snapshot, report, timing, frozen job,
chart or Git state was changed. No proposal/lexer/parser module was imported,
no fixture/discovery/hash/audit/build was run. The two Python files passed
syntax-only `compile(source, path, 'exec')` using the pinned Python executable
with `-B -S`; this is not a behavior or fixture pass receipt. The fixture file
contains 62 individually named unittest methods, all unexecuted. The initial
43-method draft was expanded after the audit parent's static review; there
has been no intervening execution or qualification.

## API and source evidence

The audit parent approved the bounded API/grammar plan before implementation.

`collect_cpp_value_owners(cleaned, language, type_environment, *,
declaration_resolver, type_prefix_resolver, macro_token_gates=None,
conditional_regions=(), literal_spans=None)` returns pure proposal data:
`owners`, `type_owners`, `blocked_owners`, `blocked_regions`, diagnostics, evidence,
`source_length`, `source_mask`, written macro directive offsets and
`proposal_only=True`. It calls no lexer, compiler, file reader or normalizer.
All accepted, blocked and region owners have `units=0`; there is no events or
LOC counting API.

The positive declaration callback is the existing ordinary `identify_decl`
resolver, not the new direct object/prototype count helper. The type callback
is the existing exact-offset `_type_prefix`. Type absence, uppercase names,
familiar callees and direct count-invariance profiles are not value evidence.
The callbacks are supplied rather than imported by the proposal module.

The macro callback contract is
`macro_token_gates(text, absolute_start, role=...) -> list/tuple[issue]`.
A caller can close over the actual preprocessor type environment when using
its source-keyed macro token gate. Callback issues are retained as source
evidence. No expansion or guessed compiler flags are introduced.

`resolve_cpp_value_owner(inventory, name, offset, *,
macro_token_gates=None)` returns an exact source role proof, an explicit
rejected `role='unknown'` proof, or `None` for no eligible source owner.
It checks that the query is the exact written complete identifier token,
not an arbitrary nearby/subword offset. It selects the nearest eligible
scope, rejects competing same-scope owners and respects declaration order
and scope expiry. Blocked names and unsupported regions produce rejected
proofs, rather than entering the direct helper's unknown bare-name
invariance path.

The added `type_owners` field copies exact bare-name alias, source aggregate
and explicit imported type visibility/scope metadata from the supplied type
environment. Only the existing reviewed source/import categories are accepted;
malformed spans or unknown categories reject the inventory instead of
reconstructing an owner. The metadata contributes zero LOC units. A visible
type owner conservatively rejects value lookup, including an otherwise visible
outer variable of the same name. This applies at the actual declaration point
and expires at the actual type scope end. Conditional or otherwise flagged
owners remain uncertain lookup evidence, not positive value proof.

The value-only resolver does not assign arbitrary priority to class/type and
value namespaces, even if the value is closer or both owners share a scope.
It returns type/value evidence in the rejected proof. A future joint role
adapter must inspect that evidence and separately prove context-specific type
or value name lookup; choosing a type callback first or a value callback first
is not sufficient. A type-only rejected value query is not a declaration that
the source has no valid type role.

No callback is stored in the returned inventory. Fresh macro proof at use
is therefore an explicit resolver argument. If source writes any define or
undef and no macro callback is provided, collection and use remain gated.
Positive macro status at a declaration never proves status at a later use.

Each ordinary owner records its exact written name bounds, declaration
point, visibility start, scope bounds, owner span/text and positive type
evidence. Array declaration points include the written array suffix.
Function formal declaration points remain the written declarator points;
usable visibility starts inside the positively owned function body.

`literal_spans` deliberately defaults to `None`, which is a missing-provenance
gate. The caller must explicitly supply the complete actual lexical inventory,
including an empty sequence when no string masks exist. The actual working
lexer field is `masked_literal_spans`. Its exact bounds, same-width original
source text, literal kind and mask correspondence are checked. No code guesses
that identifiers named `S` or `SSS` are literals. This first stage gates any
declaration/formal overlapping literal masks instead of admitting a new
literal-expression grammar.

## Bounded grammar

The inventory admits single-row positively typed ordinary independent
declarations in translation-unit, named function and ordinary block scopes.
Pointer/reference/array/ordinary `=` declarators reuse the existing resolver.
Initializer calls or chained assignments do not create additional declared
value owners. Their source remains untouched and retains its existing
runtime/LOC ownership. A packed declaration is admitted together, or none of
its rows becomes a positive owner.

Named function definitions require a written positive return type, ordinary
name, balanced formal group and immediately attached braced body. Formals
require a complete single-row positive type prefix and exactly one named
ordinary declarator each. Commas inside an already proven template type
prefix are structural; unknown angle syntax never receives comma protection.
Defaults, wrapped/unknown formal groups, callbacks, complex function headers
and direct declarator profiles remain gated. One bad formal rejects the
entire formal list, and formerly provisional evidence is marked rejected.
Prototype formal names never become surrounding body values.

Classic `for` initializer declarations have an exact lifetime through their
header and controlled statement. A controlled body can be braced, a
recursively complete classic `for`, or a bounded simple semicolon statement.
The recursive unbraced path is sufficient for the written Mandelbrot shape:

```cpp
int n = atoi(argv[1]);
for(int y=0; y<n; ++y)
  for(int x=0; x<n; ++x)
  {
    std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);
  }
```

The actual upstream source was read in the earlier integration review.
`n` is an ordinary main-body declaration. `y` belongs to the outer unbraced
classic loop and `x` to the inner classic loop, whose braced close bounds
both loops. The generic scope proof requires no benchmark name, dimension
or input-specific knowledge. The fixture is an ownership excerpt, not a
replacement benchmark source or a complete benchmark qualification.

The proposed fixture requests source proofs for the `x`, `y` and `n` leaves
at their written positions inside the compound complex constructor arguments.
The `Z` and `C` direct declarations deliberately do not become positive
inventory rows: their object/prototype count profile is separate from source
value/data-member certification. Existing unknown if/else owners elsewhere
in the excerpt remain blocked regions. The fixture describes the intended
leaf proofs; it has not run.

## Explicit exclusions and conservative gates

Unsupported imperative/callback/control bodies, function qualifiers,
wrapped declarations/formals, formal defaults, range-for, labels/conditional
unbraced bodies and uncertain delimiter/lifetime ownership remain gated.
No statement is silently skipped to produce a value proof from the absence
of a type. A possible unknown declaration can block later surrounding value
ownership, including otherwise hidden packed names.

An unknown loop body blocks the loop's potential binding through the enclosing
scope and records an explicit blocked region; it does not guess an unbraced
lifetime. Conditional control ownership and active/uncertain macro ownership
of the complete loop/function header also block the body scope proof.

Aggregate, member, namespace and template body ownership is not implemented.
No member is fed into constructor data-field certification. Unscoped enum
exports and unresolved aggregate instance suffixes conservatively block
surrounding subsequent value lookup because they can hide an outer binding.

Any unsupported region rejects even a visible outer value query within
that region. Positive declarations inside a previously blocked owner do not
become usable inventory rows. This is intentionally conservative; its
diagnostics do not certify universal C++ semantic ownership.

An active macro statement can have unknown expansion/ownership through its
scope, so a later unconditional undef does not retroactively repair that
unsupported statement. Conversely, a define/undef with no prior unknown
executable invocation can permit a later name use after fresh source macro
proof. Macro issues are identified by their actual callback provenance,
not by a required diagnostic-kind spelling.

After static review, macro ownership is checked on every actual statement
envelope before dispatch, including skipped `using`, `typedef`, `return`
and other `_NON_DECL` statements. Function/control/scope headers are checked
separately from body statements. Unsupported aggregate/control bodies receive
a bounded macro-only envelope sweep that admits no fields, formals or value
bindings. Thus an active expansion in a skipped statement or scope header
cannot leave subsequent surrounding value lookup falsely usable, and an
expansion later in a body does not retroactively erase an earlier proof merely
because it shared a function's full source span. The supplied macro callback
must use actual protected literal provenance so mask letters cannot become
active macro names.

## Unexecuted fixture coverage and integration boundary

The 62 named methods specify ordinary packed names and exact offsets,
initializer source preservation, declaration order, array points, nested
scope expiry, competing owners, named formals, prototype non-escape,
all-or-nothing formal rejection, wrapped/default/unknown formals,
callback/direct-profile rejection, exact literal provenance versus actual
`S` identifiers, missing lexical metadata, macro owner/header/use/statement
gates, fresh undef proof, conditional owners, braced/unbraced/nested classic
loop lifetime, actual Mandelbrot compound leaves, unsupported range/if loop
bodies, unknown packed types, aggregate exports and exact query tokens. The
review additions contrast inner aliases/classes against outer values, type
declaration order and scope expiry, same-scope coexistence, conservative
inner-value/outer-type competition, conditional types, valid EOF visibility,
malformed type metadata/category rejection, unrelated alias/value lookup,
macro versus nonmacro skipped using/typedef/return, missing macro callbacks,
macro-only aggregate sweeps and control header scope uncertainty.
They invoke the actual lexical/type environment dependencies only during a
future locked fixture run. They do not read source files or produce LOC totals.

The ordinary constructor member resolver must remain unchanged. A future
integration may construct this zero-unit inventory from the actual mask and
positive type environment, then combine positive plain type proof and exact
value proof for the direct helper's role callback with a reviewed joint role
ownership decision. Rejected proof flags must remain rejected. The owner
consumer must still check active macros at called
names and builtin/type tokens because those helper paths bypass a value-role
callback.

No core hook or queue registration was added. Review the module and fixtures
statically first; freeze all dependencies with the prior direct-declaration,
literal provenance and macro-owner proposals; then run fixtures and source
review only in a parent-owned common-mutex job. No benchmark or chart
admission is implied by syntax compilation.
