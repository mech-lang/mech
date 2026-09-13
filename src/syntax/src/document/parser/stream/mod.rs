//! Retained canonical document input. Scheduling is synchronous and owned by
//! the caller; no transport, execution, or asynchronous runtime lives here.
use super::*;
use crate::document::{DocumentId, Revision};
use canonical::document::continuation::{Continuation, Progress};
use core::sync::atomic::{AtomicU64, Ordering};
mod edit;
mod types;
mod view;
pub use types::*;
pub use view::StreamView;

pub struct DocumentStream {
    source: TextSnapshot,
    parser_source: TextSnapshot,
    lookup_work: Arc<AtomicU64>,
    ids: IdGenerator,
    parser: Option<ParserState>,
    continuation: Continuation<'static>,
    state: StreamState,
    progress: StreamProgress,
    limits: StreamLimits,
    config: ParseConfig,
    interpretation: u64,
    work: StreamWork,
    work_base: StreamWork,
    published_events: usize,
    published_diagnostics: usize,
    final_snapshot: Option<Arc<SyntaxSnapshot>>,
    preview: Option<ProvisionalSnapshot>,
}
impl DocumentStream {
    pub fn new(document: DocumentId, config: ParseConfig) -> Self {
        Self::with_limits(document, config, StreamLimits::default())
    }
    pub fn with_limits(document: DocumentId, config: ParseConfig, limits: StreamLimits) -> Self {
        let source = TextSnapshot::new(document, Revision(0), "").expect("empty document");
        Self::from_source(source, config, limits)
    }
    fn from_source(source: TextSnapshot, config: ParseConfig, limits: StreamLimits) -> Self {
        let lookup_work = Arc::new(AtomicU64::new(0));
        let parser_source = source.with_lookup_work(lookup_work.clone());
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &parser_source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        parser.set_resource_rule(rules::PARSE);
        let parser = Some(parser.suspend());
        let work = StreamWork {
            retained_source_bytes: source.byte_len().0,
            peak_source_pieces: source.piece_count(),
            ..StreamWork::default()
        };
        Self {
            source,
            parser_source,
            lookup_work,
            ids,
            parser,
            continuation: Continuation::document_root(),
            state: StreamState::Open,
            progress: StreamProgress::NeedInput,
            limits,
            config,
            interpretation: 0,
            work,
            work_base: StreamWork::default(),
            published_events: 0,
            published_diagnostics: 0,
            final_snapshot: None,
            preview: None,
        }
    }
    pub fn state(&self) -> StreamState {
        self.state
    }
    pub fn work(&self) -> StreamWork {
        let mut work = self.work;
        work.source_lookup_steps = self.lookup_work.load(Ordering::Relaxed);
        work
    }
    pub fn source(&self) -> &TextSnapshot {
        &self.source
    }
    pub fn identity(&self) -> StreamIdentity {
        StreamIdentity {
            document: self.source.document(),
            revision: self.source.revision(),
            interpretation: self.interpretation,
            kind: match self.state {
                StreamState::Finished => StreamInterpretation::Finalized,
                StreamState::Limited => StreamInterpretation::Limited,
                StreamState::Cancelled => StreamInterpretation::Cancelled,
                _ => StreamInterpretation::Live,
            },
            state: self.state,
        }
    }
    fn invalidate(&mut self) {
        self.interpretation = self.interpretation.saturating_add(1);
        self.preview = None;
    }
    pub fn append(&mut self, text: &str, allowance: u64) -> Result<StreamUpdate, StreamError> {
        if self.state != StreamState::Open {
            return Err(StreamError::Closed(self.state));
        }
        if text.is_empty() {
            return Ok(self.unchanged());
        }
        if text.len()
            > self
                .limits
                .max_source_bytes
                .saturating_sub(self.source.byte_len().0) as usize
        {
            return Err(StreamError::SourceLimit);
        }
        let (source, work) = self
            .parser_source
            .append_with_work(text)
            .map_err(StreamError::Source)?;
        self.source = source.without_lookup_work();
        self.parser_source = source;
        self.work.accepted_bytes += work.accepted_bytes;
        self.work.source_bytes_copied += work.source_bytes_copied;
        self.work.source_index_bytes += work.index_bytes_scanned;
        self.work.source_nodes_allocated += work.piece_nodes_allocated + work.line_nodes_allocated;
        self.work.source_node_body_bytes += work.storage_node_body_bytes;
        self.invalidate();
        let progress = self.process(allowance);
        Ok(self.publish(progress, text.len()))
    }
    pub fn advance(&mut self, allowance: u64) -> StreamUpdate {
        if !matches!(self.state, StreamState::Open | StreamState::Finishing) {
            return self.unchanged();
        }
        let progress = self.process(allowance);
        self.publish(progress, 0)
    }
    pub fn finish(&mut self, allowance: u64) -> StreamUpdate {
        if self.state == StreamState::Open {
            self.state = StreamState::Finishing;
            self.invalidate();
        }
        self.advance(allowance)
    }
    pub fn cancel(&mut self) -> StreamUpdate {
        if matches!(self.state, StreamState::Open | StreamState::Finishing) {
            self.state = StreamState::Cancelled;
            self.invalidate();
        }
        self.publish(self.current_progress(), 0)
    }
    fn current_progress(&self) -> StreamProgress {
        match self.state {
            StreamState::Open => self.progress,
            StreamState::Finishing => StreamProgress::NeedsProcessing,
            StreamState::Finished => StreamProgress::Finished,
            StreamState::Cancelled => StreamProgress::Cancelled,
            StreamState::Limited => StreamProgress::Limited,
        }
    }
    fn process(&mut self, allowance: u64) -> StreamProgress {
        if !matches!(self.state, StreamState::Open | StreamState::Finishing) {
            return self.current_progress();
        }
        let remaining = self
            .limits
            .max_parser_work
            .saturating_sub(self.work.parser_work)
            .saturating_sub(self.work.preview_parser_work);
        if remaining == 0 {
            self.state = StreamState::Limited;
            self.invalidate();
            return StreamProgress::Limited;
        }
        let mut allowance = allowance.min(remaining);
        if allowance == 0 {
            return StreamProgress::NeedsProcessing;
        }
        let before = allowance;
        let parser_state = self.parser.take().expect("retained parser state");
        let mut parser = Parser::resume(&self.parser_source, parser_state, &mut self.ids);
        let progress = self.continuation.advance(
            &mut parser,
            self.state == StreamState::Finishing,
            &mut allowance,
        );
        let halted = parser.is_halted();
        self.parser = Some(parser.suspend());
        self.work.parser_work += before - allowance;
        self.work.continuation_resumes += 1;
        self.work.peak_document_frames = self
            .work
            .peak_document_frames
            .max(self.continuation.peak_frames);
        self.invalidate();
        if halted || matches!(progress, Progress::Limited) {
            self.state = StreamState::Limited;
            return StreamProgress::Limited;
        }
        match progress {
            Progress::NeedInput => StreamProgress::NeedInput,
            Progress::NeedsProcessing => StreamProgress::NeedsProcessing,
            Progress::Complete(_) if self.state == StreamState::Finishing => {
                self.state = StreamState::Finished;
                StreamProgress::Finished
            }
            Progress::Complete(_) => StreamProgress::NeedInput,
            Progress::Limited => unreachable!("handled limited progress"),
        }
    }
    pub fn view(&mut self) -> StreamView {
        self.work.publications += 1;
        self.shared_view()
    }
    fn unchanged(&self) -> StreamUpdate {
        let parser = self.parser.as_ref().expect("retained parser");
        StreamUpdate {
            progress: self.current_progress(),
            accepted_bytes: 0,
            syntax: StreamChange {
                from: parser.events.len(),
                old_len: parser.events.len(),
                new_len: parser.events.len(),
            },
            diagnostics: StreamChange {
                from: parser.diagnostics.len(),
                old_len: parser.diagnostics.len(),
                new_len: parser.diagnostics.len(),
            },
            view: self.shared_view(),
            work: self.work,
        }
    }
    fn shared_view(&self) -> StreamView {
        let parser = self.parser.as_ref().expect("retained parser view");
        StreamView {
            identity: self.identity(),
            source: self.source.clone(),
            parsed_through: parser.cursor.offset,
            pending_source: TextRange::new(
                parser.covered_end.min(self.source.byte_len()),
                self.source.byte_len(),
            ),
            events: parser.events.clone(),
            diagnostics: parser.diagnostics.clone(),
        }
    }
    fn publish(&mut self, progress: StreamProgress, accepted_bytes: usize) -> StreamUpdate {
        self.progress = progress;
        let parser = self.parser.as_mut().expect("retained parser publication");
        let syntax = StreamChange {
            from: parser
                .events
                .changed_from
                .min(parser.events.len())
                .min(self.published_events),
            old_len: self.published_events,
            new_len: parser.events.len(),
        };
        let diagnostics = StreamChange {
            from: parser
                .diagnostics
                .changed_from
                .min(parser.diagnostics.len())
                .min(self.published_diagnostics),
            old_len: self.published_diagnostics,
            new_len: parser.diagnostics.len(),
        };
        self.published_events = syntax.new_len;
        self.published_diagnostics = diagnostics.new_len;
        parser.events.changed_from = usize::MAX;
        parser.diagnostics.changed_from = usize::MAX;
        self.observe_work();
        StreamUpdate {
            progress,
            accepted_bytes,
            syntax,
            diagnostics,
            view: self.view(),
            work: self.work,
        }
    }
    fn observe_work(&mut self) {
        let parser = self.parser.as_ref().expect("retained work state");
        self.work.source_lookup_steps = self.lookup_work.load(Ordering::Relaxed);
        self.work.retained_source_bytes = self.source.byte_len().0;
        self.work.peak_source_pieces = self.work.peak_source_pieces.max(self.source.piece_count());
        self.work.journal_nodes_allocated = self.work_base.journal_nodes_allocated
            + parser.events.allocations
            + parser.diagnostics.allocations;
        self.work.journal_mutations = self.work_base.journal_mutations
            + parser.events.mutations
            + parser.diagnostics.mutations;
        self.work.discarded_entries = self.work_base.discarded_entries
            + parser.events.discarded
            + parser.diagnostics.discarded;
        self.work.checkpoint_rewinds = self.work_base.checkpoint_rewinds + parser.rewinds;
        self.work.rewound_bytes = self.work_base.rewound_bytes + parser.rewound_bytes;
        self.work.cached_replay_bytes =
            self.work_base.cached_replay_bytes + parser.cached_replay_bytes;
        self.work.peak_rule_depth = self.work.peak_rule_depth.max(parser.rules.peak_depth());
        self.work.peak_open_markers = self.work.peak_open_markers.max(parser.peak_open_markers);
        let tree_work = parser.tree_cache.total_work();
        self.work.tree_nodes_materialized =
            self.work_base.tree_nodes_materialized + tree_work.nodes;
        self.work.tree_token_bytes_hashed =
            self.work_base.tree_token_bytes_hashed + tree_work.token_bytes;
        self.work.tree_child_storage_nodes =
            self.work_base.tree_child_storage_nodes + tree_work.child_storage_nodes;
        self.work.tree_nodes_reused = self.work_base.tree_nodes_reused + tree_work.reused;
        self.work.peak_pending_tree_nodes = self
            .work
            .peak_pending_tree_nodes
            .max(parser.tree_cache.peak_pending);
        self.work.peak_events = self.work.peak_events.max(parser.peak_events);
    }
    /// Explicit finite-prefix parsing/export. It never changes live alternatives,
    /// fuel, or recovery. The returned snapshot always carries provisional identity.
    pub fn preview(&mut self) -> ProvisionalSnapshot {
        if let Some(preview) = &self.preview {
            return preview.clone();
        }
        let limits = StreamLimits {
            max_parser_work: self
                .limits
                .max_parser_work
                .saturating_sub(self.work.parser_work)
                .saturating_sub(self.work.preview_parser_work),
            ..self.limits
        };
        let mut preview = Self::from_source(self.source.clone(), self.config, limits);
        preview.ids = self.ids.clone();
        loop {
            let update = preview.finish(u64::MAX);
            if update.progress != StreamProgress::NeedsProcessing {
                break;
            }
        }
        let snapshot = preview
            .materialize()
            .expect("sealed preview or limited envelope");
        self.work.preview_work += preview.work.total();
        self.work.preview_parser_work += preview.work.parser_work;
        // Preview allocates from the session identity sequence without touching
        // the live parser's grammar, events, fuel, or recovery state.
        self.ids = preview.ids;
        let result = ProvisionalSnapshot {
            identity: StreamIdentity {
                kind: StreamInterpretation::FinitePreview,
                ..self.identity()
            },
            snapshot,
        };
        self.preview = Some(result.clone());
        result
    }
    /// Explicit full canonical tree/index export. Repeated calls share the same
    /// immutable result and perform no repeated parse or traversal.
    pub fn materialize(&mut self) -> Result<Arc<SyntaxSnapshot>, StreamError> {
        if !matches!(self.state, StreamState::Finished | StreamState::Limited) {
            return Err(StreamError::NotFinal);
        }
        if let Some(snapshot) = &self.final_snapshot {
            return Ok(snapshot.clone());
        }
        let state = self.parser.take().expect("retained materialization state");
        let mut parser = Parser::resume(&self.parser_source, state, &mut self.ids);
        if self.state == StreamState::Limited {
            parser.halt();
            loop {
                let mut allowance = u64::MAX;
                let progress = self.continuation.advance(&mut parser, true, &mut allowance);
                self.work.export_work += u64::MAX - allowance;
                if matches!(progress, Progress::Complete(_)) {
                    break;
                }
            }
        }
        // Keep the event view shareable after exporting. Grammar execution is
        // over; cloning these journals preserves both source and diagnostics.
        let state = parser.suspend();
        let output = ParserOutput {
            events: state.events.clone(),
            diagnostics: state.diagnostics.clone(),
            stats: ParseStats {
                events_emitted: state.events.len() as u64,
                diagnostics_emitted: state.diagnostics.len() as u64,
                ..state.stats
            },
        };
        self.work.export_work += output.events.len() as u64
            + self.source.byte_len().0 as u64
            + output.diagnostics.len() as u64;
        self.parser = Some(state);
        if self.state == StreamState::Limited {
            // Resource finalization changes the retained journals. Publish its
            // new interpretation and baseline before exposing subsequent views.
            self.invalidate();
            self.publish(StreamProgress::Limited, 0);
        }
        let mut snapshot = finish_snapshot(
            self.parser_source.clone(),
            output,
            &mut self.ids,
            SyntaxKind::Document,
        );
        snapshot.source = self.source.clone();
        let snapshot = Arc::new(snapshot);
        self.work.export_work += snapshot.nodes.node_count() as u64
            + snapshot.nodes.token_count() as u64
            + snapshot.restarts.as_slice().len() as u64;
        self.observe_work();
        self.final_snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }
}
