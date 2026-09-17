use alloc::vec::Vec;

use crate::document::{AstNode, ExpressionSyntax, IdentifierSyntax, SyntaxKind, SyntaxToken};

use super::{child, children, direct_token};

recursive_ast_node!(ArgumentListSyntax, ArgumentList);
recursive_ast_node!(CallArgumentSyntax, CallArgument);
recursive_ast_node!(BoundCallArgumentSyntax, BoundCallArgument);
recursive_ast_node!(FunctionCallSyntax, FunctionCall);

#[derive(Clone, Debug)]
pub enum AnyCallArgumentSyntax {
    Positional(CallArgumentSyntax),
    Bound(BoundCallArgumentSyntax),
}

impl AstNode for AnyCallArgumentSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::CallArgument | SyntaxKind::BoundCallArgument
        )
    }

    fn cast(syntax: crate::document::SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::CallArgument => CallArgumentSyntax::cast(syntax).map(Self::Positional),
            SyntaxKind::BoundCallArgument => BoundCallArgumentSyntax::cast(syntax).map(Self::Bound),
            _ => None,
        }
    }

    fn syntax(&self) -> &crate::document::SyntaxNode {
        match self {
            Self::Positional(value) => value.syntax(),
            Self::Bound(value) => value.syntax(),
        }
    }
}

impl ArgumentListSyntax {
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }

    pub fn arguments(&self) -> Vec<AnyCallArgumentSyntax> {
        children(&self.0)
    }

    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}

impl CallArgumentSyntax {
    pub fn value(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

impl BoundCallArgumentSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }

    pub fn colon(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Colon, 0)
    }

    pub fn value(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

impl FunctionCallSyntax {
    pub fn function(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }

    pub fn arguments(&self) -> Option<ArgumentListSyntax> {
        child(&self.0)
    }
}
