# S8 replacement audit — reconciled contracts and review boundaries

**The audit baseline is frozen at S8B `662d29b79`.** This branch contains audit
evidence and review boundaries; the parked numeric WIP remains excluded. The
planning baseline is accepted and confirmed corrective work has resumed on
separate PRs, tracked in [RECOVERY-STATUS.md](RECOVERY-STATUS.md). Original
observations continue to describe the frozen baseline. New corrective-qualification
findings are tracked separately in [RECOVERY-FINDINGS.md](RECOVERY-FINDINGS.md).

The checkpoint review is answered in [REVIEW-RESPONSE.md](REVIEW-RESPONSE.md).
Start with [SCOPE.md](SCOPE.md) and [PR-STACK.md](PR-STACK.md). The register now
contains **25 demonstrated defect, prerequisite or contract groups**, linked to
finite acceptance work. It is not an upper bound on defects an unexecuted test
could discover. The proposed corrective stack has **23 separate review boundaries**, including explicit capability-acceptance prerequisites;
substantial semantic prerequisites are separated from compiler adapters.

The contract reconciliation resolves the old ambiguities about source visibility,
configured-target admission, dimensionless reified kinds, FSMs, activation scopes
and functions. Scalar range constraints retain six specific design decisions. Numeric/layout capabilities and computed-mask/downstream shape capabilities also retain explicit target scope acceptance; their current rejection behavior cannot close positive milestone coverage.
Those decisions and the enumerated untested/blocked cells remain visible; nothing
silently drops a language family or treats a passing inventory as execution proof.

| Evidence | Files |
| --- | --- |
| Root causes, owning layers and executable witnesses | [gaps.tsv](gaps.tsv), [semantic-obligations.tsv](semantic-obligations.tsv), [WITNESSES.md](WITNESSES.md) |
| Reconciled existing language/target contracts | [CONTRACT-DECISIONS-TYPES.md](CONTRACT-DECISIONS-TYPES.md), [CONTRACT-DECISIONS-CONTROL.md](CONTRACT-DECISIONS-CONTROL.md) |
| Compiler and frontend responsibilities | [compiler-acceptance-cells.tsv](compiler-acceptance-cells.tsv): 53 cells; [compiler-methods.tsv](compiler-methods.tsv), [compiler-internal-entrypoints.tsv](compiler-internal-entrypoints.tsv), [frontend-apis.tsv](frontend-apis.tsv) |
| Frozen production consumers | [consumer-acceptance-cells.tsv](consumer-acceptance-cells.tsv): 54 positive/negative cells across 27 contracts; [CONSUMER-ACCEPTANCE.md](CONSUMER-ACCEPTANCE.md) |
| Type, schema and binding obligations | [schema-acceptance-cells.tsv](schema-acceptance-cells.tsv): 64 cells; [SCHEMA-BOUNDARY-RECONCILIATION.md](SCHEMA-BOUNDARY-RECONCILIATION.md), [scalar-types.tsv](scalar-types.tsv), [schema-families.tsv](schema-families.tsv) |
| Catalog overload qualification | [catalog-acceptance-cells.tsv](catalog-acceptance-cells.tsv): 480 exact candidates / 120 names / 34 shared families; [CATALOG-COVERAGE.md](CATALOG-COVERAGE.md) |
| Current target rejection versus milestone capability | [TARGET-REJECTION-RECONCILIATION.md](TARGET-REJECTION-RECONCILIATION.md), [target-rejection-cells.tsv](target-rejection-cells.tsv): 46 cells; [CONTROL-TARGET-RECONCILIATION.md](CONTROL-TARGET-RECONCILIATION.md) |
| Rule and control accounting | [rule-crosswalk.tsv](rule-crosswalk.tsv), [rule-acceptance-links.tsv](rule-acceptance-links.tsv), [control-acceptance-cells.tsv](control-acceptance-cells.tsv): 33 control cells |
| Recorded observations and limits | [observations.json](observations.json), [observation-metadata.json](observation-metadata.json), [schema-observations.json](schema-observations.json), [validation.json](validation.json), [retirement-observations.json](retirement-observations.json) |
| Remaining qualification ownership | [qualification-ownership.tsv](qualification-ownership.tsv), [QUALIFICATION-OWNERSHIP.md](QUALIFICATION-OWNERSHIP.md): 341 explicit inventory/group relations across the review boundaries |
| Existing B extraction | [PR-STACK.md](PR-STACK.md), [EXTRACTION-DEPENDENCIES.md](EXTRACTION-DEPENDENCIES.md), [patch-ownership.tsv](patch-ownership.tsv), [extraction-results.json](extraction-results.json), [extraction-manifests](extraction-manifests) |

The source census has **375 observations**: 317 execute successfully and 58 stop
or produce a wrong value. Fifteen of those successful executions wrongly admit
Internal named calls, so 302 execution observations match their stated oracles. Another 27 match explicit current-target rejection oracles, leaving 46 unmatched observations. Those 27 rejection matches do not satisfy their retained positive capability witnesses. Two original control positives are invalid and
remain marked as historical samples, replaced by valid declared/stable-trigger
witnesses. Cases without an independent expected value are equivalence-only.
These counts describe observations, not certified capabilities or distinct bugs.

The separate strict schema suite has 31 tests: **seven pass, 24 fail**. All 26
positive fixtures pass live source and decoded-bytecode publication with exact
schema/value identity. Twenty-three share a constant-bound schema-order decoding
defect (G25); already-Dynamic binding changes value identity (G26). Bool/String
constant binding and all five actual validation-boundary rejection tests pass.
The earlier raw-capture and Dynamic-empty negative expectations were audit fixture
mistakes and are documented; their failed runs are not counted as defects.

The initial checkpoint at `3d04a2e06` remains in `checkpoint-observations.json`.
Current source records include provenance logs and a fixture content hash; later
targeted reruns replace only their matching observations. C evidence is pinned to
`e31d08260fac40bcf7982e854cc5d2623a606347`: 713 runtime passes/two failures, 134
prepared consumer-adapter passes, deleted-parser engine-test build failures, and
an unresolved real browser document transport. An isolated WASM build does not
certify that loader.

To check accounting:

```sh
python3 docs/design/grammar-audit/s8-replacement-audit/verify-inventory.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-acceptance-cells.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-rule-acceptance-links.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-extraction-results.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-qualification-ownership.py
```

The checks verify inventory membership, exact test symbols, observation/fixture
consistency and linked responsibilities. They do not execute unrun acceptance
cells. `run-probes.sh` records source observations; `MECH_AUDIT_REQUIRE_PASS=1`
makes unmatched contracts fail. The schema suite is always strict. See WITNESSES.md
for exact commands and nonzero-selection guarantees.
