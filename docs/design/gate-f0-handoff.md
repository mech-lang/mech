# Gate F0 handoff: evidence-only final qualification

## Opening state

F0 begins from the merged integration stack, not from an unmerged E4 branch.

- Base branch: `integration/value-executor-v0.4`
- Exact base commit: `6c0923339e4b1a7d47f9b109135059487e6919ae`
- Exact base tree: `39125894b400e2960b6ecf188682d4289435e629`
- Head branch: `qualification/f0-final-evidence`
- Pull request state: draft until final qualification is complete

Opening F0 does not authorize merging it. Merge is a separate, explicit action after
all F0 evidence and release gates pass.

## Purpose

F0 owns evidence refresh and release qualification only. It is not another
implementation, deletion, migration, or cleanup phase.

## Authorized scope

F0 must:

1. refresh controlled Gate B evidence against the exact final stack;
2. refresh controlled Gate D evidence where required;
3. remove the controlled stale-evidence allowance from CI;
4. run the strict value-system checker with zero findings;
5. pin Chrome, ChromeDriver, wasm-pack, Rust, and every other qualification tool;
6. run final release, distribution, and retained-product qualification;
7. produce final zero-interpreter and zero-fallback reachability evidence; and
8. record exact commits, trees, runner identities, commands, and immutable evidence
   manifests.

F0 may change qualification workflows, evidence runners, evidence reports,
architecture contracts, and qualification documentation. It must not change
shipping runtime, compiler, host, bytecode, language, or product behavior.

## Failure routing

Environment and evidence-infrastructure defects may be corrected in F0 when they
do not alter the product under qualification.

Any runtime, compiler, host, bytecode, language, or retained-product defect must
be corrected in a separate pull request against
`integration/value-executor-v0.4`. After that correction merges, F0 must move to
the corrected integration head and discard evidence generated against the
superseded product tree. The merged E4 pull request is not reopened or amended.

Performance thresholds, workloads, retained-product assertions, and architecture
requirements must not be weakened to make F0 pass.

## Retained compiler boundary

The private semantic compiler may continue using `Interpreter` and
`LegacyValue` as ephemeral planning coordinates. Replacing those internals with
a direct AST-to-artifact compiler is future, non-gating compiler modernization.

## Merge gate

F0 remains draft until all required Gate B and Gate D reports pass, the strict
value-system checker reports zero findings, exact-head selected and full
qualification pass, evidence provenance verifies, and the shipping product tree
is unchanged except through separately reviewed corrective pull requests.
