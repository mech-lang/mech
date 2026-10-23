//! Explicit semantic conversion and numeric-promotion planning.

use super::{BuiltinScalarKind, ResolvedType, TypeConstraintFailure, TypeResolutionError};
#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f32",
    feature = "complex"
))]
use crate::snapshot::F32Bits;
#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f64",
    feature = "complex"
))]
use crate::snapshot::F64Bits;
use crate::snapshot::OptionDraft;
#[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
use crate::snapshot::{Complex32Bits, Complex64Bits};
use crate::{KindExpr, MechErrorKind, ValueDataDraft};
#[cfg(any(not(feature = "selective-convert"), feature = "string"))]
use alloc_string::String;
use core::fmt::{self, Display, Formatter};

#[cfg(all(
    feature = "no_std",
    any(
        not(feature = "selective-convert"),
        feature = "complex",
        feature = "r64"
    )
))]
use alloc::format;
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
    MatrixToSet(Box<ConversionPlan>),
    OptionPayload(Box<ConversionPlan>),
    OptionPresent(Box<ConversionPlan>),
}

impl ConversionStep {
    /// Shared structural classification; never a factory or execution permit.
    pub fn contains_matrix_to_set(&self) -> bool {
        match self {
            Self::MatrixToSet(_) => true,
            Self::MatrixElements(inner)
            | Self::OptionPayload(inner)
            | Self::OptionPresent(inner) => inner.step.contains_matrix_to_set(),
            Self::Identity | Self::Scalar(_) => false,
        }
    }
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
        (
            KindExpr::Matrix {
                element: source_element,
                ..
            },
            KindExpr::Set {
                element: target_element,
                ..
            },
        ) if mode == ConversionMode::Explicit => {
            let source_element = ResolvedType::new(
                *source_element.clone(),
                source.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            let target_element = ResolvedType::new(
                *target_element.clone(),
                target.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            ConversionStep::MatrixToSet(Box::new(plan_conversion(
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
        (_, KindExpr::Option(target_payload)) if mode == ConversionMode::Explicit => {
            let target_payload = ResolvedType::new(
                *target_payload.clone(),
                target.dimension_parameters().to_vec().into_boxed_slice(),
            )?;
            ConversionStep::OptionPresent(Box::new(plan_conversion(source, &target_payload, mode)?))
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
        ConversionStep::MatrixElements(inner)
        | ConversionStep::MatrixToSet(inner)
        | ConversionStep::OptionPayload(inner)
        | ConversionStep::OptionPresent(inner) => inner.cost.checked_add(1).ok_or_else(|| {
            TypeResolutionError::incompatible(
                "conversion",
                TypeConstraintFailure::InvalidScheme {
                    reason: "conversion-plan cost overflow".into(),
                },
            )
        }),
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

#[path = "conversion_declared.rs"]
mod declared;
#[path = "conversion_population.rs"]
mod population;
#[path = "conversion_sort_work.rs"]
mod sort_work;
#[path = "conversion_staging.rs"]
mod staging;
pub use declared::{
    conversion_output_shape, conversion_output_shape_for_data, conversion_target_schema_body,
    execute_conversion_draft_with_schema,
};
pub use population::{ConversionPopulationError, conversion_draft_population};
pub use sort_work::{
    conversion_comparison_work, conversion_dense_comparison_work, conversion_sort_comparison_units,
};
pub use staging::{
    ConversionSchemaStagingBound, conversion_materialization_phases,
    conversion_option_present_depth, conversion_schema_staging_bound, conversion_staging_demand,
};

/// Executes a closed semantic conversion plan against canonical value data.
///
/// Source evaluation and resident artifact execution share this storage-blind
/// authority so feature-limited resident builds cannot diverge from compiler
/// conversion semantics.
pub fn execute_conversion_draft(
    draft: ValueDataDraft,
    step: &ConversionStep,
) -> Result<ValueDataDraft, ConversionExecutionError> {
    execute_conversion_draft_impl(draft, step, None)
}

fn execute_conversion_draft_impl(
    draft: ValueDataDraft,
    step: &ConversionStep,
    target: Option<declared::DeclaredTarget<'_, '_>>,
) -> Result<ValueDataDraft, ConversionExecutionError> {
    match step {
        ConversionStep::Identity => Ok(draft),
        ConversionStep::Scalar(ScalarConversion::Builtin { source, target, .. }) => {
            execute_scalar_conversion(draft, *source, *target)
        }
        ConversionStep::MatrixElements(element_plan) => {
            let ValueDataDraft::Matrix(elements) = draft else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            let child = target
                .map(|target| {
                    let crate::SchemaBody::Matrix { element, .. } = target.body else {
                        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                    };
                    Ok(target.child(element))
                })
                .transpose()?;
            let converted = elements
                .into_vec()
                .into_iter()
                .map(|element| execute_conversion_draft_impl(element, &element_plan.step, child))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ValueDataDraft::Matrix(converted.into_boxed_slice()))
        }
        ConversionStep::MatrixToSet(element_plan) => {
            let ValueDataDraft::Matrix(elements) = draft else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            let child = target
                .map(|target| {
                    let crate::SchemaBody::Set { element, .. } = target.body else {
                        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                    };
                    Ok(target.child(element))
                })
                .transpose()?;
            let converted = elements
                .into_vec()
                .into_iter()
                .map(|element| execute_conversion_draft_impl(element, &element_plan.step, child))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(target) = child {
                // Inner conversions may have changed a Set-valued key's
                // cardinality. Resolve that real data before outer key
                // finalization, retaining the SAME declared Matrix axes.
                let shape = crate::shape_for_value_components(
                    target.root,
                    converted.iter().map(|data| (target.body, data)),
                    target.shape,
                )
                .map_err(|_| ConversionExecutionError::ConversionShapeMismatch)?;
                let values = crate::snapshot::normalize_set_conversion_drafts_with_context(
                    target.body,
                    converted.into_boxed_slice(),
                    &shape,
                    target.context,
                )
                .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
                return Ok(ValueDataDraft::Set(values));
            }
            let body = super::resolved_value::schema_body_from_resolved(
                element_plan.target.kind(),
                element_plan.target.dimension_parameters(),
            )
            .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            let schema = crate::SchemaDraft {
                body,
                dimension_parameters: element_plan
                    .target
                    .dimension_parameters()
                    .to_vec()
                    .into_boxed_slice(),
            }
            .finalize()
            .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            let mut builder = crate::SchemaTableBuilder::new();
            let handle = builder
                .insert(schema)
                .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            let build = builder
                .finish()
                .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            let id = build
                .resolve(handle)
                .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            let schemas = build.into_parts().0;
            let values = crate::snapshot::normalize_set_conversion_drafts(
                schemas.get(id).expect("constructed element schema"),
                converted.into_boxed_slice(),
                &schemas,
            )
            .map_err(|_| ConversionExecutionError::ConversionExecutionUnsupported)?;
            Ok(ValueDataDraft::Set(values))
        }
        ConversionStep::OptionPresent(payload_plan) => {
            let child = target
                .map(|target| {
                    let crate::SchemaBody::Option(element) = target.body else {
                        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                    };
                    Ok(target.child(element))
                })
                .transpose()?;
            Ok(ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(execute_conversion_draft_impl(
                    draft,
                    &payload_plan.step,
                    child,
                )?)),
            }))
        }
        ConversionStep::OptionPayload(payload_plan) => {
            let ValueDataDraft::Option(option) = draft else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            let child = target
                .map(|target| {
                    let crate::SchemaBody::Option(element) = target.body else {
                        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                    };
                    Ok(target.child(element))
                })
                .transpose()?;
            let value = option
                .value
                .map(|value| {
                    execute_conversion_draft_impl(*value, &payload_plan.step, child).map(Box::new)
                })
                .transpose()?;
            Ok(ValueDataDraft::Option(OptionDraft {
                present: option.present,
                value,
            }))
        }
    }
}

#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "numeric scratch variants are shared across feature-selected source and target conversions"
)]
enum RuntimeNumber {
    Unsigned(u128),
    Signed(i128),
    F32(f32),
    F64(f64),
    #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
    C32(f32, f32),
    #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
    C64(f64, f64),
    #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
    Rational(i64, u64),
}

fn scalar_conversion_kind_compiled(kind: BuiltinScalarKind) -> bool {
    match kind {
        BuiltinScalarKind::U8 => cfg!(any(not(feature = "selective-convert"), feature = "u8")),
        BuiltinScalarKind::U16 => cfg!(any(not(feature = "selective-convert"), feature = "u16")),
        BuiltinScalarKind::U32 => cfg!(any(not(feature = "selective-convert"), feature = "u32")),
        BuiltinScalarKind::U64 => cfg!(any(not(feature = "selective-convert"), feature = "u64")),
        BuiltinScalarKind::U128 => cfg!(any(not(feature = "selective-convert"), feature = "u128")),
        BuiltinScalarKind::I8 => cfg!(any(not(feature = "selective-convert"), feature = "i8")),
        BuiltinScalarKind::I16 => cfg!(any(not(feature = "selective-convert"), feature = "i16")),
        BuiltinScalarKind::I32 => cfg!(any(not(feature = "selective-convert"), feature = "i32")),
        BuiltinScalarKind::I64 => cfg!(any(not(feature = "selective-convert"), feature = "i64")),
        BuiltinScalarKind::I128 => cfg!(any(not(feature = "selective-convert"), feature = "i128")),
        BuiltinScalarKind::F32 => cfg!(any(not(feature = "selective-convert"), feature = "f32")),
        BuiltinScalarKind::F64 => cfg!(any(not(feature = "selective-convert"), feature = "f64")),
        BuiltinScalarKind::C32 => {
            cfg!(any(not(feature = "selective-convert"), feature = "complex"))
        }
        BuiltinScalarKind::C64 => {
            cfg!(any(not(feature = "selective-convert"), feature = "complex"))
        }
        BuiltinScalarKind::R64 => cfg!(any(not(feature = "selective-convert"), feature = "r64")),
        BuiltinScalarKind::Bool => cfg!(any(not(feature = "selective-convert"), feature = "bool")),
        BuiltinScalarKind::String => {
            cfg!(any(not(feature = "selective-convert"), feature = "string"))
        }
    }
}

fn runtime_number(
    draft: ValueDataDraft,
    expected: BuiltinScalarKind,
) -> Result<RuntimeNumber, ConversionExecutionError> {
    let (number, actual) = match draft {
        #[cfg(any(not(feature = "selective-convert"), feature = "u8"))]
        ValueDataDraft::U8(value) => (RuntimeNumber::Unsigned(value.into()), BuiltinScalarKind::U8),
        #[cfg(any(not(feature = "selective-convert"), feature = "u16"))]
        ValueDataDraft::U16(value) => (
            RuntimeNumber::Unsigned(value.into()),
            BuiltinScalarKind::U16,
        ),
        #[cfg(any(not(feature = "selective-convert"), feature = "u32"))]
        ValueDataDraft::U32(value) => (
            RuntimeNumber::Unsigned(value.into()),
            BuiltinScalarKind::U32,
        ),
        #[cfg(any(not(feature = "selective-convert"), feature = "u64"))]
        ValueDataDraft::U64(value) => (
            RuntimeNumber::Unsigned(value.into()),
            BuiltinScalarKind::U64,
        ),
        #[cfg(any(not(feature = "selective-convert"), feature = "u128"))]
        ValueDataDraft::U128(value) => (RuntimeNumber::Unsigned(value), BuiltinScalarKind::U128),
        #[cfg(any(not(feature = "selective-convert"), feature = "i8"))]
        ValueDataDraft::I8(value) => (RuntimeNumber::Signed(value.into()), BuiltinScalarKind::I8),
        #[cfg(any(not(feature = "selective-convert"), feature = "i16"))]
        ValueDataDraft::I16(value) => (RuntimeNumber::Signed(value.into()), BuiltinScalarKind::I16),
        #[cfg(any(not(feature = "selective-convert"), feature = "i32"))]
        ValueDataDraft::I32(value) => (RuntimeNumber::Signed(value.into()), BuiltinScalarKind::I32),
        #[cfg(any(not(feature = "selective-convert"), feature = "i64"))]
        ValueDataDraft::I64(value) => (RuntimeNumber::Signed(value.into()), BuiltinScalarKind::I64),
        #[cfg(any(not(feature = "selective-convert"), feature = "i128"))]
        ValueDataDraft::I128(value) => (RuntimeNumber::Signed(value), BuiltinScalarKind::I128),
        #[cfg(any(not(feature = "selective-convert"), feature = "f32"))]
        ValueDataDraft::F32(value) => (RuntimeNumber::F32(value.to_f32()), BuiltinScalarKind::F32),
        #[cfg(any(not(feature = "selective-convert"), feature = "f64"))]
        ValueDataDraft::F64(value) => (RuntimeNumber::F64(value.to_f64()), BuiltinScalarKind::F64),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        ValueDataDraft::Complex32(value) => (
            RuntimeNumber::C32(value.real().to_f32(), value.imaginary().to_f32()),
            BuiltinScalarKind::C32,
        ),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        ValueDataDraft::Complex64(value) => (
            RuntimeNumber::C64(value.real().to_f64(), value.imaginary().to_f64()),
            BuiltinScalarKind::C64,
        ),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        ValueDataDraft::Rational64 {
            numerator,
            denominator,
        } => (
            RuntimeNumber::Rational(numerator, denominator),
            BuiltinScalarKind::R64,
        ),
        _ => {
            return Err(if scalar_conversion_kind_compiled(expected) {
                ConversionExecutionError::ConversionPlanSourceMismatch
            } else {
                ConversionExecutionError::ConversionExecutionUnsupported
            });
        }
    };
    if actual != expected {
        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
    }
    Ok(number)
}

pub fn execute_scalar_conversion(
    draft: ValueDataDraft,
    source: BuiltinScalarKind,
    target: BuiltinScalarKind,
) -> Result<ValueDataDraft, ConversionExecutionError> {
    #[cfg(any(not(feature = "selective-convert"), feature = "bool"))]
    if source == BuiltinScalarKind::Bool {
        let ValueDataDraft::Bool(value) = draft else {
            return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
        };
        return match target {
            #[cfg(any(not(feature = "selective-convert"), feature = "string"))]
            BuiltinScalarKind::String => Ok(ValueDataDraft::String(value.to_string())),
            _ => Err(ConversionExecutionError::ConversionExecutionUnsupported),
        };
    }
    let number = runtime_number(draft, source)?;
    use BuiltinScalarKind as K;
    match target {
        #[cfg(any(not(feature = "selective-convert"), feature = "u8"))]
        K::U8 => integer_target(number, false, 8).map(|value| ValueDataDraft::U8(value as u8)),
        #[cfg(any(not(feature = "selective-convert"), feature = "u16"))]
        K::U16 => integer_target(number, false, 16).map(|value| ValueDataDraft::U16(value as u16)),
        #[cfg(any(not(feature = "selective-convert"), feature = "u32"))]
        K::U32 => integer_target(number, false, 32).map(|value| ValueDataDraft::U32(value as u32)),
        #[cfg(any(not(feature = "selective-convert"), feature = "u64"))]
        K::U64 => integer_target(number, false, 64).map(|value| ValueDataDraft::U64(value as u64)),
        #[cfg(any(not(feature = "selective-convert"), feature = "u128"))]
        K::U128 => {
            integer_target(number, false, 128).map(|value| ValueDataDraft::U128(value as u128))
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "i8"))]
        K::I8 => integer_target(number, true, 8).map(|value| ValueDataDraft::I8(value as i8)),
        #[cfg(any(not(feature = "selective-convert"), feature = "i16"))]
        K::I16 => integer_target(number, true, 16).map(|value| ValueDataDraft::I16(value as i16)),
        #[cfg(any(not(feature = "selective-convert"), feature = "i32"))]
        K::I32 => integer_target(number, true, 32).map(|value| ValueDataDraft::I32(value as i32)),
        #[cfg(any(not(feature = "selective-convert"), feature = "i64"))]
        K::I64 => integer_target(number, true, 64).map(|value| ValueDataDraft::I64(value as i64)),
        #[cfg(any(not(feature = "selective-convert"), feature = "i128"))]
        K::I128 => integer_target(number, true, 128).map(ValueDataDraft::I128),
        #[cfg(any(not(feature = "selective-convert"), feature = "f32"))]
        K::F32 => number_to_f32(number).map(|value| ValueDataDraft::F32(F32Bits::from_f32(value))),
        #[cfg(any(not(feature = "selective-convert"), feature = "f64"))]
        K::F64 => number_to_f64(number).map(|value| ValueDataDraft::F64(F64Bits::from_f64(value))),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        K::C32 => number_to_complex32(number).map(|(real, imaginary)| {
            ValueDataDraft::Complex32(Complex32Bits::new(
                F32Bits::from_f32(real),
                F32Bits::from_f32(imaginary),
            ))
        }),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        K::C64 => number_to_complex64(number).map(|(real, imaginary)| {
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(real),
                F64Bits::from_f64(imaginary),
            ))
        }),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        K::R64 => {
            number_to_rational(number).map(|(numerator, denominator)| ValueDataDraft::Rational64 {
                numerator,
                denominator,
            })
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "string"))]
        K::String => Ok(ValueDataDraft::String(number_display(number))),
        #[cfg(any(not(feature = "selective-convert"), feature = "bool"))]
        K::Bool => Err(ConversionExecutionError::ConversionExecutionUnsupported),
        #[cfg(all(
            feature = "selective-convert",
            not(all(
                feature = "u8",
                feature = "u16",
                feature = "u32",
                feature = "u64",
                feature = "u128",
                feature = "i8",
                feature = "i16",
                feature = "i32",
                feature = "i64",
                feature = "i128",
                feature = "f32",
                feature = "f64",
                feature = "complex",
                feature = "r64",
                feature = "bool",
                feature = "string"
            ))
        ))]
        _ => Err(ConversionExecutionError::ConversionExecutionUnsupported),
    }
}

