use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamState {
    Open,
    Finishing,
    Finished,
    Cancelled,
    Limited,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamProgress {
    NeedInput,
    NeedsProcessing,
    Finished,
    Cancelled,
    Limited,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamLimits {
    pub max_source_bytes: u32,
    /// Cumulative canonical continuation transitions, including scanners and
    /// speculative work. Final resource-envelope export is accounted separately.
    pub max_parser_work: u64,
}
impl Default for StreamLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: u32::MAX,
            max_parser_work: 100_000_000,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamError {
    Closed(StreamState),
    SourceLimit,
    Source(crate::document::SourceError),
    NotFinal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamInterpretation {
    Live,
    FinitePreview,
    Finalized,
    Limited,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamIdentity {
    pub document: DocumentId,
    pub revision: Revision,
    pub interpretation: u64,
    pub kind: StreamInterpretation,
    pub state: StreamState,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamWork {
    pub accepted_bytes: u64,
    pub retained_source_bytes: u32,
    pub peak_source_pieces: usize,
    pub source_bytes_copied: u64,
    pub source_index_bytes: u64,
    pub source_nodes_allocated: u64,
    pub source_node_body_bytes: u64,
    pub parser_work: u64,
    pub continuation_resumes: u64,
    pub checkpoint_rewinds: u64,
    pub rewound_bytes: u64,
    pub cached_replay_bytes: u64,
    pub peak_rule_depth: usize,
    pub peak_open_markers: usize,
    pub journal_nodes_allocated: u64,
    pub journal_mutations: u64,
    pub tree_nodes_materialized: u64,
    pub tree_token_bytes_hashed: u64,
    pub tree_child_storage_nodes: u64,
    pub tree_nodes_reused: u64,
    pub peak_pending_tree_nodes: usize,
    pub discarded_entries: u64,
    pub publications: u64,
    pub preview_work: u64,
    pub export_work: u64,
    pub full_document_restarts: u64,
    pub edit_restart_work: u64,
    pub peak_document_frames: usize,
    pub peak_events: usize,
}
impl StreamWork {
    pub fn total(&self) -> u64 {
        self.source_bytes_copied
            .saturating_add(self.source_index_bytes)
            .saturating_add(self.source_nodes_allocated)
            .saturating_add(self.parser_work)
            .saturating_add(self.journal_nodes_allocated)
            .saturating_add(self.journal_mutations)
            .saturating_add(self.tree_child_storage_nodes)
            .saturating_add(self.discarded_entries)
            .saturating_add(self.publications)
            .saturating_add(self.edit_restart_work)
            .saturating_add(self.preview_work)
            .saturating_add(self.export_work)
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamChange {
    /// The replaced suffix of the canonical event/diagnostic journal. A view
    /// shares its prefix; this never enumerates unchanged entries.
    pub from: usize,
    pub old_len: usize,
    pub new_len: usize,
}
#[derive(Clone, Debug)]
pub struct StreamUpdate {
    pub progress: StreamProgress,
    pub accepted_bytes: usize,
    pub syntax: StreamChange,
    pub diagnostics: StreamChange,
    pub view: StreamView,
    pub work: StreamWork,
}
#[derive(Clone, Debug)]
pub struct ProvisionalSnapshot {
    pub identity: StreamIdentity,
    pub snapshot: Arc<SyntaxSnapshot>,
}
