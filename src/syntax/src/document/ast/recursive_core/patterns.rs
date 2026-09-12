use alloc::vec::Vec;

use crate::document::{
    AstNode, ExpressionSyntax, IdentifierSyntax, SyntaxKind, SyntaxNode, SyntaxToken,
    WildcardPatternSyntax,
};

use super::{child, children, direct_token};

recursive_ast_node!(PatternSyntax, Pattern);
recursive_ast_node!(ArrayPatternSyntax, ArrayPattern);
recursive_ast_node!(ArrayPatternElementSyntax, ArrayPatternElement);
recursive_ast_node!(AtomStructPatternSyntax, AtomStructPattern);
recursive_ast_node!(TuplePatternSyntax, TuplePattern);
recursive_ast_node!(TupleStructPatternSyntax, TupleStructPattern);

/// The transparent `pattern-array-item` rule is exactly a `Pattern` node.
pub type PatternArrayItemSyntax = PatternSyntax;

#[derive(Clone, Debug)]
pub enum PatternValueSyntax {
    AtomStruct(AtomStructPatternSyntax),
    TupleStruct(TupleStructPatternSyntax),
    Wildcard(WildcardPatternSyntax),
    Array(ArrayPatternSyntax),
    Tuple(TuplePatternSyntax),
    Expression(ExpressionSyntax),
}

impl AstNode for PatternValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::AtomStructPattern
                | SyntaxKind::TupleStructPattern
                | SyntaxKind::WildcardPattern
                | SyntaxKind::ArrayPattern
                | SyntaxKind::TuplePattern
                | SyntaxKind::Expression
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::AtomStructPattern => {
                AtomStructPatternSyntax::cast(syntax).map(Self::AtomStruct)
            }
            SyntaxKind::TupleStructPattern => {
                TupleStructPatternSyntax::cast(syntax).map(Self::TupleStruct)
            }
            SyntaxKind::WildcardPattern => WildcardPatternSyntax::cast(syntax).map(Self::Wildcard),
            SyntaxKind::ArrayPattern => ArrayPatternSyntax::cast(syntax).map(Self::Array),
            SyntaxKind::TuplePattern => TuplePatternSyntax::cast(syntax).map(Self::Tuple),
            SyntaxKind::Expression => ExpressionSyntax::cast(syntax).map(Self::Expression),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::AtomStruct(value) => value.syntax(),
            Self::TupleStruct(value) => value.syntax(),
            Self::Wildcard(value) => value.syntax(),
            Self::Array(value) => value.syntax(),
            Self::Tuple(value) => value.syntax(),
            Self::Expression(value) => value.syntax(),
        }
    }
}

impl PatternSyntax {
    pub fn value(&self) -> Option<PatternValueSyntax> {
        child(&self.0)
    }
}

impl ArrayPatternSyntax {
    pub fn opening_bracket(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBracket, 0)
    }
    pub fn elements(&self) -> Vec<ArrayPatternElementSyntax> {
        children(&self.0)
    }
    pub fn closing_bracket(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBracket, 0)
    }
}

impl ArrayPatternElementSyntax {
    pub fn pattern(&self) -> Option<PatternSyntax> {
        child(&self.0)
    }
    pub fn spread(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::SpreadOperator, 0)
    }
    pub fn rest(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Semicolon, 0)
    }
}

macro_rules! tuple_pattern_accessors {
    ($name:ident) => {
        impl $name {
            pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::LeftParen, 0)
            }
            pub fn items(&self) -> Vec<PatternSyntax> {
                children(&self.0)
            }
            pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::RightParen, 0)
            }
        }
    };
}

tuple_pattern_accessors!(TuplePatternSyntax);
tuple_pattern_accessors!(AtomStructPatternSyntax);
tuple_pattern_accessors!(TupleStructPatternSyntax);

impl AtomStructPatternSyntax {
    pub fn prefix(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Colon, 0)
    }
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}

impl TupleStructPatternSyntax {
    pub fn prefix(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Grave, 0)
    }
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}
