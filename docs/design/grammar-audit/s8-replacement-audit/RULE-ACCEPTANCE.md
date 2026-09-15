# Rule inventory acceptance accounting

`rule-acceptance-links.tsv` now accounts for every one of the **211 rows in the
frozen replacement rule crosswalk**: 80 Phase 2I rows and 131 S7 rows. These are
inventory entries, not 211 execution capabilities and not 211 defects.
Production remains frozen. This audit did not run Cargo or change production.
For every linked operator/layout cell, semantic validity, current target
capability and milestone coverage are separate checks. Pure activation binding
may reject an unsupported target, and the production loader preflights before
installation. That rejection does not close missing milestone work or authorize
a feature exclusion; retain its finite positive witnesses for scope review.
A supported scalar/schema value still retains its positive transport contract.

Each row names its responsibility class, owning layer and acceptance cells.
Namespaced links resolve to the existing compiler (`compiler:Pxx/Fxx`), schema
(`schema:Qxx/QT-*/QG-*`), catalog (`catalog:Cxx`) and control
(`control:FSMxx/ACTxx/FUNxx`) ledgers, or to the 32 grouped responsibilities in
`rule-acceptance-cells.tsv` (`rule:Rxx`). These `rule:Rxx` acceptance IDs are separate from the corrective
PR boundary numbers in PR-STACK.md; they do not add 32 PRs. The original 42 control mappings remain
in the compatible `control-acceptance-cells` column. A cell shared by ten grammar
helpers is one responsibility, not ten additional implementation tickets.

The additional ledger has an exact positive source or test, a concrete negative
or boundary case, an expected result/policy, evidence status and owner for every
row. Existing tests are **source-inspected, not newly passing**. A specified test
recipe remains unexecuted until added and run. Recorded failures are explicitly
marked. No aggregate row count is an implementation-readiness claim.

## What the syntax tables actually establish

- Phase 2I's 80-row table has individual accepted, rejected and recovery sources.
  `canonical_phase_2i_certification::certification_table_executes_every_direct_accept_reject_and_recovery_case`
  executes those direct grammar contracts, checks typed access and compares
  normalized evidence. It does not execute 80 independent language programs.
  `argument-list`, record `binding`, selectors, headers, rows and pattern helpers
  delegate semantic behavior to their containing value, call or control owner.
- S7's executable syntax table has **112 positive direct-rule samples**, comprising
  110 document dependencies and 2 maintained roots. The separate 131-row
  disposition test checks that inventory plus 17 historical command entries and
  2 entries outside the active document closure. It has no individual malformed
  source or recovery matrix for all 112 rules. The link table states that limit
  directly; it does not invent negative failures for valid prose fallback.
- An empty program or direct fragment such as `~>1 + 2{}` is a syntax witness,
  not proof of executable activation semantics. The valid stable-trigger and
  fully declared FSM witnesses remain in the control ledger and their observed
  lowering failures remain G15/G14.
- The inactive `match-expression` and `table-column` **direct grammar entries**
  remain present as inactive inventory metadata. This does not exclude reachable
  ordinary match expressions or Table values. Their execution links point to
  R11/G09 and the active table constructor/schema cells, respectively.
- Executable table syntax and Mechdown presentation tables are distinct owners.
  Mechdown alignment is renderer behavior; it does not create a resident Table
  value or a duplicate Table schema authority.

The source-json fields preserve the exact original syntax fragments. These new
files use ordinary TSV/CSV quoting; read them with `csv.DictReader(delimiter='\t')`
then decode the `*-json` field as JSON. They do not replace the Rust certification
tables, whose original tab-split encoding stays unchanged.

## Finite remaining rule-level qualification work

The new cells expose the following bounded work; they do not authorize reopening
implementation during the audit:

