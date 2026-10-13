# Unapplied standard-template integration proposal

The companion `c-std-template-integration-proposal.patch` is a static unified diff, generated from read-only source text and reviewed at the API boundaries below. It has not been applied, imported, compiled, tested, normalized or hashed. Only this note and the patch file were created for this follow-up. `c_type_environment.py`, `c_named_types.py`, `c_std_templates.py`, `normalizer.py`, queues, source bytes, producers, reports and frozen jobs were not edited by this proposal. Job17 is already frozen and waiting for the common timing mutex; this proposal does not amend it or start another fixture job.

The patch proposes changes to `c_type_environment.py`, `c_named_types.py` and the previously unhooked `c_std_templates.py`, plus a new `test_c_std_template_integration.py` containing 14 proposed fixture methods. Its exact old-text seams were matched once each against the read-only working sources; its hunks were not application-checked. Rebase and review the diff against any intervening parent edits before applying it.

## Collection and prefix seams

The existing `semantic_imports` iterable is materialized once so both the original scalar/header contracts and the new template contract collector see the same exact records. After the existing `_INCLUDES` loop, and before any `using`/`typedef` RHS proof or final shadow pass, the environment calls:

`collect_std_template_contracts(cleaned, language, environment, resolver=_resolve, semantic_imports=semantic_imports, scope_ranges=ranges, conditional_regions=conditional)`.

Returned entries and evidence are merged without replacing source class owners, existing imported scalar types or unrelated diagnostics. The collector retains exact namespace directive order and enclosing scope lifetime. It is intentionally conservative when the direct header follows the namespace directive.

`_type_prefix(code, environment, offset)` first asks `prove_std_template_prefix(code, environment, offset, resolver=_resolve)` for a complete imported specialization. The existing builtin/tag/plain-type pathway remains the fallback. Plain named lookup now supplies `offset + cursor`, matching the actual first type token after leading modifiers and whitespace. A bare imported class-template name cannot fall through as a plain type; class template argument deduction remains outside this bounded proof.

Both simple `using` and `typedef` RHS calls now use the existing `body_start`, rather than the alias keyword's `start`. The synthetic using-alias name remains only a bounded declarator terminator. All real RHS type and argument tokens retain their original absolute offsets.

## Source aliases and final shadows

Template imports must precede both final shadow passes. The patch keeps `_collect_shadows` and `collect_named_type_shadows` in their existing order and preserves every source class entry, collected shadow, blocked owner, evidence row and diagnostic.

The named-type companion currently considers only source aggregate names. The proposal adds exact unqualified `explicit-standard-template-namespace-import` names to its shadow-name set, enabling the existing `auto`, enumerator, ordinary function, constructor/formal, pointer-value, uncertain-lifetime and conditional-owner grammar to protect `vector`, `complex` and `array`. Its class/constructor owner list remains source-only: an imported template is never promoted into a source constructor owner.

Formal splitting accepts an optional `parameter_parts(text, environment, absolute_offset)` callback. The proposed `_type_parameter_parts` skips commas only inside a positively proven type-prefix span, then uses the existing ordinary balanced-group logic. Unknown/dependent/template-looking expressions do not establish an angle owner and cannot protect an arbitrary comma. The ordinary function shadow pass uses the same callback. The patch also corrects two existing offset seams: stripping a direct access label advances the segment's source start, and stripping a function prefix advances its first-token offset.

Aliases are provisionally collected before these final shadows. Any simple type alias can therefore initially appear valid even when a real value shadow is discovered later in that phase. This matters for template arguments too: `typedef int T; ... int T; using Bad=T; std::vector<Bad> ...` must not turn an invalid scalar alias into a proven template type argument. For every simple typed using/typedef alias, the patch retains its exact RHS probe, probe offset, prefix end and type evidence. Aggregate typedef aliases remain on their existing separate ownership path.

After both shadow passes, `_revalidate_source_type_aliases(environment)` rechecks every marked alias at its original RHS offset. Failed alias entries and their alias spans cease to be accepted owners; their existing evidence stays labelled unresolved/review-required and receives a source-keyed `c-typedef-declarator-owner-review` diagnostic. Revalidation is monotone: another pass gates aliases depending on removed owners. A completed alias survives a later value shadow of its original type spelling, because its RHS is rechecked at the original declaration point, not at a subsequent alias use. This phase makes the supplied resolver's source-type proof promise valid for scalar aliases as well as direct template aliases. It retains unrelated diagnostics rather than filtering them.

