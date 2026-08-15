Mech Reflective Runtime Specification
===============================================================================

This specification is the canonical policy surface for the first reflective
conformance vertical slice. The host reports runtime, repository, backend, and
benchmark facts. The predicates below decide whether those facts are allowed.

The live rollback result is {transaction-abort-status}. Its typed semantic link
depends on the same observation relation as the integrity predicate evaluated
by the resident Mech executor.

contract-profile := "mech-contract-1"

-- @contract-profile mech-contract-1
-- @glossary resident-route
-- @glossary carried-state
-- @glossary transaction-record
-- @glossary committed-state
-- @glossary semantic-event
-- @glossary runtime-ingress
-- @glossary capability-grant
-- @glossary runnable-instance
-- @glossary parser-structure
-- @glossary backend-admission
-- @glossary benchmark-protocol

Resident execution
-------------------------------------------------------------------------------

-- @requirement-id RES-001
-- @title Resident programs use the production route
-- @level must
-- @area execution
-- @normative Every resident case must execute through the production resident-pure route.
-- @claim all resident-case use resident-route
-- @contract-claim all resident-case use resident-route
-- @term resident-route
-- @contract case-runs-residently!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example resident-route case-runs-residently! resident-route="resident-pure"
-- @counterexample missing-route case-runs-residently! resident-route="none"
-- @outside-scope source-compilation-without-activation
-- @mutation accept-any-route case-runs-residently! critical resident-route === resident-route
-- @ratification 2e4cedb99ba072383d3a03fcb535cd52208c053d5172a6812735dcb53d6617b7 corey
-- @end-requirement

-- @requirement RES-001
case-runs-residently! := resident-route === "resident-pure"

-- @requirement-id RES-002
-- @title Resident carried state persists across turns
-- @level must
-- @area execution
-- @normative Every dependent resident turn must preserve declared carried state and advance the turn counter.
-- @claim all dependent-resident-turn preserve carried-state
-- @contract-claim all dependent-resident-turn preserve carried-state
-- @term carried-state
-- @contract resident-state-persists!
-- @contract resident-turn-advances!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example state-persists resident-state-persists! resident-initial-state="state-after";resident-next-turn-state="state-after"
-- @counterexample state-lost resident-state-persists! resident-initial-state="state-after";resident-next-turn-state="state-before"
-- @outside-scope reset-migrate-or-shutdown
-- @mutation compare-initial-to-itself resident-state-persists! critical resident-initial-state === resident-initial-state
-- @ratification acd3c29b984a136a2d8681fef1fede2ec92933493cd17c7b54650ae7db4d10a9 corey
-- @end-requirement

-- @requirement RES-002
resident-state-persists! := resident-initial-state === resident-next-turn-state

-- @requirement RES-002
resident-turn-advances! := resident-turns-advanced === true

Activation authority
-------------------------------------------------------------------------------

-- @requirement-id ACT-001
-- @title Missing hard grants reject activation
-- @level must
-- @area activation
-- @normative Every resident activation missing a hard capability grant must be rejected as an authorization denial.
-- @claim all missing-hard-grant produce capability-grant
-- @contract-claim all missing-hard-grant produce capability-grant
-- @term capability-grant
-- @contract activation-missing-grant-rejected!
-- @contract activation-denial-classified!
-- @profile resident-cpu must-pass
-- @evidence activation activation-trace/1 observed
-- @example rejected activation-missing-grant-rejected! activation-outcome="rejected"
-- @counterexample admitted activation-missing-grant-rejected! activation-outcome="accepted"
-- @outside-scope activation-with-all-hard-grants
-- @mutation accept-any-outcome activation-missing-grant-rejected! critical activation-outcome === activation-outcome
-- @ratification 787a6345b4b74389b695ac1363bea3bb8f3a76f1af891c2a8886a64d5838db12 corey
-- @end-requirement

