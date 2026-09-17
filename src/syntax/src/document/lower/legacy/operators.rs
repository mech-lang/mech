//! Compatibility lowering for the closed Phase 2D operator layer.

use alloc::string::String;

use mech_core::nodes::{
    AddSubOp, ComparisonOp, FormulaOperator, LogicOp, MulDivOp, PowerOp, RangeOp, SetOp, TableOp,
    VecOp,
};

use crate::document::ast::operators::{
    AddSubOperatorSyntax, ComparisonOperatorSyntax, LogicOperatorSyntax, MatrixOperatorSyntax,
    MulDivOperatorSyntax, OperatorSyntax, PowerOperatorSyntax, RangeOperatorSyntax,
    SetOperatorSyntax, TableOperatorSyntax,
};
use crate::document::{AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode};

use super::common;

/// The legacy value associated with a node-valued Phase 2D operator leaf.
///
/// This remains package-private so direct parity tests can compare exact
/// legacy values without adding a public lowerer for each individual leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyOperatorValue {
    AddSub(AddSubOp),
    MulDiv(MulDivOp),
    Power(PowerOp),
    Matrix(VecOp),
    Range(RangeOp),
    Comparison(ComparisonOp),
    Logic(LogicOp),
    Table(TableOp),
    Set(SetOp),
}

/// Lower the selected `add-sub-operator` alternative.
pub fn lower_legacy_add_sub_operator(
    syntax: &AddSubOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::AddSubOperator,
        "add-sub-operator",
        OperatorFamily::AddSub,
    )? {
        LegacyOperatorValue::AddSub(operator) => Ok(FormulaOperator::AddSub(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "add-sub-operator"),
    }
}

/// Lower the selected `mul-div-operator` alternative.
pub fn lower_legacy_mul_div_operator(
    syntax: &MulDivOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::MulDivOperator,
        "mul-div-operator",
        OperatorFamily::MulDiv,
    )? {
        LegacyOperatorValue::MulDiv(operator) => Ok(FormulaOperator::MulDiv(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "mul-div-operator"),
    }
}

/// Lower the selected `power-operator` alternative.
pub fn lower_legacy_power_operator(
    syntax: &PowerOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::PowerOperator,
        "power-operator",
        OperatorFamily::Power,
    )? {
        LegacyOperatorValue::Power(operator) => Ok(FormulaOperator::Power(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "power-operator"),
    }
}

/// Lower the selected `matrix-operator` alternative.
pub fn lower_legacy_matrix_operator(
    syntax: &MatrixOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::MatrixOperator,
        "matrix-operator",
        OperatorFamily::Matrix,
    )? {
        LegacyOperatorValue::Matrix(operator) => Ok(FormulaOperator::Vec(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "matrix-operator"),
    }
}

/// Lower the selected `range-operator` alternative.
pub fn lower_legacy_range_operator(
    syntax: &RangeOperatorSyntax,
) -> Result<RangeOp, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::RangeOperator,
        "range-operator",
        OperatorFamily::Range,
    )? {
        LegacyOperatorValue::Range(operator) => Ok(operator),
        _ => incompatible_aggregate(syntax.syntax(), "range-operator"),
    }
}

/// Lower the selected `comparison-operator` alternative.
pub fn lower_legacy_comparison_operator(
    syntax: &ComparisonOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::ComparisonOperator,
        "comparison-operator",
        OperatorFamily::Comparison,
    )? {
        LegacyOperatorValue::Comparison(operator) => Ok(FormulaOperator::Comparison(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "comparison-operator"),
    }
}

/// Lower the selected `logic-operator` alternative.
pub fn lower_legacy_logic_operator(
    syntax: &LogicOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::LogicOperator,
        "logic-operator",
        OperatorFamily::Logic,
    )? {
        LegacyOperatorValue::Logic(operator) => Ok(FormulaOperator::Logic(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "logic-operator"),
    }
}

/// Lower the selected `table-operator` alternative.
pub fn lower_legacy_table_operator(
    syntax: &TableOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::TableOperator,
        "table-operator",
        OperatorFamily::Table,
    )? {
        LegacyOperatorValue::Table(operator) => Ok(FormulaOperator::Table(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "table-operator"),
    }
}

