use alloc::vec::Vec;

use crate::document::{AstNode, ExpressionSyntax, SyntaxKind, SyntaxNode};

use super::{
    AdditiveExpressionSyntax, ComparisonExpressionSyntax, FactorSyntax, FsmPipeSyntax,
    LogicExpressionSyntax, MatchArmSyntax, MatrixComprehensionSyntax,
    MultiplicativeExpressionSyntax, PowerExpressionSyntax, RangeExpressionSyntax,
    SetComprehensionSyntax, SetExpressionSyntax, TableExpressionSyntax, child, children,
};

/// The transparent `formula` rule viewed as the physical precedence node it emitted.
#[derive(Clone, Debug)]
pub enum FormulaSyntax {
    Logic(LogicExpressionSyntax),
    Comparison(ComparisonExpressionSyntax),
    Additive(AdditiveExpressionSyntax),
    Multiplicative(MultiplicativeExpressionSyntax),
    Power(PowerExpressionSyntax),
    Table(TableExpressionSyntax),
    Set(SetExpressionSyntax),
    Factor(FactorSyntax),
}

impl AstNode for FormulaSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LogicExpression
                | SyntaxKind::ComparisonExpression
                | SyntaxKind::AdditiveExpression
                | SyntaxKind::MultiplicativeExpression
                | SyntaxKind::PowerExpression
                | SyntaxKind::TableExpression
                | SyntaxKind::SetExpression
                | SyntaxKind::Factor
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::LogicExpression => LogicExpressionSyntax::cast(syntax).map(Self::Logic),
            SyntaxKind::ComparisonExpression => {
                ComparisonExpressionSyntax::cast(syntax).map(Self::Comparison)
            }
            SyntaxKind::AdditiveExpression => {
                AdditiveExpressionSyntax::cast(syntax).map(Self::Additive)
            }
            SyntaxKind::MultiplicativeExpression => {
                MultiplicativeExpressionSyntax::cast(syntax).map(Self::Multiplicative)
            }
            SyntaxKind::PowerExpression => PowerExpressionSyntax::cast(syntax).map(Self::Power),
            SyntaxKind::TableExpression => TableExpressionSyntax::cast(syntax).map(Self::Table),
            SyntaxKind::SetExpression => SetExpressionSyntax::cast(syntax).map(Self::Set),
            SyntaxKind::Factor => FactorSyntax::cast(syntax).map(Self::Factor),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Logic(value) => value.syntax(),
            Self::Comparison(value) => value.syntax(),
            Self::Additive(value) => value.syntax(),
            Self::Multiplicative(value) => value.syntax(),
            Self::Power(value) => value.syntax(),
            Self::Table(value) => value.syntax(),
            Self::Set(value) => value.syntax(),
            Self::Factor(value) => value.syntax(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExpressionBodySyntax {
    FsmPipe(FsmPipeSyntax),
    SetComprehension(SetComprehensionSyntax),
    MatrixComprehension(MatrixComprehensionSyntax),
    Range(RangeExpressionSyntax),
    Formula(FormulaSyntax),
}

impl AstNode for ExpressionBodySyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::FsmPipe
                | SyntaxKind::SetComprehension
                | SyntaxKind::MatrixComprehension
                | SyntaxKind::RangeExpression
        ) || FormulaSyntax::can_cast(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::FsmPipe => FsmPipeSyntax::cast(syntax).map(Self::FsmPipe),
            SyntaxKind::SetComprehension => {
                SetComprehensionSyntax::cast(syntax).map(Self::SetComprehension)
            }
            SyntaxKind::MatrixComprehension => {
                MatrixComprehensionSyntax::cast(syntax).map(Self::MatrixComprehension)
            }
            SyntaxKind::RangeExpression => RangeExpressionSyntax::cast(syntax).map(Self::Range),
            _ => FormulaSyntax::cast(syntax).map(Self::Formula),
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::FsmPipe(value) => value.syntax(),
            Self::SetComprehension(value) => value.syntax(),
            Self::MatrixComprehension(value) => value.syntax(),
            Self::Range(value) => value.syntax(),
            Self::Formula(value) => value.syntax(),
        }
    }
}

impl ExpressionSyntax {
    pub fn body(&self) -> Option<ExpressionBodySyntax> {
        child(self.syntax()).or_else(|| {
            child::<ExpressionSyntax>(self.syntax()).and_then(|expression| expression.body())
        })
    }

    pub fn match_arms(&self) -> Vec<MatchArmSyntax> {
        children(self.syntax())
    }
}
