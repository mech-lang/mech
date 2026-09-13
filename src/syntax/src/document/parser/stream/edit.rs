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
        let source = self
            .source
            .apply_edits(edits)
            .map_err(StreamError::Source)?;
        if source.byte_len().0 > self.limits.max_source_bytes {
            return Err(StreamError::SourceLimit);
        }
        self.work.full_document_restarts += 1;
        // Arbitrary edits rebuild source descriptors and the line index. Charge
        // a documented bound: bytes plus at most 64 storage steps per piece or
        // line entry. This is separate from measured append allocations.
        self.work.edit_restart_work += u64::from(source.byte_len().0)
            + 64 * (self.source.piece_count()
                + source.piece_count()
                + source.line_index().line_count()) as u64;
        self.work_base = self.work;
        self.source = source;
        let mut parser = Parser::new(
            &self.source,
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
    ) -> Self {
        let mut stream = Self::from_source(source, config, StreamLimits::default());
        stream.ids = ids;
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
                self.work.export_work +=
                    snapshot.nodes.node_count() as u64 + snapshot.nodes.token_count() as u64;
                snapshot.as_ref().clone()
            }
        };
        Ok((
            DocumentSession::from_stream_parts(snapshot, self.ids, self.config),
            self.work,
        ))
    }
}
