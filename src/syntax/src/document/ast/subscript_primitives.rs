//! Typed syntax views for the closed Phase 2G subscript primitives.

use crate::document::ast::literals::IntegerLiteralSyntax;
use crate::document::{AstNode, IdentifierSyntax, SyntaxKind, SyntaxNode};

macro_rules! subscript_ast_node {
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

/// A typed view over any node-valued Phase 2G subscript primitive.
#[derive(Clone, Debug)]
pub struct SubscriptPrimitiveSyntax(SyntaxNode);

impl AstNode for SubscriptPrimitiveSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::SelectAllSubscript
                | SyntaxKind::SwizzleSubscript
                | SyntaxKind::DotSubscript
                | SyntaxKind::DotSubscriptInt
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        Self::can_cast(syntax.kind()).then_some(Self(syntax))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

subscript_ast_node!(SelectAllSubscriptSyntax, SelectAllSubscript);
subscript_ast_node!(SwizzleSubscriptSyntax, SwizzleSubscript);
subscript_ast_node!(DotSubscriptSyntax, DotSubscript);
subscript_ast_node!(DotSubscriptIntSyntax, DotSubscriptInt);

impl SwizzleSubscriptSyntax {
    pub fn identifiers(&self) -> impl Iterator<Item = IdentifierSyntax> {
        self.0.children().filter_map(IdentifierSyntax::cast)
    }
}

impl DotSubscriptSyntax {
    pub fn identifier(&self) -> Option<IdentifierSyntax> {
        self.0.children().find_map(IdentifierSyntax::cast)
    }
}

impl DotSubscriptIntSyntax {
    pub fn integer(&self) -> Option<IntegerLiteralSyntax> {
        self.0.children().find_map(IntegerLiteralSyntax::cast)
    }
}
