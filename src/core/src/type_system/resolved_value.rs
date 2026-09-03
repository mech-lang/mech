//! Storage-blind certificates connecting resolved types to canonical schemas.

use super::{BuiltinScalarKind, ResolvedType, TypeConstraintFailure, TypeResolutionError};
use crate::{
    CardinalitySpec, DimensionExpr, DimensionParameterDeclaration, KindExpr, KindField, Schema,
    SchemaBody, SchemaDraft, SchemaField, SemanticModelError, ShapeInstance, exact_type_equal,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeSet, format, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, format, vec, vec::Vec};

/// Semantic authority for constructing one resolved output schema. These
/// rules are selected with the source overload and never by a runtime factory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedOutputSchemaRule {
    FromResolvedType,
    FromInput(usize),
    Declared(SchemaBody),
    DynamicSetCartesianProduct,
    DynamicSetPowerset,
}

/// A validated semantic type, canonical schema, and current shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedValueDescriptor {
    resolved_type: ResolvedType,
    schema: Schema,
    shape: ShapeInstance,
}

impl ResolvedValueDescriptor {
    pub fn new(
        resolved_type: ResolvedType,
        schema: Schema,
        shape: ShapeInstance,
    ) -> Result<Self, TypeResolutionError> {
        let shape = schema
            .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
            .map_err(TypeResolutionError::semantic)?;
        let derived = ResolvedType::from_schema(&schema, &shape)?;
        if !exact_type_equal(&resolved_type, &derived) {
            return Err(descriptor_mismatch(&resolved_type, &derived));
        }
        Ok(Self {
            resolved_type,
            schema,
            shape,
        })
    }

    pub fn from_schema(schema: Schema, shape: ShapeInstance) -> Result<Self, TypeResolutionError> {
        let resolved_type = ResolvedType::from_schema(&schema, &shape)?;
        Self::new(resolved_type, schema, shape)
    }

    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }

    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    pub const fn shape(&self) -> &ShapeInstance {
        &self.shape
    }

    pub fn current_extents(&self) -> Result<Box<[u64]>, SemanticModelError> {
        let mut dimensions = Vec::new();
        match self.schema.body() {
            SchemaBody::Matrix {
                dimensions: matrix_dimensions,
                ..
            } => dimensions.extend(matrix_dimensions.iter()),
            SchemaBody::Table {
                rows: CardinalitySpec::Exact(rows),
                ..
            } => dimensions.push(rows),
            SchemaBody::Set {
                cardinality: CardinalitySpec::Exact(cardinality),
                ..
            }
            | SchemaBody::Map {
                cardinality: CardinalitySpec::Exact(cardinality),
                ..
            } => dimensions.push(cardinality),
            _ => {}
        }
        dimensions
            .into_iter()
            .map(|dimension| self.shape.resolve_dimension(dimension))
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }
}

/// Materializes a canonical output descriptor from semantic authority only.
pub fn materialize_resolved_output(
    resolved: &ResolvedType,
    rule: &ResolvedOutputSchemaRule,
    inputs: &[ResolvedValueDescriptor],
    current_extents: Box<[u64]>,
) -> Result<ResolvedValueDescriptor, TypeResolutionError> {
    let template = match rule {
        ResolvedOutputSchemaRule::FromResolvedType => {
            schema_body_from_resolved(resolved.kind(), resolved.dimension_parameters())?
        }
        ResolvedOutputSchemaRule::FromInput(index) => inputs
            .get(*index)
            .map(|input| input.schema().body().clone())
            .ok_or_else(|| invalid_rule(format!("input {index} is unavailable")))?,
        ResolvedOutputSchemaRule::Declared(body) => body.clone(),
        ResolvedOutputSchemaRule::DynamicSetCartesianProduct => {
            if inputs.len() != 2 {
                return Err(invalid_rule(format!(
                    "set Cartesian product requires two inputs, received {}",
                    inputs.len()
                )));
            }
            SchemaBody::Set {
                element: Box::new(SchemaBody::Tuple(
                    vec![set_element(&inputs[0])?, set_element(&inputs[1])?].into_boxed_slice(),
                )),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            }
        }
        ResolvedOutputSchemaRule::DynamicSetPowerset => {
            let input = inputs
                .first()
                .ok_or_else(|| invalid_rule("set powerset requires one input"))?;
            let SchemaBody::Set {
                element,
                cardinality,
            } = input.schema().body()
            else {
                return Err(invalid_rule("set powerset input is not a set"));
            };
            let upper_bound = match cardinality {
                CardinalitySpec::Exact(cardinality) => Some(cardinality.clone()),
                CardinalitySpec::Dynamic { upper_bound } => upper_bound.clone(),
            };
            SchemaBody::Set {
                element: Box::new(SchemaBody::Set {
                    element: element.clone(),
                    cardinality: CardinalitySpec::Dynamic { upper_bound },
                }),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            }
        }
    };
    let schema = schema_for_resolved(resolved, &template)?;
    let shape = shape_for_extents(&schema, &current_extents)?;
    ResolvedValueDescriptor::new(resolved.clone(), schema, shape)
}

