use crate::document::{NodeId, SyntaxKind, SyntaxSnapshot, TextRange};

use super::ChangeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReparseRoot {
    pub node: NodeId,
    pub kind: SyntaxKind,
    pub range: TextRange,
}

/// Canonical document edits currently use one full-root parse. A smaller
/// restart is eligible only after its canonical enclosing context is proven;
/// prototype fragment rules are not an editing fallback.
pub fn select_reparse_root(snapshot: &SyntaxSnapshot, _changes: &ChangeMap) -> ReparseRoot {
    document_root(snapshot)
}

pub fn parent_supported_root(snapshot: &SyntaxSnapshot, root: ReparseRoot) -> Option<ReparseRoot> {
    (root.node != snapshot.root.id).then(|| document_root(snapshot))
}

fn document_root(snapshot: &SyntaxSnapshot) -> ReparseRoot {
    ReparseRoot {
        node: snapshot.root.id,
        kind: SyntaxKind::Document,
        range: snapshot.source.full_range(),
    }
}
