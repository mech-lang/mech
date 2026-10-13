# Bounded C# and Dart callback composition proposal after job15

This additive note reads the exact source paths and saved ownership records in `jobs/boundary-audit-15/candidate-audit.json`, plus `foreign_braced.py`, `test_foreign_braced.py`, the relevant shared suffix helper, and prior policy receipts. It ran no imports, lexing, normalization, hash calculations, tests, builds, timing, Git operations, or source sweep. Source identities below are copied from the frozen metadata, not freshly verified. It changes no parser, fixture, source, chart, receipt, or frozen job. Job16 is already frozen/queued and is outside this note.

The two identities represent four displayed registrations. Saved normalized counts are unadmitted proposals, not counts established by this static review.

| Exact identity and registrations | Exact source path, relative to `benchmarks/clbg/` | Saved job15 proposal and gate |
|---|---|---|
| `970cf62b15e5986fe68169d6ac92556b6cde985e8f80d7ddae782f869d929fb0`; `binarytrees:binarytrees-csharpcore-3-jit`, `binarytrees:binarytrees-csharpcore-3-aot` | `remaining-languages-comparison/20261011/sources/csharp/binarytrees-csharpcore-3.cs` | `physical_loc: 62`, `normalized_loc: 56`; `foreign-callback-surrounding-expression-review` at offset 1339, line 39, column 22 |
| `fd39860f284bded2df51f859e4e82c045b13f94e07406e180d3900d3230d47b9`; `knucleotide:knucleotide-dart-3`, `knucleotide:knucleotide-dart-jit-3` | `remaining-languages-comparison/20261011/sources/dart/knucleotide-dart-3.dart` | `physical_loc: 143`, `normalized_loc: 145`; the same gate at offset 2200, line 90, column 5 |

The settled boundary rule is written executable callback header 1, opener 0, independent body units, and written executable close 1. An actual expression-only lambda/call composition has one composition unit and no invented close. A proved argument list or delimiter is structural zero. Real following expressions or invocations require their own proved owner; call names, assumed purity, and callback compactness cannot justify deleting them.

The C# exact source region is lines 33–41:

```csharp
Parallel.For(1, iterations + 1,
    () => 0,
    (i, loop, localCheck) =>
    {
        return localCheck +
            (TreeNode.bottomUpTree(depth)).itemCheck();
    },
    localCheck => Interlocked.Add(ref check, localCheck)
);
```

The block callback owns one formal/header group, one written `return` statement containing member/call composition, and one written close: hand count 3. Splitting the return expression across lines 37–38 introduces no second independent statement. The preceding `() => 0` is an actual expression lambda. The following `localCheck => Interlocked.Add(ref check, localCheck)` is also an actual expression lambda whose body is one invocation; `ref check` writes no assignment syntax and causes no imputed extra assignment or close. This is a syntax classification, with no claim about the invocation's purity.

Job15 already records the block header `[1154,1199)` at line 35 with units 1, opener `[1199,1200)` with units 0, close `[1338,1339)` at line 39 with units 1, and following comma `[1339,1340)` with units 0. The diagnostic marks that proved comma because the complete following argument still lacks reviewed surrounding ownership. The delimiter-only line 41 must receive zero only after the matching `Parallel.For` argument owner is proved. Neither the real expression lambda at line 40 nor an arbitrary suffix after a close may be covered by a punctuation-only event.

A hand proposal for the whole call is 5: one prefix expression/call composition containing `() => 0`, block callback header/body/close 3, and final expression lambda/call composition 1, with list/delimiter units 0. This requires an explicit decision about how the enclosing call prefix and expression arguments share a composition owner in a mixed scalar/block argument list. It is a review proposal, not an admitted whole-call count or a new whole-source total. The current source region receives 8 from saved boundary events plus remaining row fallback; that fallback does not settle the mixed-argument policy.

The Dart exact source region is lines 79–93:

```dart
return stdin
    .map((s) => String.fromCharCodes(s))
    .transform(LineSplitter())
    .skipWhile((line) => !line.startsWith(">THREE"))
    .skip(1)
    .takeWhile((line) => !line.startsWith(">"))
    .forEach((line) {
  final units = line.codeUnits;
  for (int i = 0; i < units.length; i++) {
    codeList.add((units[i] >> 1) & 3);
  }
}).then((_) {
  codeList.flush();
  return codeList;
});
```

The two written block callbacks require separate executable owners. The first has header 1, local binding 1, loop header 1, loop body call 1, loop written close 1, and callback written close 1: hand count 6. The loop's initializer, condition and increment remain formal header structure. The second has header 1, `flush` call 1, `return` statement 1 and written close 1: hand count 4. Their explicit executable units total 10, independent of the unresolved surrounding chain owner. At line 90, the old callback close 1 plus the new callback header 1 gives exactly 2; `.then` cannot be removed as a blanket structural suffix.