-- @requirement ACT-001
activation-missing-grant-rejected! := activation-outcome === "rejected"

-- @requirement ACT-001
activation-denial-classified! := activation-failure-class === "authorization-denied"

-- @requirement-id ACT-002
-- @title Failed activation creates no runnable instance
-- @level must
-- @area activation
-- @normative Every rejected activation must leave no runnable reactive instance behind.
-- @claim all rejected-activation avoid runnable-instance
-- @contract-claim all rejected-activation avoid runnable-instance
-- @term runnable-instance
-- @contract failed-activation-has-no-instance!
-- @profile resident-cpu must-pass
-- @evidence activation activation-trace/1 observed
-- @example no-instance failed-activation-has-no-instance! activation-instance-created=false
-- @counterexample leaked-instance failed-activation-has-no-instance! activation-instance-created=true
-- @outside-scope successfully-activated-instance
-- @mutation accept-created-instance failed-activation-has-no-instance! critical activation-instance-created === activation-instance-created
-- @ratification 32cae69f44f7e87c255449fd1c211fa69599e2ba254238586391055e32460eb5 corey
-- @end-requirement

-- @requirement ACT-002
failed-activation-has-no-instance! := activation-instance-created === false

Committed transactions
-------------------------------------------------------------------------------

-- @requirement-id TURN-001
-- @title Commit is durably and semantically recorded
-- @level must
-- @area transaction
-- @normative Every committed transaction must produce a durable record and semantic commit event.
-- @claim all committed-transaction produce transaction-record
-- @contract-claim all committed-transaction produce transaction-record
-- @term transaction-record
-- @term semantic-event
-- @contract transaction-commit-outcome!
-- @contract transaction-commit-recorded!
-- @contract transaction-commit-event-recorded!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example committed transaction-commit-outcome! commit-outcome="commit"
-- @counterexample aborted transaction-commit-outcome! commit-outcome="abort"
-- @outside-scope read-only-transaction
-- @mutation accept-any-outcome transaction-commit-outcome! critical commit-outcome === commit-outcome
-- @ratification 03100f9be0165510d302a066b977119578ab7df164fdea4decad7fbf34350113 corey
-- @end-requirement

-- @requirement TURN-001
transaction-commit-outcome! := commit-outcome === "commit"

-- @requirement TURN-001
transaction-commit-recorded! := commit-record-observed === true

-- @requirement TURN-001
transaction-commit-event-recorded! := commit-event-observed === true

-- @requirement-id TURN-002
-- @title Commit publishes transaction-visible state
-- @level must
-- @area transaction
-- @normative Every successful commit must make transaction-visible state durable.
-- @claim all successful-commit publish committed-state
-- @contract-claim all successful-commit publish committed-state
-- @term committed-state
-- @contract transaction-commit-applies-state!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example commit-state transaction-commit-applies-state! commit-visible-state="state-after";commit-after-state="state-after"
-- @counterexample commit-lost transaction-commit-applies-state! commit-visible-state="state-after";commit-after-state="state-before"
-- @outside-scope aborted-transaction
-- @mutation ignore-durable-state transaction-commit-applies-state! critical commit-visible-state === commit-visible-state
-- @ratification 5ca3bf6980a975ebc4afe92786e6792ef483c91a9589568813f8d8e9ad4765b3 corey
-- @end-requirement

-- @requirement TURN-002
transaction-commit-applies-state! := commit-visible-state === commit-after-state

Aborted transactions
-------------------------------------------------------------------------------

