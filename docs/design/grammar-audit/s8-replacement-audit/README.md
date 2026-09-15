# S8 replacement audit — reconciled scope for review

**Implementation remains frozen at S8B `662d29b79`.** The audit identifies remaining
work and restores proposed review boundaries; it does not certify the replacement
or authorize another implementation round. The extracted PR heads have not been
created. The original numeric WIP remains excluded.

Start with [SCOPE.md](SCOPE.md), then [PR-STACK.md](PR-STACK.md).

The final probe census is **365 cases: 317 passing observations and 48 failures**.
The initial checkpoint's 70 failures included 23 corrected fixture mistakes; one
new targeted witness isolates recursion from pattern-function lowering. There are
**24 tracked gap/decision groups**, not 48 independent defects. Six named
qualification packages retain the explicit untested/blocked obligations.

| Evidence | Files |
| --- | --- |
| Gap ownership and executable source witnesses | [gaps.tsv](gaps.tsv), [semantic-obligations.tsv](semantic-obligations.tsv), [WITNESSES.md](WITNESSES.md) |
| Complete inventory membership and explicit coverage limits | [rule-crosswalk.tsv](rule-crosswalk.tsv), [frontend-apis.tsv](frontend-apis.tsv), [compiler-methods.tsv](compiler-methods.tsv), [consumer-contracts.tsv](consumer-contracts.tsv) |
| Type and catalog coverage | [scalar-types.tsv](scalar-types.tsv), [schema-families.tsv](schema-families.tsv), [catalog-overloads.tsv](catalog-overloads.tsv), [catalog-signatures.tsv](catalog-signatures.tsv) |
| Observed results | [observations.json](observations.json), [retirement-observations.json](retirement-observations.json) |
| Existing B extraction ownership | [patch-ownership.tsv](patch-ownership.tsv), [PR-STACK.md](PR-STACK.md) |

The crosswalk accounts for 80 Phase 2I rules, 131 S7 dispositions (separate from
112 direct S7 syntax witnesses), 24 public frontend methods, 36 public compiler
methods, all 27 frozen consumers, 17 scalar kinds, 20 schema variants, and 120
catalog exports with 480 explicit overload/intrinsic rows. It also accounts for
all 91 paths changed by accumulated B work.

Run `python3 docs/design/grammar-audit/s8-replacement-audit/verify-inventory.py`
to check inventory membership, uniqueness, failure ownership and evidence counts.
This verifies accounting, not semantic completeness. No passing suite count closes
an untested crosswalk cell. In particular, the 480 signature rows are enumerated
qualification obligations, not 480 tested signatures.

To record probes, run:

```sh
./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh
```

The default harness records failures without failing its aggregate test. To expose
a failure as a failing regression, use `MECH_AUDIT_REQUIRE_PASS=1`; select a source
case with `MECH_AUDIT_CASE`. See WITNESSES.md for the graph, transport, retirement,
and certification reproductions and limitations. Cases without an independent
expected result remain explicitly equivalence-only.

`checkpoint-observations.json` preserves the original 3d04a2e06 discovery record;
`observations.json` contains the corrected serial run. The C evidence is pinned to
`e31d08260fac40bcf7982e854cc5d2623a606347`, not the divergent C branch. Its runtime
library has two failures, its selected consumer adapters pass 134 tests, its engine
library test build fails on deleted parser references, and its real document
transport remains blocked. A standalone WASM build passing does not override that.
