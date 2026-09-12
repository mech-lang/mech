use alloc::vec::Vec;

use crate::document::{
    AstNode, ExpressionSyntax, IdentifierSyntax, KindAnySyntax, KindAtomSyntax, KindEmptySyntax,
    SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::{LiteralSyntax, RangeExpressionSyntax, child, children, direct_token, nth_child};

recursive_ast_node!(KindSyntax, Kind);
recursive_ast_node!(KindAnnotationSyntax, KindAnnotation);
recursive_ast_node!(KindKindSyntax, KindKind);
recursive_ast_node!(KindMapSyntax, KindMap);
recursive_ast_node!(KindMatrixSyntax, KindMatrix);
recursive_ast_node!(KindRecordSyntax, KindRecord);
recursive_ast_node!(KindScalarSyntax, KindScalar);
recursive_ast_node!(KindSetSyntax, KindSet);
recursive_ast_node!(TableKindSyntax, TableKind);
recursive_ast_node!(KindTupleSyntax, KindTuple);
recursive_ast_node!(KindWithOptionSyntax, KindWithOption);

#[derive(Clone, Debug)]
pub enum KindValueSyntax {
    Any(KindAnySyntax),
    Empty(KindEmptySyntax),
    Atom(KindAtomSyntax),
    Nested(KindKindSyntax),
    Table(TableKindSyntax),
    Set(KindSetSyntax),
    Map(KindMapSyntax),
    Record(KindRecordSyntax),
    Matrix(KindMatrixSyntax),
    Tuple(KindTupleSyntax),
    Scalar(KindScalarSyntax),
}

impl AstNode for KindValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::KindAny
                | SyntaxKind::KindEmpty
                | SyntaxKind::KindAtom
                | SyntaxKind::KindKind
                | SyntaxKind::TableKind
                | SyntaxKind::KindSet
                | SyntaxKind::KindMap
                | SyntaxKind::KindRecord
                | SyntaxKind::KindMatrix
                | SyntaxKind::KindTuple
                | SyntaxKind::KindScalar
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::KindAny => KindAnySyntax::cast(syntax).map(Self::Any),
            SyntaxKind::KindEmpty => KindEmptySyntax::cast(syntax).map(Self::Empty),
            SyntaxKind::KindAtom => KindAtomSyntax::cast(syntax).map(Self::Atom),
            SyntaxKind::KindKind => KindKindSyntax::cast(syntax).map(Self::Nested),
            SyntaxKind::TableKind => TableKindSyntax::cast(syntax).map(Self::Table),
            SyntaxKind::KindSet => KindSetSyntax::cast(syntax).map(Self::Set),
            SyntaxKind::KindMap => KindMapSyntax::cast(syntax).map(Self::Map),
            SyntaxKind::KindRecord => KindRecordSyntax::cast(syntax).map(Self::Record),
            SyntaxKind::KindMatrix => KindMatrixSyntax::cast(syntax).map(Self::Matrix),
            SyntaxKind::KindTuple => KindTupleSyntax::cast(syntax).map(Self::Tuple),
            SyntaxKind::KindScalar => KindScalarSyntax::cast(syntax).map(Self::Scalar),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Any(value) => value.syntax(),
            Self::Empty(value) => value.syntax(),
            Self::Atom(value) => value.syntax(),
            Self::Nested(value) => value.syntax(),
            Self::Table(value) => value.syntax(),
            Self::Set(value) => value.syntax(),
            Self::Map(value) => value.syntax(),
            Self::Record(value) => value.syntax(),
            Self::Matrix(value) => value.syntax(),
            Self::Tuple(value) => value.syntax(),
            Self::Scalar(value) => value.syntax(),
        }
    }
}

impl KindSyntax {
    pub fn value(&self) -> Option<KindValueSyntax> {
        child(&self.0)
    }
}

impl KindWithOptionSyntax {
    pub fn kind(&self) -> Option<KindSyntax> {
        child(&self.0)
    }
    pub fn question_mark(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Question, 0)
    }
}

impl KindAnnotationSyntax {
    pub fn opening_angle(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftAngle, 0)
    }
    pub fn kind(&self) -> Option<KindWithOptionSyntax> {
        child(&self.0)
    }
    pub fn closing_angle(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightAngle, 0)
    }
}

impl KindKindSyntax {
    pub fn opening_angle(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftAngle, 0)
    }
    pub fn kind(&self) -> Option<KindWithOptionSyntax> {
        child(&self.0)
    }
    pub fn closing_angle(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightAngle, 0)
    }
}

impl KindMapSyntax {
    fn role_owner(&self) -> SyntaxNode {
        child::<KindSetSyntax>(&self.0)
            .map(|owner| owner.syntax().clone())
            .unwrap_or_else(|| self.0.clone())
    }

    pub fn opening_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.role_owner(), SyntaxKind::LeftBrace, 0)
    }
    pub fn key(&self) -> Option<KindSyntax> {
        nth_child(&self.role_owner(), 0)
    }
    pub fn colon(&self) -> Option<SyntaxToken> {
        direct_token(&self.role_owner(), SyntaxKind::Colon, 0)
    }
    pub fn value(&self) -> Option<KindSyntax> {
        nth_child(&self.role_owner(), 1)
    }
    pub fn closing_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.role_owner(), SyntaxKind::RightBrace, 0)
    }
}

impl KindSetSyntax {
    pub fn opening_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBrace, 0)
    }
    pub fn element(&self) -> Option<KindSyntax> {
        child(&self.0)
    }
    pub fn literal_constraint(&self) -> Option<LiteralSyntax> {
        child(&self.0)
    }
    pub fn closing_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBrace, 0)
    }
}

impl KindMatrixSyntax {
    pub fn opening_bracket(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBracket, 0)
    }
    pub fn element(&self) -> Option<KindWithOptionSyntax> {
        child(&self.0)
    }
    pub fn dimensions(&self) -> Vec<LiteralSyntax> {
        children(&self.0)
    }
    pub fn closing_bracket(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBracket, 0)
    }
}

impl KindTupleSyntax {
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }
    pub fn items(&self) -> Vec<KindSyntax> {
        children(&self.0)
    }
    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}

impl KindScalarSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
    pub fn constraint(&self) -> Option<RangeExpressionSyntax> {
        child(&self.0)
    }

    /// The partial first constraint bound retained when resource limits stop range recognition.
    pub fn recovered_expression(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

impl KindRecordSyntax {
    pub fn opening_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBrace, 0)
    }
    pub fn fields(&self) -> Vec<IdentifierSyntax> {
        children(&self.0)
    }
    pub fn field_kinds(&self) -> Vec<KindAnnotationSyntax> {
        children(&self.0)
    }
    pub fn closing_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBrace, 0)
    }
}

impl TableKindSyntax {
    pub fn opening_bar(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Bar, 0)
    }
    pub fn field_names(&self) -> Vec<IdentifierSyntax> {
        children(&self.0)
    }
    pub fn field_kinds(&self) -> Vec<KindAnnotationSyntax> {
        children(&self.0)
    }
    pub fn closing_bar(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Bar, 1)
    }
    pub fn constraint(&self) -> Option<LiteralSyntax> {
        child(&self.0)
    }
}
