# Response to the replacement-audit checkpoint review

Production implementation remains frozen at S8B `662d29b79`. The review of
`3d04a2e06` correctly identified that inventory and observational passes were not
a bounded replacement plan. This revision reconciles those observations, assigns
the remaining acceptance work and publishes the existing-code review boundaries.
It does not claim the implementation is ready to seal.

## Unsupported behavior and unfinished implementation

Several unsupported families are required work that has not been implemented:
composite comprehension storage, nested control, computed and structural patterns,
nominal declarations, pattern functions, recursive calls, FSMs and activation
scopes. Their prerequisite owners are explicit in SCOPE.md and gaps.tsv. Existing
syntax support or a clean error never proved those semantic families complete.

The current target also lacks numeric and composed-layout capabilities. Absence
of a factory proves current unavailability; it does not establish an agreed
milestone exclusion. G02/G17/G18 retain positive value/shape witnesses and explicit
scope acceptance. The 27 exact current-target rejection observations are a separate
axis from those unfinished capabilities. No exclusion is inferred from them.
A strict c32 witness was run twice: the current-rejection oracle passes, while
the positive-capability oracle fails. Both results are recorded in validation.json.

Pure ProgramCompiler artifact construction and resident activation have different
contracts. Production loading already preflights before installation and effects;
a later rejection in a direct pure-artifact probe does not by itself prove a
missing compiler preflight. Visibility is different: Internal and unimported
ModuleOnly named calls must be rejected by the source environment, and the audit
preserves those demonstrated wrong admissions as G03.

## Reconciliation of the review findings

| Review concern | Revised evidence and disposition |
| --- | --- |
| Missing semantic families were mixed into adapters | 25 demonstrated defect, prerequisite or contract groups have owning layers, witness links and 23 proposed corrective review boundaries. Semantic prerequisites are separate from compiler adapters. |
| Successful compilation concealed wrong values | G04 retains the mixed/nested occurrence-ordered assignment witnesses; G16 retains the transitive explicit-root two-turn witness. G25 and G26 add independently checked constant-binding identity defects, not a ticket for every failing type. |
| Failure counts contained fixture mistakes | Matrix reduction orientation, valid unsigned domains, Bessel input kinds, source-visible operator spellings, imports and invalid original control positives are reconciled. Schema-boundary mistakes are documented separately from production defects. |
| Entry-point probes lacked distinctive obligations | 53 compiler/frontend cells cover nonempty values/defaults, providers, context ownership, ordering, options identity, rollback and internal entrances. Existing evidence and 14 unrun constructions are separate. |
| Catalog inventory was presented as execution coverage | 480 candidate rows cover all 120 names with exact candidate layouts, kind/target domains, source visibility, boundary cases and independent oracles. Census, sample execution and overload qualification remain different results. |
| Source/bytecode agreement lacked independent values | Every source row states its oracle strength. The strict schema suite checks independently owned values and schemas through live and bound source/bytecode paths. Equivalence-only samples remain named acceptance work. |
| The 27 production contracts lacked complete application evidence | 54 positive/negative cells name the actual consumer boundary, construction, expected policy, evidence and blocker. Prepared-adapter passes do not certify the real browser loader or every configured application. |
| Rule/type crosswalk and deduplicated register were unfinished | All 211 rule dispositions, 17 scalar leaves, 20 schema variants, compiler/frontend entrances, catalog candidates and 27 consumer contracts link to their specific acceptance responsibilities. Accounting scripts verify membership and links without claiming execution. |
| Remaining qualification work could accumulate in the last PR | qualification-ownership.tsv assigns 341 inventory/group relations, including every untested compiler/schema construction and all 115 equivalence-only observations, to their E/R owners. Evidence stays in the original ledgers; the ownership crosswalk adds no second evidence authority. |
| Accumulated S8B work still lacked review boundaries | Eleven draft extraction PRs (#831–#841) now contain the existing code by responsibility. All 91 original changed paths are assigned; the complete E11 tree exactly equals frozen S8B, without exclusions. Intermediate checks are recorded by head. |

The source run contains 375 observations: 317 execute, including 15 incorrect
Internal admissions. There are 329 current-oracle matches, including 27 target
rejections, and 46 unmatched observations. Two invalid original control positives
remain historical records. These totals are neither certified capabilities nor
independent defect counts. The default six-test aggregate deliberately records
failures; strict selected witnesses and the 31-test schema suite fail on unmet
expectations.

The final harness rejects unknown test filters before Cargo and records compiled
fixture/harness identities at execution. The recorder refuses stale identities
and incomplete record censuses. Both negative checks were exercised without
modifying stored evidence, followed by fresh full and strict witness runs.

The scope is finite at the inventory, acceptance-cell and review-owner level.
It is not a promise that unrun tests cannot expose another root cause or that
23 boundaries are all small patches. Any such finding must update its owner and
acceptance scope before production work, rather than silently expanding S8B.
The constrained-type decisions and retained target capabilities remain explicit
scope decisions. The implementation freeze stays active for review of this plan.
