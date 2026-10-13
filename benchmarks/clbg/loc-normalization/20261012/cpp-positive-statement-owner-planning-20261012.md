# Separate positive single-row statement ownership

This is read-only planning for a new ownership pass required by `cpp_direct_counter_adapter.py`. Only this note was created. The wrapped helper, its 28 fixtures, the normalizer, adapter, queues, frozen job19 inputs, source bytes, and proposal patches were not changed. No module import, compilation, fixture/test execution, source normalization/hash, benchmark/timing work, or Git command occurred. No job19 result or new count/admission is claimed. Implementation and fixture decisions remain after the actual job19 receipt.

The wrapped helper's source/span validation and statement traversal are useful building blocks, but its diagnostic windows are not positive complete-statement certificates. Neither a window callback, absence of wrapped diagnostics, a physical row, nor a semicolon alone can justify `ownership_proven=True`. A separate pass must prove the contracts below against the direct adapter's unchanged source mask, then emit an owner record. The adapter retains responsibility for declaration/type/value/literal/macro analysis and complete counter terms.

## Proposed record and source contract

The existing adapter requires this envelope:

```python
{
    'kind': 'independent-semicolon-statement',
    'start': exact_left,
    'end': exact_right,
    'terminator_offset': exact_right - 1,
    'ownership_proven': True,
    'start_boundary': positive_boundary_record,
}
```

The positive pass would return complete owner records plus exact-offset unknown diagnostics, never counts, replacement masks, header events, or repairs to inventories. It should not require a declaration/type-prefix proof to establish a statement interval; those semantic decisions belong to the adapter. A complete expression statement may have a structural owner without being a declaration.

All inputs must refer to one checked mask and identical offsets: the actual unchanged lexical `source_mask`, its context mapping and diagnostics, full canonical/structural ownership evidence, protected-owner spans, preprocessing directive inventory, and source length. The adapter independently requires same-source type/value inventories and the complete actual lexical literal inventory. The wrapped helper's `type_source=cleaned` is the checked mask before canonical event projection; that is not automatically the adapter's unchanged lexical mask if attribute/value/field projections have already changed it. Length equality alone cannot equate those stages.

Use the consolidated canonical events after appended initializer, field, attribute, and alias ownership. The earlier `canonical['boundaries']` is insufficient. A body projection may help locate remaining work, but the positive owner and adapter segment must cover the entire unchanged original interval. If erased non-whitespace source text remains between the proved preceding boundary and the proposed first token, the interval cannot be shortened past that text. In particular, removing an operand or attribute does not create a new statement start.

## Positive endpoint and context requirements

The first stage should export only a complete interval satisfying every condition:

1. Its end is immediately after an actual `;` in the unchanged mask. First-token and terminator contexts are valid, complete, and identical. There is no relevant unmatched frame after the nearest accepted executable frame: group/call/formal/control/for parentheses, brackets, data braces, and unknown scope frames remain structural. An inner accepted executable callback body may make outer argument frames irrelevant, using the same nearest-executable-frame rule as the adapter.
2. There is exactly one independent terminator in the interval. Internal semicolons in protected structural contexts do not split it; an internal independent semicolon requires separate intervals. Data braces, commas, closing parentheses, physical newlines, and arbitrary event endpoints never supply the missing boundary.
3. A positive preceding boundary is retained with exact source/context evidence. It is not inferred merely by scanning backward to the nearest punctuation.
4. The candidate interval does not overlap an existing structural/executable owner passed as an exclusion to the adapter. This includes complete field/formal/header/profile/alias and composed-value ownership where that ownership covers the candidate. Internal executable header/open/close ownership cannot be discarded to count the surrounding expression again. Empty interior point events contribute no boundary or positive certificate.
5. The exported interval contains no `\n` or `\r`. Its left edge can exclude preceding whitespace/newlines only when the positive boundary-to-left gap is entirely whitespace in the unchanged mask. Its right edge remains the terminator plus one, without trailing row breaks. The caller passes the segment at an offset inside this owner covering every original token, except allowed surrounding whitespace and the final semicolon.
6. Context provenance must exclude phantom executable frames originating inside comments, literals, or preprocessing directives. A generic `code` label is not enough when the opener lies in protected preprocessing text or its header role is unresolved. Global/scoped delimiter, phase-2, conditional, or boundary uncertainty must remain explicit rather than be cleared by a later clean punctuation mark.

