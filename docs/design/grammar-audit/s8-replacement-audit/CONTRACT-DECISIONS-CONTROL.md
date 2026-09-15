# Control and callable prerequisite reconciliation

This is an audit of frozen B `662d29b79`, not an implementation change or a
completion claim. It resolves G14, G15, G19 and G20 against the repository's
language authority and identifies the finite acceptance cells still owed.
The root agent subsequently executed the additions in the 375-case audit run.
The observed result reconciliation appears below; these are frozen-gap
observations, not passing behavior certification.

## Decisions already established

| Gap | Reconciled decision | Responsible implementation owners |
| --- | --- | --- |
| G14 | FSM execution, including asynchronous continuation, is required. The committed specification already decides yield eligibility, commit/abort, capture, cancellation and fairness. It is not an unanswered product choice and cannot be closed as an expected user rejection. | Engine canonical declaration/type environment and typed control artifact; resident prepared/committed machine state; runtime scheduler eligibility/wakeups. |
| G15 | Fixed and patterned activation have existing observable sampling, dispatch and atomic-register contracts. Their missing resident owner is a prerequisite. Activation-scope context sends remain explicitly excluded from v0.4; they are not additional implementation scope. | Engine canonical trigger/pattern/control lowering; resident sampled inputs, captures and candidate registers; runtime trigger scheduling. |
| G19 | Ordered pattern-bodied functions are existing language behavior. Typed statement-bodied inlining does not replace them. | Engine callable declaration/body lowering and canonical pattern binding; shared type environment; typed artifact control/call representation. |
| G20 | Self-recursion is explicitly supported. The missing task is bounded resident call execution, not deciding whether recursion exists. No unbounded compiler inlining, source interpretation or compatibility reader is acceptable. | Typed callable artifact, resident call-frame/work/memory admission and candidate failure owner. |

`canonical-source-boundary.mec:4–8` makes `specification.mec` the language
authority. The reference documents and former implementation/tests below are
supporting evidence about observable behavior, not a second semantic authority.
They are not to remain production dependencies after cutover.

## G14: FSM has a specified lifecycle, not a blank design space

The authoritative grammar is `specification.mec:3248–3295`. In particular,
`->` has state, statement and block forms, `=>` publishes an output, and `~>`
is an asynchronous state transition. Bare invocation, declaration,
specification and implementation are distinct retained forms.

The normative continuation contract is at `specification.mec:3996–4051`:

- A yield becomes eligible only on the next committed reactive turn; the
  scheduler requests that turn without needing an external input.
- State, continuation and other writes prepare and commit together. A failed
  or discarded new yield creates no durable wakeup. A previously committed
  continuation survives a failed/discarded resume.
- Consecutive yields cross distinct turns, including fixed-point passes.
- Invocation arguments and bindings live across a yield survive by value in
  managed canonical storage. External reads after resumption use the resumed
  turn's input capture; earlier lexical bindings retain their prior values.
- The last committed output remains published. An instance with no output has
  no invented default.
- Reset/replacement invalidates the instance generation. Wakeups check that
  generation, coalesce per continuation and resume at most once per turn.
- Turn work and scheduler drain budgets bound progress; internally requested
  turns interleave with other runnable work.

`docs/reference/state-machine.mec:5–33` supplies declaration/payload consistency
and ordered transition examples. TrafficLight(6) returns 0; Fibonacci(10)
returns 55. These examples must use canonical spelling and explicit numeric
kinds in executable fixtures; historical typography alone is not a parse oracle.

The frozen `artifact/fsm.rs:42–46` stores a machine name, invocation arguments
and a list of pipe stages. It does **not** contain machine state declarations,
guards or implementation bodies. Thus G14 is both a missing declaration/body
representation and a resident/scheduler prerequisite. Adding an FSM resident
factory to the current pipe shell would not satisfy it.