One extra guard is proposed in the unhooked helper: a namespace using-directive cannot supply an unqualified template specialization when another visible non-namespace type owner has that bare name. This prevents the existing resolver's latest-visible ranking from incorrectly treating `struct vector {}; using namespace std; vector<int> ...` as an imported template. The qualified `std::vector` owner remains independently resolvable. Ordinary value shadows still come from the supplied resolver and the real shadow passes.

## Proposed fixture decisions

No fixture below has run, and this note supplies no actual source LOC totals or source-count correction. The proposed file exercises the real lexer/environment/normalizer only within a future reviewed common-mutex job.

| Fixture seam | Required decision |
| --- | --- |
| Actual spectralnorm `vector<double> u(N), v(N)` and Mandelbrot `std::complex<double> Z(...),C(...)` | Prove the imported type prefix and zero-unit structural arguments; retain the constructor-declarator suffix gate until that separate owner is reviewed. Potential declarator names must remain `u,v` and `Z,C`. |
| Ordinary packed and expanded nested template declarations | Independent declaration names are separate owners; nested template arguments contribute zero units. Packed and formatting-only expanded forms must agree without inventing a benchmark total. |
| Fasta `IUB`, `Random(std::vector<IUB>& i):i(i){}` and direct `std::vector<IUB>& i` field | Source aggregate argument ownership is proven before the member is resolved; the existing constructor helper owns the written member initializer separately. |
| Using/typedef alias RHS | Every template and source argument proof uses the original RHS token offsets, including alias chains. |
| Namespace order and block lifetime | No import before its directive or after its scope end; no qualified owner guessed from the bare name. |
| Function profile, prototype, ordinary arguments and `for` initializer | Formal/control/data spans remain structural; proving a type never creates independent parameter or loop-header units. |
| Nested `std::array<IUB,4>&` formal | The exact header contract protects the type comma; source argument ownership and parameter lifetime remain checked. |
| `auto vector`, ordinary `vector()` function, formal `int vector`, source class `vector`, and source `IUB` value shadows | Real source shadows remain effective; qualified owners retain their separate lookup. |
| Simple type aliases and final shadow discoveries | Invalid provisional template/scalar owners and their descendants are gated, while a completed alias survives later shadowing of its original spelling. A scalar alias rejected after real shadow collection cannot prove a template argument. |
| Unknown/dependent types, `NUMBER_TYPE`, named `SIZE`, conditional imports and quoted local header | Ownership remains gated; the actual dependent Mandelbrot `NumericArray` alias is not admitted by an array header contract alone. |
| Existing multiline literal/data initializer wrapping | Template ownership must preserve the mask's offsets/newlines and agree with an equivalent scalar-array data owner; literal/data rows cannot become independent declaration names. |

The constructor-declarator path is intentionally a separate integration dependency. The current `_declarators` accepts named pointers/references/array suffixes and `=` initializers, and rejects `u(N)`/`Z(...)` suffixes. Therefore the template-only fixture explicitly retains that gate. Once a separate bounded constructor-declarator helper is reviewed, its companion positive fixture must compare the actual packed forms against separately expanded `u`/`v` and `Z`/`C` declarations while protecting constructor arguments and their existing wrapping. Template proof alone does not authorize that suffix or provide its unit count.

## Future qualification boundary

No `normalizer.py` hunk is required for the type-prefix connection: its existing original-mask environment collection, independent declaration callback and constructor member resolver already consume `identify_decl`. Their structural context exclusions and literal/data handling remain in force. This does not establish that every downstream grammar is qualified.

Before a later immutable job, the parent must review the combined template, constructor-declarator and dependency changes; reconcile the new proposed fixture gate with whichever constructor suffix owner is adopted; freeze the new helper/tests and all changed dependency modules together; and include the complete required original legacy coverage. The existing queued job17 cannot certify this later patch. No fixture, partial replay, hash or pool audit should run outside the existing common timing mutex. A later guarded result still needs explicit source-keyed review and admission.
