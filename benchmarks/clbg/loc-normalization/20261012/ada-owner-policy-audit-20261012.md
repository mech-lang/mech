# Bounded Ada owner review after job15

This is an additive, read-only source-owner review of four exact identities from `jobs/boundary-audit-15/candidate-audit.json`. Identity strings and paths below are copied metadata, not freshly computed hashes. The review inspected source slices and the current `foreign_keyword.py` / `test_foreign_keyword.py`; it ran no imports, lexer, normalization, hashes, tests, builds, timing, or Git commands and changed no source, normalizer, fixture, or frozen job. This note contains no newly admitted count.

| Exact identity | Metadata path, relative to this checkout | Relevant source and job15 gate |
|---|---|---|
| `aa50bc0d5de178ef377fbf0741affa7eef61fc9f9fa3a0762a77f82c703261e4:ada` | `benchmarks/clbg/remaining-languages-comparison/20261011/sources/ada/binarytrees-gnat-4-native.adb` | Lines 111–116: `return New_Worker : Depth_Worker (Start => Start_Index, Finish => End_Index) do` with one assignment and `end return;`. Close-name gates occur at 116 and 117. |
| `0b3d2bdd6e8486b1a901a81941eb661cad6e140d920ce50fd0a91561dd62ca6d:ada` | `benchmarks/clbg/remaining-languages-comparison/20261011/sources/ada/pidigits-gnat-2.adb` | Lines 122–123: `<<Work_Loop>>` immediately precedes `while I < N loop`; `goto Work_Loop;` is written at 127 and 131. Owner mismatch occurs at `end loop;` 141, then unmatched close at 150. |
| `b01f7863f150f945775dc7733413f726b77ad58280ba4cb786234202af88527b:ada` | `benchmarks/clbg/remaining-languages-comparison/20261011/sources/ada/fasta-gnat-2-faithful.adb` | Generic introducer 264, formal floating-point type `type Real is digits <>;` 266, package specification 268–274. Generic complex-owner gate occurs at 264. |
| `0c6c91e4c027b3cbab4a820cb0574b959ec1a7c9b31060084e4afcaacd9d4c53:ada` | `benchmarks/clbg/remaining-languages-comparison/20261011/sources/ada/spectralnorm-gnat-3.adb` | Generic introducer 53, formal floating-point type 55, formal objects `No_Of_Tasks : Positive;` 56 and `Matrix_Size : Positive;` 57, package specification 59–92. Generic complex-owner gate occurs at 53. |

## Decisions requested from the root owner

The established rule supplies function/control header 1, opening delimiter 0, written executable close 1, structural formals/arguments, separately counted independent bindings, and one unit for a pure expression composition. It does not explicitly resolve these three new forms.

1. For the extended return, is `New_Worker` structural return-object metadata within one executable return header, or a separately counted independent object binding? The exercised return region is consequently header 1 + body assignment 1 + written `end return` 1, plus the unresolved return-object contribution. Its discriminant associations remain arguments; `do` supplies no extra opening unit. No separate unit decision is inferred from the object name merely appearing after `return`.
2. Does a proved `<<Work_Loop>>` statement label contribute structural 0 or one separate written label/declaration unit? It is distinct from a named loop identifier `Work : loop`; it must neither populate the named-loop identifier nor imply that `end loop Work_Loop;` is valid. Each written `goto Work_Loop;` remains one action.
3. How is a proved generic formal part counted: generic introducer, formal type, and formal objects? One candidate policy is introducer 0 and all generic formals structural 0 under the following package header 1. An alternative preserves some written formal-declaration units, with grouped independent names separated. Root approval must distinguish these explicitly before any numeric fixture is added. There is no written `end generic` in either source.

## General grammar pathways

