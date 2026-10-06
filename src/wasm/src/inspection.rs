//! Browser inspection of the public canonical syntax APIs. No parser or semantic
//! rules are implemented here. Large integer identities/counters cross as BigInt.
use mech_syntax::document::parser::event::Event;
use mech_syntax::document::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::*;
use web_time::Instant;

fn encode(value: &impl Serialize) -> Result<JsValue, JsValue> {
    value
        .serialize(
            &serde_wasm_bindgen::Serializer::new()
                .serialize_maps_as_objects(true)
                .serialize_large_number_types_as_bigints(true),
        )
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
fn error(value: impl std::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("{value:?}"))
}
fn range(value: TextRange) -> [u32; 2] {
    [value.start.0, value.end.0]
}
fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[derive(Serialize)]
struct Identity {
    document: u64,
    revision: u64,
    interpretation: u64,
    kind: String,
    state: String,
}
impl From<StreamIdentity> for Identity {
    fn from(value: StreamIdentity) -> Self {
        Self {
            document: value.document.0,
            revision: value.revision.0,
            interpretation: value.interpretation,
            kind: format!("{:?}", value.kind),
            state: format!("{:?}", value.state),
        }
    }
}
#[derive(Serialize)]
struct Change {
    from: usize,
    old_len: usize,
    new_len: usize,
}
impl From<StreamChange> for Change {
    fn from(value: StreamChange) -> Self {
        Self {
            from: value.from,
            old_len: value.old_len,
            new_len: value.new_len,
        }
    }
}
#[derive(Serialize)]
struct EventRow {
    event: String,
    kind: Option<String>,
    id: Option<u64>,
    range: Option<[u32; 2]>,
    flags: u16,
}
fn event_row(event: &Event) -> EventRow {
    let mut row = EventRow {
        event: String::new(),
        kind: None,
        id: None,
        range: None,
        flags: 0,
    };
    match event {
        Event::Start {
            identity,
            cached,
            kind,
            flags,
        } => {
            row.event = "Start".into();
            row.kind = Some(format!("{kind:?}"));
            row.id = identity.map(|id| id.0);
            row.flags = flags.0;
            row.range = cached.as_ref().map(|cached| range(cached.range));
        }
        Event::Token {
            kind,
            range: span,
            flags,
        } => {
            row.event = "Token".into();
            row.kind = Some(format!("{kind:?}"));
            row.range = Some(range(*span));
            row.flags = flags.0;
        }
        Event::Reuse { node } => {
            row.event = "Reuse".into();
            row.kind = Some(format!("{:?}", node.kind));
            row.id = Some(node.id.0);
            row.flags = node.flags.0;
        }
        Event::Finish => row.event = "Finish".into(),
        Event::Tombstone => row.event = "Tombstone".into(),
    }
    row
}
macro_rules! counters {
    ($value:expr; $($field:ident),* $(,)?) => {{
        let value = $value;
        BTreeMap::from([$( (stringify!($field), value.$field as u64) ),*])
    }};
}
fn work(value: StreamWork) -> BTreeMap<&'static str, u64> {
    counters!(value; accepted_bytes, retained_source_bytes, peak_source_pieces,
        source_bytes_copied, source_index_bytes, source_lookup_steps, source_nodes_allocated,
        source_node_body_bytes, parser_work, continuation_resumes, checkpoint_rewinds,
        rewound_bytes, cached_replay_bytes, peak_rule_depth, peak_open_markers,
        journal_nodes_allocated, journal_mutations, tree_nodes_materialized,
        tree_token_bytes_hashed, tree_child_storage_nodes, tree_nodes_reused,
        peak_pending_tree_nodes, discarded_entries, publications, preview_work,
        preview_parser_work, export_work, full_document_restarts, edit_restart_work,
        peak_document_frames, peak_events)
}
#[derive(Serialize)]
struct Publication {
    identity: Identity,
    progress: String,
    accepted_bytes: usize,
    parsed_through: u32,
    pending_source: [u32; 2],
    source_bytes: u32,
    syntax: Change,
    diagnostics: Change,
    events: Vec<EventRow>,
    diagnostic_records: Vec<Diagnostic>,
    work: BTreeMap<&'static str, u64>,
    operation_ms: f64,
    view_prepare_ms: f64,
}
fn publication(update: StreamUpdate, operation_ms: f64) -> Publication {
    let start = Instant::now();
    let view = &update.view;
    let events = view
        .events_from(update.syntax.from)
        .map(event_row)
        .collect();
    let diagnostic_records = (update.diagnostics.from..view.diagnostic_count())
        .filter_map(|i| view.diagnostic(i))
        .collect();
    Publication {
        identity: view.identity.into(),
        progress: format!("{:?}", update.progress),
        accepted_bytes: update.accepted_bytes,
        parsed_through: view.parsed_through.0,
        pending_source: range(view.pending_source),
        source_bytes: view.source.byte_len().0,
        syntax: update.syntax.into(),
        diagnostics: update.diagnostics.into(),
        events,
        diagnostic_records,
        work: work(update.work),
        operation_ms,
        view_prepare_ms: elapsed(start),
    }
}
#[derive(Serialize)]
struct TreeRow {
    id: u64,
    token: bool,
    kind: String,
    range: [u32; 2],
    flags: u16,
    depth: usize,
}
fn tree(root: SyntaxNode) -> Vec<TreeRow> {
    let mut output = Vec::new();
    let mut pending = vec![(SyntaxElement::Node(root), 0)];
    while let Some((element, depth)) = pending.pop() {
        match element {
            SyntaxElement::Node(node) => {
                output.push(TreeRow {
                    id: node.id().0,
                    token: false,
                    kind: format!("{:?}", node.kind()),
                    range: range(node.range()),
                    flags: node.flags().0,
                    depth,
                });
                pending.extend(
                    node.children_with_tokens()
                        .into_iter()
                        .rev()
                        .map(|child| (child, depth + 1)),
                );
            }
            SyntaxElement::Token(token) => output.push(TreeRow {
                id: token.id().0,
                token: true,
                kind: format!("{:?}", token.kind()),
                range: range(token.range()),
                flags: token.flags().0,
                depth,
            }),
        }
    }
    output
}
#[derive(Serialize)]
struct Snapshot {
    document: u64,
    revision: u64,
    source: String,
    strictly_clean: bool,
    lossless: bool,
    tree: Vec<TreeRow>,
    diagnostics: Vec<Diagnostic>,
    diagnostic_ranges: Vec<Option<[u32; 2]>>,
    parse_work: BTreeMap<&'static str, u64>,
    typed_document: bool,
    contains_executable_source: bool,
    semantic_status: &'static str,
}
fn snapshot(value: &SyntaxSnapshot) -> Snapshot {
    let document = DocumentSyntax::cast(value.syntax());
    Snapshot {
        document: value.document.0,
        revision: value.revision.0,
        source: value.source.to_contiguous_string(),
        strictly_clean: value.is_strictly_clean(),
        lossless: validate_lossless(&value.root, &value.source).is_ok(),
        tree: tree(value.syntax()),
        diagnostics: value.diagnostics.iter().cloned().collect(),
        diagnostic_ranges: value
            .diagnostics
            .iter()
            .map(|d| d.primary.resolve(value.revision, &value.nodes).map(range))
            .collect(),
        parse_work: counters!(value.stats; source_bytes, parser_steps, events_emitted, diagnostics_emitted, recovery_bytes),
        typed_document: document.is_some(),
        contains_executable_source: document.is_some_and(|d| d.contains_executable_source()),
        semantic_status: "Syntax inspected; semantic checking unverified; artifact construction unverified; activation unverified; execution unverified.",
    }
}

