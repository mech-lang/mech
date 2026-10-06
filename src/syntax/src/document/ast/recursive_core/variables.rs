use crate::document::{
    AstNode, IdentifierSyntax, PrefixedContextPathSyntax, SyntaxKind, SyntaxNode, SyntaxToken,
    VariableDefineSyntax,
};

use super::{KindAnnotationSyntax, child, direct_token};

recursive_ast_node!(VariableSyntax, Variable);

#[derive(Clone, Debug)]
pub enum VariableStemSyntax {
    Identifier(IdentifierSyntax),
    Context(PrefixedContextPathSyntax),
}

impl AstNode for VariableStemSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::Identifier | SyntaxKind::PrefixedContextPath
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::Identifier => IdentifierSyntax::cast(syntax).map(Self::Identifier),
            SyntaxKind::PrefixedContextPath => {
                PrefixedContextPathSyntax::cast(syntax).map(Self::Context)
            }
            _ => None,
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Identifier(value) => value.syntax(),
            Self::Context(value) => value.syntax(),
        }
    }
}

impl VariableSyntax {
    pub fn stem(&self) -> Option<VariableStemSyntax> {
        child(&self.0)
    }
    pub fn annotation(&self) -> Option<KindAnnotationSyntax> {
        child(&self.0)
    }
}

impl VariableDefineSyntax {
    pub fn mutability_marker(&self) -> Option<SyntaxToken> {
        direct_token(self.syntax(), SyntaxKind::Tilde, 0)
    }

    pub fn variable(&self) -> Option<VariableSyntax> {
        child(self.syntax())
    }
}
