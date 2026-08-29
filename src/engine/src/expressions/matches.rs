#[cfg(feature = "enum")]
use super::MatchNonExhaustiveError;
#[cfg(feature = "enum")]
use super::MatchNonExhaustiveVariantsError;
use super::{
    Environment, InvalidGuardExpressionError, MatchArmKindMismatchError, MatchNoArmMatchedError,
    expression, expression_cell,
};
#[cfg(feature = "matrix")]
use crate::CannotConvertToTypeError;
#[cfg(feature = "enum")]
use crate::Literal;
#[cfg(feature = "enum")]
use crate::hash_str;
use crate::{
    Expression, FunctionValueRepresentation, InterpreterExecution, MResult, MatchArm,
    MatchExpression, MechError, Pattern, SchemaBody, SpecializationInput, Token, ValueCell,
    ValueCellSnapshotFailure, ValueData,
};
use mech_core::snapshot::SequenceView;
#[cfg(feature = "matrix")]
use mech_core::snapshot::{OptionDraft, ValueDataDraft};
#[cfg(feature = "enum")]
use std::collections::HashSet;

pub fn match_expression(
    match_expr: &MatchExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let source = expression(&match_expr.source, env, p)?;
    let mut base_env = env.cloned().unwrap_or_default();
    let SpecializationInput::Cell(detached_source) = source else {
        if let Some(arm) = match_expr
            .arms
            .iter()
            .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
        {
            if arm
                .guard
                .as_ref()
                .map(|guard| guard_expression_true(guard, &base_env, p))
                .transpose()?
                .unwrap_or(true)
            {
                return expression_cell(&arm.expression, Some(&base_env), p);
            }
        }
        return Err(MechError::new(MatchNoArmMatchedError, None)
            .with_compiler_loc()
            .with_tokens(match_expr.source.tokens()));
    };
    if let Expression::Var(var) = &match_expr.source {
        base_env.insert(var.name.hash(), detached_source.clone());
    }
    if !match_expr
        .arms
        .iter()
        .any(|arm| matches!(arm.pattern, Pattern::Wildcard))
    {
        #[cfg(feature = "enum")]
        if let Some((enum_name, missing_patterns)) =
            infer_missing_enum_match_patterns(match_expr, &detached_source, p)
        {
            if missing_patterns.is_empty() {
                // Exhaustive enum matches do not require a wildcard arm.
                validate_match_arm_output_kinds(match_expr, &base_env, p)?;
            } else {
                return Err(MechError::new(
                    MatchNonExhaustiveVariantsError {
                        enum_name,
                        missing_patterns,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
            }
        } else {
            return Err(MechError::new(MatchNonExhaustiveError, None)
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
        }
    }
    if value_contains_empty(&detached_source)? && !has_identity_wildcard_coalesce_arms(match_expr) {
        if let Some(arm) = match_expr
            .arms
            .iter()
            .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
        {
            let passed_guard = match &arm.guard {
                Some(guard) => guard_expression_true(guard, &base_env, p)?,
                None => true,
            };
            if passed_guard {
                return expression_cell(&arm.expression, Some(&base_env), p);
            }
        }
    }

    for (arm_ix, arm) in match_expr.arms.iter().enumerate() {
        let mut guard_env = base_env.clone();
        let matched = match &arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(
                &arm.pattern,
                &detached_source,
                &mut guard_env,
                p,
            )?,
        };
        if !matched {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &guard_env, p)?,
            None => true,
        };
        if passed_guard {
            #[cfg(feature = "matrix")]
            if value_contains_empty(&detached_source)? && is_identity_option_matrix_arm(arm) {
                if let Some(wildcard_arm) = match_expr
                    .arms
                    .iter()
                    .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
                {
                    let wildcard_passed = match &wildcard_arm.guard {
                        Some(guard) => guard_expression_true(guard, &base_env, p)?,
                        None => true,
                    };
                    if wildcard_passed {
                        let fallback =
                            expression_cell(&wildcard_arm.expression, Some(&base_env), p)?;
                        let coalesced =
                            coalesce_option_matrix_with_fallback(&detached_source, &fallback)?;
                        return Ok(coalesced);
                    }
                }
            }
            let output = expression_cell(&arm.expression, Some(&guard_env), p)?;
            match_validate_arm_kinds(
                match_expr,
                arm_ix,
                output.representation(),
                &detached_source,
                &base_env,
                p,
            )?;
            return Ok(output);
        }
    }

    Err(MechError::new(MatchNoArmMatchedError, None)
        .with_compiler_loc()
        .with_tokens(match_expr.source.tokens()))
}

#[cfg(feature = "enum")]
fn infer_missing_enum_match_patterns(
    match_expr: &MatchExpression,
    source: &ValueCell,
    p: &InterpreterExecution<'_>,
) -> Option<(String, Vec<String>)> {
    let SchemaBody::Enum { key, variants } = source.closed_schema_body().ok()? else {
        return None;
    };

    let mut arm_tags: HashSet<u64> = HashSet::new();
    for arm in &match_expr.arms {
        match &arm.pattern {
            Pattern::Expression(Expression::Literal(Literal::Atom(atom))) => {
                arm_tags.insert(atom.name.hash());
            }
            #[cfg(feature = "atom")]
            Pattern::TupleStruct(pattern_tuple_struct) => {
                arm_tags.insert(pattern_tuple_struct.name.hash());
            }
            _ => {}
        }
    }
    if arm_tags.is_empty() {
        return None;
    }

    let state = p.state.borrow();
    let enum_def = state.enums.values().find(|definition| {
        matches!(
            crate::structures::enum_schema(definition),
            Ok(SchemaBody::Enum { key: candidate, .. }) if candidate == key
        )
    })?;
    let missing_patterns = variants
        .iter()
        .filter(|variant| !arm_tags.contains(&hash_str(&variant.name)))
        .map(|variant| {
            if variant.payload.is_some() {
                format!(":{}(…)", variant.name)
            } else {
                format!(":{}", variant.name)
            }
        })
        .collect::<Vec<String>>();
    Some((enum_def.name.clone(), missing_patterns))
}

fn match_validate_arm_kinds(
    match_expr: &MatchExpression,
    matched_arm_ix: usize,
    matched_kind: FunctionValueRepresentation,
    source: &ValueCell,
    base_env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    for (ix, arm) in match_expr.arms.iter().enumerate() {
        if ix == matched_arm_ix {
            continue;
        }
        if matches!(arm.pattern, Pattern::Wildcard) {
            continue;
        }
        let mut arm_env = base_env.clone();
        let applicable = match arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(&arm.pattern, source, &mut arm_env, p)?,
        };
        if !applicable {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &arm_env, p)?,
            None => true,
        };
        if !passed_guard {
            continue;
        }
        let arm_value = expression_cell(&arm.expression, Some(&arm_env), p)?;
        let arm_kind = arm_value.representation();
        if arm_kind != matched_kind {
            return Err(MechError::new(
                MatchArmKindMismatchError {
                    expected: matched_kind,
                    found: arm_kind,
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(arm.expression.tokens()));
        }
    }
    Ok(())
}

#[cfg(feature = "enum")]
fn validate_match_arm_output_kinds(
    match_expr: &MatchExpression,
    env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    let mut expected: Option<FunctionValueRepresentation> = None;
    for arm in &match_expr.arms {
        let arm_kind = match expression_cell(&arm.expression, Some(env), p) {
            Ok(value) => value.representation(),
            Err(_) => continue,
        };
        if let Some(expected_kind) = &expected {
            if *expected_kind != arm_kind {
                return Err(MechError::new(
                    MatchArmKindMismatchError {
                        expected: expected_kind.clone(),
                        found: arm_kind,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(arm.expression.tokens()));
            }
        } else {
            expected = Some(arm_kind);
        }
    }
    Ok(())
}

fn guard_expression_true(
    guard: &Expression,
    env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<bool> {
    let guard_result = expression_cell(guard, Some(env), p)?;
    let flag = validate_guard_expression_result(guard_result, guard.tokens())?;
    Ok(matches!(flag.snapshot()?.data(), ValueData::Bool(true)))
}

pub(crate) fn validate_guard_expression_result(
    guard_result: ValueCell,
    tokens: Vec<Token>,
) -> MResult<ValueCell> {
    if matches!(guard_result.snapshot()?.data(), ValueData::Bool(_)) {
        Ok(guard_result)
    } else {
        Err(MechError::new(
            InvalidGuardExpressionError {
                found: guard_result.representation(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(tokens))
    }
}

fn is_identity_option_matrix_arm(arm: &MatchArm) -> bool {
    match (&arm.pattern, &arm.expression) {
        (Pattern::Expression(Expression::Var(pattern_var)), Expression::Var(expr_var)) => {
            pattern_var.name.hash() == expr_var.name.hash()
        }
        _ => false,
    }
}

fn has_identity_wildcard_coalesce_arms(match_expr: &MatchExpression) -> bool {
    let has_identity = match_expr.arms.iter().any(is_identity_option_matrix_arm);
    let has_wildcard = match_expr
        .arms
        .iter()
        .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
    has_identity && has_wildcard
}

#[cfg(feature = "matrix")]
fn coalesce_option_matrix_with_fallback(
    source: &ValueCell,
    fallback: &ValueCell,
) -> MResult<ValueCell> {
    let fallback_schema = fallback.closed_schema_body()?;
    let fallback_draft = fallback
        .snapshot()?
        .canonical_data_draft()
        .map_err(|error| {
            MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
        })?;
    match source.closed_schema_body()? {
        SchemaBody::Option(inner) => {
            if fallback_schema != *inner {
                return Err(MechError::new(
                    CannotConvertToTypeError {
                        target_type: "option element schema",
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let draft = source.snapshot()?.canonical_data_draft().map_err(|error| {
                MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
            })?;
            let ValueDataDraft::Option(option) = draft else {
                unreachable!("validated option schema retains option data")
            };
            ValueCell::from_schema_data(
                *inner,
                option.value.map(|value| *value).unwrap_or(fallback_draft),
            )
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let SchemaBody::Option(inner) = *element else {
                return Ok(source.clone());
            };
            if fallback_schema != *inner {
                return Err(MechError::new(
                    CannotConvertToTypeError {
                        target_type: "option matrix element schema",
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let draft = source.snapshot()?.canonical_data_draft().map_err(|error| {
                MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
            })?;
            let ValueDataDraft::Matrix(values) = draft else {
                unreachable!("validated matrix schema retains matrix data")
            };
            let values = values
                .into_vec()
                .into_iter()
                .map(|value| match value {
                    ValueDataDraft::Option(OptionDraft { value, .. }) => Ok(value
                        .map(|value| *value)
                        .unwrap_or_else(|| fallback_draft.clone())),
                    _ => Err(MechError::new(
                        CannotConvertToTypeError {
                            target_type: "option matrix element",
                        },
                        None,
                    )
                    .with_compiler_loc()),
                })
                .collect::<MResult<Vec<_>>>()?;
            let concrete = dimensions
                .iter()
                .map(|dimension| match dimension {
                    crate::DimensionExpr::Constant(value) => Ok(*value),
                    _ => unreachable!("closed matrix schema has concrete dimensions"),
                })
                .collect::<MResult<Vec<_>>>()?;
            ValueCell::dynamic_matrix(
                *inner,
                concrete.into_boxed_slice(),
                values.into_boxed_slice(),
            )
        }
        _ => Ok(source.clone()),
    }
}

fn value_contains_empty(value: &ValueCell) -> MResult<bool> {
    fn contains(data: &ValueData) -> bool {
        match data {
            ValueData::Option(None) => true,
            ValueData::Option(Some(value)) => contains(value),
            ValueData::Tuple(values) => values.iter().any(contains),
            ValueData::Matrix(matrix) => match matrix.elements() {
                SequenceView::Values(values) => values.iter().any(contains),
                _ => false,
            },
            _ => false,
        }
    }
    Ok(contains(value.snapshot()?.data()))
}
