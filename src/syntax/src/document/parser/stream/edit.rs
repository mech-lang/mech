use super::*;
use crate::document::{DocumentSession, TextEdit};

impl DocumentStream {
    /// Invalidate all suspended grammar/materialization state and restart on the
    /// edited source. The session work limit remains cumulative. Node identities
    /// from the previous interpretation remain valid only in historical views.
    pub fn apply_edits(
        &mut self,
        edits: &[TextEdit],
        allowance: u64,
    ) -> Result<StreamUpdate, StreamError> {
        if self.state == StreamState::Cancelled {
            return Err(StreamError::Closed(self.state));
        }
        if edits.is_empty() {
            return Ok(self.unchanged());
        }
        // Validate against the untracked public source, then reject capacity
        // before allocating pieces or scanning inserted text for its line index.
        self.source
            .validate_edits(edits)
            .map_err(StreamError::Source)?;
        let mut new_len = u64::from(self.source.byte_len().0);
        for edit in edits {
            new_len = new_len.saturating_sub(u64::from(edit.delete.len().0));
        }
        for edit in edits {
            new_len = new_len.saturating_add(edit.insert.len() as u64);
        }
        if new_len > u64::from(self.limits.max_source_bytes) {
            return Err(StreamError::SourceLimit);
        }
        let source = self
            .parser_source
            .apply_edits(edits)
            .map_err(StreamError::Source)?;
        self.work.full_document_restarts += 1;
        // Arbitrary edits rebuild source descriptors and the line index. The
        // implementation may traverse old descriptors once per edit range, so
        // conservatively charge that multiplicity rather than presenting a
        // multi-edit rebuild as a single linear pass. This is separate from the
        // measured append allocation path.
        let descriptors = self
            .source
            .piece_count()
            .saturating_add(source.piece_count())
            .saturating_add(self.source.line_index().line_count())
            .saturating_add(source.line_index().line_count())
            .saturating_add(edits.len()) as u64;
        let edit_passes = edits.len().saturating_add(1) as u64;
        self.work.edit_restart_work =
            self.work
                .edit_restart_work
                .saturating_add(u64::from(source.byte_len().0).saturating_add(
                    64_u64.saturating_mul(descriptors.saturating_mul(edit_passes)),
                ));
        self.work_base = self.work;
        self.source = source.without_lookup_work();
        self.parser_source = source;
        let mut parser = Parser::new(
            &self.parser_source,
            LexicalMode::CanonicalSourceFragment,
            self.config,
            &mut self.ids,
        );
        parser.set_resource_rule(rules::PARSE);
        self.parser = Some(parser.suspend());
        self.continuation = Continuation::document_root();
        self.state = StreamState::Open;
        self.progress = StreamProgress::NeedsProcessing;
        self.final_snapshot = None;
        self.invalidate();
        let progress = self.process(allowance);
        // The complete old event/diagnostic interpretations are invalidated.
        let mut update = self.publish(progress, 0);
        update.syntax.from = 0;
        update.diagnostics.from = 0;
        Ok(update)
    }
    pub(crate) fn from_session(
        source: TextSnapshot,
        ids: IdGenerator,
        config: ParseConfig,
        interpretation: u64,
    ) -> Self {
        let mut stream = Self::from_source(source, config, StreamLimits::default());
        stream.ids = ids;
        stream.interpretation = interpretation;
        // This fresh parse issues new nodes, including without any source edit.
        stream.invalidate();
        stream.work.full_document_restarts = 1;
        stream.progress = StreamProgress::NeedsProcessing;
        stream
    }
    /// Transfer a finalized document into ordinary editing ownership. Full
    /// snapshot/index materialization is explicit and included in returned work.
    pub fn into_session(mut self) -> Result<(DocumentSession, StreamWork), StreamError> {
        let snapshot = self.materialize()?;
        self.final_snapshot = None;
        let snapshot = match Arc::try_unwrap(snapshot) {
            Ok(snapshot) => snapshot,
            Err(snapshot) => {
                self.work.export_work += snapshot.nodes.node_count() as u64
                    + snapshot.nodes.token_count() as u64
                    + snapshot.restarts.as_slice().len() as u64
                    + snapshot
                        .diagnostics
                        .iter()
                        .map(diagnostic_clone_work)
                        .sum::<u64>();
                snapshot.as_ref().clone()
            }
        };
        Ok((
            DocumentSession::from_stream_parts(
                snapshot,
                self.ids,
                self.config,
                self.interpretation,
            ),
            self.work,
        ))
    }
}

// Charge each cloned record/vector element and all owned UTF-8 payload bytes.
fn diagnostic_clone_work(diagnostic: &crate::document::Diagnostic) -> u64 {
    use crate::document::{ExpectedSyntax, RecoveryAction};
    fn expected(value: &ExpectedSyntax) -> usize {
        match value {
            ExpectedSyntax::Token(_) => 0,
            ExpectedSyntax::Production(text) => text.len(),
        }
    }
    let mut work = 1
        + diagnostic.code.0.len()
        + diagnostic.message.len()
        + diagnostic.labels.len()
        + diagnostic.expected.len()
        + diagnostic.fixes.len()
        + diagnostic.related.len();
    work += diagnostic
        .labels
        .iter()
        .map(|label| label.message.len())
        .sum::<usize>();
    work += diagnostic.expected.iter().map(expected).sum::<usize>();
    work += diagnostic
        .found
        .as_ref()
        .and_then(|found| found.text.as_ref())
        .map_or(0, |text| text.len());
    for fix in &diagnostic.fixes {
        work += fix.title.len() + fix.edits.len();
        work += fix
            .edits
            .iter()
            .map(|edit| edit.insert.len())
            .sum::<usize>();
    }
    if let Some(RecoveryAction::Insert { syntax, .. }) = &diagnostic.recovery {
        work += expected(syntax);
    }
    work as u64
}
