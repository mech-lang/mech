use super::{
    Environment, InvalidGuardExpressionError, MatchArmKindMismatchError, MatchNoArmMatchedError,
    MatchNonExhaustiveError, MatchNonExhaustiveVariantsError, expression,
};
#[cfg(feature = "matrix")]
use crate::Matrix;
#[cfg(feature = "enum")]
use crate::MechEnum;
use crate::{
    CannotConvertToTypeError, Expression, InterpreterExecution, LegacyValue, Literal, MResult,
    MatchArm, MatchExpression, MechError, Pattern, Ref, Token, ValueKind,
};
#[cfg(feature = "enum")]
use std::collections::HashSet;

pub fn match_expression(
    match_expr: &MatchExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let source = expression(&match_expr.source, env, p)?;
    let detached_source = match &source {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
        _ => source.clone(),
    };
    let mut base_env = env.cloned().unwrap_or_default();
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
    if value_contains_empty(&detached_source) && !has_identity_wildcard_coalesce_arms(match_expr) {
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
                return expression(&arm.expression, Some(&base_env), p);
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
            if value_contains_empty(&detached_source) && is_identity_option_matrix_arm(arm) {
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
                        let fallback = expression(&wildcard_arm.expression, Some(&base_env), p)?;
                        let coalesced =
                            coalesce_option_matrix_with_fallback(&detached_source, &fallback)?;
                        return Ok(coalesced);
                    }
                }
            }
            let output = expression(&arm.expression, Some(&guard_env), p)?;
            match_validate_arm_kinds(
                match_expr,
                arm_ix,
                &output.kind(),
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
    source: &LegacyValue,
    p: &InterpreterExecution<'_>,
) -> Option<(String, Vec<String>)> {
    let (source_enum_id, source_tag) = match source {
        LegacyValue::Enum(enum_value) => {
            let enum_brrw = enum_value.borrow();
            if enum_brrw.variants.len() != 1 {
                (Some(enum_brrw.id), None)
            } else {
                (Some(enum_brrw.id), Some(enum_brrw.variants[0].0))
            }
        }
        LegacyValue::Atom(atom) => (None, Some(atom.borrow().id())),
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(tuple_val) => {
            let tuple_brrw = tuple_val.borrow();
            match tuple_brrw.elements.first() {
                Some(tag) => match tag.as_ref() {
                    LegacyValue::Atom(atom) => (None, Some(atom.borrow().id())),
                    _ => (None, None),
                },
                None => (None, None),
            }
        }
        _ => (None, None),
    };
    let source_tag = source_tag?;

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

    let state_brrw = p.state.borrow();
    let enum_def = if let Some(enum_id) = source_enum_id {
        state_brrw.enums.get(&enum_id)?
    } else {
        let candidates: Vec<&MechEnum> = state_brrw
            .enums
            .values()
            .filter(|enm| {
                let variant_ids: HashSet<u64> = enm.variants.iter().map(|(id, _)| *id).collect();
                variant_ids.contains(&source_tag) && arm_tags.is_subset(&variant_ids)
            })
            .collect();
        if candidates.len() != 1 {
            return None;
        }
        candidates[0]
    };
    let variant_ids: HashSet<u64> = enum_def.variants.iter().map(|(id, _)| *id).collect();
    let missing_ids: Vec<u64> = variant_ids.difference(&arm_tags).cloned().collect();
    let names_brrw = enum_def.names.borrow();
    let missing_patterns = enum_def
        .variants
        .iter()
        .filter(|(id, _)| missing_ids.contains(id))
        .map(|(id, payload_kind)| {
            let variant_name = names_brrw
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            if payload_kind.is_some() {
                format!(":{}(…)", variant_name)
            } else {
                format!(":{}", variant_name)
            }
        })
        .collect::<Vec<String>>();
    Some((enum_def.name(), missing_patterns))
}

fn match_validate_arm_kinds(
    match_expr: &MatchExpression,
    matched_arm_ix: usize,
    matched_kind: &ValueKind,
    source: &LegacyValue,
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
        let arm_value = expression(&arm.expression, Some(&arm_env), p)?;
        let arm_kind = arm_value.kind();
        if arm_kind != *matched_kind {
            return Err(MechError::new(
                MatchArmKindMismatchError {
                    expected: matched_kind.clone(),
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

fn validate_match_arm_output_kinds(
    match_expr: &MatchExpression,
    env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    let mut expected: Option<ValueKind> = None;
    for arm in &match_expr.arms {
        let arm_kind = match expression(&arm.expression, Some(env), p) {
            Ok(value) => value.kind(),
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
    let guard_result = expression(guard, Some(env), p)?;
    let flag = validate_guard_expression_result(guard_result, guard.tokens())?;
    let result = *flag.borrow();
    Ok(result)
}

pub(crate) fn validate_guard_expression_result(
    guard_result: LegacyValue,
    tokens: Vec<Token>,
) -> MResult<Ref<bool>> {
    match guard_result {
        #[cfg(feature = "bool")]
        LegacyValue::Bool(flag) => Ok(flag),
        _ => Err(MechError::new(
            InvalidGuardExpressionError {
                found: guard_result.kind(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(tokens)),
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
    source: &LegacyValue,
    fallback: &LegacyValue,
) -> MResult<LegacyValue> {
    let source_kind = source.kind();
    if let ValueKind::Option(inner_kind) = source_kind.clone() {
        let raw = match source {
            LegacyValue::Typed(inner, _) => inner.as_ref().clone(),
            value => value.clone(),
        };
        let candidate = match raw {
            LegacyValue::Empty | LegacyValue::EmptyKind(_) => fallback.clone(),
            value => value,
        };
        return candidate.convert_to(inner_kind.as_ref()).ok_or_else(|| {
            MechError::new(
                CannotConvertToTypeError {
                    target_type: "requested type",
                },
                None,
            )
            .with_compiler_loc()
        });
    }
    let (inner_kind, shape) = match source_kind {
        ValueKind::Matrix(element_kind, shape) => match *element_kind {
            ValueKind::Option(inner) => (*inner, shape),
            _ => return Ok(source.clone()),
        },
        _ => return Ok(source.clone()),
    };
    let values = match crate::patterns::matrix_like_values(source) {
        Some(values) => values,
        None => return Ok(source.clone()),
    };
    let fill_value = fallback.convert_to(&inner_kind).ok_or_else(|| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "requested type",
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let converted_values = values
        .into_iter()
        .map(|value| {
            let raw = match value {
                LegacyValue::Empty | LegacyValue::EmptyKind(_) => fill_value.clone(),
                other => other,
            };
            raw.convert_to(&inner_kind).ok_or_else(|| {
                MechError::new(
                    CannotConvertToTypeError {
                        target_type: "requested type",
                    },
                    None,
                )
                .with_compiler_loc()
            })
        })
        .collect::<MResult<Vec<LegacyValue>>>()?;
    Ok(LegacyValue::MatrixValue(Matrix::from_vec(
        converted_values,
        shape[0],
        shape[1],
    )))
}

fn value_contains_empty(value: &LegacyValue) -> bool {
    match value {
        LegacyValue::Empty | LegacyValue::EmptyKind(_) => true,
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(matrix) => matrix
            .as_vec()
            .iter()
            .any(|value| value_contains_empty(value)),
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(tuple) => tuple
            .borrow()
            .elements
            .iter()
            .any(|value| value_contains_empty(value.as_ref())),
        LegacyValue::Typed(value, _) => value_contains_empty(value),
        LegacyValue::MutableReference(reference) => value_contains_empty(&reference.borrow()),
        _ => false,
    }
}