The six-row `phase-2i-semantic-completion.tsv` already marks `fsm-runtime` as an
S4 obligation required for S6. Its `intentionally-unavailable` result records
an implementation stop. `comprehension-certification-migration.md:83–91`
explicitly says the artifact witness is not behavior completion or an expected
user error. No new permission is needed to classify that omission accurately.

Finite acceptance cells (each must be source → artifact → bytecode → resident,
with independent expected values or publication/turn counters):

| Cell | Required assertion | Current evidence |
| --- | --- | --- |
| FSM01 | A declared machine binds positional and named inputs; rejects duplicate/missing inputs, unknown machine/state, incompatible payload/output kinds and duplicate declarations at retained anchors. | Grammar/reference contract; complete document collector rejects these units. |
| FSM02 | State and guarded transitions select the intended branch; declaration payloads survive transitions; TrafficLight(6)=0 and Fibonacci(10)=55. | Reference examples; resident behavior untested and blocked by G14. |
| FSM03 | Statement/block transitions use ordinary state-candidate ordering; outputs publish only a completed successful transition. | Grammar plus atomic continuation contract; blocked by G14. |
| FSM04 | One yield resumes without any external input; creating turn cannot also resume it. | Explicit specification; blocked by G14. |
| FSM05 | Two yields cross two distinct later turns; no extra same-turn fixed-point resume. | Explicit specification; blocked by G14. |
| FSM06 | Failed and discarded new yields publish neither state nor output nor durable wakeup. | Explicit specification; blocked by G14. |
| FSM07 | A failed/discarded resumed turn preserves the earlier committed continuation; retry advances it exactly once. | Explicit specification; blocked by G14. |
| FSM08 | Reset and replacement reject queued stale generations. | Explicit specification; blocked by G14. |
| FSM09 | Earlier lexical binding differs from changed external input read after resume; managed composite captures remain valid. | Explicit specification; blocked by G14 and shared structural-value qualification. |
| FSM10 | No output means no default value; yielding preserves the last committed output until an output transition succeeds. | Explicit specification; blocked by G14. |
| FSM11 | Duplicate wakeups coalesce; each instance resumes at most once per turn. | Explicit specification; blocked by G14. |
| FSM12 | A repeated async chain exhausts a bounded drain while another runnable instance progresses, leaving pending work for a later drain. | Explicit specification; blocked by G14. |
| FSM13 | Malformed typed machine/control references and excessive work/storage fail admission or the candidate without partial publication. | Existing artifact/turn memory authorities; new FSM representation is not present. |

Current executable frontier witness:

```sh
MECH_AUDIT_CASE=fsm-pipe MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
```

This fixture invokes an undeclared `#machine`. It demonstrates the current
artifact-to-target stop, **not** correct behavior of a complete valid machine.
`canonical_source_semantics::typed_fsm_fails_closed_until_the_resident_continuation_owner_lands`
separately asserts that temporary fail-closed behavior. Neither is positive
FSM certification. FSM01/FSM02 need a declared-machine audit fixture before
this group can claim a positive executable acceptance witness.

## G15: migrate sampled activation behavior without enlarging the language

`specification.mec:3297–3307` and `:3983–4001` distinguish activation from FSM
asynchrony. The FSM's internally requested next-turn rule must not be copied
onto an ordinary activation trigger.

The old `statements/tests/activation_scope.rs` contains exactly **26 tests**.
The public behavior they assert is useful migration evidence: no execution at
load, execution on the trigger, sampling current external values on that
trigger, local propagation, ordered patterned dispatch, isolated lexical
bindings, atomic register changes and rollback. Private node IDs, interpreter
cells and topology layouts are not retained product contracts. The canonical
replacement must prove observable values, work bounds and publication through
resident activation/host turns instead.

Important exclusions and fixture correction:

- The existing audit source is `~>1 + 2{}`. It is a **syntax** witness copied from
  the S7 syntax certification. Frozen execution requires a stable variable
  reference (`mechdown.rs:885–894`), so this is not a known valid positive
  activation-program witness. Keep the observation, but do not use it to demand
  arbitrary-expression trigger semantics. A valid positive frontier fixture
  should use `tick := 0; ~x := 0; ~> tick { x = x + 1 }; x`, with actual trigger
  packets in the eventual behavior test.
