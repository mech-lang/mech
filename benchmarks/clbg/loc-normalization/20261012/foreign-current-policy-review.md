# Frozen foreign-source LOC policy review

This review catalogs **219 displayed registrations across 177 distinct language-family/source-SHA pins** from [job04](jobs/boundary-audit-04/candidate-audit.json). The complete exact-key catalog is in [foreign-current-policy-review.json](foreign-current-policy-review.json). It excludes the 17 APL/Uiua registrations handled separately and later current-data additions.

Every registration matches a saved source-audit row by **program key and source SHA**, and its frozen producer LOC matches the corresponding producer summary's `final_loc`. This establishes provenance and historical count reconciliation. It does **not** certify the count under `canonical-source-units-20261012-approved-rule-v2`.

The frozen dataset reference is `overview/explorer/dist/data.json`, expected SHA `65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f`. This review did not read the volatile dataset. It read saved metadata and static code, inspected three bounded source excerpts, and wrote only these review artifacts. It executed no imports, lexers, hash computation, tests, builds, or timings and changed no normalizer or frozen job.

## Family catalog

| Family | Display registrations | Distinct source SHAs | Registrations with saved expansion events | Prior producer audit |
|---|---:|---:|---:|---|
| C# | 39 | 20 | 18 | remaining source-audit-v2 |
| F# | 30 | 25 | 8 | remaining source-audit-v2 |
| Dart | 36 | 18 | 18 | remaining source-audit-v2 |
| Zig | 16 | 16 | 11 | zig/odin/roc/elm source-audit-complete |
| Odin | 15 | 15 | 5 | zig/odin/roc/elm source-audit-complete |
| Roc | 11 | 11 | 0 | zig/odin/roc/elm source-audit-complete |
| Elm | 11 | 11 | 0 | zig/odin/roc/elm source-audit-complete |
| Ada | 17 | 17 | 16 | remaining source-audit-v2 plus one Ada regex repair |
| Pascal | 15 | 15 | 13 | remaining source-audit-v2 |
| Fortran | 16 | 16 | 2 | remaining source-audit-v2 |
| Smalltalk | 13 | 13 | 6 | remaining source-audit-v2 |

Duplicate execution modes account for the difference between registration and distinct-source counts. F# has different source revisions in some AOT/JIT entries; language family alone is insufficient to merge them.

The frozen `source_report` mapping is 52 registrations from `zig-odin-roc-elm-comparison/20261010/source-audit-complete.json`, 165 from `remaining-languages-comparison/20261011/source-audit-v2.json`, one Ada repair from `remaining-languages-comparison/20261011/ada-regex-repair-source-audit.json`, and one later Zig nbody compiler-confirmation source report. The last remains tied to the exact earlier Zig source audit and LOC summary; its current source report is `nbody/experiments/20261011-shared-products/summary.json`.

