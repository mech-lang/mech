use crate::document::parser::{ParseConfig, parse_canonical_document_with_ids};
use crate::document::{
    DocumentId, IdGenerator, Revision, SourceError, SyntaxSnapshot, TextEdit, TextRange, TextSize,
    TextSnapshot,
};

use super::delta::DocumentUpdate;
use super::reparse::reparse;

pub struct DocumentSession {
    current: SyntaxSnapshot,
    ids: IdGenerator,
    config: ParseConfig,
    // Retained across ownership handoffs even when the source revision is unchanged.
    interpretation: u64,
}

impl DocumentSession {
    pub fn new(source: &str, config: ParseConfig) -> Self {
        Self::new_with_document(DocumentId(1), source, config)
    }

    pub fn new_with_document(document: DocumentId, source: &str, config: ParseConfig) -> Self {
        let source = TextSnapshot::new(document, Revision(0), source)
            .expect("session source must fit the document text range");
        let mut ids = IdGenerator::new();
        let current = parse_canonical_document_with_ids(source, config, &mut ids);
        Self {
            current,
            ids,
            config,
            interpretation: 0,
        }
    }

    pub fn apply_edits(&mut self, edits: &[TextEdit]) -> DocumentUpdate {
        self.try_apply_edits(edits)
            .expect("document edits must be sorted, non-overlapping UTF-8 ranges")
    }

    pub fn try_apply_edits(&mut self, edits: &[TextEdit]) -> Result<DocumentUpdate, SourceError> {
        let old_revision = self.current.revision;
        if edits.is_empty() {
            return Ok(DocumentUpdate {
                old_revision,
                new_revision: old_revision,
                changed_range: TextRange::empty(TextSize::ZERO),
                reparsed_roots: alloc::vec![],
                reused_roots: alloc::vec![],
                diagnostics: Default::default(),
                stats: Default::default(),
            });
        }
        let result = reparse(&self.current, edits, self.config, &mut self.ids)?;
        let new_revision = result.snapshot.revision;
        let changed_range = super::ChangeMap::new(edits).new_changed_range();
        self.current = result.snapshot;
        Ok(DocumentUpdate {
            old_revision,
            new_revision,
            changed_range,
            reparsed_roots: result.reparsed_roots,
            reused_roots: result.reused_roots,
            diagnostics: result.diagnostics,
            stats: result.stats,
        })
    }

    /// Start a fresh retained interpretation without changing the source revision.
    pub fn into_stream(self) -> crate::document::DocumentStream {
        crate::document::DocumentStream::from_session(
            self.current.source,
            self.ids,
            self.config,
            self.interpretation,
        )
    }
    pub(crate) fn from_stream_parts(
        current: SyntaxSnapshot,
        ids: IdGenerator,
        config: ParseConfig,
        interpretation: u64,
    ) -> Self {
        Self {
            current,
            ids,
            config,
            interpretation,
        }
    }

    pub fn snapshot(&self) -> &SyntaxSnapshot {
        &self.current
    }
}
