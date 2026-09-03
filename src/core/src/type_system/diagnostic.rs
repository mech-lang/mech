//! Structured, source-located semantic type diagnostics.

use super::{BuiltinKindPredicate, BuiltinScalarKind, ResolvedType, builtin_scalar_name};
use crate::{
    CardinalitySpec, DimensionExpr, DimensionParameterId, KindExpr, KindField, KindParameterId,
    MechError, MechErrorKind, SchemaBody, SourceRange,
};
use core::fmt::{self, Display, Formatter};

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeConstraintFailure {
    Arity {
        expected: String,
        actual: usize,
    },
    ExactTypeMismatch {
        expected: String,
        actual: String,
    },
    ConversionNotPermitted {
        source: String,
        target: String,
    },
    PredicateNotSatisfied {
        kind: String,
        predicate: BuiltinKindPredicate,
    },
    PromotionUnavailable {
        left: String,
        right: String,
    },
    ConflictingNominalTypes {
        expected: String,
        actual: String,
    },
    StructuralMismatch {
        expected: String,
        actual: String,
    },
    IncompatibleDimensions {
        expected: String,
        actual: String,
    },
    InvalidDynamicFixedExtent {
        expected: String,
        actual: String,
    },
    DimensionBoundNotProven {
        relation: String,
    },
    UnresolvedKindHole,
    UnresolvedDimensionHole,
    UnresolvedKindVariable {
        parameter: KindParameterId,
    },
    UnresolvedDimensionVariable {
        parameter: DimensionParameterId,
    },
    CyclicKindBinding {
        parameter: KindParameterId,
    },
    CyclicDimensionBinding {
        parameter: DimensionParameterId,
    },
    OutputTypeMismatch {
        expected: String,
        actual: String,
    },
    InvalidScheme {
        reason: String,
    },
}

impl Display for TypeConstraintFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arity { expected, actual } => {
                write!(formatter, "expected {expected} inputs, received {actual}")
            }
            Self::ExactTypeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "expected exact type {expected}, received {actual}"
                )
            }
            Self::ConversionNotPermitted { source, target } => {
                write!(formatter, "cannot implicitly convert {source} to {target}")
            }
            Self::PredicateNotSatisfied { kind, predicate } => {
                write!(formatter, "{kind} does not satisfy {predicate:?}")
            }
            Self::PromotionUnavailable { left, right } => {
                write!(
                    formatter,
                    "{left} and {right} have no lossless common numeric type"
                )
            }
            Self::ConflictingNominalTypes { expected, actual } => {
                write!(formatter, "nominal type {actual} conflicts with {expected}")
            }
            Self::StructuralMismatch { expected, actual } => {
                write!(formatter, "structure {actual} does not match {expected}")
            }
            Self::IncompatibleDimensions { expected, actual } => {
                write!(formatter, "dimensions {actual} do not match {expected}")
            }
            Self::InvalidDynamicFixedExtent { expected, actual } => write!(
                formatter,
                "extent evolution {actual} is more dynamic than {expected}"
            ),
            Self::DimensionBoundNotProven { relation } => {
                write!(formatter, "dimension bound cannot be proven: {relation}")
            }
            Self::UnresolvedKindHole => formatter.write_str("unresolved type hole"),
            Self::UnresolvedDimensionHole => formatter.write_str("unresolved dimension hole"),
            Self::UnresolvedKindVariable { parameter } => {
                write!(formatter, "unresolved type variable T{}", parameter.get())
            }
            Self::UnresolvedDimensionVariable { parameter } => {
                write!(
                    formatter,
                    "unresolved dimension variable d{}",
                    parameter.get()
                )
            }
            Self::CyclicKindBinding { parameter } => {
                write!(formatter, "cyclic type binding for T{}", parameter.get())
            }
            Self::CyclicDimensionBinding { parameter } => {
                write!(
                    formatter,
                    "cyclic dimension binding for d{}",
                    parameter.get()
                )
            }
            Self::OutputTypeMismatch { expected, actual } => {
                write!(formatter, "resolved output {expected}, produced {actual}")
            }
            Self::InvalidScheme { reason } => write!(formatter, "invalid type scheme: {reason}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeConstraintOrigin {
    pub semantic_name: String,
    pub source: Option<SourceRange>,
}