fn set_element(input: &ResolvedValueDescriptor) -> Result<SchemaBody, TypeResolutionError> {
    match input.schema().body() {
        SchemaBody::Set { element, .. } => Ok(element.as_ref().clone()),
        _ => Err(invalid_rule("set operation input is not a set")),
    }
}

fn schema_for_resolved(
    resolved: &ResolvedType,
    template: &SchemaBody,
) -> Result<Schema, TypeResolutionError> {
    let mut dynamic_parameters = BTreeSet::new();
    collect_dynamic_extent_parameters(
        resolved.kind(),
        template,
        resolved.dimension_parameters(),
        &mut dynamic_parameters,
    )?;
    let mut old_to_new = vec![None; resolved.dimension_parameters().len()];
    let mut next = 0_u32;
    for old in 0..resolved.dimension_parameters().len() {
        let old = crate::DimensionParameterId::new(old as u32);
        if !dynamic_parameters.contains(&old) {
            old_to_new[old.get() as usize] = Some(crate::DimensionParameterId::new(next));
            next = next
                .checked_add(1)
                .ok_or_else(|| invalid_rule("too many output dimensions"))?;
        }
    }
    let dimension_parameters = resolved
        .dimension_parameters()
        .iter()
        .filter(|declaration| !dynamic_parameters.contains(&declaration.id))
        .map(|declaration| {
            let id = old_to_new[declaration.id.get() as usize]
                .ok_or_else(|| invalid_rule(format!("unknown dimension {:?}", declaration.id)))?;
            Ok(DimensionParameterDeclaration {
                id,
                origin: declaration.origin,
                lifetime: declaration.lifetime,
                lower_bound: crate::rewrite_dimension_references(
                    &declaration.lower_bound,
                    &old_to_new,
                )
                .map_err(TypeResolutionError::semantic)?,
                upper_bound: declaration
                    .upper_bound
                    .as_ref()
                    .map(|bound| crate::rewrite_dimension_references(bound, &old_to_new))
                    .transpose()
                    .map_err(TypeResolutionError::semantic)?,
            })
        })
        .collect::<Result<Vec<_>, TypeResolutionError>>()?
        .into_boxed_slice();
    let body = resolved_schema_body(
        resolved.kind(),
        template,
        resolved.dimension_parameters(),
        &old_to_new,
    )?;
    SchemaDraft {
        dimension_parameters,
        body,
    }
    .finalize()
    .map_err(TypeResolutionError::semantic)
}