The static importers assign `loc_metric='statement-normalized'` at [remaining_languages_data.py](../../overview/explorer/remaining_languages_data.py#L94) and [zig_odin_roc_elm_data.py](../../overview/explorer/zig_odin_roc_elm_data.py#L105). Job04 does not separately preserve each current `loc_metric`/`loc_rule` field, so the JSON distinguishes this importer claim from frozen field evidence. A policy label or required-work approval does not certify LOC normalization.

## What the old tools actually count

[loc.py](../../zig-odin-roc-elm-comparison/20261010/loc.py) and [metrics_final.py](../../remaining-languages-comparison/20261011/metrics_final.py) start from the number of nonblank, comment-free physical lines. They then add some packed-statement/binding expansions. Both retain standalone opening braces in that baseline. Neither applies the approved owner-aware **header 1 / executable opener 0 / independent body units / each written executable close 1** rule.

Zig/Odin's tool uses regular-expression assignment/command counts, Zig semicolon counts, and limited literal/loop-header masking. Roc/Elm have no independent-unit expansion branch. Their empty expansion arrays therefore do not prove that every physical line contains exactly one canonical unit.

The remaining-language tool masks strings/comments, protects C#/Dart loop-header semicolons and some F# literal semicolons, and expands selected semicolon/assignment sequences. It does not count uninitialized comma declarators in C#/Dart. Its Smalltalk splitting does not establish the distinction between message cascades and independent statements. Ada/Pascal/Fortran's semicolon evidence does not establish keyword block ownership or declaration groups.

Saved expansion records are useful source-specific evidence. Their complete physical-line-based totals cannot be preserved wholesale.

## Three concrete source proofs

1. **C# binary trees:** `binarytrees:binarytrees-csharpcore-3-jit`, [source](../../remaining-languages-comparison/20261011/sources/csharp/binarytrees-csharpcore-3.cs), SHA `970cf62b15e5986fe68169d6ac92556b6cde985e8f80d7ddae782f869d929fb0`. Producer physical/normalized LOC are both 62, with no expansions. Line 52 is `private TreeNode left, right;`: two independent uninitialized fields are packed on one line. Line 19 contains the `Main` header and line 20 is solely `{`, which remains in the old physical baseline. Line 22 packs an `if` header and assignment body. These require both declaration and boundary decisions; no revised total is admitted here.

2. **Dart nbody:** `nbody:nbody-dart-8`, [source](../../remaining-languages-comparison/20261011/sources/dart/nbody-dart-8.dart), SHA `51815aad44799c3a08bb1e095f635ff8b97baa632415720eef7cec8d1a104596`. Producer physical LOC is 112 and normalized LOC 114; its sole expansion is line 20, `var px = 0.0, py = 0.0, pz = 0.0;`, adding two units. Line 14 is `double x, y, z, vx, vy, vz, mass;`: seven independent uninitialized fields were not expanded. The constructor formal parameters on line 16 must remain protected from a blanket comma rule.

3. **F# nbody:** `nbody:nbody-fsharpcore-4-jit`, [source](../../remaining-languages-comparison/20261011/sources/fsharp/nbody-fsharpcore-4.fs), SHA `64a6570863ecc8665e5217c12d722cf745878134fd9e10d890ab07fba76e7cf7`. Producer physical LOC is 94 and normalized LOC 99, with expansions at lines 12 (+2), 14 (+2), and 74 (+1). The three mutable fields at line 12 are concrete independent declaration evidence. However, `type Body = {` at line 11, `let planets = [|` at line 17, and the record literal `{` at line 18 are structural type/data scopes; they cannot inherit executable brace handling without ownership review.

## Reuse and pending grammar gates

| Family group | Safe evidence to reuse | Required review before canonical admission |
|---|---|---|
| C#, Dart, Zig, Odin | Exact source pins, comment/string masking approach, prior binding expansions, shared braced event machinery after language-specific review | Uninitialized and chained declarations; named/anonymous headers; packed branches/loops; standalone opener removal; written executable close events; record/array/literal ownership |
| F#, Roc, Elm | Exact source pins and F# field/sequencing expansion rows | Layout/indentation ownership; function/control headers; independent bindings versus expression continuations; record/list/tuple separators and written closes |
| Ada, Pascal, Fortran | Exact source pins, prior semicolon expansions, source-specific declaration evidence | Keyword-owned headers/openers/ends; procedure/type/formal scopes; independent declarators; branches and continuation lines |
| Smalltalk | Exact source pins, comment/string masking approach, source-specific assignment/return evidence | Method/block headers and closes; cascades versus independent expressions; temporaries and chunk delimiters |

The next pass should normalize distinct exact source pins, then reuse reviewed metrics across matching execution-mode aliases. It must preserve producer LOC/ratios and all timing samples as historical records, with separately reviewed display overlays. Root review of language grammar and focused fixtures precedes execution under the common mutex. No unsupported fallback count or policy comment should become an admission shortcut.

## Existing expected provenance pins

These values are copied from job04 `report_pins` and saved producer summaries; they were **not freshly hashed by this review**. A locked pass must verify them before reuse.

| Producer artifact | Expected SHA-256 |
|---|---|
| zig/odin/roc/elm `source-audit-complete.json` | `0d38ea376a6a95a639ad1154b60cdbf0345dd64adb6199fe156eaf93b8e4c5ae` |
| zig/odin/roc/elm `results/summary.json` | `a36b0f227e246511753c4b8b6b94603706c19bfa819ec01c38f6a1dcc0b245f6` |
| zig/odin/roc/elm `loc.py` (summary-declared) | `617a8b126ea3858cea85c9ca4679e8ffdde6f2475f4136503d7dac4652df632a` |
| remaining `source-audit-v2.json` | `e5917fc2950b4a20e9e70fde421043c9cd4f5775a1852be3645c52d965c7dfd6` |
| remaining `results/summary.json` | `27e7114a52e4f8e71d80963628c27b8732fe823bab9d25deafebe01a0e247cff` |
| remaining `metrics_final.py` (summary-declared) | `db866bfb1b339a324b75f50d4919727192defd73370f9cb5064eda90be3ee778` |
| remaining `ada-regex-repair-source-audit.json` | `99fc6a9bdcb12f839578caf26e8c453a1de96afaecd286a972145153fdab2cd1` |
| nbody shared-products `summary.json` | `7aa2e45102130cd8e254c80641ea83486e2c6428c8ff234a806a9c392a9ab987` |

The JSON retains all exact keys, source identities/paths/SHAs, producer counts, prior audit counts/expansions, final metrics, source-report mappings, family counts, policy text, static importer claim provenance, and pending gates. It contains no admitted new score.