impl TypeConstraintOrigin {
    pub fn new(semantic_name: impl Into<String>, source: Option<SourceRange>) -> Self {
        Self {
            semantic_name: semantic_name.into(),
            source,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeResolutionError {
    Ambiguous {
        origin: TypeConstraintOrigin,
        alternatives: Box<[Box<[ResolvedType]>]>,
    },
    Incompatible {
        origin: TypeConstraintOrigin,
        failures: Box<[TypeConstraintFailure]>,
    },
}

impl TypeResolutionError {
    pub(crate) fn semantic(error: crate::SemanticModelError) -> Self {
        let failure = match error {
            crate::SemanticModelError::UnresolvedKindHole => {
                TypeConstraintFailure::UnresolvedKindHole
            }
            crate::SemanticModelError::UnresolvedDimensionHole => {
                TypeConstraintFailure::UnresolvedDimensionHole
            }
            other => TypeConstraintFailure::InvalidScheme {
                reason: format!("{other:?}"),
            },
        };
        Self::incompatible("type", failure)
    }

    pub fn incompatible(semantic_name: impl Into<String>, failure: TypeConstraintFailure) -> Self {
        Self::Incompatible {
            origin: TypeConstraintOrigin::new(semantic_name, None),
            failures: vec![failure].into_boxed_slice(),
        }
    }

    pub fn origin(&self) -> &TypeConstraintOrigin {
        match self {
            Self::Ambiguous { origin, .. } | Self::Incompatible { origin, .. } => origin,
        }
    }

    pub fn with_origin(mut self, origin: TypeConstraintOrigin) -> Self {
        match &mut self {
            Self::Ambiguous { origin: target, .. } | Self::Incompatible { origin: target, .. } => {
                *target = origin
            }
        }
        self
    }
}

impl Display for TypeResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ambiguous {
                origin,
                alternatives,
            } => {
                write!(
                    formatter,
                    "type resolution for `{}` is ambiguous",
                    origin.semantic_name
                )?;
                for alternative in alternatives.iter() {
                    let outputs = alternative
                        .iter()
                        .map(ResolvedType::semantic_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(formatter, "; candidate returns ({outputs})")?;
                }
                Ok(())
            }
            Self::Incompatible { origin, failures } => {
                write!(
                    formatter,
                    "type resolution for `{}` failed",
                    origin.semantic_name
                )?;
                for failure in failures.iter() {
                    write!(formatter, "; {failure}")?;
                }
                Ok(())
            }
        }
    }
}

impl MechErrorKind for TypeResolutionError {
    fn name(&self) -> &str {
        match self {
            Self::Ambiguous { .. } => "TypeAmbiguity",
            Self::Incompatible { .. } => "TypeIncompatibility",
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<TypeResolutionError> for MechError {
    fn from(error: TypeResolutionError) -> Self {
        let source = error.origin().source.clone();
        let mut mech = MechError::new(error, None).with_compiler_loc();
        if let Some(source) = source {
            mech = mech.with_annotation(source);
        }
        mech
    }
}

pub fn semantic_kind_name(kind: &KindExpr) -> String {
    match kind {
        KindExpr::Wildcard => "*".into(),
        KindExpr::Never => "never".into(),
        KindExpr::Hole => "?type".into(),
        KindExpr::Parameter(id) => format!("T{}", id.get()),
        KindExpr::Named(_) => builtin_scalar_name(kind)
            .map(String::from)
            .unwrap_or_else(|| match kind {
                KindExpr::Named(id) => format!("kind#{}", id.get()),
                _ => unreachable!(),
            }),
        KindExpr::Id => "id".into(),
        KindExpr::Index => "index".into(),
        KindExpr::Atom(key) => format!("atom({key:?})"),
        KindExpr::Enum(key) => format!("enum({key:?})"),
        KindExpr::Matrix {
            element,
            dimensions,
        } => format!(
            "matrix<{};{}>",
            semantic_kind_name(element),
            dimensions
                .iter()
                .map(semantic_dimension_name)
                .collect::<Vec<_>>()
                .join("×")
        ),
        KindExpr::Option(element) => format!("option<{}>", semantic_kind_name(element)),
        KindExpr::Tuple(elements) => format!(
            "({})",
            elements
                .iter()
                .map(semantic_kind_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        KindExpr::Record(fields) => format_fields("record", fields),
        KindExpr::Table { columns, rows } => format!(
            "{}[{}]",
            format_fields("table", columns),
            semantic_dimension_name(rows)
        ),
        KindExpr::Set {
            element,
            cardinality,
        } => format!(
            "set<{};{}>",
            semantic_kind_name(element),
            semantic_dimension_name(cardinality)
        ),
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => format!(
            "map<{}, {};{}>",
            semantic_kind_name(key),
            semantic_kind_name(value),
            semantic_dimension_name(cardinality)
        ),
        KindExpr::Reference(element) => format!("ref<{}>", semantic_kind_name(element)),
        KindExpr::TypeOf(element) => format!("type<{}>", semantic_kind_name(element)),
    }
}

pub fn semantic_schema_body_name(body: &SchemaBody) -> String {
    if let Some(kind) = BuiltinScalarKind::from_schema_body(body) {
        return kind.canonical_name().into();
    }
    match body {
        SchemaBody::Dynamic => "dynamic".into(),
        SchemaBody::Id => "id".into(),
        SchemaBody::Index => "index".into(),
        SchemaBody::Atom(key) => format!("atom({key:?})"),
        SchemaBody::Enum { key, .. } => format!("enum({key:?})"),
        SchemaBody::Option(element) => format!("option<{}>", semantic_schema_body_name(element)),
        SchemaBody::Tuple(elements) => format!(
            "({})",
            elements
                .iter()
                .map(semantic_schema_body_name)
                .collect::<Vec<_>>()
                .join(",")
        ),
        SchemaBody::Record(fields) => format_schema_fields("record", fields),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => format!(
            "matrix<{};{}>",
            semantic_schema_body_name(element),
            dimensions
                .iter()
                .map(semantic_dimension_name)
                .collect::<Vec<_>>()
                .join("×")
        ),
        SchemaBody::Table { columns, rows } => format!(
            "{}[{}]",
            format_schema_fields("table", columns),
            semantic_cardinality_name(rows)
        ),
        SchemaBody::Set {
            element,
            cardinality,
        } => format!(
            "set<{};{}>",
            semantic_schema_body_name(element),
            semantic_cardinality_name(cardinality)
        ),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => format!(
            "map<{},{};{}>",
            semantic_schema_body_name(key),
            semantic_schema_body_name(value),
            semantic_cardinality_name(cardinality)
        ),
        SchemaBody::ReifiedType => "type".into(),
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String => unreachable!("builtin scalar handled above"),
    }
}

fn semantic_cardinality_name(cardinality: &CardinalitySpec) -> String {
    match cardinality {
        CardinalitySpec::Exact(dimension) => semantic_dimension_name(dimension),
        CardinalitySpec::Dynamic { upper_bound: None } => "dynamic".into(),
        CardinalitySpec::Dynamic {
            upper_bound: Some(bound),
        } => format!("dynamic<={}", semantic_dimension_name(bound)),
    }
}

fn format_schema_fields(category: &str, fields: &[crate::SchemaField]) -> String {
    format!(
        "{category}{{{}}}",
        fields
            .iter()
            .map(|field| format!(
                "{}:{}",
                field.name,
                semantic_schema_body_name(&field.schema)
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn format_fields(prefix: &str, fields: &[KindField]) -> String {
    format!(
        "{prefix}{{{}}}",
        fields
            .iter()
            .map(|field| format!("{}:{}", field.name, semantic_kind_name(&field.kind)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn semantic_dimension_name(dimension: &DimensionExpr) -> String {
    match dimension {
        DimensionExpr::Hole => "?extent".into(),
        DimensionExpr::Constant(value) => value.to_string(),
        DimensionExpr::Parameter(id) => format!("d{}", id.get()),
        DimensionExpr::Add(children) => format_dimension_operator("+", children),
        DimensionExpr::Multiply(children) => format_dimension_operator("*", children),
        DimensionExpr::Min(children) => format_dimension_call("min", children),
        DimensionExpr::Max(children) => format_dimension_call("max", children),
    }
}

fn format_dimension_operator(operator: &str, children: &[DimensionExpr]) -> String {
    format!(
        "({})",
        children
            .iter()
            .map(semantic_dimension_name)
            .collect::<Vec<_>>()
            .join(operator)
    )
}

fn format_dimension_call(function: &str, children: &[DimensionExpr]) -> String {
    format!(
        "{function}({})",
        children
            .iter()
            .map(semantic_dimension_name)
            .collect::<Vec<_>>()
            .join(",")
    )
}