fn collect_dynamic_extent_parameters(
    kind: &KindExpr,
    template: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
    parameters: &mut BTreeSet<crate::DimensionParameterId>,
) -> Result<(), TypeResolutionError> {
    match (kind, template) {
        (
            KindExpr::Matrix { element, .. },
            SchemaBody::Matrix {
                element: template, ..
            },
        )
        | (KindExpr::Option(element), SchemaBody::Option(template)) => {
            collect_dynamic_extent_parameters(element, template, declarations, parameters)?;
        }
        (KindExpr::Tuple(elements), SchemaBody::Tuple(templates)) => {
            if elements.len() != templates.len() {
                return Err(invalid_rule(
                    "tuple output does not match its schema template",
                ));
            }
            for (element, template) in elements.iter().zip(templates) {
                collect_dynamic_extent_parameters(element, template, declarations, parameters)?;
            }
        }
        (KindExpr::Record(fields), SchemaBody::Record(templates)) => {
            collect_dynamic_field_parameters(fields, templates, declarations, parameters)?;
        }
        (
            KindExpr::Table { columns, rows },
            SchemaBody::Table {
                columns: templates,
                rows: extent,
            },
        ) => {
            collect_dynamic_field_parameters(columns, templates, declarations, parameters)?;
            collect_dynamic_extent_parameter(rows, extent, declarations, parameters)?;
        }
        (
            KindExpr::Set {
                element,
                cardinality,
            },
            SchemaBody::Set {
                element: template,
                cardinality: extent,
            },
        ) => {
            collect_dynamic_extent_parameters(element, template, declarations, parameters)?;
            collect_dynamic_extent_parameter(cardinality, extent, declarations, parameters)?;
        }
        (
            KindExpr::Map {
                key,
                value,
                cardinality,
            },
            SchemaBody::Map {
                key: key_template,
                value: value_template,
                cardinality: extent,
            },
        ) => {
            collect_dynamic_extent_parameters(key, key_template, declarations, parameters)?;
            collect_dynamic_extent_parameters(value, value_template, declarations, parameters)?;
            collect_dynamic_extent_parameter(cardinality, extent, declarations, parameters)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_dynamic_field_parameters(
    fields: &[KindField],
    templates: &[SchemaField],
    declarations: &[DimensionParameterDeclaration],
    parameters: &mut BTreeSet<crate::DimensionParameterId>,
) -> Result<(), TypeResolutionError> {
    if fields.len() != templates.len() {
        return Err(invalid_rule(
            "record output does not match its schema template",
        ));
    }
    for (field, template) in fields.iter().zip(templates) {
        if field.name != template.name {
            return Err(invalid_rule(
                "record field does not match its schema template",
            ));
        }
        collect_dynamic_extent_parameters(&field.kind, &template.schema, declarations, parameters)?;
    }
    Ok(())
}

fn collect_dynamic_extent_parameter(
    dimension: &DimensionExpr,
    extent: &CardinalitySpec,
    declarations: &[DimensionParameterDeclaration],
    parameters: &mut BTreeSet<crate::DimensionParameterId>,
) -> Result<(), TypeResolutionError> {
    if collection_extent_is_dynamic(dimension, extent, declarations) {
        let DimensionExpr::Parameter(parameter) = dimension else {
            return Err(invalid_rule(format!(
                "dynamic collection extent requires one dimension parameter, received {dimension:?}"
            )));
        };
        parameters.insert(*parameter);
    }
    Ok(())
}

fn collection_extent_is_dynamic(
    dimension: &DimensionExpr,
    template: &CardinalitySpec,
    declarations: &[DimensionParameterDeclaration],
) -> bool {
    if matches!(template, CardinalitySpec::Dynamic { .. }) {
        return true;
    }
    let DimensionExpr::Parameter(parameter) = dimension else {
        return false;
    };
    declarations
        .get(parameter.get() as usize)
        .is_some_and(|declaration| declaration.lifetime == crate::DimensionLifetime::Turn)
}

fn schema_body_from_resolved(
    kind: &KindExpr,
    declarations: &[DimensionParameterDeclaration],
) -> Result<SchemaBody, TypeResolutionError> {
    Ok(match kind {
        KindExpr::Wildcard => SchemaBody::Dynamic,
        KindExpr::Named(id) => BuiltinScalarKind::from_kind_id(*id)
            .map(BuiltinScalarKind::schema_body)
            .ok_or_else(|| invalid_rule(format!("named kind {id:?} has no canonical schema")))?,
        KindExpr::Id => SchemaBody::Id,
        KindExpr::Index => SchemaBody::Index,
        KindExpr::Atom(key) => SchemaBody::Atom(*key),
        KindExpr::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(schema_body_from_resolved(element, declarations)?),
            dimensions: dimensions.clone(),
        },
        KindExpr::Option(payload) => {
            SchemaBody::Option(Box::new(schema_body_from_resolved(payload, declarations)?))
        }
        KindExpr::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| schema_body_from_resolved(element, declarations))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindExpr::Record(fields) => SchemaBody::Record(schema_fields(fields, declarations)?),
        KindExpr::Table { columns, rows } => SchemaBody::Table {
            columns: schema_fields(columns, declarations)?,
            rows: CardinalitySpec::Exact(rows.clone()),
        },
        KindExpr::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(schema_body_from_resolved(element, declarations)?),
            cardinality: CardinalitySpec::Exact(cardinality.clone()),
        },
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(schema_body_from_resolved(key, declarations)?),
            value: Box::new(schema_body_from_resolved(value, declarations)?),
            cardinality: CardinalitySpec::Exact(cardinality.clone()),
        },
        KindExpr::TypeOf(_) => SchemaBody::ReifiedType,
        KindExpr::Enum(_) => {
            return Err(invalid_rule(
                "nominal enum output requires FromInput or Declared schema authority",
            ));
        }
        KindExpr::Never | KindExpr::Hole | KindExpr::Parameter(_) | KindExpr::Reference(_) => {
            return Err(invalid_rule(format!(
                "closed output kind cannot be materialized: {kind:?}"
            )));
        }
    })
}