- Activation-scope context sends are deliberately unsupported in v0.4:
  `archive/gate-e2-closeout.md:42–44,61`. They must stay a positioned negative
  case; G15 does not authorize adding effects inside activation scopes.
- Fixed-scope nested activation/declarations, own-trigger writes and mutable
  definitions already reject in `mechdown.rs:741–785`. Patterned arms require
  a final unguarded irrefutable arm, reject an early unguarded wildcard, and
  isolate arm-local declarations (`activation/validation.rs:104–168`).
- The old static-pure guard checker rejects nested control and some function
  calls as an implementation limit. The invariant is pure, lazy guarded
  execution; those old implementation restrictions are not proof that the
  language forbids every pure composed guard. Shared G07/G09/G19 lowering must
  decide admission from the canonical control/effect contract, not preserve an
  interpreter topology limitation as new language semantics.

Finite acceptance cells:

| Cell | Required assertion | Source of observable oracle |
| --- | --- | --- |
| ACT01 | Load/activation does not run the scope body or advance its registers. | `activation_scope_does_not_execute_during_load` (185). |
| ACT02 | A trigger causes one body evaluation; sampled external changes alone cause none; the next trigger observes the latest external value. | tests at 207,221,263,287,298. |
| ACT03 | Body-local dependent expressions see current local results, without retriggering the scope after commit. | tests at 245,343. |
| ACT04 | Two register writes commit atomically; body failure/discard preserves published registers and permits retry. | test at 328; candidate failure contract; failure/discard resident port still owed. |
| ACT05 | First matching successful guard wins; unselected bodies do not execute failing operations. | tests at 585,616 with expected 120,205,-1 and retained -1 after failure. |
| ACT06 | Tuple/tagged/array/rest/repeated/computed patterns bind correct values; no bindings leak across arms or overwrite the outer binding. | tests at 515,559,914,1074; depends on G06/G08/G09/G11. |
| ACT07 | Pattern expressions and called-function results sample only on trigger, using the current capture. | tests at 1100,1140; depends on G19. |
| ACT08 | Nonexhaustive arms, early wildcard, incompatible repeated binding, impure guard, invalid trigger and own-trigger write reject before partial installation. | tests at 395,428,446,465,653,757,824,883 and stable-reference check. |
| ACT09 | Context sends and excluded nested definitions remain positioned rejections before effects or registration. | explicit E2 target contract and preflight source checks. |
| ACT10 | Repeated triggers do not grow executable topology or retained capture storage; reset/replacement releases or invalidates the old owner. | tests at 548,559,585,914 plus resident lifecycle contract; storage/lifecycle port still owed. |

The current executable frontier command is:

```sh
MECH_AUDIT_CASE=activation-scope MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
```

It must remain labeled syntax-only until supplemented by the valid stable
trigger fixture and the resident trigger tests above. The collector's explicit
ActivationScope rejection establishes the missing family independently.

## G19/G20: functions already include patterns and recursion

`specification.mec:1456` says branches match in order and can recurse. The
current grammar at `:3209–3238` retains both statement bodies and match arms,
including named arguments. Specification `:1081–1089` defines homogeneous
collection lifting. `docs/reference/function.mec:31–38` gives first-match
selection, expression bodies, nested calls and vector/matrix broadcast.
`examples/working/factorial.mec` is the maintained 5 → 120 canary.

Frozen `source_semantics/document_functions.rs:92–99` rejects an active
function name (and deep inlining); `:135–144` accepts only
FunctionDefineStatements. These are two demonstrated owners, G20 and G19,
not one generic failure. G19 can be qualified with nonrecursive pattern bodies
before G20 changes execution. `MAX_CONTROL_DEPTH = 8` at
`artifact/control.rs:335` is a structural control-graph limit, not evidence that
the language permits only eight recursive calls.

