//! Dimension relations without value conversion or runtime shape authority.
use super::{Schema, SchemaBody, SchemaDraft};
use crate::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin,
};
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaDimensionRelation {
    Exact,
    /// Rename only independent, unbounded Turn axes of unsigned matrices.
    /// Scalar siblings retain their exact kinds; this never permits reshaping.
    FreeTurnUnsignedAxes,
}

/// A bijection between the finalized schemas' parameter identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionParameterMapping {
    targets: Box<[DimensionParameterId]>,
}
impl DimensionParameterMapping {
    pub fn target(&self, source: DimensionParameterId) -> Option<DimensionParameterId> {
        self.targets.get(source.get() as usize).copied()
    }
}

fn free_turn(
    lifetime: DimensionLifetime,
    lower: &DimensionExpr,
    upper: Option<&DimensionExpr>,
) -> bool {
    lifetime == DimensionLifetime::Turn && lower == &DimensionExpr::Constant(0) && upper.is_none()
}
fn free_turn_declarations(parameters: &[DimensionParameterDeclaration]) -> bool {
    parameters
        .iter()
        .all(|p| free_turn(p.lifetime, &p.lower_bound, p.upper_bound.as_ref()))
}

/// Compare geometry under an explicit relation, returning the actual parameter
/// mapping. Fixed axes, repeated axes, bounds, lifetimes and element kinds are
/// preserved. Unsupported structures retain their existing conversion path.
pub fn schema_dimension_parameter_mapping(
    actual: &Schema,
    expected: &Schema,
    relation: SchemaDimensionRelation,
) -> Option<DimensionParameterMapping> {
    let count = actual.dimension_parameters().len();
    if actual == expected {
        return Some(DimensionParameterMapping {
            targets: (0..count)
                .map(|i| DimensionParameterId::new(i as u32))
                .collect(),
        });
    }
    if relation == SchemaDimensionRelation::Exact
        || count == 0
        || count != expected.dimension_parameters().len()
        || !actual
            .dimension_parameters()
            .iter()
            .chain(expected.dimension_parameters())
            .all(|p| free_turn(p.lifetime(), p.lower_bound(), p.upper_bound()))
    {
        return None;
    }
    let mut mapping = Renaming {
        forward: vec![None; count],
        reverse: vec![None; count],
    };
    if !mapping.same_body(actual.body(), expected.body())
        || mapping.reverse.iter().any(Option::is_none)
    {
        return None;
    }
    Some(DimensionParameterMapping {
        targets: mapping
            .forward
            .into_iter()
            .map(|p| p.map(|p| DimensionParameterId::new(p as u32)))
            .collect::<Option<Box<[_]>>>()?,
    })
}

struct Renaming {
    forward: Vec<Option<usize>>,
    reverse: Vec<Option<usize>>,
}
impl Renaming {
    fn same_axis(&mut self, actual: &DimensionExpr, expected: &DimensionExpr) -> bool {
        match (actual, expected) {
            (DimensionExpr::Constant(a), DimensionExpr::Constant(b)) => a == b,
            (DimensionExpr::Parameter(a), DimensionExpr::Parameter(b)) => {
                let (a, b) = (a.get() as usize, b.get() as usize);
                let (Some(forward), Some(reverse)) =
                    (self.forward.get_mut(a), self.reverse.get_mut(b))
                else {
                    return false;
                };
                if forward.is_some_and(|v| v != b) || reverse.is_some_and(|v| v != a) {
                    return false;
                }
                *forward = Some(b);
                *reverse = Some(a);
                true
            }
            _ => false,
        }
    }
    fn same_body(&mut self, actual: &SchemaBody, expected: &SchemaBody) -> bool {
        match (actual, expected) {
            (
                SchemaBody::Matrix {
                    element: a,
                    dimensions: ad,
                },
                SchemaBody::Matrix {
                    element: b,
                    dimensions: bd,
                },
            ) if a == b
                && ad == bd
                && ad.iter().all(|d| matches!(d, DimensionExpr::Constant(_)))
                && (SchemaDraft {
                    body: a.as_ref().clone(),
                    dimension_parameters: Box::new([]),
                })
                .finalize()
                .is_ok() =>
            {
                true
            }
            (
                SchemaBody::Matrix {
                    element: a,
                    dimensions: ad,
                },
                SchemaBody::Matrix {
                    element: b,
                    dimensions: bd,
                },
            ) if a == b && matches!(a.as_ref(), SchemaBody::UnsignedInteger(_)) => {
                ad.len() == bd.len() && ad.iter().zip(bd).all(|(a, b)| self.same_axis(a, b))
            }
            (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| self.same_body(a, b))
            }
            (SchemaBody::Record(a), SchemaBody::Record(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.name == b.name && self.same_body(&a.schema, &b.schema))
            }
            (
                SchemaBody::Bool
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_),
                _,
            ) => actual == expected,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DimensionOriginPolicy {
    Preserve,
    /// Let canonicalization assign first-use identities to unconstrained Turn
    /// axes. Actual embedding still assigns fresh IDs using its parameter map.
    InferFreeTurnAxes,
}
impl SchemaDraft {
    pub fn with_dimension_origin_policy(mut self, policy: DimensionOriginPolicy) -> Self {
        if policy == DimensionOriginPolicy::InferFreeTurnAxes
            && free_turn_declarations(&self.dimension_parameters)
        {
            for parameter in &mut self.dimension_parameters {
                parameter.origin = DimensionParameterOrigin::Inferred;
            }
        }
        self
    }
}

fn has_parameter(dimension: &DimensionExpr) -> bool {
    match dimension {
        DimensionExpr::Parameter(_) => true,
        DimensionExpr::Add(xs)
        | DimensionExpr::Multiply(xs)
        | DimensionExpr::Min(xs)
        | DimensionExpr::Max(xs) => xs.iter().any(has_parameter),
        _ => false,
    }
}
fn computed(dimension: &DimensionExpr) -> bool {
    !matches!(dimension, DimensionExpr::Parameter(_)) && has_parameter(dimension)
}

/// Generalize computed numeric-matrix axes, separately from equivalence and
/// freshening. Equal expressions share one fresh parameter; constants and
/// existing parameter relationships remain. Nested element shapes, bounds and
/// persistent axes are not generalized.
pub fn generalize_computed_matrix_axes(source: &SchemaDraft) -> Option<SchemaDraft> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = &source.body
    else {
        return None;
    };
    if !free_turn_declarations(&source.dimension_parameters)
        || !dimensions.iter().any(computed)
        || !matches!(
            element.as_ref(),
            SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Bool
        )
    {
        return None;
    }
    let mut parameters = source.dimension_parameters.to_vec();
    let mut mapping = Vec::<(DimensionExpr, DimensionParameterId)>::new();
    let dimensions = dimensions
        .iter()
        .map(|d| {
            if !computed(d) {
                return Some(d.clone());
            }
            if let Some((_, id)) = mapping.iter().find(|(old, _)| old == d) {
                return Some(DimensionExpr::Parameter(*id));
            }
            let id = DimensionParameterId::new(u32::try_from(parameters.len()).ok()?);
            parameters.push(DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            });
            mapping.push((d.clone(), id));
            Some(DimensionExpr::Parameter(id))
        })
        .collect::<Option<Box<[_]>>>()?;
    Some(SchemaDraft {
        body: SchemaBody::Matrix {
            element: element.clone(),
            dimensions,
        },
        dimension_parameters: parameters.into_boxed_slice(),
    })
}
