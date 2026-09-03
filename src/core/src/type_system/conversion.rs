//! Explicit semantic conversion and numeric-promotion planning.

use super::{BuiltinScalarKind, ResolvedType, TypeConstraintFailure, TypeResolutionError};
use crate::{KindExpr, MechErrorKind};
use core::fmt::{self, Display, Formatter};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::ToString, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::ToString, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeRelation {
    ExactEquality,
    PermittedConversion,
    NumericPromotion,
    ExplicitCast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionMode {
    Implicit,
    Explicit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionPlan {
    pub source: ResolvedType,
    pub target: ResolvedType,
    pub step: ConversionStep,
    pub cost: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionStep {
    Identity,
    Scalar(ScalarConversion),
    MatrixElements(Box<ConversionPlan>),
    OptionPayload(Box<ConversionPlan>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarConversion {
    Builtin {
        source: BuiltinScalarKind,
        target: BuiltinScalarKind,
        mode: ConversionMode,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlan {
    pub result: ResolvedType,
    pub left: ConversionPlan,
    pub right: ConversionPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionExecutionError {
    ConversionOutOfRange,
    ConversionNonFinite,
    ConversionImaginaryPartNonZero,
    ConversionShapeMismatch,
    ConversionPlanSourceMismatch,
    ConversionExecutionUnsupported,
}

impl Display for ConversionExecutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConversionOutOfRange => "conversion result is outside the target range",
            Self::ConversionNonFinite => "conversion requires a finite value",
            Self::ConversionImaginaryPartNonZero => {
                "complex-to-real conversion requires a zero imaginary component"
            }
            Self::ConversionShapeMismatch => "conversion source and target shapes differ",
            Self::ConversionPlanSourceMismatch => {
                "conversion plan does not match the live source type"
            }
            Self::ConversionExecutionUnsupported => {
                "the resolved conversion has no execution implementation"
            }
        })
    }
}

impl MechErrorKind for ConversionExecutionError {
    fn name(&self) -> &str {
        match self {
            Self::ConversionOutOfRange => "ConversionOutOfRange",
            Self::ConversionNonFinite => "ConversionNonFinite",
            Self::ConversionImaginaryPartNonZero => "ConversionImaginaryPartNonZero",
            Self::ConversionShapeMismatch => "ConversionShapeMismatch",
            Self::ConversionPlanSourceMismatch => "ConversionPlanSourceMismatch",
            Self::ConversionExecutionUnsupported => "ConversionExecutionUnsupported",
        }
    }

    fn message(&self) -> alloc_string::String {
        self.to_string()
    }
}

#[cfg(feature = "no_std")]
mod alloc_string {
    pub type String = alloc::string::String;
}
#[cfg(not(feature = "no_std"))]
mod alloc_string {
    pub type String = std::string::String;
}

pub fn exact_type_equal(left: &ResolvedType, right: &ResolvedType) -> bool {
    left == right
}

pub fn plan_implicit_conversion(
    source: &ResolvedType,
    target: &ResolvedType,
) -> Result<ConversionPlan, TypeResolutionError> {
    plan_conversion(source, target, ConversionMode::Implicit)
}

pub fn plan_explicit_cast(
    source: &ResolvedType,
    target: &ResolvedType,
) -> Result<ConversionPlan, TypeResolutionError> {
    plan_conversion(source, target, ConversionMode::Explicit)
}

pub fn permitted_conversion(source: &ResolvedType, target: &ResolvedType) -> bool {
    plan_implicit_conversion(source, target).is_ok()
}

pub fn explicit_cast_allowed(source: &ResolvedType, target: &ResolvedType) -> bool {
    plan_explicit_cast(source, target).is_ok()
}

fn plan_conversion(
    source: &ResolvedType,
    target: &ResolvedType,
    mode: ConversionMode,
) -> Result<ConversionPlan, TypeResolutionError> {
    if exact_type_equal(source, target) {
        return Ok(ConversionPlan {
            source: source.clone(),
            target: target.clone(),
            step: ConversionStep::Identity,
            cost: 0,
        });
    }
    let fail = || {
        TypeResolutionError::incompatible(
            "conversion",
            TypeConstraintFailure::ConversionNotPermitted {
                source: source.semantic_name(),
                target: target.semantic_name(),
            },
        )
    };
    let step = match (source.kind(), target.kind()) {
        (KindExpr::Named(source_kind), KindExpr::Named(target_kind)) => {
            let source_kind = BuiltinScalarKind::from_kind_id(*source_kind).ok_or_else(fail)?;
            let target_kind = BuiltinScalarKind::from_kind_id(*target_kind).ok_or_else(fail)?;
            let allowed = match mode {
                ConversionMode::Implicit => implicit_scalar_allowed(source_kind, target_kind),
                ConversionMode::Explicit => explicit_scalar_allowed(source_kind, target_kind),
            };
            if !allowed {
                return Err(fail());
            }
            ConversionStep::Scalar(ScalarConversion::Builtin {
                source: source_kind,
                target: target_kind,
                mode,
            })
        }
        (
            KindExpr::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            KindExpr::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) if source_dimensions == target_dimensions
            && source.dimension_parameters() == target.dimension_parameters() =>
        {
            let source_element = ResolvedType::new(
                *source_element.clone(),
                source.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            let target_element = ResolvedType::new(
                *target_element.clone(),
                target.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            ConversionStep::MatrixElements(Box::new(plan_conversion(
                &source_element,
                &target_element,
                mode,
            )?))
        }
        (KindExpr::Option(source_payload), KindExpr::Option(target_payload)) => {
            let source_payload = ResolvedType::new(
                *source_payload.clone(),
                source.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            let target_payload = ResolvedType::new(
                *target_payload.clone(),
                target.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            ConversionStep::OptionPayload(Box::new(plan_conversion(
                &source_payload,
                &target_payload,
                mode,
            )?))
        }
        _ => return Err(fail()),
    };
    let cost = conversion_step_cost(&step)?;
    Ok(ConversionPlan {
        source: source.clone(),
        target: target.clone(),
        step,
        cost,
    })
}

fn conversion_step_cost(step: &ConversionStep) -> Result<u32, TypeResolutionError> {
    match step {
        ConversionStep::Identity => Ok(0),
        ConversionStep::Scalar(ScalarConversion::Builtin { source, target, .. }) => {
            Ok(if source == target {
                0
            } else {
                scalar_conversion_cost(*source, *target)
            })
        }
        ConversionStep::MatrixElements(inner) | ConversionStep::OptionPayload(inner) => {
            inner.cost.checked_add(1).ok_or_else(|| {
                TypeResolutionError::incompatible(
                    "conversion",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "conversion-plan cost overflow".into(),
                    },
                )
            })
        }
    }
}

fn scalar_conversion_cost(source: BuiltinScalarKind, target: BuiltinScalarKind) -> u32 {
    if scalar_family(source) == scalar_family(target) {
        1 + scalar_width_rank(target).abs_diff(scalar_width_rank(source))
    } else {
        4 + scalar_width_rank(target)
    }
}

const fn scalar_family(kind: BuiltinScalarKind) -> u8 {
    use BuiltinScalarKind as K;
    match kind {
        K::U8 | K::U16 | K::U32 | K::U64 | K::U128 => 0,
        K::I8 | K::I16 | K::I32 | K::I64 | K::I128 => 1,
        K::F32 | K::F64 => 2,
        K::C32 | K::C64 => 3,
        K::R64 => 4,
        K::String => 5,
        K::Bool => 6,
    }
}

const fn scalar_width_rank(kind: BuiltinScalarKind) -> u32 {
    use BuiltinScalarKind as K;
    match kind {
        K::U8 | K::I8 => 0,
        K::U16 | K::I16 => 1,
        K::U32 | K::I32 | K::F32 | K::C32 => 2,
        K::U64 | K::I64 | K::F64 | K::C64 | K::R64 => 3,
        K::U128 | K::I128 => 4,
        K::String | K::Bool => 0,
    }
}

fn implicit_scalar_allowed(source: BuiltinScalarKind, target: BuiltinScalarKind) -> bool {
    use BuiltinScalarKind as K;
    if source == target {
        return true;
    }
    match (integer_description(source), integer_description(target)) {
        (Some((false, source_width)), Some((false, target_width))) => {
            return source_width < target_width;
        }
        (Some((true, source_width)), Some((true, target_width))) => {
            return source_width < target_width;
        }
        (Some((false, source_width)), Some((true, target_width))) => {
            return source_width < target_width;
        }
        _ => {}
    }
    match (source, target) {
        (K::F32, K::F64) | (K::C32, K::C64) => true,
        (K::U8 | K::U16 | K::I8 | K::I16, K::F32) => true,
        (K::U8 | K::U16 | K::U32 | K::I8 | K::I16 | K::I32, K::F64) => true,
        (K::U8 | K::U16 | K::U32 | K::I8 | K::I16 | K::I32 | K::I64, K::R64) => true,
        (K::F32, K::C32 | K::C64) | (K::F64, K::C64) => true,
        (integer, K::C32) => implicit_scalar_allowed(integer, K::F32),
        (integer, K::C64) => implicit_scalar_allowed(integer, K::F64),
        _ => false,
    }
}

fn explicit_scalar_allowed(source: BuiltinScalarKind, target: BuiltinScalarKind) -> bool {
    use BuiltinScalarKind as K;
    if source == target {
        return true;
    }
    if target == K::String {
        return source.satisfies(super::BuiltinKindPredicate::Number) || source == K::Bool;
    }
    if source == K::String || source == K::Bool || target == K::Bool {
        return false;
    }
    if integer_description(source).is_some() {
        return integer_description(target).is_some()
            || matches!(target, K::F32 | K::F64 | K::C32 | K::C64 | K::R64);
    }
    if matches!(source, K::F32 | K::F64) {
        return integer_description(target).is_some()
            || matches!(target, K::F32 | K::F64 | K::C32 | K::C64);
    }
    if matches!(source, K::C32 | K::C64) {
        return integer_description(target).is_some()
            || matches!(target, K::F32 | K::F64 | K::C32 | K::C64);
    }
    if source == K::R64 {
        return integer_description(target).is_some()
            || matches!(target, K::F32 | K::F64 | K::C32 | K::C64);
    }
    false
}

const fn integer_description(kind: BuiltinScalarKind) -> Option<(bool, u16)> {
    use BuiltinScalarKind as K;
    match kind {
        K::U8 => Some((false, 8)),
        K::U16 => Some((false, 16)),
        K::U32 => Some((false, 32)),
        K::U64 => Some((false, 64)),
        K::U128 => Some((false, 128)),
        K::I8 => Some((true, 8)),
        K::I16 => Some((true, 16)),
        K::I32 => Some((true, 32)),
        K::I64 => Some((true, 64)),
        K::I128 => Some((true, 128)),
        _ => None,
    }
}

pub fn plan_numeric_promotion(
    left: &ResolvedType,
    right: &ResolvedType,
) -> Result<Option<PromotionPlan>, TypeResolutionError> {
    if exact_type_equal(left, right) {
        let identity = plan_implicit_conversion(left, right)?;
        return Ok(Some(PromotionPlan {
            result: left.clone(),
            left: identity.clone(),
            right: identity,
        }));
    }
    match (left.kind(), right.kind()) {
        (
            KindExpr::Matrix {
                element: left_element,
                dimensions: left_dimensions,
            },
            KindExpr::Matrix {
                element: right_element,
                dimensions: right_dimensions,
            },
        ) if left_dimensions == right_dimensions
            && left.dimension_parameters() == right.dimension_parameters() =>
        {
            let left_element = ResolvedType::new(
                *left_element.clone(),
                left.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            let right_element = ResolvedType::new(
                *right_element.clone(),
                right.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            let Some(element) = plan_numeric_promotion(&left_element, &right_element)? else {
                return Ok(None);
            };
            let result = ResolvedType::new(
                KindExpr::Matrix {
                    element: Box::new(element.result.kind().clone()),
                    dimensions: left_dimensions.clone(),
                },
                left.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            Ok(Some(PromotionPlan {
                left: plan_implicit_conversion(left, &result)?,
                right: plan_implicit_conversion(right, &result)?,
                result,
            }))
        }
        (KindExpr::Named(left_id), KindExpr::Named(right_id)) => {
            let (Some(left_kind), Some(right_kind)) = (
                BuiltinScalarKind::from_kind_id(*left_id),
                BuiltinScalarKind::from_kind_id(*right_id),
            ) else {
                return Ok(None);
            };
            if !left_kind.satisfies(super::BuiltinKindPredicate::Number)
                || !right_kind.satisfies(super::BuiltinKindPredicate::Number)
            {
                return Ok(None);
            }
            let candidates = promotion_candidates(left_kind, right_kind);
            for candidate in candidates {
                let result = ResolvedType::new(candidate.kind_expr(), Box::new([]))?;
                if let (Ok(left_plan), Ok(right_plan)) = (
                    plan_implicit_conversion(left, &result),
                    plan_implicit_conversion(right, &result),
                ) {
                    return Ok(Some(PromotionPlan {
                        result,
                        left: left_plan,
                        right: right_plan,
                    }));
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

pub fn numeric_promotion(
    left: &ResolvedType,
    right: &ResolvedType,
) -> Result<Option<ResolvedType>, TypeResolutionError> {
    Ok(plan_numeric_promotion(left, right)?.map(|plan| plan.result))
}

fn promotion_candidates(
    left: BuiltinScalarKind,
    right: BuiltinScalarKind,
) -> Vec<BuiltinScalarKind> {
    use BuiltinScalarKind as K;
    let mut candidates = Vec::new();
    if integer_description(left).is_some() && integer_description(right).is_some() {
        candidates.extend([
            K::U8,
            K::I8,
            K::U16,
            K::I16,
            K::U32,
            K::I32,
            K::U64,
            K::I64,
            K::U128,
            K::I128,
        ]);
    }
    if left == K::R64 || right == K::R64 {
        candidates.push(K::R64);
    }
    if matches!(left, K::F32 | K::F64) || matches!(right, K::F32 | K::F64) {
        candidates.extend([K::F32, K::F64]);
    }
    if matches!(left, K::C32 | K::C64) || matches!(right, K::C32 | K::C64) {
        candidates.extend([K::C32, K::C64]);
    }
    candidates
}
