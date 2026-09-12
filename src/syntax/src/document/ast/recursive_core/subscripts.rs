use alloc::vec::Vec;

use crate::document::{
    AstNode, DotSubscriptIntSyntax, DotSubscriptSyntax, IdentifierSyntax,
    PrefixedContextPathSyntax, SelectAllSubscriptSyntax, SubscriptPrimitiveSyntax,
    SwizzleSubscriptSyntax, SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::{FormulaSyntax, RangeExpressionSyntax, child, children, direct_token};

recursive_ast_node!(SliceSyntax, Slice);
recursive_ast_node!(SubscriptListSyntax, SubscriptList);
recursive_ast_node!(BracketSubscriptSyntax, BracketSubscript);
recursive_ast_node!(BraceSubscriptSyntax, BraceSubscript);
recursive_ast_node!(FormulaSubscriptSyntax, FormulaSubscript);
recursive_ast_node!(RangeSubscriptSyntax, RangeSubscript);

#[derive(Clone, Debug)]
pub enum SliceStemSyntax {
    Identifier(IdentifierSyntax),
    Context(PrefixedContextPathSyntax),
}

impl AstNode for SliceStemSyntax {
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

#[derive(Clone, Debug)]
pub enum SubscriptItemSyntax {
    SelectAll(SelectAllSubscriptSyntax),
    Swizzle(SwizzleSubscriptSyntax),
    Dot(DotSubscriptSyntax),
    DotInteger(DotSubscriptIntSyntax),
    Bracket(BracketSubscriptSyntax),
    Brace(BraceSubscriptSyntax),
}

impl AstNode for SubscriptItemSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        SubscriptPrimitiveSyntax::can_cast(kind)
            || matches!(
                kind,
                SyntaxKind::BracketSubscript | SyntaxKind::BraceSubscript
            )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::SelectAllSubscript => {
                SelectAllSubscriptSyntax::cast(syntax).map(Self::SelectAll)
            }
            SyntaxKind::SwizzleSubscript => SwizzleSubscriptSyntax::cast(syntax).map(Self::Swizzle),
            SyntaxKind::DotSubscript => DotSubscriptSyntax::cast(syntax).map(Self::Dot),
            SyntaxKind::DotSubscriptInt => {
                DotSubscriptIntSyntax::cast(syntax).map(Self::DotInteger)
            }
            SyntaxKind::BracketSubscript => BracketSubscriptSyntax::cast(syntax).map(Self::Bracket),
            SyntaxKind::BraceSubscript => BraceSubscriptSyntax::cast(syntax).map(Self::Brace),
            _ => None,
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::SelectAll(value) => value.syntax(),
            Self::Swizzle(value) => value.syntax(),
            Self::Dot(value) => value.syntax(),
            Self::DotInteger(value) => value.syntax(),
            Self::Bracket(value) => value.syntax(),
            Self::Brace(value) => value.syntax(),
        }
    }
}

impl SliceSyntax {
    pub fn stem(&self) -> Option<SliceStemSyntax> {
        child(&self.0)
    }
    pub fn subscripts(&self) -> Option<SubscriptListSyntax> {
        child(&self.0)
    }
}

impl SubscriptListSyntax {
    pub fn items(&self) -> Vec<SubscriptItemSyntax> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub enum SubscriptValueSyntax {
    SelectAll(SelectAllSubscriptSyntax),
    Range(RangeSubscriptSyntax),
    Formula(FormulaSubscriptSyntax),
}

impl AstNode for SubscriptValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::SelectAllSubscript
                | SyntaxKind::RangeSubscript
                | SyntaxKind::FormulaSubscript
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::SelectAllSubscript => {
                SelectAllSubscriptSyntax::cast(syntax).map(Self::SelectAll)
            }
            SyntaxKind::RangeSubscript => RangeSubscriptSyntax::cast(syntax).map(Self::Range),
            SyntaxKind::FormulaSubscript => FormulaSubscriptSyntax::cast(syntax).map(Self::Formula),
            _ => None,
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::SelectAll(value) => value.syntax(),
            Self::Range(value) => value.syntax(),
            Self::Formula(value) => value.syntax(),
        }
    }
}

macro_rules! delimited_subscript {
    ($name:ident, $open:ident, $close:ident) => {
        impl $name {
            pub fn opening_delimiter(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::$open, 0)
            }
            pub fn values(&self) -> Vec<SubscriptValueSyntax> {
                children(&self.0)
            }
            pub fn closing_delimiter(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::$close, 0)
            }
        }
    };
}

delimited_subscript!(BracketSubscriptSyntax, LeftBracket, RightBracket);
delimited_subscript!(BraceSubscriptSyntax, LeftBrace, RightBrace);

impl FormulaSubscriptSyntax {
    pub fn formula(&self) -> Option<FormulaSyntax> {
        child(&self.0)
    }
}

impl RangeSubscriptSyntax {
    pub fn range(&self) -> Option<RangeExpressionSyntax> {
        child(&self.0)
    }
}
