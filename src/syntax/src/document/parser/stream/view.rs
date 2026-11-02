use super::*;

/// An immutable view of retained canonical events and accepted source. Open
/// grammar frames remain open; finite-prefix recovery is an explicit preview.
/// This representation is the parser's event stream, not another syntax model.
#[derive(Clone, Debug)]
pub struct StreamView {
    pub identity: StreamIdentity,
    pub source: TextSnapshot,
    pub parsed_through: TextSize,
    pub pending_source: TextRange,
    pub(crate) events: Journal<Event>,
    pub(super) diagnostics: Journal<PendingDiagnostic>,
}
impl StreamView {
    pub fn kind(&self) -> SyntaxKind {
        SyntaxKind::Document
    }
    pub fn completed_node(&self, event: usize) -> Option<crate::document::SyntaxNode> {
        self.completed_node_with_work(event).0
    }
    /// Return a completed node and the actual retained-journal lookup steps.
    /// This caller-requested work never mutates the live parser counters.
    pub fn completed_node_with_work(
        &self,
        event: usize,
    ) -> (Option<crate::document::SyntaxNode>, u64) {
        let (entry, steps) = self.events.get_with_work(event);
        let node = match entry {
            Some(Event::Start {
                cached: Some(cached),
                ..
            }) => Some(crate::document::SyntaxNode::new_root_at(
                cached.node.clone(),
                self.source.clone(),
                cached.range.start,
            )),
            _ => None,
        };
        (node, steps + 1)
    }
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn event(&self, index: usize) -> Option<&Event> {
        self.events.get(index)
    }
    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }
    pub fn events_from(&self, index: usize) -> impl Iterator<Item = &Event> {
        self.events.iter_range(index..self.events.len())
    }
    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.len()
    }
    /// Export only the requested diagnostic. Absolute anchors bind to this view,
    /// so appending never rewrites all historical diagnostic records.
    pub fn diagnostic(&self, index: usize) -> Option<Diagnostic> {
        let mut diagnostic = self.diagnostics.get(index)?.diagnostic.clone();
        rebind(&mut diagnostic, self.identity.revision);
        Some(diagnostic)
    }
}
pub(super) fn rebind(diagnostic: &mut Diagnostic, revision: Revision) {
    for anchor in core::iter::once(&mut diagnostic.primary)
        .chain(diagnostic.labels.iter_mut().map(|label| &mut label.anchor))
    {
        if let DiagnosticAnchor::Absolute { revision: old, .. } = anchor {
            *old = revision;
        }
    }
}