#[wasm_bindgen]
pub struct WasmSyntaxStream {
    stream: DocumentStream,
}
#[wasm_bindgen]
impl WasmSyntaxStream {
    #[wasm_bindgen(constructor)]
    pub fn new(document: u64, max_source_bytes: u32, max_parser_work: u64) -> Self {
        Self {
            stream: DocumentStream::with_limits(
                DocumentId(document),
                ParseConfig::default(),
                StreamLimits {
                    max_source_bytes,
                    max_parser_work,
                },
            ),
        }
    }
    pub fn append(&mut self, text: &str, allowance: u64) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let update = self.stream.append(text, allowance).map_err(error)?;
        encode(&publication(update, elapsed(start)))
    }
    pub fn advance(&mut self, allowance: u64) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let update = self.stream.advance(allowance);
        encode(&publication(update, elapsed(start)))
    }
    pub fn finish(&mut self, allowance: u64) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let update = self.stream.finish(allowance);
        encode(&publication(update, elapsed(start)))
    }
    pub fn cancel(&mut self) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let update = self.stream.cancel();
        encode(&publication(update, elapsed(start)))
    }
    pub fn preview(&mut self) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let preview = self.stream.preview();
        let operation_ms = elapsed(start);
        encode(&(
            Identity::from(preview.identity),
            snapshot(&preview.snapshot),
            work(self.stream.work()),
            operation_ms,
        ))
    }
    /// Explicit export. Limited exports require resynchronization; resync() is
    /// deliberately separate so the caller cannot mistake it for an update.
    pub fn materialize(&mut self) -> Result<JsValue, JsValue> {
        let start = Instant::now();
        let value = self.stream.materialize().map_err(error)?;
        let operation_ms = elapsed(start);
        encode(&(
            Identity::from(self.stream.identity()),
            snapshot(&value),
            work(self.stream.work()),
            operation_ms,
        ))
    }
    pub fn resync(&mut self) -> Result<JsValue, JsValue> {
        let view = self.stream.view();
        let mut result = publication(
            StreamUpdate {
                progress: match self.stream.state() {
                    StreamState::Finished => StreamProgress::Finished,
                    StreamState::Cancelled => StreamProgress::Cancelled,
                    StreamState::Limited => StreamProgress::Limited,
                    _ => StreamProgress::NeedsProcessing,
                },
                accepted_bytes: 0,
                syntax: StreamChange {
                    from: 0,
                    old_len: 0,
                    new_len: view.event_count(),
                },
                diagnostics: StreamChange {
                    from: 0,
                    old_len: 0,
                    new_len: view.diagnostic_count(),
                },
                view,
                work: self.stream.work(),
            },
            0.0,
        );
        result.progress = "Resynchronized".into();
        encode(&result)
    }
}