fn schema_fields(
    fields: &[KindField],
    declarations: &[DimensionParameterDeclaration],
) -> Result<Box<[SchemaField]>, TypeResolutionError> {
    fields
        .iter()
        .map(|field| {
            Ok(SchemaField {
                name: field.name.clone(),
                schema: schema_body_from_resolved(&field.kind, declarations)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn resolved_schema_body(
    kind: &KindExpr,
    template: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
    old_to_new: &[Option<crate::DimensionParameterId>],
) -> Result<SchemaBody, TypeResolutionError> {
    match (kind, template) {
        (KindExpr::Named(id), _) => BuiltinScalarKind::from_kind_id(*id)
            .map(BuiltinScalarKind::schema_body)
            .ok_or_else(|| invalid_rule(format!("named kind {id:?} has no canonical schema"))),
        (KindExpr::Id, _) => Ok(SchemaBody::Id),
        (KindExpr::Index, _) => Ok(SchemaBody::Index),
        (KindExpr::Atom(expected), SchemaBody::Atom(actual)) if expected == actual => {
            Ok(template.clone())
        }
        (KindExpr::Enum(expected), SchemaBody::Enum { key, .. }) if expected == key => {
            Ok(template.clone())
        }
        (
            KindExpr::Matrix {
                element,
                dimensions,
            },
            SchemaBody::Matrix {
                element: template_element,
                ..
            },
        ) => Ok(SchemaBody::Matrix {
            element: Box::new(resolved_schema_body(
                element,
                template_element,
                declarations,
                old_to_new,
            )?),
            dimensions: dimensions
                .iter()
                .map(|dimension| crate::rewrite_dimension_references(dimension, old_to_new))
                .collect::<Result<Vec<_>, _>>()
                .map_err(TypeResolutionError::semantic)?
                .into_boxed_slice(),
        }),
        (KindExpr::Option(payload), SchemaBody::Option(template_payload)) => {
            Ok(SchemaBody::Option(Box::new(resolved_schema_body(
                payload,
                template_payload,
                declarations,
                old_to_new,
            )?)))
        }
        (KindExpr::Tuple(elements), SchemaBody::Tuple(templates))
            if elements.len() == templates.len() =>
        {
            Ok(SchemaBody::Tuple(
                elements
                    .iter()
                    .zip(templates)
                    .map(|(element, template)| {
                        resolved_schema_body(element, template, declarations, old_to_new)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ))
        }
        (KindExpr::Record(fields), SchemaBody::Record(templates))
            if fields.len() == templates.len() =>
        {
            Ok(SchemaBody::Record(resolved_template_fields(
                fields,
                templates,
                declarations,
                old_to_new,
            )?))
        }
        (
            KindExpr::Table { columns, rows },
            SchemaBody::Table {
                columns: templates,
                rows: extent,
            },
        ) if columns.len() == templates.len() => Ok(SchemaBody::Table {
            columns: resolved_template_fields(columns, templates, declarations, old_to_new)?,
            rows: resolved_extent(rows, extent, declarations, old_to_new)?,
        }),
        (
            KindExpr::Set {
                element,
                cardinality,
            },
            SchemaBody::Set {
                element: template_element,
                cardinality: extent,
            },
        ) => Ok(SchemaBody::Set {
            element: Box::new(resolved_schema_body(
                element,
                template_element,
                declarations,
                old_to_new,
            )?),
            cardinality: resolved_extent(cardinality, extent, declarations, old_to_new)?,
        }),
        (
            KindExpr::Map {
                key,
                value,
                cardinality,
            },
            SchemaBody::Map {
                key: template_key,
                value: template_value,
                cardinality: extent,
            },
        ) => Ok(SchemaBody::Map {
            key: Box::new(resolved_schema_body(
                key,
                template_key,
                declarations,
                old_to_new,
            )?),
            value: Box::new(resolved_schema_body(
                value,
                template_value,
                declarations,
                old_to_new,
            )?),
            cardinality: resolved_extent(cardinality, extent, declarations, old_to_new)?,
        }),
        (KindExpr::TypeOf(_), SchemaBody::ReifiedType) => Ok(SchemaBody::ReifiedType),
        (KindExpr::Wildcard, SchemaBody::Dynamic) => Ok(SchemaBody::Dynamic),
        _ => Err(invalid_rule(format!(
            "resolved kind {kind:?} does not match schema template {template:?}"
        ))),
    }
}

fn resolved_template_fields(
    fields: &[KindField],
    templates: &[SchemaField],
    declarations: &[DimensionParameterDeclaration],
    old_to_new: &[Option<crate::DimensionParameterId>],
) -> Result<Box<[SchemaField]>, TypeResolutionError> {
    fields
        .iter()
        .zip(templates)
        .map(|(field, template)| {
            if field.name != template.name {
                return Err(invalid_rule(format!(
                    "field {:?} does not match template field {:?}",
                    field.name, template.name
                )));
            }
            Ok(SchemaField {
                name: field.name.clone(),
                schema: resolved_schema_body(
                    &field.kind,
                    &template.schema,
                    declarations,
                    old_to_new,
                )?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn resolved_extent(
    dimension: &DimensionExpr,
    template: &CardinalitySpec,
    declarations: &[DimensionParameterDeclaration],
    old_to_new: &[Option<crate::DimensionParameterId>],
) -> Result<CardinalitySpec, TypeResolutionError> {
    if !collection_extent_is_dynamic(dimension, template, declarations) {
        return Ok(CardinalitySpec::Exact(
            crate::rewrite_dimension_references(dimension, old_to_new)
                .map_err(TypeResolutionError::semantic)?,
        ));
    }
    let DimensionExpr::Parameter(parameter) = dimension else {
        return Err(invalid_rule(format!(
            "dynamic collection extent requires one dimension parameter, received {dimension:?}"
        )));
    };
    let declaration = declarations
        .get(parameter.get() as usize)
        .ok_or_else(|| invalid_rule(format!("unknown dimension {parameter:?}")))?;
    Ok(CardinalitySpec::Dynamic {
        upper_bound: declaration
            .upper_bound
            .as_ref()
            .map(|bound| crate::rewrite_dimension_references(bound, old_to_new))
            .transpose()
            .map_err(TypeResolutionError::semantic)?,
    })
}

fn shape_for_extents(
    schema: &Schema,
    current_extents: &[u64],
) -> Result<ShapeInstance, TypeResolutionError> {
    let mut values = vec![0; schema.dimension_parameters().len()];
    for (index, parameter) in schema.dimension_parameters().iter().enumerate() {
        values[index] = crate::evaluate_dimension(parameter.lower_bound(), &values[..index])
            .map_err(TypeResolutionError::semantic)?;
    }
    let declared = match schema.body() {
        SchemaBody::Matrix { dimensions, .. } => dimensions.as_ref(),
        SchemaBody::Table {
            rows: CardinalitySpec::Exact(rows),
            ..
        }
        | SchemaBody::Set {
            cardinality: CardinalitySpec::Exact(rows),
            ..
        }
        | SchemaBody::Map {
            cardinality: CardinalitySpec::Exact(rows),
            ..
        } => core::slice::from_ref(rows),
        _ => &[],
    };
    if declared.len() != current_extents.len() {
        return Err(invalid_rule(format!(
            "output declares {} current extents but {} were supplied",
            declared.len(),
            current_extents.len()
        )));
    }
    for (dimension, extent) in declared.iter().zip(current_extents.iter().copied()) {
        assign_dimension_witness(dimension, extent, &mut values)?;
    }
    schema
        .instantiate_shape(values.into_boxed_slice())
        .map_err(TypeResolutionError::semantic)
}

fn assign_dimension_witness(
    expression: &DimensionExpr,
    target: u64,
    values: &mut [u64],
) -> Result<(), TypeResolutionError> {
    match expression {
        DimensionExpr::Constant(expected) if *expected == target => Ok(()),
        DimensionExpr::Parameter(parameter) => {
            let value = values
                .get_mut(parameter.get() as usize)
                .ok_or_else(|| invalid_rule(format!("unknown dimension {parameter:?}")))?;
            *value = target;
            Ok(())
        }
        DimensionExpr::Add(operands) => {
            let Some((selected_index, selected)) = operands
                .iter()
                .enumerate()
                .find(|(_, operand)| dimension_has_parameter(operand))
            else {
                return dimension_witness_mismatch(expression, target, values);
            };
            let rest = operands
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != selected_index)
                .try_fold(0_u64, |sum, (_, operand)| {
                    crate::evaluate_dimension(operand, values)
                        .map_err(TypeResolutionError::semantic)
                        .and_then(|value| {
                            sum.checked_add(value).ok_or_else(|| {
                                TypeResolutionError::semantic(
                                    SemanticModelError::DimensionOverflowV1,
                                )
                            })
                        })
                })?;
            let selected_target = target.checked_sub(rest).ok_or_else(|| {
                TypeResolutionError::semantic(SemanticModelError::DimensionOverflowV1)
            })?;
            assign_dimension_witness(selected, selected_target, values)?;
            dimension_witness_mismatch(expression, target, values)
        }
        DimensionExpr::Multiply(operands) => {
            if target == 0
                && crate::evaluate_dimension(expression, values)
                    .map_err(TypeResolutionError::semantic)?
                    == 0
            {
                return Ok(());
            }
            let adjustable = operands
                .iter()
                .enumerate()
                .filter(|(_, operand)| dimension_has_parameter(operand))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let Some(selected_index) = adjustable.first().copied() else {
                return dimension_witness_mismatch(expression, target, values);
            };
            for index in adjustable.into_iter().skip(1) {
                if crate::evaluate_dimension(&operands[index], values)
                    .map_err(TypeResolutionError::semantic)?
                    == 0
                {
                    assign_dimension_witness(&operands[index], 1, values)?;
                }
            }
            let rest = operands
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != selected_index)
                .try_fold(1_u64, |product, (_, operand)| {
                    crate::evaluate_dimension(operand, values)
                        .map_err(TypeResolutionError::semantic)
                        .and_then(|value| {
                            product.checked_mul(value).ok_or_else(|| {
                                TypeResolutionError::semantic(
                                    SemanticModelError::DimensionOverflowV1,
                                )
                            })
                        })
                })?;
            if rest == 0 || target % rest != 0 {
                return dimension_witness_mismatch(expression, target, values);
            }
            assign_dimension_witness(&operands[selected_index], target / rest, values)?;
            dimension_witness_mismatch(expression, target, values)
        }
        DimensionExpr::Min(operands) => {
            for operand in operands {
                let actual = crate::evaluate_dimension(operand, values)
                    .map_err(TypeResolutionError::semantic)?;
                if actual < target {
                    assign_dimension_witness(operand, target, values)?;
                }
            }
            dimension_witness_mismatch(expression, target, values)
        }
        DimensionExpr::Max(operands) => {
            if operands.iter().any(|operand| {
                crate::evaluate_dimension(operand, values).is_ok_and(|actual| actual > target)
            }) {
                return dimension_witness_mismatch(expression, target, values);
            }
            let Some(selected) = operands
                .iter()
                .find(|operand| dimension_has_parameter(operand))
            else {
                return dimension_witness_mismatch(expression, target, values);
            };
            assign_dimension_witness(selected, target, values)?;
            dimension_witness_mismatch(expression, target, values)
        }
        _ => dimension_witness_mismatch(expression, target, values),
    }
}

fn dimension_witness_mismatch(
    expression: &DimensionExpr,
    target: u64,
    values: &[u64],
) -> Result<(), TypeResolutionError> {
    let actual =
        crate::evaluate_dimension(expression, values).map_err(TypeResolutionError::semantic)?;
    if actual == target {
        Ok(())
    } else {
        Err(TypeResolutionError::incompatible(
            "resolved output descriptor",
            TypeConstraintFailure::IncompatibleDimensions {
                expected: format!("{actual}"),
                actual: format!("{target}"),
            },
        ))
    }
}

fn dimension_has_parameter(expression: &DimensionExpr) -> bool {
    match expression {
        DimensionExpr::Parameter(_) => true,
        DimensionExpr::Add(operands)
        | DimensionExpr::Multiply(operands)
        | DimensionExpr::Min(operands)
        | DimensionExpr::Max(operands) => operands.iter().any(dimension_has_parameter),
        DimensionExpr::Hole | DimensionExpr::Constant(_) => false,
    }
}

fn descriptor_mismatch(expected: &ResolvedType, actual: &ResolvedType) -> TypeResolutionError {
    TypeResolutionError::incompatible(
        "resolved value descriptor",
        TypeConstraintFailure::OutputTypeMismatch {
            expected: expected.semantic_name(),
            actual: actual.semantic_name(),
        },
    )
}

fn invalid_rule(reason: impl Into<String>) -> TypeResolutionError {
    TypeResolutionError::incompatible(
        "resolved output descriptor",
        TypeConstraintFailure::InvalidScheme {
            reason: reason.into(),
        },
    )
}
