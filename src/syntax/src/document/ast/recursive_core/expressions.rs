use alloc::vec::Vec;

use crate::document::{AstNode, ExpressionSyntax, IdentifierSyntax, SyntaxKind, SyntaxNode};

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
    Application(ApplicationExpressionSyntax),
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
            SyntaxKind::ApplicationExpression
                | SyntaxKind::FsmPipe
                | SyntaxKind::SetComprehension
                | SyntaxKind::MatrixComprehension
                | SyntaxKind::RangeExpression
        ) || FormulaSyntax::can_cast(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::ApplicationExpression => {
                ApplicationExpressionSyntax::cast(syntax).map(Self::Application)
            }
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
            Self::Application(value) => value.syntax(),
            Self::FsmPipe(value) => value.syntax(),
            Self::SetComprehension(value) => value.syntax(),
            Self::MatrixComprehension(value) => value.syntax(),
            Self::Range(value) => value.syntax(),
            Self::Formula(value) => value.syntax(),
        }
    }
}

recursive_ast_node!(ApplicationExpressionSyntax, ApplicationExpression);
recursive_ast_node!(ApplicationStageSyntax, ApplicationStage);
recursive_ast_node!(ApplicationOperatorSyntax, ApplicationOperator);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationCountError {
    Invalid,
    Overflow,
}

impl ApplicationExpressionSyntax {
    pub fn seed(&self) -> Option<ExpressionBodySyntax> {
        child(self.syntax())
    }

    pub fn stages(&self) -> Vec<ApplicationStageSyntax> {
        children(self.syntax())
    }
}

impl ApplicationStageSyntax {
    pub fn function(&self) -> Option<IdentifierSyntax> {
        child(self.syntax())
    }

    pub fn operator(&self) -> Option<ApplicationOperatorSyntax> {
        child(self.syntax())
    }

    /// None is ordinary `|>`. A counted stage carries the exact checked u64;
    /// no source normalization or function-name interpretation is involved.
    pub fn count(&self) -> Result<Option<u64>, ApplicationCountError> {
        let operator = self.operator().ok_or(ApplicationCountError::Invalid)?;
        let text = operator
            .syntax()
            .text()
            .map_err(|_| ApplicationCountError::Invalid)?;
        let digits = text
            .strip_prefix('|')
            .and_then(|text| text.strip_suffix('>'))
            .ok_or(ApplicationCountError::Invalid)?;
        if digits.is_empty() {
            return Ok(None);
        }
        let mut value = 0u64;
        for digit in digits.bytes() {
            if !digit.is_ascii_digit() {
                return Err(ApplicationCountError::Invalid);
            }
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(digit - b'0')))
                .ok_or(ApplicationCountError::Overflow)?;
        }
        Ok(Some(value))
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
