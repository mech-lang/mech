use alloc::vec::Vec;

use crate::document::{
    AddSubOperatorSyntax, AstNode, ComparisonOperatorSyntax, ExpressionSyntax, LogicOperatorSyntax,
    MatrixOperatorSyntax, MulDivOperatorSyntax, NotOperationSyntax, PowerOperatorSyntax,
    RangeOperatorSyntax, SetOperatorSyntax, SyntaxKind, SyntaxNode, SyntaxToken,
    TableOperatorSyntax,
};

use super::{
    ExpressionBodySyntax, FormulaSyntax, FunctionCallSyntax, LiteralSyntax,
    MatrixComprehensionSyntax, PatternSyntax, SliceSyntax, StructureSyntax, VariableSyntax, child,
    children, direct_token, selected_child,
};

recursive_ast_node!(FactorSyntax, Factor);
recursive_ast_node!(NegateFactorSyntax, NegateFactor);
recursive_ast_node!(NotFactorSyntax, NotFactor);
recursive_ast_node!(ParentheticalExpressionSyntax, ParentheticalExpression);
recursive_ast_node!(RangeExpressionSyntax, RangeExpression);
recursive_ast_node!(MatchArmSyntax, MatchArm);
recursive_ast_node!(LogicExpressionSyntax, LogicExpression);
recursive_ast_node!(ComparisonExpressionSyntax, ComparisonExpression);
recursive_ast_node!(AdditiveExpressionSyntax, AdditiveExpression);
recursive_ast_node!(MultiplicativeExpressionSyntax, MultiplicativeExpression);
recursive_ast_node!(PowerExpressionSyntax, PowerExpression);
recursive_ast_node!(TableExpressionSyntax, TableExpression);
recursive_ast_node!(SetExpressionSyntax, SetExpression);

#[derive(Clone, Debug)]
pub enum FactorValueSyntax {
    Parenthetical(ParentheticalExpressionSyntax),
    Negate(NegateFactorSyntax),
    Not(NotFactorSyntax),
    Structure(StructureSyntax),
    Literal(LiteralSyntax),
    Call(FunctionCallSyntax),
    MatrixComprehension(MatrixComprehensionSyntax),
    Slice(SliceSyntax),
    Variable(VariableSyntax),
}

impl AstNode for FactorValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::ParentheticalExpression
                | SyntaxKind::NegateFactor
                | SyntaxKind::NotFactor
                | SyntaxKind::Structure
                | SyntaxKind::Literal
                | SyntaxKind::FunctionCall
                | SyntaxKind::MatrixComprehension
                | SyntaxKind::Slice
                | SyntaxKind::Variable
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::ParentheticalExpression => {
                ParentheticalExpressionSyntax::cast(syntax).map(Self::Parenthetical)
            }
            SyntaxKind::NegateFactor => NegateFactorSyntax::cast(syntax).map(Self::Negate),
            SyntaxKind::NotFactor => NotFactorSyntax::cast(syntax).map(Self::Not),
            SyntaxKind::Structure => StructureSyntax::cast(syntax).map(Self::Structure),
            SyntaxKind::Literal => LiteralSyntax::cast(syntax).map(Self::Literal),
            SyntaxKind::FunctionCall => FunctionCallSyntax::cast(syntax).map(Self::Call),
            SyntaxKind::MatrixComprehension => {
                MatrixComprehensionSyntax::cast(syntax).map(Self::MatrixComprehension)
            }
            SyntaxKind::Slice => SliceSyntax::cast(syntax).map(Self::Slice),
            SyntaxKind::Variable => VariableSyntax::cast(syntax).map(Self::Variable),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Parenthetical(value) => value.syntax(),
            Self::Negate(value) => value.syntax(),
            Self::Not(value) => value.syntax(),
            Self::Structure(value) => value.syntax(),
            Self::Literal(value) => value.syntax(),
            Self::Call(value) => value.syntax(),
            Self::MatrixComprehension(value) => value.syntax(),
            Self::Slice(value) => value.syntax(),
            Self::Variable(value) => value.syntax(),
        }
    }
}

