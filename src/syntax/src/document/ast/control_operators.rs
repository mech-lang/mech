//! Typed syntax views for the closed Phase 2G control-operator primitives.

use crate::document::{AstNode, SyntaxKind, SyntaxNode};

macro_rules! control_ast_node {
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

/// The compatibility meaning selected by an assignment operator leaf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalOpAssign {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
}

/// A typed view over any node-valued Phase 2G assignment primitive.
#[derive(Clone, Debug)]
pub struct OpAssignPrimitiveSyntax(SyntaxNode);

impl AstNode for OpAssignPrimitiveSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::OpAssignOperator
                | SyntaxKind::AddAssignOperation
                | SyntaxKind::SubAssignOperation
                | SyntaxKind::MulAssignOperation
                | SyntaxKind::DivAssignOperation
                | SyntaxKind::ExpAssignOperation
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        Self::can_cast(syntax.kind()).then_some(Self(syntax))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

control_ast_node!(OpAssignOperatorSyntax, OpAssignOperator);
control_ast_node!(AddAssignOperationSyntax, AddAssignOperation);
control_ast_node!(SubAssignOperationSyntax, SubAssignOperation);
control_ast_node!(MulAssignOperationSyntax, MulAssignOperation);
control_ast_node!(DivAssignOperationSyntax, DivAssignOperation);
control_ast_node!(ExpAssignOperationSyntax, ExpAssignOperation);

impl OpAssignPrimitiveSyntax {
    pub fn semantic(&self) -> Option<CanonicalOpAssign> {
        semantic(self.0.kind())
    }
}

impl OpAssignOperatorSyntax {
    pub fn selected(&self) -> Option<OpAssignPrimitiveSyntax> {
        self.0.children().find_map(OpAssignPrimitiveSyntax::cast)
    }
}

fn semantic(kind: SyntaxKind) -> Option<CanonicalOpAssign> {
    match kind {
        SyntaxKind::AddAssignOperation => Some(CanonicalOpAssign::Add),
        SyntaxKind::SubAssignOperation => Some(CanonicalOpAssign::Sub),
        SyntaxKind::MulAssignOperation => Some(CanonicalOpAssign::Mul),
        SyntaxKind::DivAssignOperation => Some(CanonicalOpAssign::Div),
        SyntaxKind::ExpAssignOperation => Some(CanonicalOpAssign::Exp),
        _ => None,
    }
}
