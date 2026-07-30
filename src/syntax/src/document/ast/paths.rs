use alloc::vec::Vec;

use crate::document::red::{AstNode, SyntaxNode, SyntaxToken};
use crate::document::SyntaxKind;

macro_rules! path_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

/// Typed view of the existing Phase 2A path-segment node.
path_ast_node!(IdentifierPathSegmentSyntax, IdentifierPathSegment);
path_ast_node!(ContextAddressPathSyntax, ContextAddressPath);
path_ast_node!(PrefixedContextPathSyntax, PrefixedContextPath);

impl ContextAddressPathSyntax {
    /// Return the direct lexical path tokens in their physical source order.
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl PrefixedContextPathSyntax {
    pub fn context(&self) -> Option<IdentifierPathSegmentSyntax> {
        self.0.children().find_map(IdentifierPathSegmentSyntax::cast)
    }

    pub fn address(&self) -> Option<ContextAddressPathSyntax> {
        self.0.children().find_map(ContextAddressPathSyntax::cast)
    }
}
