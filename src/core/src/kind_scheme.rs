//! Quantified semantic kind schemes.

use crate::dimension::{canonicalize_dimension_environment, normalize_dimension};
use crate::kind_expr::{validate_kind_structure, visit_kind_dimensions, visit_kind_parameters};
use crate::{
    DimensionExpr, DimensionParameterDeclaration, DimensionParameterId, KindExpr, KindParameterId,
    SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KindScheme {
    kind_parameters: Box<[KindParameter]>,
    dimension_parameters: Box<[DimensionParameterDeclaration]>,
    inputs: InputKindScheme,
    outputs: Box<[KindExpr]>,
    constraints: Box<[KindConstraint]>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KindParameter {
    pub id: KindParameterId,
    pub upper_bound: Option<KindExpr>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputKindScheme {
    Fixed(Box<[KindExpr]>),
    Variadic {
        prefix: Box<[KindExpr]>,
        repeated: KindExpr,
        min_repetitions: u32,
    },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KindConstraint {
    Equal(KindExpr, KindExpr),
    Convertible(KindExpr, KindExpr),
    Keyable(KindExpr),
    DimensionEqual(DimensionExpr, DimensionExpr),
    DimensionLessEqual(DimensionExpr, DimensionExpr),
}

impl KindScheme {
    pub fn new(
        kind_parameters: Box<[KindParameter]>,
        dimension_parameters: Box<[DimensionParameterDeclaration]>,
        inputs: InputKindScheme,
        outputs: Box<[KindExpr]>,
        constraints: Box<[KindConstraint]>,
    ) -> Result<Self, SemanticModelError> {
        validate_kind_parameter_declarations(&kind_parameters)?;
        let all_dimensions = (0..dimension_parameters.len())
            .map(|index| DimensionParameterId::new(index as u32))
            .collect::<Vec<_>>();
        canonicalize_dimension_environment(&dimension_parameters, &all_dimensions)?;

        let kind_count = kind_parameters.len();
        let dimension_count = dimension_parameters.len();
        for (index, parameter) in kind_parameters.iter().enumerate() {
            if let Some(upper_bound) = &parameter.upper_bound {
                validate_kind(
                    upper_bound,
                    kind_count,
                    dimension_count,
                    Some(KindParameterId::new(index as u32)),
                )?;
            }
        }
        match &inputs {
            InputKindScheme::Fixed(inputs) => {
                for input in inputs {
                    validate_kind(input, kind_count, dimension_count, None)?;
                }
            }
            InputKindScheme::Variadic {
                prefix, repeated, ..
            } => {
                for input in prefix {
                    validate_kind(input, kind_count, dimension_count, None)?;
                }
                validate_kind(repeated, kind_count, dimension_count, None)?;
            }
        }
        for output in &outputs {
            validate_kind(output, kind_count, dimension_count, None)?;
        }
        for constraint in &constraints {
            validate_constraint(constraint, kind_count, dimension_count)?;
        }

        Ok(Self {
            kind_parameters,
            dimension_parameters,
            inputs,
            outputs,
            constraints,
        })
    }

    pub fn kind_parameters(&self) -> &[KindParameter] {
        &self.kind_parameters
    }

    pub fn dimension_parameters(&self) -> &[DimensionParameterDeclaration] {
        &self.dimension_parameters
    }

    pub const fn inputs(&self) -> &InputKindScheme {
        &self.inputs
    }

    pub fn outputs(&self) -> &[KindExpr] {
        &self.outputs
    }

    pub fn constraints(&self) -> &[KindConstraint] {
        &self.constraints
    }
}

fn validate_kind_parameter_declarations(
    parameters: &[KindParameter],
) -> Result<(), SemanticModelError> {
    let mut seen = BTreeSet::new();
    for (index, parameter) in parameters.iter().enumerate() {
        if !seen.insert(parameter.id) {
            return Err(SemanticModelError::DuplicateKindParameter { id: parameter.id });
        }
        if parameter.id.get() as usize != index {
            return Err(SemanticModelError::UnknownKindParameter { id: parameter.id });
        }
    }
    Ok(())
}

fn validate_kind(
    kind: &KindExpr,
    kind_count: usize,
    dimension_count: usize,
    upper_bound_of: Option<KindParameterId>,
) -> Result<(), SemanticModelError> {
    validate_kind_structure(kind)?;
    visit_kind_parameters(kind, &mut |referenced| {
        if referenced.get() as usize >= kind_count {
            return Err(SemanticModelError::UnknownKindParameter { id: referenced });
        }
        if let Some(parameter) = upper_bound_of {
            if referenced.get() >= parameter.get() {
                return Err(SemanticModelError::ForwardKindParameterReference {
                    parameter,
                    referenced,
                });
            }
        }
        Ok(())
    })?;
    visit_kind_dimensions(kind, &mut |dimension| {
        normalize_dimension(dimension, dimension_count).map(|_| ())
    })
}

fn validate_constraint(
    constraint: &KindConstraint,
    kind_count: usize,
    dimension_count: usize,
) -> Result<(), SemanticModelError> {
    match constraint {
        KindConstraint::Equal(left, right) | KindConstraint::Convertible(left, right) => {
            validate_kind(left, kind_count, dimension_count, None)?;
            validate_kind(right, kind_count, dimension_count, None)
        }
        KindConstraint::Keyable(kind) => validate_kind(kind, kind_count, dimension_count, None),
        KindConstraint::DimensionEqual(left, right)
        | KindConstraint::DimensionLessEqual(left, right) => {
            normalize_dimension(left, dimension_count)?;
            normalize_dimension(right, dimension_count)?;
            Ok(())
        }
    }
}