-- @requirement-id TURN-003
-- @title Abort is semantically recorded
-- @level must
-- @area transaction
-- @normative Every aborted transaction must report an abort outcome and semantic abort event.
-- @claim all aborted-transaction produce semantic-event
-- @contract-claim all aborted-transaction produce semantic-event
-- @term semantic-event
-- @contract transaction-abort-outcome!
-- @contract transaction-abort-event-recorded!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example aborted transaction-abort-outcome! abort-outcome="abort"
-- @counterexample committed transaction-abort-outcome! abort-outcome="commit"
-- @outside-scope transaction-not-started
-- @mutation accept-any-outcome transaction-abort-outcome! critical abort-outcome === abort-outcome
-- @ratification 5c22de71618cf459e6838cb64a94567012087b5b874037b9adb8e0810a93ba63 corey
-- @end-requirement

-- @requirement TURN-003
transaction-abort-outcome! := abort-outcome === "abort"

-- @requirement TURN-003
transaction-abort-event-recorded! := abort-event-observed === true

-- @requirement-id TURN-004
-- @title Abort preserves committed state
-- @level must
-- @area transaction
-- @normative Every aborted transaction must preserve committed state.
-- @claim all aborted-transaction preserve committed-state
-- @contract-claim all aborted-transaction preserve committed-state
-- @term committed-state
-- @contract transaction-abort-preserves-state!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example abort-same transaction-abort-preserves-state! abort-before-state="state-before";abort-after-state="state-before"
-- @counterexample abort-mutates transaction-abort-preserves-state! abort-before-state="state-before";abort-after-state="state-after"
-- @outside-scope committed-transaction-may-change-state
-- @mutation compare-before-to-itself transaction-abort-preserves-state! critical abort-before-state === abort-before-state
-- @ratification 4a7133b1b8565675b731a3b634fbb63927567da0ba7bcbc69f31f84073c9b43b corey
-- @end-requirement

-- @requirement TURN-004
transaction-abort-status<bool> := abort-before-state === abort-after-state

-- @requirement TURN-004
transaction-abort-preserves-state! := abort-before-state === abort-after-state

-- @requirement-id TURN-005
-- @title Committed state is visible to the next turn
-- @level must
-- @area transaction
-- @normative Every transaction after a commit must observe the committed state.
-- @claim all post-commit-turn observe committed-state
-- @contract-claim all post-commit-turn observe committed-state
-- @term committed-state
-- @contract transaction-state-persists!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example next-turn transaction-state-persists! commit-after-state="state-after";next-turn-state="state-after"
-- @counterexample stale-next-turn transaction-state-persists! commit-after-state="state-after";next-turn-state="state-before"
-- @outside-scope first-turn-before-commit
-- @mutation ignore-next-turn transaction-state-persists! critical commit-after-state === commit-after-state
-- @ratification a60f8910ed98141ec920626049d31f32b88fbb32de6b542882e833213422b1ea corey
-- @end-requirement

-- @requirement TURN-005
transaction-state-persists! := commit-after-state === next-turn-state

Runtime lifecycle
-------------------------------------------------------------------------------

-- @requirement-id LIFE-001
-- @title Shutdown closes runtime ingress
-- @level must
-- @area lifecycle
-- @normative Every shut down runtime must reject new input and emit a semantic shutdown event.
-- @claim all shutdown-runtime close runtime-ingress
-- @contract-claim all shutdown-runtime close runtime-ingress
-- @term runtime-ingress
-- @term semantic-event
-- @contract shutdown-closes-ingress!
-- @contract shutdown-rejects-input!
-- @contract shutdown-event-recorded!
-- @profile resident-cpu must-pass
-- @evidence runtime runtime-turns/1 observed
-- @example closed shutdown-closes-ingress! shutdown-ingress-closed=true
-- @counterexample open shutdown-closes-ingress! shutdown-ingress-closed=false
-- @outside-scope active-runtime
-- @mutation accept-open-ingress shutdown-closes-ingress! critical shutdown-ingress-closed === shutdown-ingress-closed
-- @ratification 1536774cdd89ce66b217ca3022ba4ca15332fb1c7ebe659b7311c6d840cda1aa corey
-- @end-requirement

-- @requirement LIFE-001
shutdown-closes-ingress! := shutdown-ingress-closed === true

