# Wrapped declaration gates: unhooked implementation

`c_wrapped_type_declarations.py` is a new diagnostics-only helper. `test_c_wrapped_type_declarations.py` contains 28 named proposed fixture methods. Neither module was imported or its code/fixtures executed. At the parent's explicit request, both saved sources passed syntax-only `compile(..., 'exec')` with the known isolated Python 3.14 interpreter and site loading disabled; exit status was 0. Compilation did not execute either code object. No source lexing/normalization/hash, fixture run, benchmark/timing work, Git command, core module edit, queue edit, or frozen-job edit was performed by this implementation step.

The earlier `c-wrapped-template-declaration-ownership-proposal-20261012.md` records the original narrow proposal. This new implementation additionally retains the independently identified named-declarator layout uncertainty. It supplies no normalization policy, count correction, event, mask, or admission result. The parent owns hook/integration review, dependency freezing, the common-lock fixture/audit run, and any later source admission.

## API and source-before-masking seam

```python
classify_wrapped_type_declaration_gates(
    body_cleaned, language, contexts, type_environment,
    type_prefix=..., declaration_probe=...,
    type_source=..., owned_boundary_events=...,
    protected_spans=..., preprocessor_directive_spans=...,
)
```

`body_cleaned` is the checked scratch after the consolidated canonical events have been blanked. `type_source` is the matching checked mask immediately before that event projection, not raw literal/comment text. It preserves complete type tokens when a partial canonical header masked a later row of a wrapped return type. Surviving characters, every newline, source/context/environment lengths, event masking, span metadata, and frame offsets are checked. Missing/malformed environment/context inputs receive `cpp-wrapped-type-source-provenance-review` rather than being passed to an unguarded `.get` operation.

The callbacks use actual source positions: `type_prefix(code, environment, absolute_offset)` returns the existing `(end, type_name, evidence)` proof; `declaration_probe(code, environment, offset=absolute_offset)` optionally supplies `identify_decl`'s complete named-tail proof. The helper copies names and their exact locations as evidence, never their binding count. Unknown type proofs remain unaccepted, and upstream macro/namespace/type/header diagnostics remain the caller's responsibility to retain.

The proposed hook is after the final canonical events are consolidated/sorted and their body projection is created, before physical-row body counting. Pass the consolidated `canonical['events']`, not the earlier `canonical['boundaries']`: the latter precedes appended initializer, field, attribute, and alias events. Pass effective contexts derived from the matching checked source.

## Independent windows and reset evidence

Complete source semicolon windows continue across rows. Semicolons inside every relevant parenthesis/group/argument/formal/for frame, bracket, or data brace are structural. An inner executable callback body remains eligible even when outer call/argument frames exist. When `type_source` is supplied, an actual independent semicolon masked by an owned event still supplies its original boundary; missing physical punctuation is not guessed from an event endpoint.

Only accepted canonical header/branch/open/close records, whole alias owners, exact full protected profile owners, and exact preprocessing directive spans reset windows. Ordinary operands, value expressions, data braces, and attribute/annotation endpoints never reset them. Empty enum-interior point records are accepted as empty ownership evidence and ignored as reset anchors. Unknown reset/protection/directive metadata retains a provenance review.

Consolidated canonical header events are boundary evidence, not a complete function profile certificate. A header may start on a later signature row. With pre-blank source available, an unowned preceding wrapped type retains the exact type review; without it, a cross-row residual before a partial header retains `cpp-wrapped-header-owner-review`. Only a separately proven complete profile span suppresses that uncertainty.

## Two distinct retained gates

`cpp-wrapped-proven-type-declaration-owner-review` requires a newline inside the positively proven type prefix itself. It records the exact statement/type spans, type evidence, terminator or header boundary, and `units=None`. A complete named-tail proof labels ordinary `a,b`; a name-shaped unsupported direct suffix/prototype remains tail review. Constructor/cast temporaries beginning directly with `(...)` or `{...}` supply no independent declaration names and are not promoted.

`cpp-wrapped-independent-declarators-owner-review` requires complete declaration proof plus a verified declarator name on a physical row different from the type-start token. Each location is checked against the exact source spelling at `type_start + declarator_start + name_offset`. This covers both `int a,\n b,c;` and `int\n a,b,c;`, including all names together on a later row. It is separate from the type-prefix gate; both can apply without contributing units. Newlines only in an inline type's initializer data/literals do not satisfy this name-location predicate when the names stay on the type row.

## Proposed protected-span construction

Every supplied protection record needs `{start, end, role, owner_proven=True}` and no conditional/macro/unresolved/review flag. Construct them only from accepted complete ownership evidence:

- Accepted whole ordinary alias events may supply `role='whole-alias'`. Aggregate alias spans must keep the existing `aggregate_alias_suffixes` body exception: their exact header and suffix are protected, while unowned field declarations inside the aggregate remain eligible. Do not blindly suppress every byte of an aggregate `alias_spans` envelope.
- Already-owned complete field/declaration spans may supply `role='field'`. Their coverage must include the complete candidate statement. The current C/C++ record-field owner chiefly supplies enum interiors; it does not certify all ordinary class/struct fields.
- Accepted exact formal, control header, type header, full function profile, and full prototype records may supply their corresponding roles. Do not relabel a partial canonical `header` event as `function-profile` or `prototype`; those whole profiles require independent proof.
- Accepted literal/data/initializer spans may protect their own operands. They never become statement reset anchors and must not hide a preceding unowned declaration type/name span.
- Preprocessing barriers use the macro helper's logical directive records only with both `extent_proven=True` and `directive_ownership_proven=True`. They remain preprocessing roles and preserve physical bytes/counting.

The helper creates no protected spans or ownership events itself. Ordinary field/prototype consumers remain unchanged by these new files.

## Unexecuted fixture scope

The 28 methods cover packed/expanded wrapped types, nested structural type arguments, builtin modifiers, whole-owned packed literal/data initializers, inline names before wrapped data, exact named declarator locations on later rows, inner function/callback bodies, structural for/group semicolons, temporaries, unowned/whole-owned prototypes, partial/full function profiles with and without pre-blank source, ordinary/aggregate aliases, exact fields, attribute/operand non-reset endpoints, unknown quoted headers, missing/malformed inputs, source projection/directive uncertainty, separate simultaneous reviews, and empty enum-interior point ownership. Their source was syntax-compiled only; there is no passing fixture or source-count receipt.
