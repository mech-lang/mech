//! Typed syntax views for the closed Phase 2D operator productions.

use crate::document::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TokenFlags};

macro_rules! operator_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

/// The semantic meaning represented by a Phase 2D operator node.
///
/// The lossless syntax keeps distinctions such as raw versus spaced
/// subtraction. Both spellings carry the same compatibility meaning here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Power,

    MatrixMultiply,
    MatrixSolve,
    DotProduct,
    CrossProduct,

    RangeInclusive,
    RangeExclusive,

    NotEqual,
    EqualTo,
    StrictNotEqual,
    StrictEqual,
    GreaterThan,
    LessThan,
    GreaterThanEqual,
    LessThanEqual,

    Or,
    And,
    Not,
    Xor,

    InnerJoin,
    LeftOuterJoin,
    RightOuterJoin,
    FullOuterJoin,
    LeftSemiJoin,
    LeftAntiJoin,

    Union,
    Intersection,
    Difference,
    Complement,
    Subset,
    Superset,
    ProperSubset,
    ProperSuperset,
    ElementOf,
    NotElementOf,
    SymmetricDifference,
}

/// A typed view over any Phase 2D node-valued operator production.
#[derive(Clone, Debug)]
pub struct OperatorSyntax(SyntaxNode);

impl AstNode for OperatorSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        is_operator_kind(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        Self::can_cast(syntax.kind()).then_some(Self(syntax))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

operator_ast_node!(AddSubOperatorSyntax, AddSubOperator);
operator_ast_node!(MulDivOperatorSyntax, MulDivOperator);
operator_ast_node!(PowerOperatorSyntax, PowerOperator);
operator_ast_node!(MatrixOperatorSyntax, MatrixOperator);
operator_ast_node!(RangeOperatorSyntax, RangeOperator);
operator_ast_node!(ComparisonOperatorSyntax, ComparisonOperator);
operator_ast_node!(LogicOperatorSyntax, LogicOperator);
operator_ast_node!(TableOperatorSyntax, TableOperator);
operator_ast_node!(SetOperatorSyntax, SetOperator);

operator_ast_node!(AddOperationSyntax, AddOperation);
operator_ast_node!(SubtractOperationSyntax, SubtractOperation);
operator_ast_node!(RawSubtractOperationSyntax, RawSubtractOperation);
operator_ast_node!(SpacedSubtractOperationSyntax, SpacedSubtractOperation);
operator_ast_node!(MultiplyOperationSyntax, MultiplyOperation);
operator_ast_node!(DivideOperationSyntax, DivideOperation);
operator_ast_node!(ModulusOperationSyntax, ModulusOperation);
operator_ast_node!(PowerOperationSyntax, PowerOperation);
operator_ast_node!(MatrixMultiplyOperationSyntax, MatrixMultiplyOperation);
operator_ast_node!(MatrixSolveOperationSyntax, MatrixSolveOperation);
operator_ast_node!(DotProductOperationSyntax, DotProductOperation);
operator_ast_node!(CrossProductOperationSyntax, CrossProductOperation);
operator_ast_node!(RangeInclusiveOperationSyntax, RangeInclusiveOperation);
operator_ast_node!(RangeExclusiveOperationSyntax, RangeExclusiveOperation);
operator_ast_node!(NotEqualOperationSyntax, NotEqualOperation);
operator_ast_node!(EqualToOperationSyntax, EqualToOperation);
operator_ast_node!(StrictNotEqualOperationSyntax, StrictNotEqualOperation);
operator_ast_node!(StrictEqualOperationSyntax, StrictEqualOperation);
operator_ast_node!(GreaterThanOperationSyntax, GreaterThanOperation);
operator_ast_node!(LessThanOperationSyntax, LessThanOperation);
operator_ast_node!(GreaterThanEqualOperationSyntax, GreaterThanEqualOperation);
operator_ast_node!(LessThanEqualOperationSyntax, LessThanEqualOperation);
operator_ast_node!(OrOperationSyntax, OrOperation);
operator_ast_node!(AndOperationSyntax, AndOperation);
operator_ast_node!(NotOperationSyntax, NotOperation);
operator_ast_node!(XorOperationSyntax, XorOperation);
operator_ast_node!(JoinOperationSyntax, JoinOperation);
operator_ast_node!(LeftJoinOperationSyntax, LeftJoinOperation);
operator_ast_node!(RightJoinOperationSyntax, RightJoinOperation);
operator_ast_node!(FullJoinOperationSyntax, FullJoinOperation);
operator_ast_node!(LeftSemiJoinOperationSyntax, LeftSemiJoinOperation);
operator_ast_node!(LeftAntiJoinOperationSyntax, LeftAntiJoinOperation);
operator_ast_node!(UnionOperationSyntax, UnionOperation);
operator_ast_node!(IntersectionOperationSyntax, IntersectionOperation);
operator_ast_node!(DifferenceOperationSyntax, DifferenceOperation);
operator_ast_node!(ComplementOperationSyntax, ComplementOperation);
operator_ast_node!(SubsetOperationSyntax, SubsetOperation);
operator_ast_node!(SupersetOperationSyntax, SupersetOperation);
operator_ast_node!(ProperSubsetOperationSyntax, ProperSubsetOperation);
operator_ast_node!(ProperSupersetOperationSyntax, ProperSupersetOperation);
operator_ast_node!(ElementOfOperationSyntax, ElementOfOperation);
operator_ast_node!(NotElementOfOperationSyntax, NotElementOfOperation);
operator_ast_node!(
    SymmetricDifferenceOperationSyntax,
    SymmetricDifferenceOperation
);

impl OperatorSyntax {
    /// Return the meaning selected by this node's canonical production kind.
    pub fn semantic(&self) -> Option<CanonicalOperator> {
        match self.0.kind() {
            SyntaxKind::AddOperation => Some(CanonicalOperator::Add),
            SyntaxKind::SubtractOperation
            | SyntaxKind::RawSubtractOperation
            | SyntaxKind::SpacedSubtractOperation => Some(CanonicalOperator::Subtract),
            SyntaxKind::MultiplyOperation => Some(CanonicalOperator::Multiply),
            SyntaxKind::DivideOperation => Some(CanonicalOperator::Divide),
            SyntaxKind::ModulusOperation => Some(CanonicalOperator::Modulus),
            SyntaxKind::PowerOperation => Some(CanonicalOperator::Power),
            SyntaxKind::MatrixMultiplyOperation => Some(CanonicalOperator::MatrixMultiply),
            SyntaxKind::MatrixSolveOperation => Some(CanonicalOperator::MatrixSolve),
            SyntaxKind::DotProductOperation => Some(CanonicalOperator::DotProduct),
            SyntaxKind::CrossProductOperation => Some(CanonicalOperator::CrossProduct),
            SyntaxKind::RangeInclusiveOperation => Some(CanonicalOperator::RangeInclusive),
            SyntaxKind::RangeExclusiveOperation => Some(CanonicalOperator::RangeExclusive),
            SyntaxKind::NotEqualOperation => Some(CanonicalOperator::NotEqual),
            SyntaxKind::EqualToOperation => Some(CanonicalOperator::EqualTo),
            SyntaxKind::StrictNotEqualOperation => Some(CanonicalOperator::StrictNotEqual),
            SyntaxKind::StrictEqualOperation => Some(CanonicalOperator::StrictEqual),
            SyntaxKind::GreaterThanOperation => Some(CanonicalOperator::GreaterThan),
            SyntaxKind::LessThanOperation => Some(CanonicalOperator::LessThan),
            SyntaxKind::GreaterThanEqualOperation => Some(CanonicalOperator::GreaterThanEqual),
            SyntaxKind::LessThanEqualOperation => Some(CanonicalOperator::LessThanEqual),
            SyntaxKind::OrOperation => Some(CanonicalOperator::Or),
            SyntaxKind::AndOperation => Some(CanonicalOperator::And),
            SyntaxKind::NotOperation => Some(CanonicalOperator::Not),
            SyntaxKind::XorOperation => Some(CanonicalOperator::Xor),
            SyntaxKind::JoinOperation => Some(CanonicalOperator::InnerJoin),
            SyntaxKind::LeftJoinOperation => Some(CanonicalOperator::LeftOuterJoin),
            SyntaxKind::RightJoinOperation => Some(CanonicalOperator::RightOuterJoin),
            SyntaxKind::FullJoinOperation => Some(CanonicalOperator::FullOuterJoin),
            SyntaxKind::LeftSemiJoinOperation => Some(CanonicalOperator::LeftSemiJoin),
            SyntaxKind::LeftAntiJoinOperation => Some(CanonicalOperator::LeftAntiJoin),
            SyntaxKind::UnionOperation => Some(CanonicalOperator::Union),
            SyntaxKind::IntersectionOperation => Some(CanonicalOperator::Intersection),
            SyntaxKind::DifferenceOperation => Some(CanonicalOperator::Difference),
            SyntaxKind::ComplementOperation => Some(CanonicalOperator::Complement),
            SyntaxKind::SubsetOperation => Some(CanonicalOperator::Subset),
            SyntaxKind::SupersetOperation => Some(CanonicalOperator::Superset),
            SyntaxKind::ProperSubsetOperation => Some(CanonicalOperator::ProperSubset),
            SyntaxKind::ProperSupersetOperation => Some(CanonicalOperator::ProperSuperset),
            SyntaxKind::ElementOfOperation => Some(CanonicalOperator::ElementOf),
            SyntaxKind::NotElementOfOperation => Some(CanonicalOperator::NotElementOf),
            SyntaxKind::SymmetricDifferenceOperation => {
                Some(CanonicalOperator::SymmetricDifference)
            }
            _ if is_aggregate_kind(self.0.kind()) => {
                direct_operator_child(&self.0).and_then(|selected| selected.semantic())
            }
            _ => None,
        }
    }

    /// Return the physical range occupied by the operator spelling.
    ///
    /// Canonical operator leaves retain surrounding horizontal space. This
    /// deliberately omits that spacing while retaining every glyph in a
    /// multi-character ASCII spelling.
    pub fn operator_token_range(&self) -> Option<TextRange> {
        let mut glyphs = self
            .0
            .tokens()
            .into_iter()
            .filter(|token| !is_spacing_token(token));
        let first = glyphs.next()?;
        let last = glyphs.last().unwrap_or_else(|| first.clone());
        Some(TextRange::new(first.range().start, last.range().end))
    }
}

macro_rules! aggregate_selected_view {
    ($name:ident) => {
        impl $name {
            pub fn selected(&self) -> Option<OperatorSyntax> {
                direct_operator_child(&self.0)
            }
        }
    };
}

aggregate_selected_view!(AddSubOperatorSyntax);
aggregate_selected_view!(MulDivOperatorSyntax);
aggregate_selected_view!(PowerOperatorSyntax);
aggregate_selected_view!(MatrixOperatorSyntax);
aggregate_selected_view!(RangeOperatorSyntax);
aggregate_selected_view!(ComparisonOperatorSyntax);
aggregate_selected_view!(LogicOperatorSyntax);
aggregate_selected_view!(TableOperatorSyntax);
aggregate_selected_view!(SetOperatorSyntax);

impl SubtractOperationSyntax {
    pub fn raw(&self) -> Option<RawSubtractOperationSyntax> {
        self.0.children().find_map(RawSubtractOperationSyntax::cast)
    }

    pub fn spaced(&self) -> Option<SpacedSubtractOperationSyntax> {
        self.0
            .children()
            .find_map(SpacedSubtractOperationSyntax::cast)
    }
}

impl SpacedSubtractOperationSyntax {
    pub fn raw(&self) -> Option<RawSubtractOperationSyntax> {
        self.0.children().find_map(RawSubtractOperationSyntax::cast)
    }
}

fn direct_operator_child(syntax: &SyntaxNode) -> Option<OperatorSyntax> {
    syntax.children().find_map(OperatorSyntax::cast)
}

fn is_spacing_token(token: &SyntaxToken) -> bool {
    token.flags().intersects(
        TokenFlags::SYNTHETIC | TokenFlags::MISSING | TokenFlags::ERROR | TokenFlags::TRIVIA,
    ) || matches!(
        token.kind(),
        SyntaxKind::Whitespace | SyntaxKind::Tab | SyntaxKind::Newline | SyntaxKind::CarriageReturn
    )
}

fn is_aggregate_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::AddSubOperator
            | SyntaxKind::MulDivOperator
            | SyntaxKind::PowerOperator
            | SyntaxKind::MatrixOperator
            | SyntaxKind::RangeOperator
            | SyntaxKind::ComparisonOperator
            | SyntaxKind::LogicOperator
            | SyntaxKind::TableOperator
            | SyntaxKind::SetOperator
    )
}

