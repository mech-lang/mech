# C and C++ packed declaration review

This is an additive, static review of every `unknown-c-declaration-type-review` diagnostic in immutable `jobs/boundary-audit-07/candidate-audit.json`. It does not certify complete normalized counts or admit chart data. Source SHA values are copied from the frozen audit; no new hashing, lexer execution, imports, tests, builds, or timings were performed. Original sources and receipts were not changed.

All 25 diagnostic registrations represent declarations: 14 distinct declaration sites in 11 source SHA identities, registered as 18 measured candidates. Eight sites use source-local typedefs; six use imported GMP or C++ standard-library types. Each declarator is a separate binding. No flagged site is a comma expression or destructuring producer.

| Source | Flagged lines | Type evidence | Additional units | Job07 count → scope-only proposal |
| --- | --- | --- | ---: | ---: |
| `binarytrees-gcc-1.c` | 74: three pointers | `typedef struct tn … } treeNode;`, lines 18–21 | 2 | 89 → 91 |
| `binarytrees-gcc-5.c` | 90: two pointers | `typedef struct node … } node;`, lines 13–15 | 1 | 120 → 121 |
| `fannkuchredux-gcc-3.c` | 17: two arrays; 27: two pointers and scalar | `typedef int elem;`, line 15 | 3 | 71 → 74 |
| `fannkuchredux-gpp-3.cpp` | 43: three arrays, last initialized | `typedef unsigned char int_t;`, line 10 | 2 | 61 → 63 |
| `nbody-gcc-3.c` | 23 and 58: two pointers each | `typedef struct planet … } planet;`, line 18 | 2 | 115 → 117 |
| `pidigits-gcc-1.c` | 11: five GMP integers; 39: three integers | `<gmp.h>`, line 9; `typedef unsigned int ui;`, line 12 | 6 | 45 → 51 |
| `pidigits-gcc-2.c` | 17: six GMP integers | `<gmp.h>`, line 15 | 5 | 48 → 53 |
| `pidigits-gpp-4.cpp` | 29: two static GMP integers | `<gmpxx.h>`, line 7 | 1 | 53 → 54 |
| `regexredux-gpp-1.cpp` | 20: two strings | `<iostream>` and `using namespace std;`, lines 10 and 14 | 1 | 56 → 57 |
| `regexredux-gpp-2.cpp` | 20: two strings | `<iostream>` and `using namespace std;`, lines 10 and 14 | 1 | 62 → 63 |
| `regexredux-gpp-3.cpp` | 21: two strings | `<iostream>` and `using namespace std;`, lines 11 and 14 | 1 | 68 → 69 |

The increments sum to 25 across distinct source SHA identities and 39 across candidate registrations. These totals only reconcile the flagged declaration sites. Unrelated frozen diagnostics remain unresolved. The JSON companion records every SHA, registration, exact source line, declarator, alias evidence, baseline, increment, and unrelated diagnostic.

Imported type definitions were read statically: local `gmp.h:165` defines `mpz_t` as `__mpz_struct[1]`; `gmpxx.h:1757` defines `mpz_class`; the SDK `string:505` defines `std::string`. Those viewed headers were not hashed or bound to the original timing build. They supply semantic evidence, not build provenance. Imported types need an explicit import/type contract and scope checks, or an exact reviewed source receipt.

Comma roles must remain separate. In `fannkuchredux-gcc-3.c`, the comma lists in lines 29 and 33 are protected `for` header expressions. Line 34 writes three separate updates, already handled by the assignment rule; it is not a declaration. In the C++ variant, line 58's `while(++permcount,next_permutation(...))` is one control expression. In nbody, comma expressions in `for` initializers remain protected. Typed constructor/array contents retain their data wrapping.

## Proposed general pathway

Collect a lexical type environment from proven `typedef` declarations before counting packed declarations. Accept builtin integral/numeric types and previously proven aliases, simple pointer aliases, and balanced `typedef struct/union/enum … } Alias;` forms. C++ `using Alias = proven_type;` is another explicit alias owner. Record declaration offset, lexical scope, lifetime, and evidence rather than making identifier capitalization determine type ownership.

At the exact declaration offset, resolve the leading type in that environment, then parse only bounded declarators: identifiers, pointer/reference prefixes, and named array suffixes, optionally followed by initializers. Split commas only outside argument/data delimiters. Return both declarator count and uninitialized count so chained initializers remain additive. The caller must exclude formal parameter groups and `for` headers before invoking this pathway.

Reject use before a typedef, a block alias used outside its scope, aliases under unresolved preprocessor conditions, ordinary identifier shadowing, macros that create types, function-pointer declarators, or ambiguous C++ templates. Retain review diagnostics for those forms. Include negative fixtures in the guarded qualification: multiplication/comma expressions resembling declarations, data/arguments, control headers, shadowing, and unrelated lowercase names.

The next step is implementation and immutable qualification under the common timing mutex, followed by exact source verification and full source-pool replay. No count or chart hook follows solely from this review.

## Aggregate suffix policy and unhooked implementation

Root subsequently approved a complete typedef aggregate header including its first attached alias. The written closing brace counts one separately; the first alias suffix adds zero. Every additional explicitly declared alias adds one. An ordinary instance tail in `struct Node { … } instance;` remains a separate binding. Unknown suffix ownership remains gated. This approval is recorded separately in `c-typedef-suffix-policy.json`; the earlier scope-only totals retain their frozen job07 baseline.

`c_type_environment.py` now proposes explicit alias environments, independent declarator counts, and `classify_alias_suffixes` spans with primary/additional alias ownership. The module does not edit or hook the shared normalizer or canonical counter. `test_c_type_environment.py` contains focused fixtures, including packed/expanded alias tails, ordinary instance tails, alias order and scope, ordinary/parameter/loop shadows, and declaration/comma-expression contrasts. These files have not been executed by this source review.

Imported headers are carried as semantic lexer metadata `{start, end, header}` for the filename token, while that token remains a literal placeholder in the cleaned source. The collector requires an exact owned include row, a translation-unit scope, an explicit supported header contract, and normal alias lifetime checks. It rejects a raw filename guess or a claimed import inside an executable scope. This avoids turning punctuation inside a filename into executable separators.

The legacy shared `DECLARATION` expression also guesses that arbitrary uppercase identifiers are types. That is unsound for `int X=2; … X *a,b;`, which can multiply by the ordinary operand `X` and then evaluate `b`. The new helper treats uppercase and lowercase spellings alike: only a builtin, explicit tag, source-owned alias, or proven import establishes type ownership. A focused negative fixture covers `X`, paired with a positive `typedef int X;` fixture. Replacing the legacy uppercase shortcut will expose additional source-review work beyond this report's original 25 diagnostics; explicit C++ class owners and imported third-party types must be reviewed or implemented before a global certificate.