There are two contract details that a naive match rewrite would miss:

1. Function first-match behavior includes an undefined-output error when no
   non-enum arm matches; old `function/mod.rs:328–479` separately checks enum
   exhaustiveness. Canonical scalar-match lowering currently requires its own
   exhaustive wildcard/bool conditions. Do not reject a partial numeric
   function's matching call merely to reuse that helper. The language text
   specifies ordered branches but does not prescribe rejection timing, so the
   acceptance cell must preserve a successful matching call and a fail-closed
   nonmatching call; a new timing guarantee is not inferred.
2. The authoritative specification at `:1081–1089` explicitly lifts any function
   taking a single value over homogeneous collections, including sets and
   matrices. The old code at `function/mod.rs:272–320` implements only a
   narrower same-input/output-kind matrix subset; that subset cannot narrow the
   replacement contract. FUN05 therefore includes sets and differing valid
   output kinds, matrix shape/order and canonical set deduplication. Multiple
   argument zip/cross broadcasting is not inferred. Explicit matrix-typed
   parameters remain a separate already-tested responsibility.

Finite acceptance cells:

| Cell | Required assertion | Owner / evidence |
| --- | --- | --- |
| FUN01 | Nonrecursive scalar pattern body, ordered overlapping arms and omitted-pattern wildcard body return the independently expected result. | G19; current `pattern-function` expects 3 on both turns. |
| FUN02 | Positional and reordered named calls bind identical typed arguments; duplicate/missing/unknown names and incompatible argument/output kinds reject at the call/definition anchor. | G19; existing `canonical_document_functions_inline_typed_named_calls_without_leaking_bindings` is statement-body evidence only. |
| FUN03 | Tuple/array/tag patterns and lexical bindings use the shared canonical pattern owner without caller/arm binding leakage. | G19 + G06/G09/G11. |
| FUN04 | Nested calls in a formula return the correct value; uncalled functions neither bind inputs nor plan providers. | G19; existing statement-body/resource tests at runtime/program/tests.rs:5521,5603,5629,6002. |
| FUN05 | Single-value functions lift over homogeneous matrices and sets, including valid differing scalar output kinds; retain matrix shape/order, deduplicate set results canonically, and make empty/failing-element behavior atomic. | G19; specification :1081–1089; old implementation covers only a subset. The matrix probe corrected single `&` to `&&`; the published-contract rerun reaches lowering and reports `unsupported-function-body`, confirming the G19 prerequisite. |
| FUN06 | A matching partial numeric function succeeds; a nonmatching call has no invented output; enum exhaustiveness is checked against declared variants. | G19 + G11; source contract and old defined-output behavior. |
| FUN07 | A self-recursive base case and non-tail factorial(5)=120 run with source/bytecode agreement on subsequent turns. | G20; `factorial` and `statement-recursion` explicitly expect 120,120. |
| FUN08 | Branching recursion fib(10)=55 and tail-recursive countdown terminate without graph expansion; local frames do not alias. | G20; reference Fibonacci; runtime architecture acceptance still owed. |
| FUN09 | Excessive/nonterminating recursion exhausts one bounded work/frame budget without host-stack overflow or partial state/output publication; valid retry succeeds. | G20; shared resident budget/publication contract, not an arbitrary eight-call language limit. |
| FUN10 | Recursive typed aggregate arguments/results obey managed storage accounting, bytecode validation and drop/abort cleanup; recursion invoked within shared control uses the same work budget. | G20 + structural value/control owners; acceptance untested. |

Executable frozen failure witnesses, independently selectable:

```sh
MECH_AUDIT_CASE=pattern-function MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=factorial MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=statement-recursion MECH_AUDIT_REQUIRE_PASS=1 \
  ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
```