fn is_operator_kind(kind: SyntaxKind) -> bool {
    is_aggregate_kind(kind)
        || matches!(
            kind,
            SyntaxKind::AddOperation
                | SyntaxKind::SubtractOperation
                | SyntaxKind::RawSubtractOperation
                | SyntaxKind::SpacedSubtractOperation
                | SyntaxKind::MultiplyOperation
                | SyntaxKind::DivideOperation
                | SyntaxKind::ModulusOperation
                | SyntaxKind::PowerOperation
                | SyntaxKind::MatrixMultiplyOperation
                | SyntaxKind::MatrixSolveOperation
                | SyntaxKind::DotProductOperation
                | SyntaxKind::CrossProductOperation
                | SyntaxKind::RangeInclusiveOperation
                | SyntaxKind::RangeExclusiveOperation
                | SyntaxKind::NotEqualOperation
                | SyntaxKind::EqualToOperation
                | SyntaxKind::StrictNotEqualOperation
                | SyntaxKind::StrictEqualOperation
                | SyntaxKind::GreaterThanOperation
                | SyntaxKind::LessThanOperation
                | SyntaxKind::GreaterThanEqualOperation
                | SyntaxKind::LessThanEqualOperation
                | SyntaxKind::OrOperation
                | SyntaxKind::AndOperation
                | SyntaxKind::NotOperation
                | SyntaxKind::XorOperation
                | SyntaxKind::JoinOperation
                | SyntaxKind::LeftJoinOperation
                | SyntaxKind::RightJoinOperation
                | SyntaxKind::FullJoinOperation
                | SyntaxKind::LeftSemiJoinOperation
                | SyntaxKind::LeftAntiJoinOperation
                | SyntaxKind::UnionOperation
                | SyntaxKind::IntersectionOperation
                | SyntaxKind::DifferenceOperation
                | SyntaxKind::ComplementOperation
                | SyntaxKind::SubsetOperation
                | SyntaxKind::SupersetOperation
                | SyntaxKind::ProperSubsetOperation
                | SyntaxKind::ProperSupersetOperation
                | SyntaxKind::ElementOfOperation
                | SyntaxKind::NotElementOfOperation
                | SyntaxKind::SymmetricDifferenceOperation
        )
}
