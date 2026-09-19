# S7A consumer closeout

S7A is still open. Passing syntax certification and resolving review comments do
not establish that every inventoried consumer can perform its job. S7B streaming
remains a separate foundation stage. S8 remains the production call-site
replacement and removal step; unfinished consumer behavior below belongs to S7A.

| Consumer | Demonstrated canonical behavior | Remaining completion work |
| --- | --- | --- |
| Document execution | Root, Mika-local and named-fence scope execution, repeated named-fence bindings within each local owner, typed fence presentation options (output suppression preserves execution), retained mutable state, whole and indexed assignment, compound assignment, nested record/tuple/matrix updates, serial statement versions, candidate discard and failed-turn rollback. Canonical imports now bind resolved dependency exports to artifact inputs in root, named-root, Mika-root and named-Mika owners; exports publish named artifact outputs. | Keep the new declaration handoff, positioned failure and bytecode/resident execution regressions in final-head qualification. Language forms whose semantic implementation remains S4-owned continue to fail with explicit capability errors. |
| Indexing and resolution | `SourceIndex::from_document` projects imports, aliases/groups, exports, context capabilities and addressed references into the existing resolver facts. Root and repeated named fences retain their scopes. FSM formal declarations are excluded while start/guard/body reads remain. `CanonicalDocumentIndex` projects every retained Mika owner with one shared coordinate projection and preserves lexical parent identities. | Keep the document-level FSM, nested-owner, Unicode/CRLF, large-line and complete nested-document work checks in final-head qualification. |
| Rendering and classification | Typed fence classification distinguishes root, named, hidden, disabled and inert fences. `CanonicalDocumentRenderer` consumes retained syntax plus owner-associated completed results directly and produces complete text/HTML documents. Tests cover title/section/prose structure, the shared mixed-document fixture, displayed and evaluated inline code, root/named/Mika result placement, hidden execution, disabled/inert display, options, escaping and foreign/missing result rejection. | S8 still owns switching shipping formatter/server/bundle/browser callers to this qualified canonical consumer. Keep direct rendering integration tests in final-head qualification. |
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
bindings were the next completion gates.

The subsequent consumer-completion candidate implements those gates without a
legacy `Program` translation. Resolver-owned imports and contexts are metadata
at engine lowering; resolved value exports bind through the existing runtime
resolver namespace rules, and document exports become named artifact outputs.
The same handoff is exercised for root, named-root, Mika-root and named-Mika
owners, including positioned unresolved/missing-export failures. The canonical
renderer walks retained document structure and accepts completed results keyed
by retained owner, scope, output kind and source range. It rejects foreign,
duplicate and missing visible result slots. Sequential derived RMW comparison
backups now use disjoint-lifetime reuse groups, with the backup arena sized to
the largest simultaneous member rather than their sum; external-effect capture
keeps its separate allocation policy. These are candidate results until the
final pushed SHA completes required CI and independent review.

The exact-head review of `1d2136550` found four additional presentation-boundary
cases. Deferred inline expressions now snapshot every already-bound local at the
inline's physical source position and snapshot each forward local as soon as its
definition becomes available; a later update can no longer leak backward into
the inline result. Direct slice stems participate in the same forward-local
discovery. Canonical indexing traverses only the value of a variable definition,
matching assignment/send write-role handling, while context declarations remain
declarations rather than addressed reads. Finally, renderer result admission now
validates the retained execution scope as well as document revision and Mika/root
owner, including outputs suppressed from presentation. Focused qualification
passes the mixed-state forward-inline, forward-slice, destination-write and
scope-relabel regressions together with all document output, renderer, index,
document-state and source-semantics integrations.

The final consumer audit also closed declaration and presentation omissions at
their failure boundaries. Source-path wildcard imports now require a resolved
dependency while compiler-module wildcards remain optional; a resolved
namespace must supply every namespace-owned program input. Source
classification no longer routes import, export or context-only documents into
the executable compiler. Visible root program slots are mandatory in both text
and HTML rendering, and retained images render as escaped figures after safe
source-scheme validation. The obsolete-package CI exception is occurrence-bound
and recognizes CSS selectors or actual quoted `class` attributes only, so
unrelated markup and similarly named attributes cannot hide a package edge.
The presentation inventory was then re-audited node by node: image URL
delimiters are selected after captions, raw links and inline code have semantic
HTML paths, inline emphasis/equation/reference forms do not leak delimiters,
and retained callouts, lists, equations, thematic breaks, tables, figures and
floats use structural containers rather than escaped source punctuation. The
document dispatch boundary now excludes lists and footnotes from Mech-code
classification, so those certified rules are reachable from a clean complete
document; multi-paragraph retained notes preserve every paragraph. Semantic
title/front-matter rendering consumes completed inline results; citations are
numbered and deferred to link-safe backmatter; image options, float direction,
table alignment and inert-fence language survive as constrained presentation
attributes; checked-list continuations and figure-grid panels retain their
structural roles. Ordered-list starts/items and citation/footnote numbering
preserve authored order rather than being regenerated from definition order.

