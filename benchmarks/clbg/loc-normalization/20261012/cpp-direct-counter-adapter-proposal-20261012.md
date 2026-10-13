# Unhooked single-row direct declaration counter adapter

`cpp_direct_counter_adapter.py` and
`test_cpp_direct_counter_adapter.py` are separate proposal files. No production
normalizer, queue, constructor/member/formal/type-shadow collector, benchmark
source, report, timing, frozen job or chart was changed. There are 49 named
unexecuted fixture methods. The only preparation verification is syntax-only
`compile` using Python `-B -S`; no dependency imports, lexer/parser calls, test
discovery, fixture execution, builds or hashing occurred.

## Proposed API

```python
analyze_cpp_direct_counter_segment(
    segment,
    offset=absolute_segment_start,
    source_mask=unchanged_lexical_mask,
    contexts=exact_effective_contexts,
    statement_owner=independently_proved_whole_statement,
    type_environment=source_type_environment,
    value_inventory=source_value_inventory,
    literal_spans=actual_masked_literal_spans,
    protected_spans=existing_header_formal_field_alias_data_spans,
)
```

The envelope contains `declaration`, `counter_terms`, `matched`, `diagnostics`,
`evidence`, `fallback_allowed`, `ordinary_fallback_used`, `shadow_obligations`
and `proposal_only`. A diagnosed result exposes no declaration or counter
terms. The adapter creates no canonical events and changes no source bytes.
Its copy of type metadata keeps even existing prefix-probe diagnostic writes
private; the caller's aliases, shadows, diagnostics and value inventory remain
unchanged. Recorded diagnostics/evidence use their original absolute offsets.

Production integration is deferred. The existing direct helper and value
inventory must first qualify under the common lock. This adapter additionally
needs a qualified complete-statement ownership provider and source review of
the exact entries it would affect. Passing fixture metadata to this API does
not establish that production owns the same statement spans.

## Complete statement and context contract

The owner must include:

```python
{
    "kind": "independent-semicolon-statement",
    "start": absolute_start,
    "end": absolute_end_after_semicolon,
    "terminator_offset": absolute_end_after_semicolon - 1,
    "ownership_proven": True,
    "start_boundary": positive_preceding_boundary,
}
```

Its span must come from an independent source ownership pass. This adapter
does not produce it by taking one physical row, searching for a semicolon or
guessing a type from a spelling. The unchanged segment must cover the whole
supplied statement, allowing only leading/trailing whitespace and omission of
its final semicolon. A complete span containing a newline is gated. Taking
only a tail row of a larger owner is also gated.

The first surviving token and the final semicolon must have identical
independent executable or translation-unit contexts. Formal, argument,
control, data and group contexts after the latest executable scope are
excluded. A second independent semicolon inside the supplied span is gated;
group-owned semicolons do not become new statement owners. Existing protected
header/formal/field/alias/data overlaps are excluded with exact bounds.

The preceding boundary must be an exact source start, independent statement
terminator, executable opener/closer, or a supplied previously owned
header/directive span. Whitespace between that boundary and the statement is
checked. An arbitrary expression, operand, annotation or physical row ending
is not an accepted boundary. These checks validate the supplied owner's
interval and context; they do not independently certify a claimed production
ownership pass.

## Counter and fallback contract

The type prefix is obtained from the unchanged stripped, semicolon-less code
at `offset + leading_whitespace`. It must be the complete positive source,
header-contract or builtin type proof, including all existing namespace,
macro, order, scope and shadow requirements. Unknown uppercase spellings,
template-like text and an absent value shadow are not type proofs.

After a positive prefix, the existing direct helper receives the original
segment/offset, the exact prefix, source role/type-ID callbacks, the unchanged
ordinary `_declarators` callback and actual literal intervals.

Ordinary fallback is allowed only when the direct envelope has
`matched=False` **and** an empty diagnostic list. A diagnostic produced before
`matched` becomes true also forbids fallback. A matched, diagnosed or partial
result cannot leak earlier heuristic binding units. A future normalizer caller
must clear/ignore its preliminary `_declaration_bindings` result whenever this
envelope is gated. If the positive prefix is absent, no direct/type guess is
made; the existing ordinary resolver and the caller's possible-declaration
review remain responsible for that unsupported segment.

Both ordinary fallback branches also retain every fresh diagnostic emitted by
the ordinary resolver into the private type environment. Such a diagnostic
clears the returned declaration, counter terms and fallback permission, even
when that resolver returned provisional binding counts. The historical
`ordinary_fallback_used=True` flag records the invocation that produced the
gate; it grants no further fallback or count. One injected-resolver fixture
checks both the absent-prefix and positively typed unmatched-helper branches
and verifies that the caller's environment remains unchanged.