fn real_number(number: RuntimeNumber) -> Result<RuntimeNumber, ConversionExecutionError> {
    match number {
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(real, imaginary) if imaginary == 0.0 => Ok(RuntimeNumber::F32(real)),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C64(real, imaginary) if imaginary == 0.0 => Ok(RuntimeNumber::F64(real)),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(_, _) | RuntimeNumber::C64(_, _) => {
            Err(ConversionExecutionError::ConversionImaginaryPartNonZero)
        }
        number => Ok(number),
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128",
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
fn integer_target(
    number: RuntimeNumber,
    signed: bool,
    bits: u16,
) -> Result<i128, ConversionExecutionError> {
    match real_number(number)? {
        RuntimeNumber::Unsigned(value) => {
            if signed {
                let maximum = if bits == 128 {
                    i128::MAX as u128
                } else {
                    (1_u128 << (bits - 1)) - 1
                };
                if value > maximum {
                    return Err(ConversionExecutionError::ConversionOutOfRange);
                }
                Ok(value as i128)
            } else {
                let maximum = if bits == 128 {
                    u128::MAX
                } else {
                    (1_u128 << bits) - 1
                };
                if value > maximum || value > i128::MAX as u128 {
                    return if bits == 128 {
                        Ok(value as i128)
                    } else {
                        Err(ConversionExecutionError::ConversionOutOfRange)
                    };
                }
                Ok(value as i128)
            }
        }
        RuntimeNumber::Signed(value) => {
            let (minimum, maximum) = if signed {
                if bits == 128 {
                    (i128::MIN, i128::MAX)
                } else {
                    (-(1_i128 << (bits - 1)), (1_i128 << (bits - 1)) - 1)
                }
            } else {
                (
                    0,
                    if bits == 128 {
                        i128::MAX
                    } else {
                        (1_i128 << bits) - 1
                    },
                )
            };
            if value < minimum || value > maximum {
                Err(ConversionExecutionError::ConversionOutOfRange)
            } else {
                Ok(value)
            }
        }
        RuntimeNumber::F32(value) => float_to_integer(value as f64, signed, bits),
        RuntimeNumber::F64(value) => float_to_integer(value, signed, bits),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        RuntimeNumber::Rational(numerator, denominator) => {
            let truncated = i128::from(numerator) / i128::from(denominator);
            integer_target(RuntimeNumber::Signed(truncated), signed, bits)
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(_, _) | RuntimeNumber::C64(_, _) => unreachable!(),
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128",
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
fn float_to_integer(value: f64, signed: bool, bits: u16) -> Result<i128, ConversionExecutionError> {
    if !value.is_finite() {
        return Err(ConversionExecutionError::ConversionNonFinite);
    }
    let value = value.trunc();
    let (minimum, maximum_exclusive) = if signed {
        (
            -(2.0_f64).powi(i32::from(bits - 1)),
            (2.0_f64).powi(i32::from(bits - 1)),
        )
    } else {
        (0.0, (2.0_f64).powi(i32::from(bits)))
    };
    if value < minimum || value >= maximum_exclusive {
        return Err(ConversionExecutionError::ConversionOutOfRange);
    }
    Ok(if signed {
        value as i128
    } else {
        (value as u128) as i128
    })
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f32",
    feature = "complex"
))]
fn number_to_f32(number: RuntimeNumber) -> Result<f32, ConversionExecutionError> {
    match real_number(number)? {
        RuntimeNumber::Unsigned(value) => finite_f32_from_finite(value as f32),
        RuntimeNumber::Signed(value) => finite_f32_from_finite(value as f32),
        RuntimeNumber::F32(value) => Ok(value),
        RuntimeNumber::F64(value) => {
            let narrowed = value as f32;
            if value.is_finite() && narrowed.is_infinite() {
                Err(ConversionExecutionError::ConversionOutOfRange)
            } else {
                Ok(narrowed)
            }
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        RuntimeNumber::Rational(numerator, denominator) => {
            finite_f32_from_finite(numerator as f32 / denominator as f32)
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(_, _) | RuntimeNumber::C64(_, _) => unreachable!(),
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f64",
    feature = "complex"
))]
fn number_to_f64(number: RuntimeNumber) -> Result<f64, ConversionExecutionError> {
    match real_number(number)? {
        RuntimeNumber::Unsigned(value) => finite_f64_from_finite(value as f64),
        RuntimeNumber::Signed(value) => finite_f64_from_finite(value as f64),
        RuntimeNumber::F32(value) => Ok(f64::from(value)),
        RuntimeNumber::F64(value) => Ok(value),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        RuntimeNumber::Rational(numerator, denominator) => {
            finite_f64_from_finite(numerator as f64 / denominator as f64)
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(_, _) | RuntimeNumber::C64(_, _) => unreachable!(),
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f32",
    feature = "complex"
))]
fn finite_f32_from_finite(value: f32) -> Result<f32, ConversionExecutionError> {
    if value.is_infinite() {
        Err(ConversionExecutionError::ConversionOutOfRange)
    } else {
        Ok(value)
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    feature = "f64",
    feature = "complex"
))]
fn finite_f64_from_finite(value: f64) -> Result<f64, ConversionExecutionError> {
    if value.is_infinite() {
        Err(ConversionExecutionError::ConversionOutOfRange)
    } else {
        Ok(value)
    }
}

#[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
fn number_to_complex32(number: RuntimeNumber) -> Result<(f32, f32), ConversionExecutionError> {
    match number {
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(real, imaginary) => Ok((real, imaginary)),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C64(real, imaginary) => {
            let narrowed_real = real as f32;
            let narrowed_imaginary = imaginary as f32;
            if (real.is_finite() && narrowed_real.is_infinite())
                || (imaginary.is_finite() && narrowed_imaginary.is_infinite())
            {
                Err(ConversionExecutionError::ConversionOutOfRange)
            } else {
                Ok((narrowed_real, narrowed_imaginary))
            }
        }
        number => number_to_f32(number).map(|real| (real, 0.0)),
    }
}

#[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
fn number_to_complex64(number: RuntimeNumber) -> Result<(f64, f64), ConversionExecutionError> {
    match number {
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(real, imaginary) => Ok((f64::from(real), f64::from(imaginary))),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C64(real, imaginary) => Ok((real, imaginary)),
        number => number_to_f64(number).map(|real| (real, 0.0)),
    }
}

#[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
fn number_to_rational(number: RuntimeNumber) -> Result<(i64, u64), ConversionExecutionError> {
    match real_number(number)? {
        RuntimeNumber::Unsigned(value) => i64::try_from(value)
            .map(|value| (value, 1))
            .map_err(|_| ConversionExecutionError::ConversionOutOfRange),
        RuntimeNumber::Signed(value) => i64::try_from(value)
            .map(|value| (value, 1))
            .map_err(|_| ConversionExecutionError::ConversionOutOfRange),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        RuntimeNumber::Rational(numerator, denominator) => Ok((numerator, denominator)),
        RuntimeNumber::F32(_) | RuntimeNumber::F64(_) => {
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        }
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(_, _) | RuntimeNumber::C64(_, _) => unreachable!(),
    }
}

#[cfg(any(not(feature = "selective-convert"), feature = "string"))]
fn number_display(number: RuntimeNumber) -> String {
    match number {
        RuntimeNumber::Unsigned(value) => value.to_string(),
        RuntimeNumber::Signed(value) => value.to_string(),
        RuntimeNumber::F32(value) => value.to_string(),
        RuntimeNumber::F64(value) => value.to_string(),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C32(real, imaginary) => complex_display(real, imaginary),
        #[cfg(any(not(feature = "selective-convert"), feature = "complex"))]
        RuntimeNumber::C64(real, imaginary) => complex_display(real, imaginary),
        #[cfg(any(not(feature = "selective-convert"), feature = "r64"))]
        RuntimeNumber::Rational(numerator, denominator) => format!("{numerator}/{denominator}"),
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    all(feature = "complex", feature = "string")
))]
fn complex_display<T>(real: T, imaginary: T) -> String
where
    T: ComplexDisplayPart,
{
    if real == T::default() {
        format!("{imaginary}i")
    } else if imaginary.is_sign_negative() && !imaginary.is_nan() {
        format!("{real}{imaginary}i")
    } else {
        // Float Display omits the sign bit of NaN. Keep that canonical token,
        // but never omit the separator between the real and imaginary parts.
        format!("{real}+{imaginary}i")
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    all(feature = "complex", feature = "string")
))]
trait ComplexDisplayPart: Copy + core::fmt::Display + Default + PartialEq {
    fn is_sign_negative(self) -> bool;
    fn is_nan(self) -> bool;
}

#[cfg(any(
    not(feature = "selective-convert"),
    all(feature = "complex", feature = "string")
))]
impl ComplexDisplayPart for f32 {
    fn is_sign_negative(self) -> bool {
        self.is_sign_negative()
    }

