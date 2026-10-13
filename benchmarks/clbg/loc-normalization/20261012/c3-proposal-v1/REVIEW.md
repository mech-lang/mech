This is an unexecuted, unhooked C3 source-accounting proposal. It changes no
existing normalizer, counter, producer, source, published report or audit18.
All imports, fixture execution, source lexing, hash sweeps and normalization
must hold /tmp/mech-clbg-timing.lock. No count is admitted to charts by this code.

The current selected compiler is official C3 0.8.5, source commit
640091d3303a393f9651d644063b837fc19d05f1. The separate C3 source author selects
fn signatures, structs/nested struct fields, typed locals with C3 pointer/array/
slice types, module/import, function-pointer/generic-type aliases, ordinary
for/foreach/while/if, simple defer calls/blocks, and C-ABI extern fn @cname
prototypes. Standard map generics appear as alias Map=HashMap{String,long};
application/runtime/GMP/PCRE binding sources remain individually countable.
No named benchmark chooses an accounting path. Native/source qualification
and all dependency bytes are separate mandatory evidence before admission.

The proposal has a C3-owned lexical mask and statement/scope parser. It does
not call a C lexer/classifier or rewrite C3 into C. It imports only the existing
language-neutral foreign_core owner/accounting mechanics and their approved
canonical source-unit rule. Root must pin that exact dependency chain in its
later snapshot before running this proposal. Struct/fn/control headers each
count once, physical opening braces add no imputed unit, each actual block close
counts once, and every independent declaration/write/statement remains visible.
Function formals, loop headers, calls, type parameters and initializer members
are structural owners. Multiple typed names and chained writes expand instead
of benefiting from packed source. Literal/data line wrapping stays visible.

A fn arrow is one composed definition only for proven builtin numeric/boolean
formal arithmetic with matching scalar return ownership. Calls, void arrows,
custom operands, unknown globals and executable callback arguments stay gated. A mutating fn arrow retains a
separate header/body and a semantic review diagnostic. A simple deferred call
is one wrapped statement across formatting; a defer block has its real header,
statements and written close. Root approved simple defer-call composition; scalar arrow ownership is
conservatively limited to the proved arithmetic subset.

The tagged 0.8.5 statement parser rejects initialization on packed multiple
local declarations. Such source gets a diagnostic; valid uninitialized packed
names retain one unit per independent declaration. The tagged parser also
requires unbraced if/while/for bodies on the closing-header line. Expanded
positive fixtures preserve that syntax; split invalid bodies are gated.

Nested /* comments */ and doubled-backtick raw strings follow the tagged
compiler lexer. Quoted/character/raw literals preserve exact character/newline
positions. Doc contracts <*...*> are semantically parsed by C3 and stay review
gated. Compile-time/reflection/macro identifiers, macros, asm, interfaces,
bitstructs, enum/constset/excuse declarations, unknown attributes, switch arms,
try/catch conditions, extended defer and encoded byte literals stay gated.
Unsupported/malformed aliases never swallow executable callbacks as one header.
Every diagnostic requires exact source-local semantic review or an additive
qualified grammar revision; candidate-parser-pass alone is not admission.

Meaningful fixtures compare packed and expanded source for actual selected
forms and reject hidden alias bodies, invalid foreach headers, mutating arrows,
comma sequences, contract/macro source, missing boundaries and malformed input.
These fixture expectations are authored, not claimed passing. Root will review
and arrange a mutex-held run with an immutable source/dependency snapshot.

Primary grammar evidence used during authoring:

- [Tagged C3 0.8.5 lexer](https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/src/compiler/lexer.c)
- [Tagged C3 0.8.5 statement grammar](https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/src/compiler/parse_stmt.c)
- [Tagged C3 0.8.5 global declaration grammar](https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/src/compiler/parse_global.c)
- [Tagged C3 0.8.5 expression grammar](https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/src/compiler/parse_expr.c)
- [Official C3 grammar](https://c3-lang.org/implementation-details/grammar/)
- [Official function grammar/examples](https://c3-lang.org/language-fundamentals/functions/)
- [Official comments/contracts](https://c3-lang.org/language-fundamentals/comments/)
- [Official defer semantics](https://c3-lang.org/language-common/defer/)

The mutable documentation is background. The tagged compiler grammar and
actual frozen authored source forms are authoritative for a qualified count.

Static authored-source coordination now covers support/{buffer,threads,gmp,
pcre2}.c3 and binarytrees/pidigits compact/optimized. Concrete additional forms
include gmp::Mpz and buffer::Buffer qualified type names, Node::size reflection
without compile-time dollar identifiers, positional/empty struct initializers,
char[65536] arrays, arbitrary-width ordinary scalar casts and std/libc call
composition. No native source, lexer or fixture has executed from this proposal.

The separate queue_fixtures.py worker copies exactly c3_mask.py, foreign_c3.py,
test_foreign_c3.py and this REVIEW.md plus foreign_core.py and canonical.py
from the already-frozen boundary-audit-18/snapshot. It also freezes itself as
the job's worker.py. Under the common mutex it pins those bytes, the actual
Python executable and every .py in the reviewed Pygments 2.19.2 package, checks
all imported proposal/dependency locations, executes only test_foreign_c3,
requires exactly sixteen reviewed fixture methods, and reconciles pins afterwards.
Its exclusive worker-owner.lock prevents duplicate
execution, and exclusive receipt/log creation preserves failures. It reads no
authored C3 benchmark source and admits no chart counts.
The parent uses a distinct queue.lock for its atomic launch, so the child cannot
fail on that launch lock while the parent still records its process identity.

Proposed command for root review, not yet run (worktree root):

```
/opt/homebrew/opt/python@3.14/bin/python3.14 -B benchmarks/clbg/loc-normalization/20261012/c3-proposal-v1/queue_fixtures.py --job benchmarks/clbg/loc-normalization/20261012/c3-proposal-v1/jobs/fixtures-v1 --core-snapshot benchmarks/clbg/loc-normalization/20261012/jobs/boundary-audit-18/snapshot --dependency-root /tmp/mech-clbg-compact-libs
```
