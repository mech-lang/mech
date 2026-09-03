//! Closed, normalized semantic types derived from canonical schemas.

use super::{
    BuiltinKindPredicate, BuiltinKindPredicateSet, BuiltinScalarKind, TypeConstraintFailure,
    TypeResolutionError, intrinsic_kind_predicates, semantic_kind_name,
};
use crate::dimension::{canonicalize_dimension_environment, normalize_dimension};
use crate::kind_expr::{
    collect_kind_dimension_references, dependency_ordered_dimension_references,
    normalize_kind_dimensions, rewrite_kind_dimensions, validate_kind_structure,
    visit_kind_dimensions, visit_kind_parameters,
};
use crate::{
    CardinalitySpec, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, KindExpr, KindField, Schema, SchemaBody,
    ShapeInstance,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, vec::Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KindPredicateEvidence {
    kind: KindExpr,
    predicates: BuiltinKindPredicateSet,
}

impl KindPredicateEvidence {
    pub(crate) const fn new(kind: KindExpr, predicates: BuiltinKindPredicateSet) -> Self {
        Self { kind, predicates }
    }

    pub(crate) const fn kind(&self) -> &KindExpr {
        &self.kind
    }

    pub(crate) const fn predicates(&self) -> BuiltinKindPredicateSet {
        self.predicates
    }
}

/// A closed semantic type. Dimension parameters remain only when declared by
/// this value, so dynamic extents stay closed without becoming fixed values.
#[derive(Clone, Debug)]
pub struct ResolvedType {
    dimension_parameters: Box<[DimensionParameterDeclaration]>,
    kind: KindExpr,
    predicate_evidence: Box<[KindPredicateEvidence]>,
}

impl PartialEq for ResolvedType {
    fn eq(&self, other: &Self) -> bool {
        self.dimension_parameters == other.dimension_parameters && self.kind == other.kind
    }
}

impl Eq for ResolvedType {}

impl ResolvedType {
    pub fn new(
        kind: KindExpr,
        dimension_parameters: Box<[DimensionParameterDeclaration]>,
    ) -> Result<Self, TypeResolutionError> {
        let evidence = intrinsic_evidence(&kind);
        Self::new_with_evidence(kind, dimension_parameters, evidence)
    }

    pub(crate) fn new_with_evidence(
        kind: KindExpr,
        dimension_parameters: Box<[DimensionParameterDeclaration]>,
        evidence: Vec<KindPredicateEvidence>,
    ) -> Result<Self, TypeResolutionError> {
        validate_kind_structure(&kind).map_err(TypeResolutionError::semantic)?;
        visit_kind_parameters(&kind, &mut |id| {
            Err(crate::SemanticModelError::KindParameterNotClosed { id })
        })
        .map_err(TypeResolutionError::semantic)?;
        visit_kind_dimensions(&kind, &mut |dimension| {
            normalize_dimension(dimension, dimension_parameters.len()).map(|_| ())
        })
        .map_err(TypeResolutionError::semantic)?;

        let normalized = normalize_kind_dimensions(kind, dimension_parameters.len())
            .map_err(TypeResolutionError::semantic)?;
        let mut references = Vec::new();
        collect_kind_dimension_references(&normalized, &mut references);
        let references =
            dependency_ordered_dimension_references(&dimension_parameters, &references)
                .map_err(TypeResolutionError::semantic)?;
        let environment = canonicalize_dimension_environment(&dimension_parameters, &references)
            .map_err(TypeResolutionError::semantic)?;
        let rewritten = rewrite_kind_dimensions(&normalized, &environment.old_to_new)
            .map_err(TypeResolutionError::semantic)?;
        let kind = normalize_kind_dimensions(rewritten, environment.parameters.len())
            .map_err(TypeResolutionError::semantic)?;

        let mut predicate_evidence = Vec::new();
        for item in evidence {
            let normalized = normalize_kind_dimensions(item.kind, dimension_parameters.len())
                .map_err(TypeResolutionError::semantic)?;
            let rewritten = rewrite_kind_dimensions(&normalized, &environment.old_to_new)
                .map_err(TypeResolutionError::semantic)?;
            let normalized = normalize_kind_dimensions(rewritten, environment.parameters.len())
                .map_err(TypeResolutionError::semantic)?;
            merge_evidence(&mut predicate_evidence, normalized, item.predicates);
        }
        predicate_evidence.sort_by(|left, right| {
            semantic_kind_name(&left.kind).cmp(&semantic_kind_name(&right.kind))
        });

        let dimension_parameters = environment
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            dimension_parameters,
            kind,
            predicate_evidence: predicate_evidence.into_boxed_slice(),
        })
    }

    pub fn from_schema(
        schema: &Schema,
        shape: &ShapeInstance,
    ) -> Result<Self, TypeResolutionError> {
        schema
            .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
            .map_err(TypeResolutionError::semantic)?;
        let declarations = schema
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(index, parameter)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
            .collect::<Vec<_>>();
        Self::from_schema_body(schema.body(), &declarations)
    }

    /// Resolves a semantic schema body with an already validated dimension
    /// environment. Boundary code uses this to plan a target before creating
    /// its physical cell.
    pub fn from_schema_body(
        body: &SchemaBody,
        dimension_parameters: &[DimensionParameterDeclaration],
    ) -> Result<Self, TypeResolutionError> {
        let mut builder = SchemaKindBuilder {
            dimensions: dimension_parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(index as u32),
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: parameter.lifetime,
                    lower_bound: parameter.lower_bound.clone(),
                    upper_bound: parameter.upper_bound.clone(),
                })
                .collect(),
            evidence: Vec::new(),
        };
        let kind = builder.kind(body)?;
        Self::new_with_evidence(
            kind,
            builder.dimensions.into_boxed_slice(),
            builder.evidence,
        )
    }

    pub const fn kind(&self) -> &KindExpr {
        &self.kind
    }

    pub fn dimension_parameters(&self) -> &[DimensionParameterDeclaration] {
        &self.dimension_parameters
    }

    pub fn semantic_name(&self) -> String {
        semantic_kind_name(&self.kind)
    }

    pub fn satisfies(&self, predicate: BuiltinKindPredicate) -> bool {
        self.predicates_for(&self.kind).contains(predicate)
    }

    pub(crate) fn predicates_for(&self, kind: &KindExpr) -> BuiltinKindPredicateSet {
        self.predicate_evidence
            .iter()
            .find_map(|item| (item.kind == *kind).then_some(item.predicates))
            .unwrap_or_else(BuiltinKindPredicateSet::empty)
    }

    pub(crate) fn evidence(&self) -> &[KindPredicateEvidence] {
        &self.predicate_evidence
    }

    pub(crate) fn with_intersected_evidence(
        &self,
        other: &Self,
    ) -> Result<Self, TypeResolutionError> {
        if self != other {
            return Err(TypeResolutionError::incompatible(
                "type evidence",
                TypeConstraintFailure::ExactTypeMismatch {
                    expected: self.semantic_name(),
                    actual: other.semantic_name(),
                },
            ));
        }
        let mut evidence = Vec::new();
        for item in self.evidence() {
            let predicates = item
                .predicates
                .intersection(other.predicates_for(&item.kind));
            merge_evidence(&mut evidence, item.kind.clone(), predicates);
        }
        Self::new_with_evidence(
            self.kind.clone(),
            self.dimension_parameters.clone(),
            evidence,
        )
    }
}

