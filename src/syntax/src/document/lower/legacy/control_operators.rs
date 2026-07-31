//! Compatibility lowering for the closed Phase 2G assignment primitives.

use alloc::string::String;

use mech_core::OpAssignOp;

use crate::document::ast::control_operators::{
    AddAssignOperationSyntax, DivAssignOperationSyntax, ExpAssignOperationSyntax,
    MulAssignOperationSyntax, OpAssignOperatorSyntax, OpAssignPrimitiveSyntax,
    SubAssignOperationSyntax,
};
use crate::document::{AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode};

use super::common;

/// The direct legacy values emitted by node-valued Phase 2G control leaves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyControlValue {
    Operator(OpAssignOp),
    Add(OpAssignOp),
    Sub(OpAssignOp),
    Mul(OpAssignOp),
    Div(OpAssignOp),
    Exp(OpAssignOp),
}

/// Lower a canonical assignment aggregate.
pub fn lower_legacy_op_assign_operator(
    syntax: &OpAssignOperatorSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::OpAssignOperator,
        "op-assign-operator",
        lower_aggregate,
    )
}

/// Lower a canonical `+=` leaf.
pub fn lower_legacy_add_assign_operation(
    syntax: &AddAssignOperationSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_leaf_value(
        syntax.syntax(),
        SyntaxKind::AddAssignOperation,
        "+=",
        OpAssignOp::Add,
    )
}

/// Lower a canonical `-=` leaf.
pub fn lower_legacy_sub_assign_operation(
    syntax: &SubAssignOperationSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_leaf_value(
        syntax.syntax(),
        SyntaxKind::SubAssignOperation,
        "-=",
        OpAssignOp::Sub,
    )
}

/// Lower a canonical `*=` leaf.
pub fn lower_legacy_mul_assign_operation(
    syntax: &MulAssignOperationSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_leaf_value(
        syntax.syntax(),
        SyntaxKind::MulAssignOperation,
        "*=",
        OpAssignOp::Mul,
    )
}

/// Lower a canonical `/=` leaf.
pub fn lower_legacy_div_assign_operation(
    syntax: &DivAssignOperationSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_leaf_value(
        syntax.syntax(),
        SyntaxKind::DivAssignOperation,
        "/=",
        OpAssignOp::Div,
    )
}

/// Lower a canonical `^=` leaf.
pub fn lower_legacy_exp_assign_operation(
    syntax: &ExpAssignOperationSyntax,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_leaf_value(
        syntax.syntax(),
        SyntaxKind::ExpAssignOperation,
        "^=",
        OpAssignOp::Exp,
    )
}

/// Lower any node-valued Phase 2G assignment primitive for direct parity
/// coverage without creating a parent statement lowerer.
pub(crate) fn lower_phase_2g_control_value(
    syntax: &OpAssignPrimitiveSyntax,
) -> Result<LegacyControlValue, DiagnosticStore> {
    let lowered = match syntax.syntax().kind() {
        SyntaxKind::OpAssignOperator => {
            lower_aggregate(syntax.syntax()).map(LegacyControlValue::Operator)
        }
        SyntaxKind::AddAssignOperation => {
            lower_assign_leaf(syntax.syntax(), "+=", OpAssignOp::Add).map(LegacyControlValue::Add)
        }
        SyntaxKind::SubAssignOperation => {
            lower_assign_leaf(syntax.syntax(), "-=", OpAssignOp::Sub).map(LegacyControlValue::Sub)
        }
        SyntaxKind::MulAssignOperation => {
            lower_assign_leaf(syntax.syntax(), "*=", OpAssignOp::Mul).map(LegacyControlValue::Mul)
        }
        SyntaxKind::DivAssignOperation => {
            lower_assign_leaf(syntax.syntax(), "/=", OpAssignOp::Div).map(LegacyControlValue::Div)
        }
        SyntaxKind::ExpAssignOperation => {
            lower_assign_leaf(syntax.syntax(), "^=", OpAssignOp::Exp).map(LegacyControlValue::Exp)
        }
        _ => Err(String::from(
            "expected a node-valued Phase 2G assignment primitive",
        )),
    };
    lowered.map_err(|message| {
        common::failure_store(
            syntax.syntax(),
            "lowering/invalid-control-operator-syntax",
            message,
        )
    })
}

fn lower_value(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    lower: impl FnOnce(&SyntaxNode) -> Result<OpAssignOp, String>,
) -> Result<OpAssignOp, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        lower(syntax)
    })();
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-control-operator-syntax", message)
    })
}

fn lower_leaf_value(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    spelling: &str,
    value: OpAssignOp,
) -> Result<OpAssignOp, DiagnosticStore> {
    lower_value(syntax, expected_kind, "assignment operator", |node| {
        lower_assign_leaf(node, spelling, value)
    })
}

fn lower_aggregate(syntax: &SyntaxNode) -> Result<OpAssignOp, String> {
    common::validate_node(syntax, SyntaxKind::OpAssignOperator, "op-assign-operator")?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Node(selected)] = elements.as_slice() else {
        return Err(String::from(
            "op-assign-operator syntax requires exactly one selected leaf",
        ));
    };
    match selected.kind() {
        SyntaxKind::AddAssignOperation => lower_assign_leaf(selected, "+=", OpAssignOp::Add),
        SyntaxKind::SubAssignOperation => lower_assign_leaf(selected, "-=", OpAssignOp::Sub),
        SyntaxKind::MulAssignOperation => lower_assign_leaf(selected, "*=", OpAssignOp::Mul),
        SyntaxKind::DivAssignOperation => lower_assign_leaf(selected, "/=", OpAssignOp::Div),
        SyntaxKind::ExpAssignOperation => lower_assign_leaf(selected, "^=", OpAssignOp::Exp),
        _ => Err(String::from(
            "op-assign-operator syntax selected an unsupported leaf",
        )),
    }
}

fn lower_assign_leaf(
    syntax: &SyntaxNode,
    spelling: &str,
    value: OpAssignOp,
) -> Result<OpAssignOp, String> {
    common::validate_clean_node(syntax, "assignment operator")?;
    let tokens = syntax.tokens();
    let mut spellings = tokens
        .iter()
        .filter(|token| token.kind() == SyntaxKind::Text);
    let Some(token) = spellings.next() else {
        return Err(String::from(
            "assignment operator syntax requires one text token",
        ));
    };
    if spellings.next().is_some()
        || token
            .text()
            .map_err(|_| String::from("cannot read assignment operator token"))?
            != spelling
    {
        return Err(String::from(
            "assignment operator syntax has an invalid text token",
        ));
    }
    Ok(value)
}