#[wasm_bindgen]
pub struct WasmSyntaxEditor {
    session: DocumentSession,
}
#[wasm_bindgen]
impl WasmSyntaxEditor {
    #[wasm_bindgen(constructor)]
    pub fn new(document: u64, source: &str) -> Self {
        Self {
            session: DocumentSession::new_with_document(
                DocumentId(document),
                source,
                ParseConfig::default(),
            ),
        }
    }
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        encode(&snapshot(self.session.snapshot()))
    }
    /// Format the retained document syntax for a passive editor preview.
    #[wasm_bindgen(js_name = renderHtml)]
    pub fn render_html(&self) -> Result<String, JsValue> {
        render_editor_document(self.session.snapshot())
            .map_err(|message| JsValue::from_str(&message))
    }
    /// Ranges are UTF-8 bytes. Empty ranges insert; empty replacement deletes.
    pub fn replace(&mut self, start: u32, end: u32, replacement: &str) -> Result<JsValue, JsValue> {
        if start > end {
            return Err(JsValue::from_str("edit start exceeds end"));
        }
        let before: BTreeSet<_> = self
            .session
            .snapshot()
            .nodes
            .nodes()
            .map(|(id, _)| id)
            .collect();
        let old_diagnostics: BTreeMap<_, _> = self
            .session
            .snapshot()
            .diagnostics
            .iter()
            .map(|d| (d.id, d.clone()))
            .collect();
        let timer = Instant::now();
        let update = self
            .session
            .try_apply_edits(&[TextEdit::replace(
                TextRange::new(TextSize(start), TextSize(end)),
                replacement,
            )])
            .map_err(error)?;
        let operation_ms = elapsed(timer);
        let current = self.session.snapshot();
        #[derive(Serialize)]
        struct EditResult {
            old_revision: u64,
            new_revision: u64,
            affected_old_range: [u32; 2],
            changed_range: [u32; 2],
            preserved_ids: Vec<u64>,
            removed_ids: Vec<u64>,
            new_ids: Vec<u64>,
            added_diagnostics: Vec<u64>,
            removed_diagnostics: Vec<u64>,
            retained_diagnostics: Vec<u64>,
            changed_diagnostics: Vec<u64>,
            work: BTreeMap<&'static str, u64>,
            operation_ms: f64,
            snapshot: Snapshot,
        }
        encode(&EditResult {
            old_revision: update.old_revision.0,
            new_revision: update.new_revision.0,
            affected_old_range: [start, end],
            changed_range: range(update.changed_range),
            preserved_ids: update.reused_roots.iter().map(|id| id.0).collect(),
            removed_ids: before
                .iter()
                .filter(|id| !current.nodes.contains_node(**id))
                .map(|id| id.0)
                .collect(),
            new_ids: current
                .nodes
                .nodes()
                .filter(|(id, _)| !before.contains(id))
                .map(|(id, _)| id.0)
                .collect(),
            added_diagnostics: update.diagnostics.added.iter().map(|id| id.0).collect(),
            removed_diagnostics: update.diagnostics.removed.iter().map(|id| id.0).collect(),
            retained_diagnostics: update.diagnostics.retained.iter().map(|id| id.0).collect(),
            changed_diagnostics: current
                .diagnostics
                .iter()
                .filter(|d| old_diagnostics.get(&d.id).is_some_and(|old| old != *d))
                .map(|d| d.id.0)
                .collect(),
            work: counters!(update.stats; source_bytes, total_parser_steps, total_events_emitted,
                reconciliation_steps, reconciliation_limit, reused_node_count, new_node_count, document_fallbacks),
            operation_ms,
            snapshot: snapshot(current),
        })
    }
}

