use crate::document::{
    AstNode, AtomLiteralSyntax, EmptyLiteralSyntax, NumberSyntax, StringLiteralSyntax, SyntaxKind,
    SyntaxNode, SyntaxToken,
};

use super::{KindAnnotationSyntax, child, direct_token};

recursive_ast_node!(LiteralSyntax, Literal);

#[derive(Clone, Debug)]
pub enum LiteralValueSyntax {
    Empty(EmptyLiteralSyntax),
    Atom(AtomLiteralSyntax),
    String(StringLiteralSyntax),
    Number(NumberSyntax),
    KindAnnotation(KindAnnotationSyntax),
}

impl AstNode for LiteralValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::EmptyLiteral
                | SyntaxKind::AtomLiteral
                | SyntaxKind::StringLiteral
                | SyntaxKind::Number
                | SyntaxKind::KindAnnotation
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::EmptyLiteral => EmptyLiteralSyntax::cast(syntax).map(Self::Empty),
            SyntaxKind::AtomLiteral => AtomLiteralSyntax::cast(syntax).map(Self::Atom),
            SyntaxKind::StringLiteral => StringLiteralSyntax::cast(syntax).map(Self::String),
            SyntaxKind::Number => NumberSyntax::cast(syntax).map(Self::Number),
            SyntaxKind::KindAnnotation => {
                KindAnnotationSyntax::cast(syntax).map(Self::KindAnnotation)
            }
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Empty(value) => value.syntax(),
            Self::Atom(value) => value.syntax(),
            Self::String(value) => value.syntax(),
            Self::Number(value) => value.syntax(),
            Self::KindAnnotation(value) => value.syntax(),
        }
    }
}

impl LiteralSyntax {
    pub fn value(&self) -> Option<LiteralValueSyntax> {
        child(&self.0)
    }
    pub fn true_token(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::True, 0)
    }
    pub fn false_token(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::False, 0)
    }

    pub fn annotation(&self) -> Option<KindAnnotationSyntax> {
        let mut annotations = self.0.children().filter_map(KindAnnotationSyntax::cast);
        if matches!(self.value(), Some(LiteralValueSyntax::KindAnnotation(_))) {
            annotations.nth(1)
        } else {
            annotations.next()
        }
    }
}