**Extended return.** Add a statement owner only for a proved `return defining_identifier : subtype_indication do` at an executable statement boundary inside a proved function. Own the complete subtype/discriminant text through the top-level `do`; do not expose named associations as independent bindings. Push an executable `extended-return` frame and require the immediate `end return;` to close that frame. The existing close dispatcher treats `return` as a named definition suffix and pops the function, causing both aa50 gates. A return close must be a reserved suffix, not a name comparison. Nested `if`/loop owners must close first. Incomplete subtype groups, absent/malformed `do`, an unmatched `end return`, and unreviewed variants retain gates. Simple expression returns continue through the existing expression accounting path.

**Statement labels.** Prove complete `<< identifier >>` sequences only at an actual statement boundary, associate them with the immediately following statement, and let that statement start after the label. The current `_statement_start` omits a proved label end, so the while owner is never pushed. Do not broaden statement starts to every `>` token; comparison operators remain expressions. Keep label ownership distinct from `_ada_loop_header_start`, which currently proves `identifier : loop/while/for`. Handle packed and expanded spellings alike; label-only, incomplete label, or unsupported following statement ownership retains a targeted gate.

**Generic formal parts.** Associate `generic` with a bounded generic formal sequence and its following package declaration. Initially prove the exercised floating-point formal `type Real is digits <>;` and simple subtype-mark object formals such as `A, B : Positive;`. Preserve their explicit owner spans so runtime binding expansion cannot accidentally count them as package locals. The following package specification and subsequent package body continue to use their existing named owners. Admit the generic keyword only after the complete formal-part/package association is proved. Unknown formal subprograms/packages, unsupported formal type kinds/defaults, missing semicolons, or a generic not followed by its proved declaration retain review gates; merely suppressing the generic keyword diagnostic is insufficient.

## Meaningful fixture plan, pending those decisions

Use paired packed and expanded source spellings, and check owner/close events as well as totals. Let `R` denote the approved extra return-object contribution, `L` the label contribution, and `G/T/O` the generic-introducer/formal-type/per-object contributions. These symbols are unresolved questions, not approved counts.

| Fixture | Expected accounting after approval |
|---|---|
| `function F return T is begin return Item : T do X:=1; end return; end F;` | Outer function header/close 2 + return header/body/close 3 + `R`; both closes must have their immediate owners. |
| Same extended return with multiline discriminant associations, and nested `if Ready then X:=1; end if;` | Associations structural; nested if adds header/body/close 3 rather than its physical rows. Return-object policy remains `R`. |
| `function F return T is begin return Make(A=>1,B=>2); end F;` | Existing simple return composition remains 1, complete function 3. |
| Wrong-order `end F;` while an extended-return frame is open; unmatched `end return;`; incomplete subtype parentheses | Review gates; a foreign or malformed closer cannot silently pop another owner. |
| `procedure P is begin <<Again>> while Ready loop goto Again; end loop; end P;` | Procedure header/close 2 + while header/close 2 + goto 1 + `L`. Label and loop identifier remain distinct. |
| `procedure P is begin <<Here>> Call; end P;` | Header/call/close 3 + `L`; proves a label is not a loop-only exception. |
| Comparison expression containing `>` or `>=`, incomplete `<<Here>`, standalone `<<Here>>`, and `<<Here>> while Ready loop null; end loop Here;` | Expressions stay composed; malformed/unowned labels and label-as-loop-name misuse retain gates. |
| `generic type Real is digits <>; A,B: Positive; package G is procedure P(X,Y:Real); end G;` | Existing package header, declaration, close 3 + `G + T + 2O`; ordinary procedure formals remain structural. |
| A generic package followed by `package body G is Local: Positive; end G;` | Formal-part policy ends at the specification; the separately written body-local binding stays independent. |
| Missing formal semicolon, incomplete `digits <>`, missing following package, and unreviewed `with procedure ...` formal | Targeted review gates rather than blanket generic admission. |

No implementation or fixture expectation should assign values to `R/L/G/T/O` until root approval is relayed. Subsequent fixture/source execution belongs to the reviewed common-mutex job, and passing fixtures alone does not admit a display overlay.
