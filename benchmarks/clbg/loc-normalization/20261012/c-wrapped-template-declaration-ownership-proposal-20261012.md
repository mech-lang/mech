# Wrapped ordinary template declaration gate

This note proposes a retained source-keyed gate; it does not change parser/normalizer code, execute fixtures, or claim a normalized count. The four bounded reviewed std-template targets motivating the existing integration are inline. This cross-row path is an independent prerequisite for broader admission.

The current full-source type helper can prove the entire type below, but an ordinary body fallback operating on each physical row cannot see its independent declarators together:

```cpp
std::vector<
  int
> a,b;
```

Proof of the cross-row type span must not silently authorize a row fallback that misses `a,b`. The type arguments remain structural; independent declaration names require their own statement owner. Literal/data rows must retain their existing written wrapping.

The proposed first implementation is a narrow gate, not a new counting rule. Its API would take checked masked source, the existing type environment, a supplied exact-offset type-prefix callback, balanced ordinary statement spans, and explicit already-owned structural/declaration spans. For an unclaimed ordinary declaration candidate whose fully proven type prefix crosses a physical newline, return `cpp-wrapped-proven-type-declaration-owner-review` at the actual type start, with the exact statement/type span, evidence, `units=None`, and `review_required=True`. No source rewrite or extra numeric units would be returned.

Statement ownership and exclusions are prerequisites. Function/control/formal/header spans, proven alias spans, preprocessor/literal spans, existing data/initializer owners, and already-owned declaration-field spans must be excluded explicitly. Do not infer exclusions from capitalization, angle operators, or a presumed function name. Use the same shadow/order-aware resolver at the type tokens' real positions. Unknown/dependent/unbalanced types and unsupported constructor suffixes retain their existing gates. If a supposedly protected header has no exact owner span, retain review rather than infer a body declaration.

| Proposed future fixture | Required reviewed outcome |
| --- | --- |
| Inline `std::vector<int> a,b;` versus a cross-row proven type with `a,b` | Either full-statement packed/expanded parity under a separately reviewed owner, or the explicit wrapped gate on the unowned cross-row form. |
| Nested cross-row `std::vector<std::complex<double>> a,b;` | Recursive arguments remain structural; the same declaration owner or retained gate is required. |
| Cross-row field declaration with two names | Use an existing exact field owner if present; otherwise retain the wrapped declaration gate. |
| Cross-row proven function formal/return header or control header | Its existing exact header/formal owner excludes the ordinary body gate. |
| Cross-row `using`/`typedef` RHS already inside an accepted alias span | Alias ownership excludes this additional gate; unresolved alias diagnostics remain. |
| Cross-row constructor suffixes `a(N),b(N)` | No ownership is invented from a type proof. The separate direct-declarator proposal and wrapped statement ownership remain required. |
| Literal/data initializer rows wrapped differently | Preserve their bytes and existing row policy; declaration proof does not flatten them. |

The eventual positive alternative is a general full-statement declaration owner that calls `identify_decl` with the raw segment start, preserves literal/data row accounting, and assigns only independent names plus separately proven initializer effects. That broader path needs a concrete integration fixture comparing packed and expanded written forms. This note authorizes no such implementation or count claim; the narrow retained gate is sufficient to keep the presently unowned case out of admission.