-- @requirement LIFE-001
shutdown-rejects-input! := shutdown-input-rejected === true

-- @requirement LIFE-001
shutdown-event-recorded! := shutdown-event-observed === true

Repository architecture
-------------------------------------------------------------------------------

-- @requirement-id ARCH-011
-- @title Resident execution is separated from parser internals
-- @level must
-- @area architecture
-- @normative Every resident execution module outside the compiler boundary must avoid parser-internal imports.
-- @claim all resident-execution-module avoid parser-structure
-- @contract-claim all resident-execution-module avoid parser-structure
-- @term parser-structure
-- @contract resident-artifact-boundary!
-- @profile resident-cpu must-pass
-- @evidence repository repository-dependencies/1 observed
-- @example clean-boundary resident-artifact-boundary! repository-resident-parser-imports=false
-- @counterexample forbidden-import resident-artifact-boundary! repository-resident-parser-imports=true
-- @outside-scope source-compiler-module
-- @mutation accept-parser-import resident-artifact-boundary! critical repository-resident-parser-imports === repository-resident-parser-imports
-- @ratification 49f6cb6979576a3fac08570e17cf98303562cfbaa0b74cfa0ecfa84f94a1bc93 corey
-- @end-requirement

-- @requirement ARCH-011
resident-artifact-boundary! := repository-resident-parser-imports === false

Backend admission
-------------------------------------------------------------------------------

-- @requirement-id GPU-001
-- @title Unavailable GPU support is rejected explicitly
-- @level must
-- @area backend
-- @normative Every unavailable GPU profile must produce an explicit unsupported admission result.
-- @claim all unavailable-gpu-profile produce backend-admission
-- @contract-claim all unavailable-gpu-profile produce backend-admission
-- @term backend-admission
-- @contract gpu-unavailable-rejected!
-- @profile resident-cpu not-applicable
-- @profile gpu may-reject
-- @evidence backend backend-admission/1 observed
-- @example unsupported gpu-unavailable-rejected! backend-admission-result="unsupported"
-- @counterexample silent-fallback gpu-unavailable-rejected! backend-admission-result="fallback"
-- @outside-scope available-gpu-profile
-- @mutation accept-fallback gpu-unavailable-rejected! critical backend-admission-result === backend-admission-result
-- @ratification b2654cc712c01ea709e6b3d13597e8d77916b2b975fa3d272a3f44d6fefb3099 corey
-- @end-requirement

-- @requirement GPU-001
gpu-unavailable-rejected! := backend-admission-result === "unsupported"

Benchmark governance
-------------------------------------------------------------------------------

-- @requirement-id BENCH-001
-- @title Compared benchmark protocols are identical
-- @level must
-- @area benchmark
-- @normative Every reported benchmark comparison must use compatible protocol identities.
-- @claim all benchmark-comparison match benchmark-protocol
-- @contract-claim all benchmark-comparison match benchmark-protocol
-- @term benchmark-protocol
-- @contract benchmark-protocols-match!
-- @profile resident-cpu must-pass
-- @evidence benchmark benchmark-protocols/1 observed
-- @example compatible benchmark-protocols-match! benchmark-reference-protocol="steady-state-v1";benchmark-candidate-protocol="steady-state-v1"
-- @counterexample incompatible benchmark-protocols-match! benchmark-reference-protocol="steady-state-v1";benchmark-candidate-protocol="compile-included-v1"
-- @outside-scope unreported-measurement
-- @mutation ignore-candidate-protocol benchmark-protocols-match! critical benchmark-reference-protocol === benchmark-reference-protocol
-- @ratification 1b80b678d6e5d91c0aaf5bde98a318f5471ed99ce1750dfaf19f83831198576e corey
-- @end-requirement

-- @requirement BENCH-001
benchmark-protocols-match! := benchmark-reference-protocol === benchmark-candidate-protocol
