use super::{Schema, SchemaBody};
use crate::{DimensionExpr, DimensionOperator, DimensionParameterId, SemanticModelError};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeInstance {
    parameter_values: Box<[u64]>,
}

impl Schema {
    pub fn instantiate_shape(
        &self,
        parameter_values: Box<[u64]>,
    ) -> Result<ShapeInstance, SemanticModelError> {
        if parameter_values.len() != self.dimension_parameters.len() {
            return Err(SemanticModelError::ShapeParameterCountMismatchV1 {
                expected: self.dimension_parameters.len() as u32,
                actual: parameter_values.len() as u32,
            });
        }
        for (index, (parameter, value)) in self
            .dimension_parameters
            .iter()
            .zip(parameter_values.iter().copied())
            .enumerate()
        {
            let resolved = &parameter_values[..index];
            let lower = evaluate_dimension(parameter.lower_bound(), resolved)?;
            let upper = parameter
                .upper_bound()
                .map(|bound| evaluate_dimension(bound, resolved))
                .transpose()?;
            if value < lower || upper.is_some_and(|upper| value > upper) {
                return Err(SemanticModelError::ShapeBoundViolationV1 {
                    parameter: DimensionParameterId::new(index as u32),
                    value,
                    lower,
                    upper,
                });
            }
        }
        evaluate_body_extents(&self.body, &parameter_values)?;
        Ok(ShapeInstance { parameter_values })
    }
}

impl ShapeInstance {
    pub fn parameter_values(&self) -> &[u64] {
        &self.parameter_values
    }

    pub fn canonical_bytes(&self) -> Box<[u8]> {
        let mut bytes = Vec::new();
        bytes.push(0x01);
        bytes.extend_from_slice(&(self.parameter_values.len() as u32).to_le_bytes());
        for value in &self.parameter_values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.into_boxed_slice()
    }

    /// Resolves a schema dimension against this validated shape instance.
    pub fn resolve_dimension(&self, expression: &DimensionExpr) -> Result<u64, SemanticModelError> {
        evaluate_dimension(expression, &self.parameter_values)
    }
}

pub(crate) fn evaluate_dimension(
    expression: &DimensionExpr,
    values: &[u64],
) -> Result<u64, SemanticModelError> {
    match expression {
        DimensionExpr::Hole => Err(SemanticModelError::UnresolvedDimensionHole),
        DimensionExpr::Constant(value) => Ok(*value),
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or(SemanticModelError::UnknownDimensionParameterV1 { id: *id }),
        DimensionExpr::Add(operands) => {
            let mut result = 0_u64;
            for operand in operands {
                result = result
                    .checked_add(evaluate_dimension(operand, values)?)
                    .ok_or(SemanticModelError::DimensionOverflowV1)?;
            }
            Ok(result)
        }
        DimensionExpr::Multiply(operands) => {
            let mut result = 1_u64;
            for operand in operands {
                result = result
                    .checked_mul(evaluate_dimension(operand, values)?)
                    .ok_or(SemanticModelError::DimensionOverflowV1)?;
            }
            Ok(result)
        }
        DimensionExpr::Min(operands) => evaluate_min_max(DimensionOperator::Min, operands, values),
        DimensionExpr::Max(operands) => evaluate_min_max(DimensionOperator::Max, operands, values),
    }
}

fn evaluate_min_max(
    operator: DimensionOperator,
    operands: &[DimensionExpr],
    values: &[u64],
) -> Result<u64, SemanticModelError> {
    let mut operands = operands.iter();
    let first = operands
        .next()
        .ok_or(SemanticModelError::EmptyMinMaxV1 { operator })?;
    let mut result = evaluate_dimension(first, values)?;
    for operand in operands {
        let operand = evaluate_dimension(operand, values)?;
        result = match operator {
            DimensionOperator::Min => result.min(operand),
            DimensionOperator::Max => result.max(operand),
            DimensionOperator::Add | DimensionOperator::Multiply => unreachable!(),
        };
    }
    Ok(result)
}

fn evaluate_body_extents(body: &SchemaBody, values: &[u64]) -> Result<(), SemanticModelError> {
    match body {
        SchemaBody::Enum { variants, .. } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    evaluate_body_extents(payload, values)?;
                }
            }
        }
        SchemaBody::Option(element) => evaluate_body_extents(element, values)?,
        SchemaBody::Tuple(elements) => {
            for element in elements {
                evaluate_body_extents(element, values)?;
            }
        }
        SchemaBody::Record(fields) => {
            for field in fields {
                evaluate_body_extents(&field.schema, values)?;
            }
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            evaluate_body_extents(element, values)?;
            let mut element_count = 1_u64;
            for dimension in dimensions {
                let extent = evaluate_dimension(dimension, values)?;
                element_count = element_count
                    .checked_mul(extent)
                    .ok_or(SemanticModelError::DimensionOverflowV1)?;
            }
        }
        SchemaBody::Table { columns, rows } => {
            for column in columns {
                evaluate_body_extents(&column.schema, values)?;
            }
            evaluate_extent(rows, values)?;
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            evaluate_body_extents(element, values)?;
            evaluate_extent(cardinality, values)?;
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            evaluate_body_extents(key, values)?;
            evaluate_body_extents(value, values)?;
            evaluate_extent(cardinality, values)?;
        }
        SchemaBody::Bool
        | SchemaBody::Dynamic
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => {}
    }
    Ok(())
}

fn evaluate_extent(extent: &crate::ExtentSpec, values: &[u64]) -> Result<(), SemanticModelError> {
    match extent {
        crate::ExtentSpec::Exact(value)
        | crate::ExtentSpec::Dynamic {
            upper_bound: Some(value),
        } => {
            evaluate_dimension(value, values)?;
        }
        crate::ExtentSpec::Dynamic { upper_bound: None } => {}
    }
    Ok(())
}
