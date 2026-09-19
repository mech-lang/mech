//! Typed syntax views for the closed Phase 2G pattern primitives.

use crate::document::{AstNode, SyntaxKind, SyntaxNode};

/// The direct wildcard pattern leaf. Spread remains transparent and therefore
/// has no wrapper syntax node.
#[derive(Clone, Debug)]
pub struct WildcardPatternSyntax(pub(crate) SyntaxNode);

impl AstNode for WildcardPatternSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::WildcardPattern
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        Self::can_cast(syntax.kind()).then_some(Self(syntax))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}