fn render_editor_document(snapshot: &SyntaxSnapshot) -> Result<String, String> {
    let document = DocumentSyntax::cast(snapshot.syntax())
        .ok_or_else(|| "editor snapshot requires canonical document syntax".to_owned())?;
    mech_runtime::CanonicalDocumentRenderer
        .format_html(&document)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod editor_render_tests {
    use super::*;

    #[test]
    fn editor_preview_formats_retained_document_structure() {
        let editor = WasmSyntaxEditor::new(
            710,
            "Calculation\n===========\n\nA **bold** result.\n\n```mech\nanswer := 6 * 7\nanswer\n```\n",
        );
        let html = render_editor_document(editor.session.snapshot()).unwrap();
        assert!(html.contains("Calculation"));
        assert!(html.contains("<strong"));
        assert!(html.contains("mech-code"));
        assert!(!html.contains("```"));
    }

    #[test]
    fn editor_preview_uses_current_revision_and_escapes_source_text() {
        let mut editor = WasmSyntaxEditor::new(711, "Before\n======\n\nA paragraph.\n");
        editor
            .session
            .try_apply_edits(&[TextEdit::replace(
                TextRange::new(TextSize(0), TextSize(6)),
                "After!",
            )])
            .unwrap();
        let html = render_editor_document(editor.session.snapshot()).unwrap();
        assert!(html.contains("After!"));
        assert!(!html.contains("Before"));
        let editor = WasmSyntaxEditor::new(
            712,
            "Text\n====\n\n```text\n<img src=x onerror=alert(1)>\n```\n",
        );
        let html = render_editor_document(editor.session.snapshot()).unwrap();
        assert!(html.contains("&lt;img"));
        assert!(!html.contains("<img src=x"));
    }

    #[test]
    fn editor_preview_renders_recovered_fence_and_repairs_its_error() {
        let source = include_str!(
            "../../syntax/tests/fixtures/document/recovery/fenced-unclosed-matrix.mec"
        );
        let mut editor = WasmSyntaxEditor::new(713, source);
        let snapshot = editor.session.snapshot();
        assert!(!snapshot.is_strictly_clean());
        let html = render_editor_document(snapshot).unwrap();
        for text in [
            "Calculation",
            "answer",
            "Section One",
            "This is the first section.",
        ] {
            assert!(html.contains(text), "{html}");
        }
        let position = source.find("5\n").unwrap() as u32 + 1;
        editor
            .session
            .try_apply_edits(&[TextEdit::insert(TextSize(position), "]")])
            .unwrap();
        assert!(editor.session.snapshot().is_strictly_clean());
        assert!(
            render_editor_document(editor.session.snapshot())
                .unwrap()
                .contains("Section One")
        );
    }

    #[test]
    fn editor_preview_preserves_later_sections_for_fenced_and_unfenced_errors() {
        let tail =
            "\n1. Section One\n------------------------------\n\nThis is the first section.\n";
        for body in [
            "answer := [1 2 3 4 5\n",
            "answer := (1 + 2\n",
            "answer := {x: 1 y: 2\n",
            "answer := [1, +, 2]\n",
            "answer := 1 +\n",
            "answer := [1 @ [2\n",
        ] {
            for fenced in [false, true] {
                let source = if fenced {
                    format!("```mech\n{body}```\n{tail}")
                } else {
                    format!("{body}{tail}")
                };
                let editor = WasmSyntaxEditor::new(714, &source);
                assert!(!editor.session.snapshot().is_strictly_clean(), "{source}");
                let html = render_editor_document(editor.session.snapshot()).unwrap();
                assert!(html.contains("Section One"), "{source}: {html}");
                assert!(
                    html.contains("This is the first section."),
                    "{source}: {html}"
                );
                assert!(html.contains("mech-subtitle"), "{source}: {html}");
            }
        }
        let source = format!("```mech{{output: }}\nanswer := 1\n```\n{tail}");
        let editor = WasmSyntaxEditor::new(715, &source);
        let html = render_editor_document(editor.session.snapshot()).unwrap();
        assert!(html.contains("mech-recovered-source"), "{html}");
        assert!(html.contains("Section One"), "{html}");
    }
}
