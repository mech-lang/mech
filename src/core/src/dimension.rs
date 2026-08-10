//! Symbolic dimensions and canonical dimension environments.

use crate::{DimensionParameterId, SemanticIdentityKind, SemanticModelError};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeSet, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, vec, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DimensionLifetime {
    CompileTime,
    Activation,
    Turn,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DimensionParameterOrigin {
    Explicit,
    Inferred,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DimensionOperator {
    Add,
    Multiply,
    Min,
    Max,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DimensionExpr {
    Hole,
    Constant(u64),
    Parameter(DimensionParameterId),
    Add(Box<[DimensionExpr]>),
    Multiply(Box<[DimensionExpr]>),
    Min(Box<[DimensionExpr]>),
    Max(Box<[DimensionExpr]>),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionParameterDeclaration {
    pub id: DimensionParameterId,
    pub origin: DimensionParameterOrigin,
    pub lifetime: DimensionLifetime,
    pub lower_bound: DimensionExpr,
    pub upper_bound: Option<DimensionExpr>,
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionParameter {
    lifetime: DimensionLifetime,
    lower_bound: DimensionExpr,
    upper_bound: Option<DimensionExpr>,
}

impl DimensionParameter {
    pub const fn lifetime(&self) -> DimensionLifetime {
        self.lifetime
    }

    pub const fn lower_bound(&self) -> &DimensionExpr {
        &self.lower_bound
    }

    pub const fn upper_bound(&self) -> Option<&DimensionExpr> {
        self.upper_bound.as_ref()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtentEvolution {
    Fixed,
    ActivationFixed,
    TurnBounded,
    TurnUnbounded,
}

#[derive(Clone, Debug, Default)]
pub struct DimensionEnvironmentBuilder {
    declarations: Vec<DimensionParameterDeclaration>,
}

impl DimensionEnvironmentBuilder {
    pub const fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    pub fn declare(
        &mut self,
        origin: DimensionParameterOrigin,
        lifetime: DimensionLifetime,
        lower_bound: DimensionExpr,
        upper_bound: Option<DimensionExpr>,
    ) -> Result<DimensionParameterId, SemanticModelError> {
        self.declare_with_limit(origin, lifetime, lower_bound, upper_bound, usize::MAX)
    }

    fn declare_with_limit(
        &mut self,
        origin: DimensionParameterOrigin,
        lifetime: DimensionLifetime,
        lower_bound: DimensionExpr,
        upper_bound: Option<DimensionExpr>,
        parameter_limit: usize,
    ) -> Result<DimensionParameterId, SemanticModelError> {
        if lifetime == DimensionLifetime::CompileTime {
            return Err(SemanticModelError::CompileTimeDimensionParameterV1);
        }
        if self.declarations.len() >= parameter_limit {
            return Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::DimensionParameterId,
            });
        }
        let ordinal = u32::try_from(self.declarations.len()).map_err(|_| {
            SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::DimensionParameterId,
            }
        })?;
        let id = DimensionParameterId::new(ordinal);
        self.declarations.push(DimensionParameterDeclaration {
            id,
            origin,
            lifetime,
            lower_bound,
            upper_bound,
        });
        Ok(id)
    }

    pub fn declarations(&self) -> &[DimensionParameterDeclaration] {
        &self.declarations
    }

    pub fn into_declarations(self) -> Box<[DimensionParameterDeclaration]> {
        self.declarations.into_boxed_slice()
    }
}

pub(crate) struct CanonicalDimensionEnvironment {
    pub(crate) parameters: Box<[DimensionParameter]>,
    pub(crate) old_to_new: Vec<Option<DimensionParameterId>>,
}

pub(crate) fn collect_dimension_references(
    expression: &DimensionExpr,
    references: &mut Vec<DimensionParameterId>,
) {
    match expression {
        DimensionExpr::Hole | DimensionExpr::Constant(_) => {}
        DimensionExpr::Parameter(id) => references.push(*id),
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            for child in children {
                collect_dimension_references(child, references);
            }
        }
    }
}

pub(crate) fn rewrite_dimension_references(
    expression: &DimensionExpr,
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<DimensionExpr, SemanticModelError> {
    Ok(match expression {
        DimensionExpr::Hole => DimensionExpr::Hole,
        DimensionExpr::Constant(value) => DimensionExpr::Constant(*value),
        DimensionExpr::Parameter(id) => DimensionExpr::Parameter(
            old_to_new
                .get(id.get() as usize)
                .and_then(|value| *value)
                .ok_or(SemanticModelError::UnknownDimensionParameterV1 { id: *id })?,
        ),
        DimensionExpr::Add(children) => DimensionExpr::Add(rewrite_children(children, old_to_new)?),
        DimensionExpr::Multiply(children) => {
            DimensionExpr::Multiply(rewrite_children(children, old_to_new)?)
        }
        DimensionExpr::Min(children) => DimensionExpr::Min(rewrite_children(children, old_to_new)?),
        DimensionExpr::Max(children) => DimensionExpr::Max(rewrite_children(children, old_to_new)?),
    })
}

fn rewrite_children(
    children: &[DimensionExpr],
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<Box<[DimensionExpr]>, SemanticModelError> {
    children
        .iter()
        .map(|child| rewrite_dimension_references(child, old_to_new))
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

pub(crate) fn canonicalize_dimension_environment(
    declarations: &[DimensionParameterDeclaration],
    root_references: &[DimensionParameterId],
) -> Result<CanonicalDimensionEnvironment, SemanticModelError> {
    validate_declaration_ids(declarations)?;
    let count = declarations.len();
    let mut normalized_bounds = Vec::with_capacity(count);
    let mut dependencies = Vec::with_capacity(count);
    for declaration in declarations {
        if declaration.lifetime == DimensionLifetime::CompileTime {
            return Err(SemanticModelError::CompileTimeDimensionParameterV1);
        }
        let lower = normalize_dimension(&declaration.lower_bound, count)?;
        let upper = declaration
            .upper_bound
            .as_ref()
            .map(|value| normalize_dimension(value, count))
            .transpose()?;
        let mut references = Vec::new();
        collect_dimension_references(&lower, &mut references);
        if let Some(upper) = &upper {
            collect_dimension_references(upper, &mut references);
        }
        normalized_bounds.push((lower, upper));
        dependencies.push(references);
    }
    ensure_known_references(root_references, count)?;

    let mut state = vec![0_u8; count];
    let mut occurrence = Vec::new();
    for id in root_references {
        visit_parameter(
            id.get() as usize,
            &dependencies,
            &mut state,
            &mut occurrence,
        )?;
    }
    let reachable = occurrence.iter().copied().collect::<BTreeSet<_>>();
    let mut retained = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            (declaration.origin == DimensionParameterOrigin::Explicit && reachable.contains(&index))
                .then_some(index)
        })
        .collect::<Vec<_>>();
    for old in occurrence {
        if declarations[old].origin == DimensionParameterOrigin::Inferred
            && !retained.contains(&old)
        {
            retained.push(old);
        }
    }

    let mut old_to_new = vec![None; count];
    for (new, old) in retained.iter().copied().enumerate() {
        old_to_new[old] = Some(DimensionParameterId::new(new as u32));
    }
    for old in retained.iter().copied() {
        let parameter = old_to_new[old].expect("retained parameter has an ordinal");
        for referenced in dependencies[old].iter().copied() {
            let referenced_new =
                old_to_new[referenced.get() as usize].expect("reachable dependency has an ordinal");
            if referenced_new.get() >= parameter.get() {
                return Err(SemanticModelError::ForwardDimensionParameterReferenceV1 {
                    parameter,
                    referenced: referenced_new,
                });
            }
        }
    }

    let retained_count = retained.len();
    let mut parameters = Vec::with_capacity(retained_count);
    for old in retained {
        let declaration = &declarations[old];
        let (normalized_lower, normalized_upper) = &normalized_bounds[old];
        let lower = rewrite_dimension_references(normalized_lower, &old_to_new)?;
        let upper = normalized_upper
            .as_ref()
            .map(|value| rewrite_dimension_references(value, &old_to_new))
            .transpose()?;
        parameters.push(DimensionParameter {
            lifetime: declaration.lifetime,
            lower_bound: normalize_dimension(&lower, retained_count)?,
            upper_bound: upper
                .as_ref()
                .map(|value| normalize_dimension(value, retained_count))
                .transpose()?,
        });
    }
    Ok(CanonicalDimensionEnvironment {
        parameters: parameters.into_boxed_slice(),
        old_to_new,
    })
}

pub(crate) fn validate_declaration_ids(
    declarations: &[DimensionParameterDeclaration],
) -> Result<(), SemanticModelError> {
    let mut seen = BTreeSet::new();
    for (index, declaration) in declarations.iter().enumerate() {
        if !seen.insert(declaration.id) {
            return Err(SemanticModelError::DuplicateDimensionParameter { id: declaration.id });
        }
        if declaration.id.get() as usize != index {
            return Err(SemanticModelError::UnknownDimensionParameterV1 { id: declaration.id });
        }
    }
    Ok(())
}

fn ensure_known_references(
    references: &[DimensionParameterId],
    count: usize,
) -> Result<(), SemanticModelError> {
    for id in references {
        if id.get() as usize >= count {
            return Err(SemanticModelError::UnknownDimensionParameterV1 { id: *id });
        }
    }
    Ok(())
}

fn visit_parameter(
    ordinal: usize,
    dependencies: &[Vec<DimensionParameterId>],
    state: &mut [u8],
    occurrence: &mut Vec<usize>,
) -> Result<(), SemanticModelError> {
    match state[ordinal] {
        1 => return Err(SemanticModelError::CyclicDimensionParameterBoundsV1),
        2 => return Ok(()),
        _ => {}
    }
    state[ordinal] = 1;
    occurrence.push(ordinal);
    for dependency in &dependencies[ordinal] {
        visit_parameter(dependency.get() as usize, dependencies, state, occurrence)?;
    }
    state[ordinal] = 2;
    Ok(())
}

pub fn normalize_dimension(
    expression: &DimensionExpr,
    parameter_count: usize,
) -> Result<DimensionExpr, SemanticModelError> {
    match expression {
        DimensionExpr::Hole => Err(SemanticModelError::UnresolvedDimensionHole),
        DimensionExpr::Constant(value) => Ok(DimensionExpr::Constant(*value)),
        DimensionExpr::Parameter(id) => {
            if id.get() as usize >= parameter_count {
                Err(SemanticModelError::UnknownDimensionParameterV1 { id: *id })
            } else {
                Ok(DimensionExpr::Parameter(*id))
            }
        }
        DimensionExpr::Add(children) => normalize_sum(children, parameter_count),
        DimensionExpr::Multiply(children) => normalize_product(children, parameter_count),
        DimensionExpr::Min(children) => {
            normalize_min_max(DimensionOperator::Min, children, parameter_count)
        }
        DimensionExpr::Max(children) => {
            normalize_min_max(DimensionOperator::Max, children, parameter_count)
        }
    }
}

fn normalize_sum(
    children: &[DimensionExpr],
    parameter_count: usize,
) -> Result<DimensionExpr, SemanticModelError> {
    let mut operands = Vec::new();
    for child in children {
        match normalize_dimension(child, parameter_count)? {
            DimensionExpr::Add(nested) => operands.extend(Vec::from(nested)),
            other => operands.push(other),
        }
    }
    let mut constant = 0_u64;
    let mut remaining = Vec::new();
    for operand in operands {
        if let DimensionExpr::Constant(value) = operand {
            constant = constant
                .checked_add(value)
                .ok_or(SemanticModelError::DimensionOverflowV1)?;
        } else {
            remaining.push(operand);
        }
    }
    if constant != 0 {
        remaining.push(DimensionExpr::Constant(constant));
    }
    finish_commutative(DimensionOperator::Add, remaining)
}

fn normalize_product(
    children: &[DimensionExpr],
    parameter_count: usize,
) -> Result<DimensionExpr, SemanticModelError> {
    let mut operands = Vec::new();
    for child in children {
        match normalize_dimension(child, parameter_count)? {
            DimensionExpr::Multiply(nested) => operands.extend(Vec::from(nested)),
            other => operands.push(other),
        }
    }
    if operands
        .iter()
        .any(|operand| matches!(operand, DimensionExpr::Constant(0)))
    {
        return Ok(DimensionExpr::Constant(0));
    }
    let mut constant = 1_u64;
    let mut remaining = Vec::new();
    for operand in operands {
        if let DimensionExpr::Constant(value) = operand {
            constant = constant
                .checked_mul(value)
                .ok_or(SemanticModelError::DimensionOverflowV1)?;
        } else {
            remaining.push(operand);
        }
    }
    if constant != 1 {
        remaining.push(DimensionExpr::Constant(constant));
    }
    finish_commutative(DimensionOperator::Multiply, remaining)
}

fn normalize_min_max(
    operator: DimensionOperator,
    children: &[DimensionExpr],
    parameter_count: usize,
) -> Result<DimensionExpr, SemanticModelError> {
    let mut operands = Vec::new();
    for child in children {
        let normalized = normalize_dimension(child, parameter_count)?;
        match (operator, normalized) {
            (DimensionOperator::Min, DimensionExpr::Min(nested))
            | (DimensionOperator::Max, DimensionExpr::Max(nested)) => {
                operands.extend(Vec::from(nested))
            }
            (_, other) => operands.push(other),
        }
    }
    operands.sort_by_key(encode_normalized_dimension);
    operands.dedup_by(|left, right| {
        encode_normalized_dimension(left) == encode_normalized_dimension(right)
    });
    if operands.is_empty() {
        return Err(SemanticModelError::EmptyMinMaxV1 { operator });
    }
    finish_commutative(operator, operands)
}

fn finish_commutative(
    operator: DimensionOperator,
    mut operands: Vec<DimensionExpr>,
) -> Result<DimensionExpr, SemanticModelError> {
    operands.sort_by_key(encode_normalized_dimension);
    match operands.len() {
        0 => Ok(DimensionExpr::Constant(match operator {
            DimensionOperator::Add => 0,
            DimensionOperator::Multiply => 1,
            DimensionOperator::Min | DimensionOperator::Max => {
                return Err(SemanticModelError::EmptyMinMaxV1 { operator });
            }
        })),
        1 => Ok(operands.pop().expect("one operand")),
        _ => {
            let operands = operands.into_boxed_slice();
            Ok(match operator {
                DimensionOperator::Add => DimensionExpr::Add(operands),
                DimensionOperator::Multiply => DimensionExpr::Multiply(operands),
                DimensionOperator::Min => DimensionExpr::Min(operands),
                DimensionOperator::Max => DimensionExpr::Max(operands),
            })
        }
    }
}

pub(crate) fn encode_normalized_dimension(expression: &DimensionExpr) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_normalized_dimension_into(expression, &mut bytes);
    bytes
}

pub(crate) fn encode_normalized_dimension_into(expression: &DimensionExpr, bytes: &mut Vec<u8>) {
    match expression {
        DimensionExpr::Hole => unreachable!("holes are not canonical"),
        DimensionExpr::Constant(value) => {
            bytes.push(0x01);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        DimensionExpr::Parameter(id) => {
            bytes.push(0x02);
            bytes.extend_from_slice(&id.get().to_le_bytes());
        }
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            bytes.push(match expression {
                DimensionExpr::Add(_) => 0x03,
                DimensionExpr::Multiply(_) => 0x04,
                DimensionExpr::Min(_) => 0x05,
                DimensionExpr::Max(_) => 0x06,
                _ => unreachable!(),
            });
            bytes.extend_from_slice(&(children.len() as u32).to_le_bytes());
            for child in children {
                let encoded = encode_normalized_dimension(child);
                bytes.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
                bytes.extend_from_slice(&encoded);
            }
        }
    }
}

pub(crate) fn encode_dimension_parameters(parameters: &[DimensionParameter], bytes: &mut Vec<u8>) {
    for parameter in parameters {
        bytes.push(match parameter.lifetime {
            DimensionLifetime::Activation => 0x01,
            DimensionLifetime::Turn => 0x02,
            DimensionLifetime::CompileTime => unreachable!("compile-time parameters are rejected"),
        });
        let lower = encode_normalized_dimension(&parameter.lower_bound);
        bytes.extend_from_slice(&(lower.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&lower);
        match &parameter.upper_bound {
            None => bytes.push(0),
            Some(upper) => {
                bytes.push(1);
                let upper = encode_normalized_dimension(upper);
                bytes.extend_from_slice(&(upper.len() as u64).to_le_bytes());
                bytes.extend_from_slice(&upper);
            }
        }
    }
}

pub fn extent_evolution(parameters: &[DimensionParameter]) -> ExtentEvolution {
    if parameters.is_empty() {
        return ExtentEvolution::Fixed;
    }
    let turn_parameters = parameters
        .iter()
        .filter(|parameter| parameter.lifetime == DimensionLifetime::Turn)
        .collect::<Vec<_>>();
    if turn_parameters.is_empty() {
        ExtentEvolution::ActivationFixed
    } else if turn_parameters
        .iter()
        .all(|parameter| parameter.upper_bound.is_some())
    {
        ExtentEvolution::TurnBounded
    } else {
        ExtentEvolution::TurnUnbounded
    }
}

#[cfg(test)]
mod vector_tests {
    use super::*;

    fn boxed(values: impl IntoIterator<Item = DimensionExpr>) -> Box<[DimensionExpr]> {
        values.into_iter().collect::<Vec<_>>().into_boxed_slice()
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |value| match value {
                    b'0'..=b'9' => value - b'0',
                    b'a'..=b'f' => value - b'a' + 10,
                    _ => panic!("invalid test hex"),
                };
                (digit(pair[0]) << 4) | digit(pair[1])
            })
            .collect()
    }

    #[test]
    fn all_positive_c0_dimension_vectors_match_exactly() {
        let parameter = DimensionExpr::Parameter(DimensionParameterId::new(0));
        let vectors = [
            (
                DimensionExpr::Add(boxed([
                    DimensionExpr::Constant(4),
                    DimensionExpr::Add(boxed([
                        DimensionExpr::Add(boxed([
                            DimensionExpr::Constant(1),
                            DimensionExpr::Constant(2),
                        ])),
                        parameter.clone(),
                    ])),
                ])),
                "0302000000090000000000000001070000000000000005000000000000000200000000",
            ),
            (
                DimensionExpr::Multiply(boxed([
                    DimensionExpr::Constant(2),
                    DimensionExpr::Multiply(boxed([
                        DimensionExpr::Constant(3),
                        DimensionExpr::Multiply(boxed([
                            DimensionExpr::Constant(1),
                            parameter.clone(),
                        ])),
                    ])),
                ])),
                "0402000000090000000000000001060000000000000005000000000000000200000000",
            ),
            (
                DimensionExpr::Min(boxed([
                    parameter.clone(),
                    DimensionExpr::Min(boxed([DimensionExpr::Constant(5), parameter.clone()])),
                    DimensionExpr::Constant(5),
                ])),
                "0502000000090000000000000001050000000000000005000000000000000200000000",
            ),
            (
                DimensionExpr::Max(boxed([
                    DimensionExpr::Max(boxed([DimensionExpr::Constant(2), parameter.clone()])),
                    DimensionExpr::Constant(2),
                    parameter,
                ])),
                "0602000000090000000000000001020000000000000005000000000000000200000000",
            ),
        ];
        for (expression, expected) in vectors {
            let normalized = normalize_dimension(&expression, 1).unwrap();
            assert_eq!(
                encode_normalized_dimension(&normalized),
                decode_hex(expected)
            );
        }
    }

    #[test]
    fn all_negative_c0_dimension_vectors_return_the_frozen_errors() {
        for expression in [
            DimensionExpr::Add(boxed([
                DimensionExpr::Constant(u64::MAX),
                DimensionExpr::Constant(1),
            ])),
            DimensionExpr::Multiply(boxed([
                DimensionExpr::Constant(u64::MAX),
                DimensionExpr::Constant(2),
            ])),
        ] {
            assert!(matches!(
                normalize_dimension(&expression, 0),
                Err(SemanticModelError::DimensionOverflowV1)
            ));
        }
        assert!(matches!(
            normalize_dimension(&DimensionExpr::Parameter(DimensionParameterId::new(1)), 1),
            Err(SemanticModelError::UnknownDimensionParameterV1 { .. })
        ));
    }

    #[test]
    fn test_only_limit_proves_dimension_parameter_identity_exhaustion() {
        let mut builder = DimensionEnvironmentBuilder::new();
        assert!(matches!(
            builder.declare_with_limit(
                DimensionParameterOrigin::Explicit,
                DimensionLifetime::Activation,
                DimensionExpr::Constant(0),
                None,
                0,
            ),
            Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::DimensionParameterId,
            })
        ));
        assert!(builder.declarations().is_empty());
    }
}