Job15 already records the first callback header `[2052,2068)` and opener `[2068,2069)`. Line 90 records old callback close `[2198,2199)` with units 1, next callback header `[2199,2210)` with units 1, and opener `[2210,2211)` with units 0. That header includes the preceding `)`; a new delimiter event must not overlap it or duplicate its ownership. The diagnostic is at the following dot, offset 2200. The second callback close `[2257,2258)` at line 93 has units 1 and suffix `[2258,2260)` has units 0. Saved loop events use scope label `foreign-function`; their numeric header/open/close units match the hand count, but a future grammar proof should identify that owner as control without expanding this repair into unrelated changes.

The prefix at lines 79–84 is a complete receiver/call chain with expression lambdas, separate from both callback bodies. Review must decide whether its composition unit is absorbed into the first executable callback header or remains one distinct surrounding composition. Its wrapping and string/data rows need the established literal-wrapping policy as well. This note does not flatten that prefix, propose a whole-source delta, or classify all post-callback member syntax as zero.

The current diagnostic in `foreign_braced.py` is a conservative local scan after the callback close: it skips certain closing characters and flags a following comma or dot. It does not prove a full argument list or the remaining expression. `canonical._structural_callback_suffix` instead uses the immediate real argument/data frame to own only proved closing `)`/`]`, one argument comma, and an attached semicolon; it stops before a real following argument/call/expression. `foreign_core.count_owned` avoids adding a suffix span already contained by a header event. Those protections must remain.

A conservative general grammar path for later review is:

1. Prove the complete enclosing invocation, its matched argument frame, and the callback's exact argument position. Partition arguments only at separators owned by that frame; commas in formals, nested calls, generic type arguments, tuples or data cannot become statement splits.
2. Preserve each explicit block callback's header/open/body/close owners. Identify each expression-only lambda by its written formal/arrow syntax and complete body span. Prove ordinary member/call/index/operator composition without consulting callee names or purity. Independent assignments, updates, declarations, sequencing, control and nested executable scopes must keep their own units or gates.
3. At a comma after a callback close, prove the complete next argument and its actual owner. At a dot, prove the complete receiver continuation and invocation. A next block callback has its own header/body/close; a following expression-only invocation must receive a real composition owner. Unknown or incomplete arguments/chains retain the existing diagnostic.
4. Store explicit envelope and operand evidence: invocation start/end, argument-frame offsets, top-level argument spans, callback spans, expression-lambda body spans, owned delimiters, and the adopted prefix/argument composition decision. Use disjoint counted/zero spans; do not add one opaque enclosing span across callback events or suppress all text following `}`.
5. Keep masking and source-position behavior unchanged. Preserve literal/data wrapping under the existing rule. Clear a specific diagnostic only after its whole surrounding owner is proved, with no changes to unproved callback compositions elsewhere.

Proposed fixture work belongs in `test_foreign_braced.py` after policy review; no fixture is added or executed here. The existing `test_actual_callback_argument_and_chain_surrounds_stay_gated`, both `Use((x)=>{return x;},another)` gate assertions, and the packed/expanded explicit callback expectations stay unchanged until replaced by a reviewed, narrowly qualified ownership expectation.

| Proposed fixture family | Required evidence/expectation |
|---|---|
| Exact-shape C# call, packed and expanded | Initial expression lambda, one block callback, final expression lambda with a `ref` argument; explicit whole-call policy decision; block 3, tail composition 1, comma/closing-call delimiters 0; no close invented for either arrow body |
| C# tail isolated from surrounding policy | Proved `},` close 1/comma 0; following expression lambda/call 1; matched `);` 0; real following tokens absent from punctuation spans |
| Exact-shape Dart receiver chain, packed and expanded | Two callback owners; callback-only units 6 and 4; combined close/new-header row 2; final `);` structural 0; full prefix ownership shown separately |
| Dart `.then` with one-expression block body | Explicit block still header 1/body 1/close 1; callback compactness never collapses it to an expression lambda |
| Expression-only versus block tail | `then((x)=>call(x))` and `then((x){return call(x);})` have distinct syntax and close policy; ordinary following `.flush()` retains real invocation/composition ownership under the adopted envelope rule |
| Structural contrasts | Commas in parameters, nested calls, generic arguments and literal/tuple data remain structural; no blank-row fallback for a fully proved call-delimiter-only row; literal/data wrapping retains its existing fixtures |
| Executable and unknown contrasts | Following assignment/update/sequenced body, nested block callback, ambiguous continuation, malformed delimiter and incomplete lambda retain independent work or a narrow review gate; unknown ordinary argument policy retains the current gate |
| Owner evidence checks | No overlapping events; no punctuation event covering a lambda body, callee identifier or member dot; all written block closes present; aliases share source grammar rather than creating separate policies |

Review decisions still needed are the whole mixed C# call's prefix/argument composition ownership; the Dart receiver prefix's sharing with the first callback header; and how wrapped nonliteral expression spans are represented alongside the existing literal/data wrapping rule. The explicit block counts and line 90 close/header pair do not depend on those choices. Parent review must adopt those decisions before parser/fixture changes, then freeze the complete helper/dependency/fixture set and qualify a future guarded job. Neither job15 nor the already frozen job16 is amended by this note.