struct SchemaKindBuilder {
    dimensions: Vec<DimensionParameterDeclaration>,
    evidence: Vec<KindPredicateEvidence>,
}

impl SchemaKindBuilder {
    fn kind(&mut self, body: &SchemaBody) -> Result<KindExpr, TypeResolutionError> {
        let kind = match body {
            body if BuiltinScalarKind::from_schema_body(body).is_some() => {
                BuiltinScalarKind::from_schema_body(body)
                    .unwrap()
                    .kind_expr()
            }
            SchemaBody::Dynamic => KindExpr::Wildcard,
            SchemaBody::Id => KindExpr::Id,
            SchemaBody::Index => KindExpr::Index,
            SchemaBody::Atom(key) => KindExpr::Atom(*key),
            SchemaBody::Enum { key, .. } => KindExpr::Enum(*key),
            SchemaBody::Option(element) => KindExpr::Option(Box::new(self.kind(element)?)),
            SchemaBody::Tuple(elements) => KindExpr::Tuple(
                elements
                    .iter()
                    .map(|element| self.kind(element))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            SchemaBody::Record(fields) => KindExpr::Record(self.fields(fields)?),
            SchemaBody::Matrix {
                element,
                dimensions,
            } => KindExpr::Matrix {
                element: Box::new(self.kind(element)?),
                dimensions: dimensions.clone(),
            },
            SchemaBody::Table { columns, rows } => KindExpr::Table {
                columns: self.fields(columns)?,
                rows: self.extent(rows)?,
            },
            SchemaBody::Set {
                element,
                cardinality,
            } => KindExpr::Set {
                element: Box::new(self.kind(element)?),
                cardinality: self.extent(cardinality)?,
            },
            SchemaBody::Map {
                key,
                value,
                cardinality,
            } => KindExpr::Map {
                key: Box::new(self.kind(key)?),
                value: Box::new(self.kind(value)?),
                cardinality: self.extent(cardinality)?,
            },
            SchemaBody::ReifiedType => KindExpr::TypeOf(Box::new(KindExpr::Wildcard)),
            SchemaBody::Bool
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
            | SchemaBody::String => unreachable!("builtin scalars were handled above"),
        };
        let predicates = schema_predicate_set(body, &kind, &self.evidence);
        merge_evidence(&mut self.evidence, kind.clone(), predicates);
        Ok(kind)
    }

    fn fields(
        &mut self,
        fields: &[crate::SchemaField],
    ) -> Result<Box<[KindField]>, TypeResolutionError> {
        fields
            .iter()
            .map(|field| {
                Ok(KindField {
                    name: field.name.clone(),
                    kind: self.kind(&field.schema)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }

    fn extent(&mut self, extent: &CardinalitySpec) -> Result<DimensionExpr, TypeResolutionError> {
        match extent {
            CardinalitySpec::Exact(expression) => Ok(expression.clone()),
            CardinalitySpec::Dynamic { upper_bound } => {
                let id = DimensionParameterId::new(self.dimensions.len() as u32);
                self.dimensions.push(DimensionParameterDeclaration {
                    id,
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: upper_bound.clone(),
                });
                Ok(DimensionExpr::Parameter(id))
            }
        }
    }
}

fn merge_evidence(
    evidence: &mut Vec<KindPredicateEvidence>,
    kind: KindExpr,
    predicates: BuiltinKindPredicateSet,
) {
    if let Some(existing) = evidence.iter_mut().find(|item| item.kind == kind) {
        existing.predicates = existing.predicates.intersection(predicates);
    } else {
        evidence.push(KindPredicateEvidence { kind, predicates });
    }
}

fn schema_predicate_set(
    body: &SchemaBody,
    kind: &KindExpr,
    evidence: &[KindPredicateEvidence],
) -> BuiltinKindPredicateSet {
    let child_kinds = match kind {
        KindExpr::Option(element)
        | KindExpr::Matrix { element, .. }
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => vec![element.as_ref()],
        KindExpr::Tuple(elements) => elements.iter().collect(),
        KindExpr::Record(fields)
        | KindExpr::Table {
            columns: fields, ..
        } => fields.iter().map(|field| &field.kind).collect(),
        KindExpr::Map { key, value, .. } => vec![key.as_ref(), value.as_ref()],
        _ => Vec::new(),
    };
    let child_predicates = child_kinds
        .into_iter()
        .map(|child| {
            evidence
                .iter()
                .find_map(|item| (item.kind == *child).then_some(item.predicates))
                .unwrap_or_else(BuiltinKindPredicateSet::empty)
        })
        .collect::<Vec<_>>();
    let mut predicates = intrinsic_kind_predicates(kind, &child_predicates);
    // Nominal enum payloads are intentionally absent from KindExpr, so their
    // closed schema remains the evidence authority for these two predicates.
    if matches!(body, SchemaBody::Enum { .. }) {
        if schema_equatable(body) {
            predicates.insert(BuiltinKindPredicate::Equatable);
        }
        if crate::schema::is_schema_body_keyable(body) {
            predicates.insert(BuiltinKindPredicate::Keyable);
        }
    }
    predicates
}

fn schema_equatable(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Dynamic => false,
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => true,
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .all(|variant| variant.payload.as_ref().is_none_or(schema_equatable)),
        SchemaBody::Option(element)
        | SchemaBody::Matrix { element, .. }
        | SchemaBody::Set { element, .. } => schema_equatable(element),
        SchemaBody::Tuple(elements) => elements.iter().all(schema_equatable),
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => fields.iter().all(|field| schema_equatable(&field.schema)),
        SchemaBody::Map { key, value, .. } => schema_equatable(key) && schema_equatable(value),
    }
}

fn intrinsic_evidence(root: &KindExpr) -> Vec<KindPredicateEvidence> {
    fn visit(
        kind: &KindExpr,
        evidence: &mut Vec<KindPredicateEvidence>,
    ) -> BuiltinKindPredicateSet {
        let children = match kind {
            KindExpr::Option(element)
            | KindExpr::Matrix { element, .. }
            | KindExpr::Set { element, .. }
            | KindExpr::Reference(element)
            | KindExpr::TypeOf(element) => vec![element.as_ref()],
            KindExpr::Tuple(elements) => elements.iter().collect(),
            KindExpr::Record(fields)
            | KindExpr::Table {
                columns: fields, ..
            } => fields.iter().map(|field| &field.kind).collect(),
            KindExpr::Map { key, value, .. } => vec![key.as_ref(), value.as_ref()],
            _ => Vec::new(),
        };
        let child_predicates = children
            .into_iter()
            .map(|child| visit(child, evidence))
            .collect::<Vec<_>>();
        let predicates = intrinsic_kind_predicates(kind, &child_predicates);
        merge_evidence(evidence, kind.clone(), predicates);
        predicates
    }
    let mut evidence = Vec::new();
    visit(root, &mut evidence);
    evidence
}