FUN01–FUN10 do not add higher-order functions, polymorphic recursion, function
overloading, new function guard syntax or multi-input broadcast rules.
`type-system-v1.md:237` excludes the inference expansions; the current grammar
and explicit callable boundary bound the rest. Mutual recursion is not needed
to satisfy the demonstrated self-recursion deficit; the declaration model must
nevertheless diagnose an unsupported cycle deterministically rather than loop
in the compiler. Its target admission belongs to G20, not an unowned future
category.

## Review boundaries and work still required before implementation

The existing R8/R9 proposal bundles genuinely different owners. Refine the
review boundaries without opening a general language-redesign PR:

1. **G19 callable pattern bodies**: FUN01–FUN06; depends on shared structural
   patterns/types and keeps the existing statement-body path's behavior.
2. **G20 bounded callable execution**: FUN07–FUN10; first review the typed call
   representation, frame lifetime, work charging and failure semantics. The
   product decision to support recursion is already settled.
3. **G14 typed machine declarations and synchronous execution**: FSM01–FSM03
   and representation/malformed portions of FSM13. This must not be called a
   completed FSM milestone by itself.
4. **G14 resident/scheduler continuation lifecycle**: FSM04–FSM13; exact
   specification contract above, including actual internally requested turns.
5. **G15 sampled fixed/patterned activation**: ACT01–ACT10; uses shared pattern,
   control and value owners, with context sends excluded. It need not wait for
   asynchronous FSM semantics merely because both use `~>` in source.

These are **33 finite acceptance cells**, not 33 defects or 33 tests already
written. Parameterized scalar/schema/backend qualification still belongs to the
closed O01/O05/O06 inventories and must link to these cells rather than generate
new unsorted scope. G14 and G20 need reviewed runtime representation designs;
neither needs the user to decide again whether the language family exists.

Before treating the control remainder as ready for implementation: replace the
syntax-only positive activation assumption; add complete declared-machine
witnesses; add independently expected function broadcast/partial-call cases;
write the typed callable/machine execution design with exact existing budget and
publication integration points; and map all 33 cells to executable canonical
acceptance tests or explicit blocked test plans. The implementation freeze
remains in place while that audit work is completed.

## Recorded control fixture reconciliation

The first `/private/tmp/mech-syntax-qualification/s8-audit-contract-probes.log`
contained nine additions in the 375-case run. Eight admitted strict syntax and
indexing; the broadcast fixture first failed because it used single `&`. The
corrected `&&` fixture was rerun in `s8-audit-reconciled-contract-probes.log` and
now also reaches the expected missing semantic owner:

| Fixtures | Observed frontier | Interpretation |
| --- | --- | --- |
| `activation-stable-no-trigger`, `activation-pattern-no-trigger` | `unsupported-document-unit` for ActivationScope | Valid stable-trigger positive sources demonstrate G15's collector gap; the old `~>1 + 2{}` syntax witness is no longer the positive evidence. |
| `fsm-declared-increment`, `fsm-declared-named-input` | `unsupported-document-unit` for FsmSpecification | Complete declared-machine sources demonstrate G14's declaration gap. They do not yet execute transitions or certify scheduling. |
| `function-pattern-partial-match`, `function-pattern-first-match` | `unsupported-function-body` | Valid sources demonstrate G19 body lowering; output oracles are 42 on both turns. |
| `function-branching-recursion`, `function-tail-recursion` | `unsupported-function-body` | Valid syntax, but G19 masks G20. Do not count these as independent recursion failures; the existing statement-body recursion witness isolates G20. |
| `function-pattern-broadcast-matrix` | corrected source: `unsupported-function-body` | The `&&` source passes syntax/index and isolates G19. The earlier single-`&` index rejection was a fixture mistake; specification :2832 requires `&&`, `∧` or `⋀`. |

No production parser change is justified by the original malformed broadcast
probe. The corrected probe demonstrates only the existing G19 body-lowering gap. `control-acceptance-cells.tsv` now retains the per-fixture observed stage
and log, and `rule-acceptance-links.tsv` links this control slice to concrete
cells without treating syntax membership as execution coverage.
