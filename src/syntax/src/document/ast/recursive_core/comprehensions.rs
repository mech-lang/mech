use alloc::vec::Vec;

use crate::document::{AstNode, ExpressionSyntax, SyntaxKind, SyntaxToken, VariableDefineSyntax};

use super::{MatrixColumnSyntax, MatrixRowSyntax, PatternSyntax, child, children, direct_token};

recursive_ast_node!(ComprehensionQualifierSyntax, ComprehensionQualifier);
recursive_ast_node!(GeneratorSyntax, Generator);
recursive_ast_node!(SetComprehensionSyntax, SetComprehension);
recursive_ast_node!(MatrixComprehensionSyntax, MatrixComprehension);

#[derive(Clone, Debug)]
pub enum ComprehensionQualifierValueSyntax {
    Generator(GeneratorSyntax),
    Definition(VariableDefineSyntax),
    Filter(ExpressionSyntax),
}

impl AstNode for ComprehensionQualifierValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::Generator | SyntaxKind::VariableDefine | SyntaxKind::Expression
        )
    }

    fn cast(syntax: crate::document::SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::Generator => GeneratorSyntax::cast(syntax).map(Self::Generator),
            SyntaxKind::VariableDefine => VariableDefineSyntax::cast(syntax).map(Self::Definition),
            SyntaxKind::Expression => ExpressionSyntax::cast(syntax).map(Self::Filter),
            _ => None,
        }
    }

    fn syntax(&self) -> &crate::document::SyntaxNode {
        match self {
            Self::Generator(value) => value.syntax(),
            Self::Definition(value) => value.syntax(),
            Self::Filter(value) => value.syntax(),
        }
    }
}

impl ComprehensionQualifierSyntax {
    pub fn value(&self) -> Option<ComprehensionQualifierValueSyntax> {
        child(&self.0)
    }
}

impl GeneratorSyntax {
    pub fn pattern(&self) -> Option<PatternSyntax> {
        child(&self.0)
    }

    pub fn arrow(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::GeneratorArrow, 0)
    }

    pub fn source(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

// Resource finalization can retain the provisional matrix row/column around
// the comprehension body. Read those exact physical owners in the shared tree.
fn comprehension_body(syntax: &crate::document::SyntaxNode) -> crate::document::SyntaxNode {
    if syntax.kind() == SyntaxKind::MatrixComprehension
        && let Some(row) = child::<MatrixRowSyntax>(syntax)
        && let Some(column) = child::<MatrixColumnSyntax>(row.syntax())
    {
        return column.syntax().clone();
    }
    syntax.clone()
}

macro_rules! comprehension_accessors {
    ($name:ident, $open:ident, $close:ident) => {
        impl $name {
            pub fn opening_delimiter(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::$open, 0)
            }

            pub fn value(&self) -> Option<ExpressionSyntax> {
                child(&comprehension_body(&self.0))
            }

            pub fn bar(&self) -> Option<SyntaxToken> {
                direct_token(&comprehension_body(&self.0), SyntaxKind::Bar, 0)
            }

            pub fn qualifiers(&self) -> Vec<ComprehensionQualifierSyntax> {
                children(&comprehension_body(&self.0))
            }

            pub fn closing_delimiter(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::$close, 0)
                    .or_else(|| direct_token(&comprehension_body(&self.0), SyntaxKind::$close, 0))
            }
        }
    };
}

comprehension_accessors!(SetComprehensionSyntax, LeftBrace, RightBrace);
comprehension_accessors!(MatrixComprehensionSyntax, LeftBracket, RightBracket);