| Acceptance owner | Named cells | Concrete remainder |
| --- | --- | --- |
| Syntax/semantic certification | R01, R02, R04 | Reuse the existing direct-rule tests; repair the two shape-sensitive certification obligations under G24 before accepting new fingerprints. Distinguish inventory, typed syntax and execution totals. |
| Runtime request boundary | R03 | Existing registry test covers current command minima. Encode the 17 exact negative/boundary rows in one parameterized request test. Current `:symbols` and `:s` rejection is intentional; `:whos` is the maintained inspection request. No historical document command parser is restored. |
| Expression lowering | R05 | Five source results fix the precedence/parentheses obligation: 14,18,true,20,36. Catalog C01–C34 retain their separate type/domain/layout and exposure contracts; arithmetic probes cannot certify every overload. |
| Comprehension/control prerequisites | R07–R11 | Preserve working generator/filter/destructuring baselines; add exact retained-kind/compound values (G06), nested-control values/shapes (G07), computed-pattern capture/filtering (G08), and structural ordinary-match success/fallthrough (G09). The schema ledger closes the finite payload families. These are existing gap owners, not grammar-specific defects. |
| Declarations and constrained types | R12, R13, Q27, Q28/Q33 | G10 aliases have named value and duplicate/cycle/unknown-reference cases. G11 owns nominal enum declaration/payload behavior. G13 owns independent dimension parameters in reified kinds. G12 retains six specific language-design decisions in CONTRACT-DECISIONS-TYPES.md; the corrected document fixture now reproduces the existing honest constrained-expression lowering rejection (s8-audit-constraint-contract.log). No interval semantics are invented here. |
| State updates and dynamic consumers | R14, R15 | G04 fixes promoted/nested repeated occurrences while preserving anchored errors and rollback. G18 retains the finite unimplemented downstream shape capability and explicit concat/transpose positive witnesses for milestone scope review. The current fixed target can correctly reject during pure activation/loader preflight; that is neither a demonstrated preflight defect nor an accepted milestone exclusion. G07 remains the distinct nested-control prerequisite. See CONTROL-TARGET-RECONCILIATION.md. |
| Retained document renderer | R18–R29, R32 | Existing exact tests cover source order, scope identity, headings/front matter, markup, callouts, lists, tables, figures, references, floats, Mika and fences. Add one nine-block escaping table (R22), center/default/changed-column alignment pair (R24), and exact highlight markup/escaping pair (R32). R32 is qualification for implemented `mark.mech-highlight`, not a demonstrated missing lowering case. |
| Working semantic and document adapters | R06, R16, R17, R30, R31 plus compiler/schema cells | Reuse explicit literal/error, tuple-destructuring, statement-function, resource/invariant and composite projection assertions. Do not treat them as proof of blocked pattern functions, recursion, FSM or activation. |

For Mika, `canonical_document_root::mika_glyph_roles_are_determined_by_position`
asserts that `╭◉╮` has one nose and no eyes, while `(◉◯◉)` has one nose and two
eyes. The glyph can have either role according to its position. This is a
registry/typed syntax and namespace contract; individual eye/arm grammar entries
have no separate evaluator.

`document_render.rs:1117–1131` maps center to `mech-align-center`, right to
`mech-align-right`, and left/unspecified alignment to `mech-align-left`.
`document_render.rs:1493–1500,1651–1652` maps highlight to
`<mark class='mech-highlight'>…</mark>`. The new test recipes use these inspected
contracts rather than deferring a product question.

## Production boundaries and verification

Renderer/frontend tests above exercise their named native API; they do not
certify a CLI invocation, browser loader or interactive replacement. Those are
separately enumerated by all 54 positive/negative cells for the 27 frozen
consumers in `consumer-acceptance-cells.tsv`. Likewise, source-artifact/bytecode
agreement without an independent expected value remains insufficient.

Run the read-only accounting check:

```sh
python3 docs/design/grammar-audit/s8-replacement-audit/verify-rule-acceptance-links.py
```

It checks the exact 211 inventory keys, 80/112 original source fragments, all
namespaced links, 32 named rule responsibilities and existing Rust test symbols.
It checks that the 19 excluded entries are marked inactive in generated port
metadata. It runs no Rust tests and reports no capability as passing.