    fn is_nan(self) -> bool {
        self.is_nan()
    }
}

#[cfg(any(
    not(feature = "selective-convert"),
    all(feature = "complex", feature = "string")
))]
impl ComplexDisplayPart for f64 {
    fn is_sign_negative(self) -> bool {
        self.is_sign_negative()
    }

    fn is_nan(self) -> bool {
        self.is_nan()
    }
}

#[cfg(all(test, feature = "selective-convert", feature = "u64", feature = "f64"))]
mod selective_tests {
    use super::*;

    #[test]
    fn enabled_conversion_uses_the_shared_semantics() {
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::U64(42),
                BuiltinScalarKind::U64,
                BuiltinScalarKind::F64
            ),
            Ok(ValueDataDraft::F64(F64Bits::from_f64(42.0)))
        );
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::F64(F64Bits::from_f64(f64::NAN)),
                BuiltinScalarKind::F64,
                BuiltinScalarKind::U64
            ),
            Err(ConversionExecutionError::ConversionNonFinite)
        );
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::String("wrong source".into()),
                BuiltinScalarKind::U64,
                BuiltinScalarKind::F64,
            ),
            Err(ConversionExecutionError::ConversionPlanSourceMismatch)
        );
    }

    #[cfg(not(feature = "complex"))]
    #[test]
    fn excluded_complex_source_and_target_are_rejected() {
        let source = ValueDataDraft::Complex64(crate::snapshot::Complex64Bits::new(
            F64Bits::from_f64(1.0),
            F64Bits::from_f64(0.0),
        ));
        assert_eq!(
            execute_scalar_conversion(source, BuiltinScalarKind::C64, BuiltinScalarKind::F64),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::U64(1),
                BuiltinScalarKind::U64,
                BuiltinScalarKind::C64
            ),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
    }

    #[cfg(not(feature = "r64"))]
    #[test]
    fn excluded_rational_source_and_target_are_rejected() {
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::Rational64 {
                    numerator: 3,
                    denominator: 2
                },
                BuiltinScalarKind::R64,
                BuiltinScalarKind::F64
            ),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::U64(1),
                BuiltinScalarKind::U64,
                BuiltinScalarKind::R64
            ),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
    }

    #[cfg(not(feature = "u128"))]
    #[test]
    fn excluded_integer_width_is_rejected() {
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::U64(1),
                BuiltinScalarKind::U64,
                BuiltinScalarKind::U128
            ),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
        assert_eq!(
            execute_scalar_conversion(
                ValueDataDraft::U128(1),
                BuiltinScalarKind::U128,
                BuiltinScalarKind::F64,
            ),
            Err(ConversionExecutionError::ConversionExecutionUnsupported)
        );
    }
}