/// Lower the selected `set-operator` alternative.
pub fn lower_legacy_set_operator(
    syntax: &SetOperatorSyntax,
) -> Result<FormulaOperator, DiagnosticStore> {
    match lower_aggregate(
        syntax.syntax(),
        SyntaxKind::SetOperator,
        "set-operator",
        OperatorFamily::Set,
    )? {
        LegacyOperatorValue::Set(operator) => Ok(FormulaOperator::Set(operator)),
        _ => incompatible_aggregate(syntax.syntax(), "set-operator"),
    }
}

/// Lower a direct Phase 2D operator leaf to its exact legacy enum value.
pub(crate) fn lower_phase_2d_operator_value(
    syntax: &OperatorSyntax,
) -> Result<LegacyOperatorValue, DiagnosticStore> {
    lower_operator_value(syntax.syntax()).map_err(|message| {
        common::failure_store(syntax.syntax(), "lowering/invalid-operator-syntax", message)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OperatorFamily {
    AddSub,
    MulDiv,
    Power,
    Matrix,
    Range,
    Comparison,
    Logic,
    Table,
    Set,
}

impl OperatorFamily {
    fn accepts(self, value: &LegacyOperatorValue) -> bool {
        matches!(
            (self, value),
            (Self::AddSub, LegacyOperatorValue::AddSub(_))
                | (Self::MulDiv, LegacyOperatorValue::MulDiv(_))
                | (Self::Power, LegacyOperatorValue::Power(_))
                | (Self::Matrix, LegacyOperatorValue::Matrix(_))
                | (Self::Range, LegacyOperatorValue::Range(_))
                | (Self::Comparison, LegacyOperatorValue::Comparison(_))
                | (Self::Logic, LegacyOperatorValue::Logic(_))
                | (Self::Table, LegacyOperatorValue::Table(_))
                | (Self::Set, LegacyOperatorValue::Set(_))
        )
    }
}

fn lower_aggregate(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    family: OperatorFamily,
) -> Result<LegacyOperatorValue, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        let selected = selected_operator(syntax, name)?;
        let value = lower_operator_value(selected.syntax())?;
        if !family.accepts(&value) {
            return Err(alloc::format!(
                "{name} syntax selected an operator from another family"
            ));
        }
        Ok(value)
    })();
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-operator-syntax", message)
    })
}

fn selected_operator(syntax: &SyntaxNode, name: &str) -> Result<OperatorSyntax, String> {
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Node(node)] = elements.as_slice() else {
        return Err(alloc::format!(
            "{name} syntax requires exactly one selected operator child"
        ));
    };
    OperatorSyntax::cast(node.clone())
        .ok_or_else(|| alloc::format!("{name} syntax has an unsupported selected operator child"))
}

