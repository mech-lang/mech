This additive C3 0.8.5 grammar proposal is source-only and unexecuted. It preserves
all c3-proposal-v1/v2 inputs, snapshots and actual failed receipts. No benchmark
source, shared normalizer, LOC18 dependency, measured report or chart is changed.
Every parser/import/fixture/hash/reconciliation operation must hold the common
/tmp/mech-clbg-timing.lock. No launch is authorized before root reads this packet.

Actual v2 outcome: nineteen exact reviewed methods started, two failures, zero
errors/skips/expected failures/unexpected successes. Raw evidence remains at
c3-proposal-v2/jobs/fixtures-v2/{fixtures.log,fixture-receipt.json,
fixture-receipt-pin.json}. V3 retains the same nineteen method IDs and every
previous positive/negative expectation; the two failing expectations are not
weakened.

The first failure was emit({.value=call(1,2),.count=3}); counted as two independent
writes instead of one call statement. The C3 parser already proved both '='
tokens belonged directly to dot-designated initializer fields, but group(args)
skipped the entire argument expression and retained only its args scope.
Language-neutral accounting correctly does not exempt assignments merely
because they appear in call arguments. V3 carries each exact proved enclosing
initializer brace into a C3-owned data scope. It does not enable a generic
named-equals exemption: call(value=next()), object.field=next(), nested runtime
writes, mutations and executable callback/semicolon bodies retain diagnostics.
The existing method adds a nested-call positive and a nested-write negative.

The second failure was int x=1 followed by int y=2; without the first semicolon.
V2 looked for a nested declaration only in the line fragment passed by the
language-neutral counter. That counter deliberately splits remaining source by
physical line, so the check could not see the missing boundary. V3 checks the
complete C3 semicolon-terminated statement before line accounting. It preserves
the first typed declaration's actual type/name head, skips parsed delimiter
scopes, and diagnoses any subsequent proved typed owner at the outer statement
level. Missing boundaries across newlines/comments, same-line declarations,
qualified types and pointer types stay gated. A legitimate wrapped initial
type/name head remains valid. The existing malformed-input method includes
these regression subcases; no fixture ID is added or removed.

All prior bounded policies remain: C3-owned lexical masking; fn/type/control
headers and real written closes; separate declarations/chained writes; typed
formals/loop/data scopes; bounded scalar arrow arithmetic; selected scalar
case/default arms and independently counted arm work; one typed extern variable;
no silent macro/contract/unknown syntax exemptions. Diagnostic source must have
an actual semantic disposition or a new qualified grammar revision before any
source-count admission. No benchmark-specific dispatch is present.

Tagged primary evidence remains the official C3 v0.8.5 commit
640091d3303a393f9651d644063b837fc19d05f1 lexer, parse_stmt.c, parse_global.c and
parse_expr.c. Only language-neutral foreign_core.py and canonical.py from the
already frozen boundary-audit-18 snapshot are reused. C/C++ lexers/classifiers
are never selected.

Packet inventory: c3_mask.py (byte copy of v2), foreign_c3.py, test_foreign_c3.py,
REVIEW.md, queue_fixtures.py. The direct worker freezes the first four plus exact
foreign_core.py/canonical.py and itself. It requires the actual v2 nineteen/two
failed outcome, exact discovered/started IDs and successful pin reconciliation;
pins/reconciles its original and copied receipt plus the original failed raw
log and receipt-pin file under the common lock. It then requires exact nineteen
actual v3 executions and zero failures/errors/skips/expected/unexpected outcomes.
Pygments 2.19.2 and Python executable dependency pins remain required. Failed
jobs are terminal and preserved, with no retry. This fixture job imports/counts
no scored C3 benchmark source and admits no source count to a chart.
The worker's terminal finally stays inside the actual common flock and pins
the immutable saved receipt plus every available fixture/exception log on
success and failure. A separate terminal_worker_status records failure after
an already saved pass without rewriting that pass. Receipt-pin creation is
exclusive; pin/read/write failures propagate fatally. Actual Python 3.14 and
the queued executable identity are asserted before importing proposal code.

Proposed direct command for root review, not executed:

```
/opt/homebrew/opt/python@3.14/bin/python3.14 -B benchmarks/clbg/loc-normalization/20261012/c3-proposal-v3/queue_fixtures.py --direct --job benchmarks/clbg/loc-normalization/20261012/c3-proposal-v3/jobs/fixtures-v3 --core-snapshot benchmarks/clbg/loc-normalization/20261012/jobs/boundary-audit-18/snapshot --dependency-root /tmp/mech-clbg-compact-libs --prior-fixture-receipt benchmarks/clbg/loc-normalization/20261012/c3-proposal-v2/jobs/fixtures-v2/fixture-receipt.json
```
