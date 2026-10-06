# Streaming and editor inspection boundary

Baseline: `c4777b7015fe8ff47fdfa48d18606c49aace7d97` (`integration/v0.4`). Inspection performed in the isolated audit checkout. The modification adds an inspection adapter over the existing parsing, recovery and reconciliation APIs.

## Existing implementation

`mech_syntax::document::DocumentStream` accepts valid UTF-8 strings, stores canonical parser continuation state, and advances with an explicit work allowance. `append`, `advance`, `finish`, and `cancel` return an update containing an immutable view and replacement-suffix deltas. Each delta is relative to the immediately preceding returned update, including `NeedsProcessing`; dropping intermediate updates is invalid. `preview` performs an explicitly provisional finite-prefix parse. Semantic consumers require finalized, materialized and strictly validated syntax.

`StreamIdentity` consists of document, revision, interpretation counter, interpretation kind, and lifecycle state. The interpretation counter, kind and lifecycle state distinguish previews, finalization, cancellation and ownership handoffs within a revision. The browser also needs a controller generation identifier to prevent a scheduled callback from updating a restarted instance.

`DocumentSession::try_apply_edits` validates sorted UTF-8 byte ranges. `incremental/reparse.rs` performs a full canonical parse of the edited document, then reconciles identities and diagnostics. `total_parser_steps` and `reconciliation_steps` are separate counters. `reused_node_count` measures identity reuse. The parser processes the complete document on each edit. Reconciliation uses an explicit budget; on exhaustion, the canonical parse retains fresh identities.

Relevant implementation: `src/syntax/src/document/parser/stream/{mod,types,view,edit}.rs`; `src/syntax/src/document/incremental/{session,reparse,stats,delta}.rs`. Existing proof coverage includes every legal scalar cut, malformed recovery, publication mirrors, resource limits, cancellation, preview isolation, and arbitrary edit sequences.

## Browser integration finding

At baseline, `src/wasm/src/canonical_document.rs` exposes retained browser source internally, and `WasmDocument` provides the application controller. The requested browser exports for `DocumentStream` and `DocumentSession` are absent at baseline. The adapter uses the same `mech-wasm` browser package and calls these public Rust APIs directly. JavaScript owns transport, controls, rendering and evidence comparisons.

`src/wasm/src/inspection.rs` exports `WasmSyntaxStream` and `WasmSyntaxEditor`, under an explicit `syntax_inspection` feature that includes `browser_project_core`; the normal browser profiles exclude these audit inspection exports. It forwards lifecycle calls, structured diagnostics, suffix events, full identities, lossless tree rows, typed document source classification, and work counters. Every `u64` is serialized as JavaScript `BigInt`. UTF-8 transport decoding and UTF-16 selection conversion belong to the browser UI. A `TextDecoder` in streaming mode is required when transport chunks may split encoded scalar values.

The inspection status records syntax results. Semantic checking, artifact construction, activation and execution are unverified in this inspector. The type and application demonstrations provide evidence for those stages.

## Measurement boundaries

Core operation time is measured around the public Rust operation. View preparation time measures suffix export construction. Browser call elapsed time includes serialization and bridge overhead. Explicit preview and materialization report their own timings and counters. These elapsed times are diagnostic measurements from this environment. Controlled performance comparisons remain unverified. `StreamWork.parser_work` counts continuation transitions, including scanners and speculative work. `preview_parser_work` records explicit preview transitions. Editor `total_parser_steps` reports the complete canonical parse counter; `reconciliation_steps` records identity reconciliation operations. These counters have distinct definitions.

The browser retains its latest mirror, optional current full snapshot, and a user-selected bounded log. Historical immutable Rust views are released after serialization. Snapshot materialization is an explicit export operation. Resource limits are configured at stream creation. The UI records transport position and accepted source separately. Rejected decoded chunks remain pending until successful admission or restart.

## Acceptance checks

Existing native suites: `document_streaming_lifecycle`, `document_streaming_equivalence`, `document_streaming_recovery`, `document_streaming_publication`, `document_streaming_resources`, `document_incremental`, `document_incremental_properties`. The recorded baseline run passed 44 tests across these seven suites, with zero failures. The exhaustive equivalence suite took 199.24 seconds on the recorded native environment. Commands and scope are in `evidence/streaming-native.json`; the complete result is in `evidence/streaming-native-tests.log`.

The adapter browser checks must consume every suffix update, compare final lossless trees and normalized diagnostics against a one-shot `DocumentSession` snapshot, and separately assert simple grammar expectations. Short fixtures use every scalar split. A byte-transport test splits Unicode into individual bytes before decoding. Additional cases cover preview identity isolation, finish, cancel, parser-work limits, source-size rejection, insertion, replacement, repair, and deletion. The browser checks must confirm preserved and replaced identities and full-document parser work.

## Scope and unresolved requirements

The manual review covered stream lifecycle, publication representations, session edit ownership, reconciliation accounting, and browser entry points. Review coverage excludes the complete grammar-production and semantic-lowering inventories. Evidence scope is the listed cases and configurations. The architecture register records historical acceptance requirements and their evidence states.

## Browser verification and corrections

The final package is recorded in `site/artifact.json`. The page hashes the exact WASM bytes passed to initialization and compares them with that manifest. `check-streaming-browser.py` reuses the maintained `ChromeSession` harness and waits for both behavioral results and artifact verification before saving evidence. It also exercises actual buttons, source selections, Unicode range edits and lifecycle resets.

Two audit-tool corrections are preserved. `evidence/streaming-frontend-fix.json` records exact BigInt-to-array-index conversion at the browser boundary. `evidence/streaming-budget-correction.json` records the completion scenario's cumulative work-budget correction, including a native reproduction of shared preview/live accounting. The parser's resource-limit behavior matched its contract.

Visual verification covers `evidence/streaming-desktop.png`, `streaming-structures.png`, `streaming-editor.png` and `streaming-diagnostics.png`. The first three use a 1400 × 1000 desktop viewport.

Final artifact diagnostic selection and the four desktop captures are reproducible with `python3 audit/v04/check_streaming_visual.py`. The maintained malformed fixture selects bytes [9, 13). All final streaming and visual records identify the shared `f833f55e…` WASM artifact.