fn lower_operator_value(syntax: &SyntaxNode) -> Result<LegacyOperatorValue, String> {
    common::validate_clean_node(syntax, "Phase 2D operator")?;
    match syntax.kind() {
        SyntaxKind::AddOperation => Ok(LegacyOperatorValue::AddSub(AddSubOp::Add)),
        SyntaxKind::SubtractOperation
        | SyntaxKind::RawSubtractOperation
        | SyntaxKind::SpacedSubtractOperation => Ok(LegacyOperatorValue::AddSub(AddSubOp::Sub)),

        SyntaxKind::MultiplyOperation => Ok(LegacyOperatorValue::MulDiv(MulDivOp::Mul)),
        SyntaxKind::DivideOperation => Ok(LegacyOperatorValue::MulDiv(MulDivOp::Div)),
        SyntaxKind::ModulusOperation => Ok(LegacyOperatorValue::MulDiv(MulDivOp::Mod)),

        SyntaxKind::PowerOperation => Ok(LegacyOperatorValue::Power(PowerOp::Pow)),

        SyntaxKind::MatrixMultiplyOperation => Ok(LegacyOperatorValue::Matrix(VecOp::MatMul)),
        SyntaxKind::MatrixSolveOperation => Ok(LegacyOperatorValue::Matrix(VecOp::Solve)),
        SyntaxKind::DotProductOperation => Ok(LegacyOperatorValue::Matrix(VecOp::Dot)),
        SyntaxKind::CrossProductOperation => Ok(LegacyOperatorValue::Matrix(VecOp::Cross)),

        SyntaxKind::RangeInclusiveOperation => Ok(LegacyOperatorValue::Range(RangeOp::Inclusive)),
        SyntaxKind::RangeExclusiveOperation => Ok(LegacyOperatorValue::Range(RangeOp::Exclusive)),

        SyntaxKind::NotEqualOperation => {
            Ok(LegacyOperatorValue::Comparison(ComparisonOp::NotEqual))
        }
        SyntaxKind::EqualToOperation => Ok(LegacyOperatorValue::Comparison(ComparisonOp::Equal)),
        SyntaxKind::StrictNotEqualOperation => Ok(LegacyOperatorValue::Comparison(
            ComparisonOp::StrictNotEqual,
        )),
        SyntaxKind::StrictEqualOperation => {
            Ok(LegacyOperatorValue::Comparison(ComparisonOp::StrictEqual))
        }
        SyntaxKind::GreaterThanOperation => {
            Ok(LegacyOperatorValue::Comparison(ComparisonOp::GreaterThan))
        }
        SyntaxKind::LessThanOperation => {
            Ok(LegacyOperatorValue::Comparison(ComparisonOp::LessThan))
        }
        SyntaxKind::GreaterThanEqualOperation => Ok(LegacyOperatorValue::Comparison(
            ComparisonOp::GreaterThanEqual,
        )),
        SyntaxKind::LessThanEqualOperation => {
            Ok(LegacyOperatorValue::Comparison(ComparisonOp::LessThanEqual))
        }

        SyntaxKind::OrOperation => Ok(LegacyOperatorValue::Logic(LogicOp::Or)),
        SyntaxKind::AndOperation => Ok(LegacyOperatorValue::Logic(LogicOp::And)),
        SyntaxKind::NotOperation => Ok(LegacyOperatorValue::Logic(LogicOp::Not)),
        SyntaxKind::XorOperation => Ok(LegacyOperatorValue::Logic(LogicOp::Xor)),

        SyntaxKind::JoinOperation => Ok(LegacyOperatorValue::Table(TableOp::InnerJoin)),
        SyntaxKind::LeftJoinOperation => Ok(LegacyOperatorValue::Table(TableOp::LeftOuterJoin)),
        SyntaxKind::RightJoinOperation => Ok(LegacyOperatorValue::Table(TableOp::RightOuterJoin)),
        SyntaxKind::FullJoinOperation => Ok(LegacyOperatorValue::Table(TableOp::FullOuterJoin)),
        SyntaxKind::LeftSemiJoinOperation => Ok(LegacyOperatorValue::Table(TableOp::LeftSemiJoin)),
        SyntaxKind::LeftAntiJoinOperation => Ok(LegacyOperatorValue::Table(TableOp::LeftAntiJoin)),

        SyntaxKind::UnionOperation => Ok(LegacyOperatorValue::Set(SetOp::Union)),
        SyntaxKind::IntersectionOperation => Ok(LegacyOperatorValue::Set(SetOp::Intersection)),
        SyntaxKind::DifferenceOperation => Ok(LegacyOperatorValue::Set(SetOp::Difference)),
        SyntaxKind::ComplementOperation => Ok(LegacyOperatorValue::Set(SetOp::Complement)),
        SyntaxKind::SubsetOperation => Ok(LegacyOperatorValue::Set(SetOp::Subset)),
        SyntaxKind::SupersetOperation => Ok(LegacyOperatorValue::Set(SetOp::Superset)),
        SyntaxKind::ProperSubsetOperation => Ok(LegacyOperatorValue::Set(SetOp::ProperSubset)),
        SyntaxKind::ProperSupersetOperation => Ok(LegacyOperatorValue::Set(SetOp::ProperSuperset)),
        SyntaxKind::ElementOfOperation => Ok(LegacyOperatorValue::Set(SetOp::ElementOf)),
        SyntaxKind::NotElementOfOperation => Ok(LegacyOperatorValue::Set(SetOp::NotElementOf)),
        SyntaxKind::SymmetricDifferenceOperation => {
            Ok(LegacyOperatorValue::Set(SetOp::SymmetricDifference))
        }

        _ => Err(String::from(
            "expected a direct node-valued Phase 2D operator production",
        )),
    }
}

fn incompatible_aggregate<T>(syntax: &SyntaxNode, name: &str) -> Result<T, DiagnosticStore> {
    Err(common::failure_store(
        syntax,
        "lowering/invalid-operator-syntax",
        alloc::format!("{name} syntax selected an incompatible operator"),
    ))
}
