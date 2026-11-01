//! Declared schema/current-shape authority for the shared conversion executor.
use super::*;
use crate::snapshot::SnapshotValidationContext;
use crate::{Schema, SchemaBody, ShapeInstance};

#[derive(Clone, Copy)]
pub(super) struct DeclaredTarget<'a, 's> {
    pub(super) root: &'a Schema,
    pub(super) body: &'a SchemaBody,
    pub(super) shape: &'a ShapeInstance,
    pub(super) context: &'a SnapshotValidationContext<'s>,
}

impl<'a, 's> DeclaredTarget<'a, 's> {
    pub(super) fn child(self, body: &'a SchemaBody) -> Self {
        Self { body, ..self }
    }
}

/// Executes the SAME conversion evaluator with the real declared target and
/// its current shape. Nominal definitions are never guessed from KindExpr.
pub fn execute_conversion_draft_with_schema(
    draft: ValueDataDraft,
    plan: &ConversionPlan,
    target: &Schema,
    shape: &ShapeInstance,
    context: &SnapshotValidationContext<'_>,
) -> Result<ValueDataDraft, ConversionExecutionError> {
    let shape = target
        .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
        .map_err(|_| ConversionExecutionError::ConversionShapeMismatch)?;
    let actual = ResolvedType::from_schema(target, &shape)
        .map_err(|_| ConversionExecutionError::ConversionPlanSourceMismatch)?;
    if actual != plan.target {
        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
    }
    super::execute_conversion_draft_impl(
        draft,
        &plan.step,
        Some(DeclaredTarget {
            root: target,
            body: target.body(),
            shape: &shape,
            context,
        }),
    )
}

/// Computes only a prospective dimension witness, never a construction permit.
/// Declared target components must match the transformed CLOSED current source;
/// no parameter index crosses schema arenas and no axes are guessed from length.
pub fn conversion_output_shape(
    source: &Schema,
    source_shape: &ShapeInstance,
    target: &Schema,
    plan: &ConversionPlan,
) -> Result<ShapeInstance, ConversionExecutionError> {
    let actual = ResolvedType::from_schema(source, source_shape)
        .map_err(|_| ConversionExecutionError::ConversionPlanSourceMismatch)?;
    if actual != plan.source {
        return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
    }
    let source = source
        .closed_body(source_shape)
        .map_err(|_| ConversionExecutionError::ConversionShapeMismatch)?;
    let mut components = Vec::new();
    fn collect(
        source: &SchemaBody,
        target: &SchemaBody,
        step: &ConversionStep,
        components: &mut Vec<(SchemaBody, SchemaBody)>,
    ) -> Result<(), ConversionExecutionError> {
        match step {
            ConversionStep::MatrixToSet(inner) => {
                let (
                    SchemaBody::Matrix {
                        element: source, ..
                    },
                    SchemaBody::Set {
                        element: target, ..
                    },
                ) = (source, target)
                else {
                    return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                };
                // The converted Set cardinality depends on canonical key
                // identity, so only bind its declared element here. The SAME
                // output data refreshes cardinalities after normalization.
                collect(source, target, &inner.step, components)?;
            }
            ConversionStep::MatrixElements(inner) => {
                let (
                    SchemaBody::Matrix {
                        element: source,
                        dimensions,
                    },
                    SchemaBody::Matrix {
                        element: target,
                        dimensions: target_dimensions,
                    },
                ) = (source, target)
                else {
                    return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                };
                // Resolve axes with the common component solver, independently
                // of any data-dependent conversion inside each element.
                components.push((
                    SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Dynamic),
                        dimensions: target_dimensions.clone(),
                    },
                    SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Dynamic),
                        dimensions: dimensions.clone(),
                    },
                ));
                collect(source, target, &inner.step, components)?;
            }
            ConversionStep::OptionPayload(inner) => {
                let (SchemaBody::Option(source), SchemaBody::Option(target)) = (source, target)
                else {
                    return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                };
                collect(source, target, &inner.step, components)?;
            }
            ConversionStep::OptionPresent(inner) => {
                let SchemaBody::Option(target) = target else {
                    return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
                };
                collect(source, target, &inner.step, components)?;
            }
            _ => components.push((target.clone(), conversion_target_schema_body(source, step)?)),
        }
        Ok(())
    }
    collect(&source, target.body(), &plan.step, &mut components)?;
    let borrowed = components
        .iter_mut()
        .map(|(expected, actual)| (&*expected, core::mem::replace(actual, SchemaBody::Dynamic)))
        .collect::<Vec<_>>();
    crate::shape_for_schema_components(target, &borrowed, None)
        .map_err(|_| ConversionExecutionError::ConversionShapeMismatch)
}

/// Refreshes data-dependent cardinalities while retaining already witnessed
/// current element axes. Matrix root extents come from that SAME shape.
pub fn conversion_output_shape_for_data(
    target: &Schema,
    current: &ShapeInstance,
    data: &ValueDataDraft,
) -> Result<ShapeInstance, ConversionExecutionError> {
    crate::shape_for_value_components(target, [(target.body(), data)], current)
        .map_err(|_| ConversionExecutionError::ConversionShapeMismatch)
}

/// Transforms an actual source schema with the SAME semantic conversion plan.
/// Identity retains complete nominal definitions. Callers still validate the
/// result against their declared target; this is not schema reconstruction
/// from a nominal key, a runtime selector, or an execution permission.
pub fn conversion_target_schema_body(
    source: &SchemaBody,
    step: &ConversionStep,
) -> Result<SchemaBody, ConversionExecutionError> {
    Ok(match step {
        ConversionStep::Identity => source.clone(),
        ConversionStep::Scalar(ScalarConversion::Builtin { target, .. }) => target.schema_body(),
        ConversionStep::MatrixElements(inner) => {
            let SchemaBody::Matrix {
                element,
                dimensions,
            } = source
            else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            SchemaBody::Matrix {
                element: Box::new(conversion_target_schema_body(element, &inner.step)?),
                dimensions: dimensions.clone(),
            }
        }
        ConversionStep::MatrixToSet(inner) => {
            let SchemaBody::Matrix { element, .. } = source else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            SchemaBody::Set {
                element: Box::new(conversion_target_schema_body(element, &inner.step)?),
                cardinality: crate::CardinalitySpec::Dynamic { upper_bound: None },
            }
        }
        ConversionStep::OptionPresent(inner) => SchemaBody::Option(Box::new(
            conversion_target_schema_body(source, &inner.step)?,
        )),
        ConversionStep::OptionPayload(inner) => {
            let SchemaBody::Option(source) = source else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            SchemaBody::Option(Box::new(conversion_target_schema_body(
                source,
                &inner.step,
            )?))
        }
    })
}
