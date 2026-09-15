# Qualification ownership within the review boundaries

[`qualification-ownership.tsv`](qualification-ownership.tsv) assigns each finite
acceptance cell to one primary review boundary in [PR-STACK.md](PR-STACK.md).
It contains 341 ownership relations: 53 compiler/frontend cells, 64 schema cells,
32 rule groups, 33 control cells, 54 production consumer cells, 34 catalog groups,
46 target contracts and 25 deduplicated gap groups. These inventories overlap;
341 is neither an independent defect count nor a test count. Each catalog group
enumerates its exact member export/overload IDs, covering all 480 candidates of
the 120-name inventory.

The linked source ledger remains authoritative for the fixture, expected result,
execution recipe, evidence status and proof limit. This crosswalk adds ownership
only. Its `owning-responsibility` identifies the review subject; it does not replace
the fuller source-ledger contract. An existing test still needs its declared
validation; a construction marked untested still needs an executable acceptance
test. Linking a construction does not establish that its behavior works.

E1–E11 identify review ownership for existing extracted responsibilities. They do
not authorize changing the published extraction heads: E11 must continue to
reproduce frozen B exactly while the audit is reviewed. After scope acceptance,
qualification of an already implemented responsibility belongs with that E
boundary. A new demonstrated production defect must enter the deduplicated gap
register and receive an accepted corrective owner before implementation resumes.
R01–R23 own qualification of the stated correction or prerequisite. A supporting
boundary identifies a shared authority needed to review that same acceptance
cell; it does not create a second implementation ticket.

For catalog groups, the primary owner supplies the semantic/physical operation
and its exact kind/layout boundary tests. R01 supplies the named-call visibility
contract, E4 supplies candidate resolution and canonical argument/result schemas,
and E9 supplies configured backend admission where those source-ledger cells
require it. The catalog specifies actual domains and supported/rejected target
behavior; this ownership link does not require an undeclared Cartesian product
across every backend. Internal names need a direct-call rejection and an intrinsic
positive source form. ModuleOnly positives require the declared import. Operator
IDs are independently retained. Discovery samples are not automatically public
API positives or proofs that every overload was selected.

The 115 O03 observations now have explicit `acceptance-cells` links in
[`semantic-obligations.tsv`](semantic-obligations.tsv): 101 point to their exact
catalog groups; the remaining 14 point to rule and schema cells for option and
Dynamic values, atoms, reified kinds, declarations, comprehensions and match
results. These observations still have only source/bytecode equivalence oracles.
Their independent expected-result assertions remain qualification work for the
linked owners; their dispositions and recorded results are unchanged.

The 18 compiler/schema constructions without existing exact test symbols have
the following owners. The source ledgers specify their concrete input, expected
outcome and finite domain; this table does not replace those recipes.

| Owner | Untested acceptance cells | Responsibility |
| --- | --- | --- |
| E4 semantic frontend | compiler:F12, F16 | Invalid input ordinal; executable-reference census across the four declared reference owners. |
| E5 ordinary compiler | compiler:P01, P29, P30, F17, F18, F19 | Raw adapter admission; finite host-value inputs/defaults/static binding; resource/supplied conflict and remap; external contracts; supplied schema narrowing. |
| E6 graph compiler | compiler:P14, P21, P22, P31 | Module options identity; function defining resource context; later-root rollback; request-wrapper error classification. |
| R03 bound schema identity | compiler:F15; schema:Q30, Q31, Q32 | Binding errors and input renumbering; independent Id, Index and generic Enum snapshots through binding/publication. |
| R14 reified matrix kinds | schema:Q33 | Exact reified schema identity through source and decoded publication. |
| R20 ordered graph identity | compiler:P20 | A transitive explicit dependency plans once and contributes one live read per turn. |

R23 owns source retirement enforcement, certification assumptions and final
distribution closure. It reruns the final consumer matrix but does not own every
missing consumer test: the positive and negative consumer cells are individually
assigned to configuration, compiler, browser and retirement boundaries. Likewise,
the compiler/schema constructions above do not default to R23.

Run the read-only accounting checks from the repository root:

```sh
python3 docs/design/grammar-audit/s8-replacement-audit/verify-qualification-ownership.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-acceptance-cells.py
python3 docs/design/grammar-audit/s8-replacement-audit/verify-rule-acceptance-links.py
```

The ownership verifier checks exact coverage, primary/supporting boundary IDs,
candidate membership and O03 links. It neither runs behavioral tests nor bounds
the number of new root causes future execution could reveal. The original comparison remains frozen. Accepted continuation and new
qualification findings are recorded in RECOVERY-STATUS.md and RECOVERY-FINDINGS.md.