impl FactorSyntax {
    pub fn value(&self) -> Option<FactorValueSyntax> {
        selected_child(&self.0)
    }

    pub fn transpose(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Apostrophe, 0)
    }
}

impl ParentheticalExpressionSyntax {
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }
    pub fn expression(&self) -> Option<ExpressionBodySyntax> {
        child::<ExpressionSyntax>(&self.0)
            .and_then(|expression| expression.body())
            .or_else(|| child(&self.0))
    }
    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}

impl NegateFactorSyntax {
    pub fn dash(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Dash, 0)
    }
    pub fn operand(&self) -> Option<FactorSyntax> {
        child(&self.0)
    }
}

impl NotFactorSyntax {
    pub fn operator(&self) -> Option<NotOperationSyntax> {
        child(&self.0)
    }
    pub fn operand(&self) -> Option<FactorSyntax> {
        child(&self.0)
    }
}

macro_rules! chain_accessors {
    ($name:ident, $operator:ty) => {
        impl $name {
            pub fn operands(&self) -> Vec<FormulaSyntax> {
                children(&self.0)
            }
            pub fn operators(&self) -> Vec<$operator> {
                children(&self.0)
            }
        }
    };
}

chain_accessors!(LogicExpressionSyntax, LogicOperatorSyntax);
chain_accessors!(ComparisonExpressionSyntax, ComparisonOperatorSyntax);
chain_accessors!(AdditiveExpressionSyntax, AddSubOperatorSyntax);
chain_accessors!(PowerExpressionSyntax, PowerOperatorSyntax);
chain_accessors!(TableExpressionSyntax, TableOperatorSyntax);
chain_accessors!(SetExpressionSyntax, SetOperatorSyntax);

#[derive(Clone, Debug)]
pub enum MultiplicativeOperatorSyntax {
    Scalar(MulDivOperatorSyntax),
    Matrix(MatrixOperatorSyntax),
}

impl AstNode for MultiplicativeOperatorSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::MulDivOperator | SyntaxKind::MatrixOperator
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::MulDivOperator => MulDivOperatorSyntax::cast(syntax).map(Self::Scalar),
            SyntaxKind::MatrixOperator => MatrixOperatorSyntax::cast(syntax).map(Self::Matrix),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Scalar(value) => value.syntax(),
            Self::Matrix(value) => value.syntax(),
        }
    }
}

impl MultiplicativeExpressionSyntax {
    pub fn operands(&self) -> Vec<FormulaSyntax> {
        children(&self.0)
    }
    pub fn operators(&self) -> Vec<MultiplicativeOperatorSyntax> {
        children(&self.0)
    }
}

impl RangeExpressionSyntax {
    pub fn bounds(&self) -> Vec<FormulaSyntax> {
        children(&self.0)
    }
    pub fn operators(&self) -> Vec<RangeOperatorSyntax> {
        children(&self.0)
    }
}

impl MatchArmSyntax {
    pub fn pattern(&self) -> Option<PatternSyntax> {
        child(&self.0)
    }
    pub fn expressions(&self) -> Vec<crate::document::ExpressionSyntax> {
        children(&self.0)
    }

    pub fn guard(&self) -> Option<crate::document::ExpressionSyntax> {
        self.expressions_around_output().0
    }

    pub fn value(&self) -> Option<crate::document::ExpressionSyntax> {
        self.expressions_around_output().1
    }

    pub fn output_operator(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::OutputOperator, 0)
    }

    fn expressions_around_output(
        &self,
    ) -> (
        Option<crate::document::ExpressionSyntax>,
        Option<crate::document::ExpressionSyntax>,
    ) {
        let Some(output) = self.output_operator() else {
            return (self.expressions().into_iter().next(), None);
        };
        let boundary = output.range().start;
        let mut guard = None;
        let mut value = None;
        for expression in self.expressions() {
            if expression.syntax().range().end <= boundary {
                guard = Some(expression);
            } else {
                value = Some(expression);
                break;
            }
        }
        (guard, value)
    }
}
