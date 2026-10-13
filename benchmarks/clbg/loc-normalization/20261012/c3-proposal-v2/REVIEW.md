This is an additive, unexecuted, unhooked C3 v2 source-accounting proposal.
The entire frozen c3-proposal-v1 packet and fixtures-v1 job remain unchanged.
This revision may not execute or queue until v1's terminal outcome is observed
and root reviews the concrete v2 packet. It changes no
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
bitstructs, enum/constset/excuse declarations, unknown attributes, complex switch arms,
try/catch conditions, extended defer and encoded byte literals stay gated.
Unsupported/malformed aliases never swallow executable callbacks as one header.
Every diagnostic requires exact source-local semantic review or an additive
qualified grammar revision; candidate-parser-pass alone is not admission.

V2 owns direct scalar case/default headers within a written switch block.
Each actual case/default header counts once, independently of each real return,
call, break or body block. Character literals are identified by the original
C3 mask's exact complete single-quote span, so string/raw literal placeholders
cannot become fictitious constant identifiers. Integer/boolean literals and
ordinary uppercase constant names compose; range/type/dynamic arms and labels
outside the direct switch block retain diagnostics. This is a C3-owned bounded
arm grammar, not an alias to a C/C++ switch classifier.
Bare body blocks have zero units for the written opening brace and retain
their real statements and close, matching the same opening-brace policy.

V2 rejects fn/macro/arrow/semicolon bodies inside initializer or index owners,
and rejects increments/decrements or non-field writes hidden in data/call
composition. A dot-field assignment is structural only directly inside proved
initializer braces: object.field=next() in a call is a runtime effect. Normal
positional/data arrays, dot-designated struct fields, scalar initializer calls,
simple indices and &allocators::LIBC_ALLOCATOR arguments remain countable.

V2 proves one typed extern variable with optional known ABI attributes as one
declaration header, covering the scored GMP version binding. Initialized or
multiple external variables remain diagnostics; packed distinct variables still
retain their independent declaration units. Extern fn bodies/arrow bodies are
also explicitly gated, following the tagged global declaration parser.

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

Static authored-source coordination now covers all twenty compact/optimized
sources and scored support/{buffer,threads,gmp,pcre2}.c3. Concrete additional forms
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
requires exactly nineteen reviewed fixture methods, and reconciles pins afterwards.
Its exclusive worker-owner.lock prevents duplicate
execution, and exclusive receipt/log creation preserves failures. It reads no
authored C3 benchmark source and admits no chart counts.
The reviewed --direct mode records process.json before exec of the frozen worker
in the same PID. It creates no detached child, so a failed identity journal cannot
leave an unjournaled worker running. A failed exec receives an additive terminal
failure receipt; no retry occurs. The old detached queue branch remains an unused
source candidate and is not the reviewed command. The atomic freeze queue.lock
and worker-owner.lock remain distinct.
V2 queueing first requires V1's actual terminal fixture receipt, copied unchanged
as prior-fixture-receipt.json beside the new snapshot. Under the common mutex
the worker pins/reconciles both the original and copied prerequisite bytes and
requires an exclusive-mutex, unscored terminal status. No V1 outcome is assumed.
The worker requires the exact nineteen reviewed fully qualified test identities
to be discovered and actually started, exactly nineteen executions, and zero
skips, expected failures or unexpected successes. The raw fixture log is pinned
in the receipt; an exclusive separate fixture-receipt-pin.json pins the completed
receipt and repeats its raw-log pin under the same actual mutex. A suite count or
wasSuccessful() alone cannot qualify this proposal.

Proposed command for root review, not yet run (worktree root):

```
/opt/homebrew/opt/python@3.14/bin/python3.14 -B benchmarks/clbg/loc-normalization/20261012/c3-proposal-v2/queue_fixtures.py --direct --job benchmarks/clbg/loc-normalization/20261012/c3-proposal-v2/jobs/fixtures-v2 --core-snapshot benchmarks/clbg/loc-normalization/20261012/jobs/boundary-audit-18/snapshot --dependency-root /tmp/mech-clbg-compact-libs --prior-fixture-receipt benchmarks/clbg/loc-normalization/20261012/c3-proposal-v1/jobs/fixtures-v1/fixture-receipt.json
```
