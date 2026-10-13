# Source-written macro type gates

This is an unhooked, unexecuted proposal. The new helper is `c_preprocessor_type_macros.py`; its 27 pure-helper fixture methods are in `test_c_preprocessor_type_macros.py`, and eight proposed integration methods are in `test_c_preprocessor_type_macros_integration.py`. The companion `c-preprocessor-type-macros-integration-proposal.patch` has not been applied. No target module import, syntax compilation, fixture execution, source lexing/normalization/hash, benchmark, timing, Git command, queue edit, or frozen-job edit occurred while preparing these artifacts. Static text reads and patch-file writes are the review evidence; they establish no normalized source count or admission.

The patch is based on the mutable imported-template integration described in `c-std-template-working-integration-20261012.md`. It proposes changes to `c_type_environment.py`, `c_std_templates.py`, `c_named_types.py`, `cpp_initializers.py`, and the normalizer's environment-call arguments. Review/rebase those source seams before applying it. The separate namespace-prefix patch is stacked after this macro patch. A later guarded job must freeze the new helper/fixture modules and every changed dependency together; frozen boundary-audit-18 cannot certify this later work.

## API and proof boundaries

`collect_type_macro_events(cleaned, language, original_source=..., protected_literal_spans=...)` consumes an offset-preserving checked mask and its matching original text. It returns written `#define`/`#undef` events, exact logical directive spans, source offsets, uncertainty spans, and diagnostics. Original text supplies backslash/newline extent evidence; masked text supplies physical comment/literal token protection. The helper returns `original_source_proven`, and each directive span records `extent_proven` and `directive_ownership_proven`. Those facts remain separate: matching original bytes do not prove phase-2 comment ownership.

`type_macro_name_gates(environment, name, offset, role=..., source_token=...)` queries every actual identifier component at its exact offset. `type_macro_token_gates` queries an already grammar-proven type span; it does not discover declaration or template grammar. `type_macro_owner_gates` queries the actual class/alias/namespace declaration point. `with_type_macro_owner_gate` returns a labelled copy, leaving the input record untouched. `without_type_macro_directives` produces a scratch mask for scope/type collectors; it keeps every source position and newline and does not replace the mask used for physical/directive/literal counting.

Written macro lifetime is independent of C++ brace scopes. An exact unconditional `#undef` resets that name for later tokens. Conditional definitions/undefs remain uncertain without branch evaluation; a later exact unconditional undef can reset their known name. Unknown split keywords/names, unmatched/unclosed conditions, missing original extents, and unproved apparent directive ownership retain independent uncertainty that an undef cannot clear. Function-like macro names remain conservative because invocation/expansion grammar is outside this helper. These choices follow the relevant preprocessing distinctions in [macro scope](https://eel.is/c++draft/cpp.scope), [macro replacement](https://eel.is/c++draft/cpp.replace), and [conditional inclusion](https://eel.is/c++draft/cpp.cond); this proposal implements gates, not a preprocessor.

No external compiler flags, header-defined macros, replacement values, token pasting, branch selection, or transitive declarations are inferred. A later complete provenance policy must decide how those external inputs are certified. The helper cannot certify them by absence from the written source.

## Header filenames and declaration points

`<vector>` in an exact include is a protected preprocessing filename. The proposal never says that its written `vector` filename component expands as an ordinary identifier. Header contracts instead query their explicit class/namespace name (`std::vector`, `std::complex`, or `std::array`) at the import point with `source_token=False`, role `imported-type-contract-macro-exposure`. An active matching macro leaves physical header declaration ownership unproved; unrelated names are not used to invent a header contract failure.

A clean repeated include does not repair an earlier exposed contract without physical header/include-guard evidence. This differs from a later clean source-written class/alias declaration: that declaration receives independent source evidence at its own offset. An affected declaration's earlier proof cannot be repaired retroactively by `#undef`.

Namespace imports query their actual `using namespace std;` tokens. A temporary `vector` macro at that directive does not freeze the bare member import if the qualified header contract was clean and the macro is unconditionally undefined before use. An active `std` macro at the directive is a separate source-token gate. All qualified use components and recursive type arguments are queried at their real positions.

## Unknown competing owners persist

The patch excludes macro-labelled entries from accepted type-owner selection, while retaining their ambiguity evidence. Exclusion is not evidence that a binding is absent. For example, a self macro can leave the source class spelling unchanged:

```cpp
#include <vector>
#define vector vector
struct vector{};
#undef vector
using namespace std;
vector<int> a,b;
```

The unqualified prefix must remain gated. The existing bare namespace conflict scan is retained. The patch also records `type_macro_lookup_blocks` with exact declaration offsets, visible start, scope start/end, source owner kind, and `accepted_type_owner=False`. An affected class/alias/namespace declaration conservatively blocks later unqualified lookup in its visible scope: unknown expansion cannot prove either the raw name's absence or the absence of another competing name. A narrower inner proven owner can shadow an outer block. A later same-scope source declaration has independent evidence but does not erase the unknown earlier effect.

An unresolved macro-affected alias that fails before `add()` also gets a lookup block, without accepting alias spans or alias units. Those blocks survive monotone alias revalidation removing provisional entries. Qualified imported contract lookup is a separate component; the new namespace-prefix proposal still has to establish that its actual `std` qualifier has no unsupported written owner. Positive qualified assertions in the macro integration fixtures test that component separation, not final admission.

## Phase-2/comment uncertainty

An apparent directive whose hash/keyword follows an earlier physical newline in a spliced logical group is not certified from the physical mask:

```cpp
// comment\
#include <vector>
#undef vector
std::vector<int> a,b;
```

The apparent include receives exact-offset `c-type-preprocessor-directive-ownership-review` evidence, and uncertainty persists through the translation-unit end. A later undef cannot prove that the import was a directive. Source owners and imported contracts both query that uncertainty.

The adjacent apparent source-owner case (`// comment\\` followed by a physical `struct IUB{};` row) receives a local logical-group ownership gate. Its declaration copy remains affected after the group ends; a later clean source-written declaration is independent. The helper detects masked line-comment markers before a written splice conservatively; it does not reconstruct phase-2 tokens or claim exact comment semantics.

## Static fixture scope and remaining gates

The pure fixtures cover event/name offsets, reset versus conditional uncertainty, brace-independent lifetime, qualified components, nested type arguments, import exposure versus filename roles, repeated include uncertainty, source declaration points, protected literal placeholders, continued replacement text, malformed directives, split names, apparent phase-2 directives, and apparent source declarations. The eight integration fixtures cover actual resolver/reporting seams, self-macro lookup ambiguity, scalar alias scope expiry, unresolved alias blocks before owner acceptance, qualified component separation, and namespace directive token roles. None has run.

Static drafting review corrected a missing fixture declaration-start field and regenerated complete unified-diff context lines. That textual repair is not an application check or Python syntax check. The two independent remaining proposal paths are `c-wrapped-template-declaration-ownership-proposal-20261012.md` and `c-std-namespace-prefix-proposal-20261012.md`. Their gates must be reviewed before broad std-template admission. Existing unrelated diagnostics, constructor suffix gates, source bytes, literal/data wrapping, producers, timings, reports, controls, and chart/admission hooks remain outside these new artifact edits.
