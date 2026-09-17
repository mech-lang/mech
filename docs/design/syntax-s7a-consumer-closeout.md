# S7A consumer closeout

S7A is still open. Passing syntax certification and resolving review comments do
not establish that every inventoried consumer can perform its job. S7B streaming
is paused until this boundary is complete. S8 remains the production call-site
replacement and removal step; unfinished consumer behavior below belongs to S7A.

| Consumer | Demonstrated canonical behavior | Remaining completion work |
| --- | --- | --- |
| Document execution | Root, Mika-local and named-fence scope execution, repeated named-fence bindings within each local owner, typed fence presentation options (output suppression preserves execution), retained mutable state, whole and indexed assignment, compound assignment, nested record/tuple/matrix updates, serial statement versions, candidate discard and failed-turn rollback. Bytecode round-trip and resident execution are exercised by `canonical_document_state`. | Declaration execution still needs its implemented handoffs; the document collector's explicit unsupported-unit errors are not completion. |
| Indexing and resolution | `SourceIndex::from_document` projects imports, aliases/groups, exports, context capabilities and addressed references into the existing resolver facts. Root and repeated named fences retain their scopes. Tests exercise dependency resolution, conflict validation, nested reads and reindexing edited snapshots. | CanonicalDocumentIndex projects separate root/named indexes for each retained Mika owner and preserves lexical parent identities. Configured fence options use the same typed presentation owner as execution. Resolved bindings still require the declaration compiler handoff. Resolver facts alone do not implement declaration execution. |
| Rendering and classification | Typed fence classification distinguishes root, named, hidden, disabled and inert fences. `DocumentSyntax::contains_executable_source` classifies canonical source while excluding display-only code, comments and recovered documents. `canonical_document_outputs` checks actual inline/fence result bindings and formatted values. | Complete formatter/HTML consumer behavior and child-scope presentation still require qualification. Excluding display-only code from execution is not a complete rendering implementation. |
| Editing | Canonical document sessions and edit-versus-fresh-parse tests preserve source, diagnostics, structural equivalence and unaffected identity. Indexing tests consume edited snapshots with updated scopes and positions. | Keep these regressions in the final-head qualification. S7B's resumable streaming optimization is a separate paused stage. |

The current registry retains the S6 activation interlock: 80 Phase 2I candidates
remain candidates while their semantic completion gate is open. S7A must not
silently convert that prerequisite into a completed activation claim.

## Qualification checkpoints

The restack onto S6 `725ee191f` retained the activation-status column and fixed
S7A's certification readers. The full syntax pass produced 792 passing tests and
two metadata/certification failures; the corrected affected targets then passed
all 14 tests. These are separate observations, not a claim that the original run
was green.

The indexed-assignment checkpoint `e0ee7c9ca` passed 11 document-state tests,
26 canonical source semantics tests, 183 resident unit tests, 33 artifact-contract
tests, and 22 workflow-contract tests locally. Operation contracts, R5 memory
planning, R6 memory runtime, bytecode fixtures and formatting checks passed.
Remote CI and review completion are tracked on PR 825; a running job is not a
passing result.

The canonical indexing increment adds eight behavioral regressions to Full CI,
alongside the indexed document-state target. Full qualification must use the
final pushed SHA, including subsequent review corrections.

The independent review of `1bb693dcf` is retained as a closeout checklist. Its
two assignment defects were corrected in `36400610d` and their PR threads were
replied to and resolved. The whole-value regressions cover both `[:]` and
`[:,:]`, matrix replacement and scalar broadcast; ordinary flattened select-all
reads remain covered by `canonical_source_review`. The broader declaration,
scope and complete document-rendering findings remain open. Where an S4 semantic
owner is unfinished, S7A must retain that dependency rather than duplicate it.

Subsequent consumer qualification also rejected missing-only recovered syntax
before indexing or source classification, and preserved the document collector's
rejection of a bare FSM pipe outside its expression owner. The full R6 static
checker mutation suite passed all 136 tests; its new shared-planner bypass test
passed separately.

The subsequent fence/resolver review corrected optional-colon namespace selection,
extended-grapheme source coordinates, and source/module import occurrence spans.
Typed option values and string literals now share canonical decoding; an `output`
option suppresses the fence result binding without suppressing its state updates.
The targeted validation passed 17 document-state tests, 46 source-review tests,
26 source-semantics tests, 12 resolver-index tests, 6 document-output tests, and
22 syntax fence/classification/edit/import/literal tests. The five canonical
registry/grammar generator checks passed. These focused results do not replace
final-head Full CI or complete the remaining consumer handoffs.

Mika-local execution/indexing now share retained DocumentScopeId owners. Tests
exercise parent/sibling/nested state isolation, named scopes within a Mika owner,
local dependency resolution, and rejection of a missing section closer. A Mika
face after executable code now ends the Mech-code run through the canonical
not-mech-code grammar rule, preserving the section owner's Mika precedence.
This adds no grammar rule or activation: 539 total rows and the 80 Phase 2I
candidates remain unchanged. Focused validation passed 19 document-state,
46 source-review, 26 source-semantics, 14 resolver-index, 6 document-output,
and 15 document-root/scope tests. Full document rendering and usable declaration
bindings remain separate completion gates.