The wrapped scanner intentionally tolerates more ambiguity because it returns uncertainty diagnostics only. Its `_structural` predicate is weaker than the adapter's `_independent` contract for unknown frames, and it does not require first/terminal context equality. Its generic sorted anchor jumps also lack a positive `start_boundary` lineage. Those checks must be added in a separate owner pass; silently upgrading the frozen helper's result would be unsafe.

## Exact compatible preceding boundaries

| Adapter boundary kind | Positive evidence required |
| --- | --- |
| `source-start` | Offset zero; all unchanged mask text before the exported left edge is whitespace. A source start cannot jump over an include, annotation, operand, or declaration. |
| `statement-terminator` | Actual preceding `;`, independently outside structural frames; its context equals the candidate's independent context, and the gap to the left edge is whitespace. A semicolon inside a for header/group/data context cannot qualify. An accepted whole alias's real terminal semicolon can qualify through this same proof. |
| `executable-opener` | Actual `{`, exact accepted executable scope/header provenance, and a context push matching the adapter's contract. Currently the adapter specifically requires the final frame `('{', 'code', opener_offset)` and its outer context to equal the preceding context. A `bare-code` opener is not accepted by that branch; do not relabel it or manufacture an owned-span workaround. |
| `executable-closer` | Actual `}`, accepted executable closer ownership, a final code/bare-code brace frame at the closer, the exact corresponding context pop, and only whitespace afterward. Data/initializer closers cannot qualify. A type/namespace body's closer must not be renamed executable without separate scope-role approval. |
| `owned-header-or-directive-boundary` | A complete accepted header/control/directive span ending before the candidate, exact `offset=owner.end`, whitespace gap, and the same owner record present in the adapter's protected spans. Membership in that list alone is not proof of a header/directive role. |

Canonical `header` events are often boundary fragments, not whole function-profile evidence. A partial wrapped return/profile must stay gated; it cannot supply a complete-header exclusion or a new positive statement start. For an executable opener that follows such a fragment, require the independently accepted full scope/header proof rather than deriving it from the fragment's end. Exact unbraced control headers may supply their own body boundary only with complete control-header ownership. Type-body fields, function prototypes/formals, and constructor inventories continue through their existing consumers; the first positive adapter scope should not broaden those consumers merely because several scope kinds currently share `code` context labels.

Exact preprocessing boundaries need both logical extent and directive ownership evidence. Known literal include filenames retain their preprocessing role. Unknown splices, conditional/uncertain boundary effects, or unproved phase-2 ownership are gates; do not expand macros or choose branches to manufacture a delimiter/context proof.

## Exclusion inventories cannot be copied blindly

The wrapped helper excludes a protected operand mainly when it contains the candidate's first token. The adapter instead rejects any overlap with a supplied protected span. Therefore its exclusion inventory must be constructed from the exact roles required by that adapter, not copied indiscriminately from the wrapped helper.

The actual lexical `literal_spans` inventory is mandatory and separate. The current adapter's first stage gates every actual literal overlap in a matched direct declaration and retains literal-data review; it does not yet support literal constructor arguments. Any future support would require its own reviewed proof through that inventory. Keep literals separate from structural exclusions so their exact lexical evidence and current gate remain visible. Already-counted composed values or executable operands retain their full exclusions and must not be clipped to expose an artificial direct declaration.

Aggregate aliases need the existing body exception. Their complete envelope cannot be used to suppress unowned fields inside the body. Their accepted header/suffix and actual final semicolon retain separate roles. Ordinary complete alias ownership cannot stand in for a complete enclosing function profile.

## Retained unknowns and decision boundary

No positive owner should be exported for malformed/mismatched mask/context/inventory provenance; a structural or missing terminator; unmatched first/terminal contexts; non-whitespace erased prefix text; ambiguous or partial preceding headers; protected-owner overlap; phantom directive scopes; unsupported type/namespace/bare-block scope transitions; or an unclaimed wrapped interval. Preserve the existing wrapped type/name/header reviews and macro/namespace/value/type-role diagnostics rather than interpreting their absence as a certificate.

Known accepted literal/data wrapping remains with its existing ownership and row policy. A single-row adapter does not flatten or recounter that source. Withhold a new adapter owner for a wrapped interval; emit an uncertainty gate only where that interval still seeks unproved ownership, rather than invalidating an already accepted literal/data owner by inventing a new policy.

After actual job19 results, a reviewed implementation could extract a new reusable source-window traversal with explicit positive boundary lineage and the stricter contracts above. That extraction needs its own frozen dependency/test stage. The current adapter fixture helper's manually selected marker intervals are intentionally not a production ownership collector. Neither those fixtures nor job19's diagnostics-only wrapped integration would certify the proposed positive pass without that separate review.