Accepted counter terms are only `declaration_bindings` and `uninitialized`.
All named rows and their source offsets must match the complete returned
declarator list. Direct names retain `initialized=False` because that legacy
field means a written top-level `=`. `direct_initialized` and `direct_role`
retain their separate meanings. Thus `int a(0),b=c=0;` supplies two declaration
bindings and `uninitialized=1`; the caller's two written assignment owners
remain additive, for three units. No RHS token, callback, body or closer is
suppressed by this adapter.

## Type, value, macro and literal ownership

Source value roles come only from `resolve_cpp_value_owner`. Source type roles
come only from the existing positive type lookup. A type-only inventory result
can use its independently resolved positive type proof. Actual competing type
and value owners, blocked regions/names, conditional/dependent/macro/shadowed
owners and unsupported qualified operand roles remain explicit unknown
proofs. The adapter never chooses a namespace role by nearest spelling,
capitalization, missing types or a preferred type/value priority.

An actual absence of both eligible roles returns `None`, preserving the direct
helper's already approved *bare-name* equal-count object/prototype profile.
An explicitly rejected owner never becomes that absence. Compound leaves
with absent roles remain gated. Callee-agnostic call composition preserves the
existing helper contract; it is syntactic composition, not runtime purity or
a fabricated source value owner for the callee.

The entire written statement receives an active/uncertain macro token sweep.
This covers callees, member spellings and internally recognized builtin types
that the helper's role callback cannot intercept. Missing macro inventory
remains gated. No directive, compiler flag or header expansion is guessed.

Positive type-prefix/type-ID evidence is also checked against independently
blocked or competing bare value owners, so an omitted legacy type-shadow
record cannot by itself permit a later direct type use. Qualified positive
type contracts retain the type environment's existing namespace/source
proof obligations; this bare-only value inventory does not claim to certify
arbitrary qualified value lookup.

The type-ID callback extends a positive base through the supported
CV/pointer/reference suffix grammar and supplies the consumed end. It does
not falsely treat the base prefix as a complete pointer/reference type ID.
Unsupported trailing grammar remains the direct helper's gate.

The caller must supply the complete actual lexical literal inventory, including
an explicit empty list when there are none. Checked token provenance, original
same-width source text, exact mask correspondence and non-overlap are checked.
Identifiers named `S` are not treated as literal masks. In this first stage,
every actual literal overlap in a matched direct statement stays gated; no
literal/data wrapping is replaced by newly invented expression units. Include
filename provenance remains the separate import-owner contract.

## Ordinary inventory ownership stays separate

`identify_decl` is unchanged and still serves member, formal and type-shadow
collection. In particular, `int m();` can supply one independent prototype
name to this counter while remaining unrecognized by the ordinary data-member
resolver. The new counter result never certifies it as a nonstatic data field.
Object/prototype equal LOC does not establish a general value role, semantic
declaration point, lifetime or constructor member ownership.

Direct names that collide with a known type-owner name create an explicit
`cpp-direct-type-shadow-collection-review` obligation. This includes:

```cpp
#include <vector>
using namespace std;
int vector(0);
vector<int> a,b;
```

and the packed `int vector(0),other(1);` variant. Existing single-call or
conservative blocked shadow records are recorded as evidence, without being
replaced or treated as repaired by a successful direct binding count. These
rows stay gated until source collector ownership is separately reconciled.
The two existing collectors use different `start` conventions: ordinary
declaration shadows begin visibility after the written name, while the named
type companion records the written name start. Evidence lookup therefore
matches same-name records by their enclosing exact written owner span, not
by `record['start'] == written_name_start`. It additionally retains all
same-name shadow records and explicitly marks this lookup incomplete, so an
empty matching subset is not evidence that a shadow is absent.
The fixture that deliberately omits the legacy `vector` shadow still requires
the independently collected blocked value owner to reject the later type
prefix. No mutation of aliases or shadows is used to obtain this result.

## Unexecuted fixture coverage

The 49 named methods cover source-owned spectral-norm and Mandelbrot excerpts,
packed/expanded arithmetic binding semantics, exact whitespace/name offsets,
actual absent bare roles versus explicit rejected roles, type/value
competition, prototype type IDs, macro callee/builtin coverage, actual literal
provenance, wrapped/structural/protected/multiple statements, diagnostics before
and after matching, unchanged ordinary fallback, additive assignments,
constructor-member resolver separation, input preservation and later type
shadow obligations. These are pending assertions. No pass receipt, source LOC,
benchmark measurement or chart admission is claimed by this proposal.
