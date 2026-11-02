//! Canonical document-local ownership shared by execution, indexing and rendering.

use crate::document::{
    AstNode, DocumentId, DocumentSyntax, MikaSectionSyntax, MikaSyntax, NodeId, SectionSyntax,
    SyntaxKind, SyntaxNode,
};
use alloc::vec::Vec;

/// A retained owner identity, scoped to its document's node-identity lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DocumentScopeId {
    pub document: DocumentId,
    pub node: NodeId,
}

impl DocumentScopeId {
    fn of(node: &SyntaxNode) -> Self {
        Self {
            document: node.source().document(),
            node: node.id(),
        }
    }
}

/// A Mika-local scope and its lexical document/Mika parent. Named fences are
/// grouped within each local owner; equal names in separate owners do not merge.
#[derive(Clone, Debug)]
pub struct MikaDocumentScope {
    pub parent: DocumentScopeId,
    pub section: MikaSectionSyntax,
}

impl MikaSyntax {
    pub fn section(&self) -> Option<MikaSectionSyntax> {
        self.syntax().children().find_map(MikaSectionSyntax::cast)
    }
}

impl MikaSectionSyntax {
    pub fn scope_id(&self) -> DocumentScopeId {
        DocumentScopeId::of(self.syntax())
    }
    pub fn body(&self) -> Option<SectionSyntax> {
        self.syntax().children().find_map(SectionSyntax::cast)
    }
}

impl DocumentSyntax {
    pub fn scope_id(&self) -> DocumentScopeId {
        DocumentScopeId::of(self.syntax())
    }

    /// Discover retained Mika owners in physical order, preserving nesting.
    /// Discovery does not authorize execution of recovered syntax.
    pub fn mika_scopes(&self) -> Vec<MikaDocumentScope> {
        let mut scopes = Vec::new();
        let mut pending = alloc::vec![(self.syntax().clone(), self.scope_id())];
        while let Some((node, mut parent)) = pending.pop() {
            if node.kind() == SyntaxKind::InlineMechCode {
                continue;
            }
            if let Some(section) = MikaSectionSyntax::cast(node.clone()) {
                let owner = section.scope_id();
                scopes.push(MikaDocumentScope { parent, section });
                parent = owner;
            }
            let children: Vec<_> = node.children().collect();
            pending.extend(children.into_iter().rev().map(|node| (node, parent)));
        }
        scopes
    }
}
