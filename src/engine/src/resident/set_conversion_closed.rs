//! Bounded closed analysis delegates the SAME maintained conversion owner.
use super::super::budget::{
    PreparedKernel, ResidentBudgetMeter, checked_cost_product as mul, checked_cost_sum as sum,
};
use super::{Plan, conversion_plan, cost, materialize};
use mech_core::snapshot::{SnapshotCanonicalizationBudget, SnapshotValidationContext};
use mech_core::*;
use std::sync::Arc;

/// The existing closed-analysis grammar, not a runtime conversion restriction.
/// A constant rank-two scalar Matrix and scalar Set with at most one leaf
/// target parameter cannot grow recursive evidence or compound dimension ASTs.
pub(in crate::resident) fn closed_scalar_setup_bytes(
    source: &Schema,
    target: &Schema,
) -> Option<u64> {
    let SchemaBody::Matrix {
        element: source_element,
        dimensions,
    } = source.body()
    else {
        return None;
    };
    let SchemaBody::Set {
        element: target_element,
        cardinality,
    } = target.body()
    else {
        return None;
    };
    let target_parameters = target.dimension_parameters();
    let target_extent_supported = match cardinality {
        CardinalitySpec::Exact(DimensionExpr::Constant(_))
        | CardinalitySpec::Dynamic {
            upper_bound: None | Some(DimensionExpr::Constant(_)),
        } => target_parameters.is_empty(),
        CardinalitySpec::Exact(DimensionExpr::Parameter(id)) => {
            id.get() == 0
                && target_parameters.len() == 1
                && matches!(
                    target_parameters[0].lower_bound(),
                    DimensionExpr::Constant(_)
                )
                && matches!(
                    target_parameters[0].upper_bound(),
                    None | Some(DimensionExpr::Constant(_))
                )
        }
        _ => false,
    };
    if !source.dimension_parameters().is_empty()
        || dimensions.len() != 2
        || !dimensions
            .iter()
            .all(|d| matches!(d, DimensionExpr::Constant(_)))
        || BuiltinScalarKind::from_schema_body(source_element).is_none()
        || BuiltinScalarKind::from_schema_body(target_element).is_none()
        || !target_extent_supported
    {
        return None;
    }
    // Preserve the bounded setup inventory: at most two kind/evidence nodes
    // per type, two leaf types and two semantic plan nodes, plus a possible
    // target parameter with constant bounds. The fixed scratch includes tiny
    // maps, current shape tapes, the private Plan/Copy metadata fields and
    // diagnostics. No nested schema or dependent dimension reaches setup.
    sum(&[
        mul(&[256, core::mem::size_of::<KindExpr>() as u64]).ok()?,
        mul(&[256, core::mem::size_of::<DimensionExpr>() as u64]).ok()?,
        mul(&[
            128,
            core::mem::size_of::<DimensionParameterDeclaration>() as u64,
        ])
        .ok()?,
        mul(&[32, core::mem::size_of::<ResolvedType>() as u64]).ok()?,
        mul(&[8, core::mem::size_of::<ConversionPlan>() as u64]).ok()?,
        mul(&[8, core::mem::size_of::<SchemaBody>() as u64]).ok()?,
        64 * 1024,
    ])
    .ok()
}

/// Compile-time analysis admission is not an ordinary turn or a fake permit.
/// Only the bounded grammar above reaches plan construction. All payload
/// preparation and materialization remain shared with the resident factory.
pub(crate) fn fold_closed(
    schemas: &Arc<SchemaTable>,
    source: &Value,
    output: SchemaId,
    target_shape: impl FnOnce() -> Result<ShapeInstance, ResidentKernelError>,
    meter: impl FnOnce() -> ResidentBudgetMeter,
    mut admit: impl FnMut(super::super::budget::KernelCostEstimate) -> Result<(), ResidentKernelError>,
) -> Result<Value, ResidentKernelError> {
    let source_entry = schemas
        .entry(source.schema())
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source_schema = source_entry.schema();
    let target_schema = schemas
        .get(output)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let setup = closed_scalar_setup_bytes(source_schema, target_schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    if source.schema_key() != source_entry.key() || !source.shape().parameter_values().is_empty() {
        return Err(ResidentKernelError::InvalidInput);
    }
    PreparedKernel::new((), super::super::budget::resident_cost! {
        temporary_bytes: setup,
        compute_work: setup.checked_mul(8).and_then(|work| work.checked_add(256)).ok_or(ResidentKernelError::InvalidShape)?,
        retained_nodes: 1,
        ..super::super::budget::KernelCostEstimate::default()
    }).prepare_for_analysis(&mut admit)?;
    // Caller facts/seed/default precedence stays lazy, after setup admission.
    // Reject a malformed count before copying the supplied parameter tape.
    let target_shape = target_shape()?;
    if target_shape.parameter_values().len() != target_schema.dimension_parameters().len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let _ = target_schema
        .instantiate_shape(target_shape.parameter_values().to_vec().into_boxed_slice())
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    drop(target_shape);
    let conversion = conversion_plan(source_schema, target_schema)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let metadata = conversion_schema_staging_bound(source_schema, target_schema)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let phases = conversion_materialization_phases(&conversion.step);
    let plan = Plan {
        conversion,
        source_schema: source.schema(),
        source_key: source_entry.key(),
        source_shape: source.shape().clone(),
        dense_shape: ResidentShape::SCALAR,
        metadata,
        phases,
    };
    // This is a genuine shallow canonical owner clone, not a materialized
    // selector, synthetic Value or detached turn plan.
    let source = [Some(source.clone())];
    let input = ResidentValueRef::Snapshot(&source);
    let mut meter = meter();
    let prepared = cost::prepare(
        input,
        None,
        schemas,
        source_schema,
        target_schema,
        &plan,
        &mut meter,
    )?;
    let canonicalization_work = PreparedKernel::new(prepared.canonicalization_work, prepared.cost)
        .prepare_for_analysis(admit)?;
    let canonicalization = SnapshotCanonicalizationBudget::new(canonicalization_work);
    let context = SnapshotValidationContext::with_shared_schemas(schemas)
        .with_canonicalization_budget(&canonicalization);
    materialize(&plan, input, source_schema, output, target_schema, &context)
}