## Independent seal audit

The candidate diff was traced from retained syntax through each affected owner,
not inferred from green smoke tests. The bounded S7A gate has this disposition:

| Seal gate | Direct implementation evidence | Qualification witness | Candidate disposition |
| --- | --- | --- | --- |
| FSM resolver roles | Canonical traversal omits specifications and implementation formals while retaining start, guard and body reads. Assignment, send and definition destinations are write roles; only their executable values are traversed. | `canonical_source_index::fsm_formal_inputs_and_specifications_are_not_resolver_reads` starts from a parsed document and asserts both excluded and retained targets. Destination regressions prove addressed RHS reads remain indexed without publishing their targets. | Complete in candidate. |
| RMW backup ownership | Derived comparison backups alone share a per-kind, disjoint-lifetime region; effect payloads retain cumulative storage. Peak and budget evaluation use the plan's effective limits. | The resident unit test proves unequal-size maximum-only arena/peak and budget admission. Document-state tests prove multiple actual updates, final-value change detection, candidate discard, failed-turn rollback and budget release. | Complete in candidate. |
| Whole-document indexing work | `CanonicalDocumentIndex` constructs one immutable coordinate projection and reuses it for root and every nested Mika owner. Standalone owner APIs remain self-contained. | The complete-document unit test parses nested Mika owners, checks each local result and asserts exactly one projection; Unicode/CRLF and 1,024-reference integration checks remain separate. | Complete in candidate. |
| Declaration handoff | The runtime handoff keeps the canonical index and compiled program together, binds resolved dependency exports using established namespace/alias rules, requires source-path wildcard edges, leaves compiler-module imports optional, excludes context aliases from source edges, validates namespace-owned inputs against resolved exports, and publishes canonical exports as artifact outputs. Construction and binding are result-valued, so an error cannot publish a partially accepted program or environment. | Root, named-root, Mika-root and named-Mika execution use imported values and publish exports. Unresolved required and wildcard dependencies, missing single/namespace exports, incomplete completed exports and unknown exported bindings retain source positions. Metadata-only documents remain outside executable classification. | Complete for implemented declaration owners; S4-owned unsupported forms remain explicit capability errors. |
| Complete rendering | The renderer walks retained canonical nodes directly, associates results by document revision, retained owner ancestry, retained root/named execution scope, output kind and source range, and does no evaluation or legacy lowering. Inline results retain source-order state versions, including mixed forward/current locals and title front matter; aggregate outputs without visible presentation ownership are not rendered, while visible root aggregates are required. Retained rich-document nodes use semantic containers without delimiter leakage, and list/footnote dispatch remains outside executable Mech classification. | Complete text/HTML tests cover the shared mixed fixture, semantic inline markup/code/equations/references, raw and labelled links, evaluated/displayed inline code, retained blank lines, semantic title/front-matter/subtitles, root/named/Mika results, hidden execution, disabled/inert fence languages, safely constrained image options and caption parentheses, callouts, ordered/unordered/checked lists and continuations, numbered deferred citations and multi-paragraph footnotes, equations, thematic breaks, aligned tables, labelled figure grids, directed floats, escaping, safe hyperlink/image schemes, and stale/foreign/same-document-misowned/scope-relabelled/duplicate/missing result rejection. | Complete in candidate; production caller replacement remains S8. |
| Exact-head authority | Full CI now invokes the handoff and renderer integrations plus the canonical-index library units that the earlier integration-only command omitted. | The final pushed SHA still requires completed required checks and a fresh clean Codex review. | Pending final-head evidence. |

No audit item changes the S6 activation prerequisite or claims implementation of
the eleven S4-owned document-unit forms that still fail closed.
