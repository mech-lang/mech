use super::{
    LegacyExtentRole, LegacyExtentSite, LegacyKindResolution, LegacyNominalResolution,
    LegacyResolvedExtent, LegacySemanticContext, LegacyTypePathSegment, LegacyTypeSource,
    LegacyValueKindTag,
};
use crate::dimension::{canonicalize_dimension_environment, normalize_dimension};
use crate::kind::Kind;
use crate::kind_expr::{validate_kind_structure, visit_kind_dimensions};
use crate::legacy_value::ValueKind;
use crate::{
    DimensionEnvironmentBuilder, DimensionExpr, DimensionParameterDeclaration, FloatWidth,
    IntegerWidth, KindExpr, KindField, NominalKind, Schema, SchemaBody, SchemaDraft, SchemaField,
    SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, vec::Vec};

pub fn kind_expr_from_legacy(
    kind: &Kind,
    context: &mut dyn LegacySemanticContext,
) -> Result<LegacyKindResolution, SemanticModelError> {
    let mut dimensions = DimensionEnvironmentBuilder::new();
    let kind = kind_from_legacy(kind, context, &mut dimensions, &mut Vec::new())?;
    let dimension_parameters = dimensions.into_declarations();
    validate_legacy_kind_resolution(&kind, &dimension_parameters)?;
    Ok(LegacyKindResolution {
        kind,
        dimension_parameters,
    })
}

fn validate_legacy_kind_resolution(
    kind: &KindExpr,
    declarations: &[DimensionParameterDeclaration],
) -> Result<(), SemanticModelError> {
    validate_kind_structure(kind)?;
    let all_declarations = declarations
        .iter()
        .map(|declaration| declaration.id)
        .collect::<Vec<_>>();
    canonicalize_dimension_environment(declarations, &all_declarations)?;
    visit_kind_dimensions(kind, &mut |dimension| {
        normalize_dimension(dimension, declarations.len()).map(|_| ())
    })
}

pub fn schema_from_legacy_value_kind(
    kind: &ValueKind,
    context: &mut dyn LegacySemanticContext,
) -> Result<Schema, SemanticModelError> {
    let mut dimensions = DimensionEnvironmentBuilder::new();
    let body = schema_body_from_legacy(kind, context, &mut dimensions, &mut Vec::new())?;
    SchemaDraft {
        dimension_parameters: dimensions.into_declarations(),
        body,
    }
    .finalize()
}

fn kind_from_legacy(
    kind: &Kind,
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    path: &mut Vec<LegacyTypePathSegment>,
) -> Result<KindExpr, SemanticModelError> {
    Ok(match kind {
        Kind::Any => KindExpr::Wildcard,
        Kind::None => KindExpr::Never,
        Kind::Empty => KindExpr::Hole,
        Kind::Scalar(legacy_id) => KindExpr::Named(context.resolve_named_kind(*legacy_id)?),
        Kind::Id => KindExpr::Id,
        Kind::Index => KindExpr::Index,
        Kind::Atom(legacy_id, legacy_name) => {
            match context.resolve_nominal(NominalKind::Atom, *legacy_id, legacy_name)? {
                LegacyNominalResolution::Atom { key } => KindExpr::Atom(key),
                LegacyNominalResolution::Enum { .. } => {
                    return Err(SemanticModelError::LegacyExtentResolutionKindMismatch);
                }
            }
        }
        Kind::Enum(legacy_id, legacy_name) => {
            match context.resolve_nominal(NominalKind::Enum, *legacy_id, legacy_name)? {
                LegacyNominalResolution::Enum { key, .. } => KindExpr::Enum(key),
                LegacyNominalResolution::Atom { .. } => {
                    return Err(SemanticModelError::LegacyExtentResolutionKindMismatch);
                }
            }
        }
        Kind::Matrix(element, legacy_dimensions) => {
            let element = with_path(path, LegacyTypePathSegment::MatrixElement, |path| {
                kind_from_legacy(element, context, dimensions, path)
            })?;
            let dimensions = if legacy_dimensions.is_empty() {
                require_dimensions(context, dimensions, LegacyTypeSource::Kind, path)?
            } else {
                legacy_dimensions
                    .iter()
                    .map(|value| checked_extent(*value).map(DimensionExpr::Constant))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice()
            };
            KindExpr::Matrix {
                element: Box::new(element),
                dimensions,
            }
        }
        Kind::Option(element) => KindExpr::Option(Box::new(with_path(
            path,
            LegacyTypePathSegment::OptionElement,
            |path| kind_from_legacy(element, context, dimensions, path),
        )?)),
        Kind::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    with_path(
                        path,
                        LegacyTypePathSegment::TupleElement(checked_index(index)?),
                        |path| kind_from_legacy(element, context, dimensions, path),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        Kind::Record(fields) => KindExpr::Record(kind_fields_from_legacy(
            fields,
            LegacyTypePathSegment::RecordField,
            context,
            dimensions,
            path,
        )?),
        Kind::Table(columns, rows) => {
            let columns = kind_fields_from_legacy(
                columns,
                LegacyTypePathSegment::TableColumn,
                context,
                dimensions,
                path,
            )?;
            let rows = if *rows == 0 {
                require_cardinality(
                    context,
                    dimensions,
                    LegacyTypeSource::Kind,
                    path,
                    LegacyExtentRole::TableRows,
                )?
            } else {
                DimensionExpr::Constant(checked_extent(*rows)?)
            };
            KindExpr::Table { columns, rows }
        }
        Kind::Set(element, size) => {
            let element = with_path(path, LegacyTypePathSegment::SetElement, |path| {
                kind_from_legacy(element, context, dimensions, path)
            })?;
            let cardinality = match size {
                Some(size) => DimensionExpr::Constant(checked_extent(*size)?),
                None => require_cardinality(
                    context,
                    dimensions,
                    LegacyTypeSource::Kind,
                    path,
                    LegacyExtentRole::SetCardinality,
                )?,
            };
            KindExpr::Set {
                element: Box::new(element),
                cardinality,
            }
        }
        Kind::Map(key, value) => {
            let key = with_path(path, LegacyTypePathSegment::MapKey, |path| {
                kind_from_legacy(key, context, dimensions, path)
            })?;
            let value = with_path(path, LegacyTypePathSegment::MapValue, |path| {
                kind_from_legacy(value, context, dimensions, path)
            })?;
            let cardinality = require_cardinality(
                context,
                dimensions,
                LegacyTypeSource::Kind,
                path,
                LegacyExtentRole::MapCardinality,
            )?;
            KindExpr::Map {
                key: Box::new(key),
                value: Box::new(value),
                cardinality,
            }
        }
        Kind::Reference(element) => KindExpr::Reference(Box::new(kind_from_legacy(
            element, context, dimensions, path,
        )?)),
        Kind::Kind(element) => KindExpr::TypeOf(Box::new(with_path(
            path,
            LegacyTypePathSegment::TypeOf,
            |path| kind_from_legacy(element, context, dimensions, path),
        )?)),
    })
}

fn kind_fields_from_legacy(
    fields: &[(String, Kind)],
    segment: fn(u32) -> LegacyTypePathSegment,
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    path: &mut Vec<LegacyTypePathSegment>,
) -> Result<Box<[KindField]>, SemanticModelError> {
    fields
        .iter()
        .enumerate()
        .map(|(index, (name, kind))| {
            Ok(KindField {
                name: name.clone(),
                kind: with_path(path, segment(checked_index(index)?), |path| {
                    kind_from_legacy(kind, context, dimensions, path)
                })?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn schema_body_from_legacy(
    kind: &ValueKind,
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    path: &mut Vec<LegacyTypePathSegment>,
) -> Result<SchemaBody, SemanticModelError> {
    Ok(match kind {
        ValueKind::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        ValueKind::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        ValueKind::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        ValueKind::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        ValueKind::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        ValueKind::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        ValueKind::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        ValueKind::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        ValueKind::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        ValueKind::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        ValueKind::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        ValueKind::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        ValueKind::C64 => SchemaBody::Complex(FloatWidth::W64),
        ValueKind::R64 => SchemaBody::Rational64,
        ValueKind::String => SchemaBody::String,
        ValueKind::Bool => SchemaBody::Bool,
        ValueKind::Id => SchemaBody::Id,
        ValueKind::Index => SchemaBody::Index,
        ValueKind::Empty | ValueKind::Any | ValueKind::None | ValueKind::Reference(_) => {
            return Err(SemanticModelError::NonInstantiableLegacyValueKind {
                kind: legacy_value_kind_tag(kind),
            });
        }
        ValueKind::Matrix(element, legacy_dimensions) => {
            // An empty heterogeneous legacy matrix has no element from which
            // to infer a concrete schema. Canonicalize only that uninhabited
            // carrier to Index, which is always available and cannot change
            // value semantics because the matrix contains no elements.
            let empty_generic_matrix = matches!(element.as_ref(), ValueKind::Any)
                && !legacy_dimensions.is_empty()
                && legacy_dimensions.contains(&0);
            let element = if empty_generic_matrix {
                SchemaBody::Index
            } else {
                with_path(path, LegacyTypePathSegment::MatrixElement, |path| {
                    schema_body_from_legacy(element, context, dimensions, path)
                })?
            };
            let dimensions = if legacy_dimensions.is_empty() {
                require_dimensions(context, dimensions, LegacyTypeSource::ValueKind, path)?
            } else {
                legacy_dimensions
                    .iter()
                    .map(|value| checked_extent(*value).map(DimensionExpr::Constant))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice()
            };
            SchemaBody::Matrix {
                element: Box::new(element),
                dimensions,
            }
        }
        ValueKind::Enum(legacy_id, legacy_name) => {
            match context.resolve_nominal(NominalKind::Enum, *legacy_id, legacy_name)? {
                LegacyNominalResolution::Enum { key, variants } => {
                    SchemaBody::Enum { key, variants }
                }
                LegacyNominalResolution::Atom { .. } => {
                    return Err(SemanticModelError::LegacyExtentResolutionKindMismatch);
                }
            }
        }
        ValueKind::Record(fields) => SchemaBody::Record(schema_fields_from_legacy(
            fields,
            LegacyTypePathSegment::RecordField,
            context,
            dimensions,
            path,
        )?),
        ValueKind::Map(key, value) => {
            let key = with_path(path, LegacyTypePathSegment::MapKey, |path| {
                schema_body_from_legacy(key, context, dimensions, path)
            })?;
            let value = with_path(path, LegacyTypePathSegment::MapValue, |path| {
                schema_body_from_legacy(value, context, dimensions, path)
            })?;
            let cardinality = require_cardinality(
                context,
                dimensions,
                LegacyTypeSource::ValueKind,
                path,
                LegacyExtentRole::MapCardinality,
            )?;
            SchemaBody::Map {
                key: Box::new(key),
                value: Box::new(value),
                cardinality,
            }
        }
        ValueKind::Atom(legacy_id, legacy_name) => {
            match context.resolve_nominal(NominalKind::Atom, *legacy_id, legacy_name)? {
                LegacyNominalResolution::Atom { key } => SchemaBody::Atom(key),
                LegacyNominalResolution::Enum { .. } => {
                    return Err(SemanticModelError::LegacyExtentResolutionKindMismatch);
                }
            }
        }
        ValueKind::Table(columns, rows) => {
            let columns = schema_fields_from_legacy(
                columns,
                LegacyTypePathSegment::TableColumn,
                context,
                dimensions,
                path,
            )?;
            let rows = if *rows == 0 {
                require_cardinality(
                    context,
                    dimensions,
                    LegacyTypeSource::ValueKind,
                    path,
                    LegacyExtentRole::TableRows,
                )?
            } else {
                DimensionExpr::Constant(checked_extent(*rows)?)
            };
            SchemaBody::Table { columns, rows }
        }
        ValueKind::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    with_path(
                        path,
                        LegacyTypePathSegment::TupleElement(checked_index(index)?),
                        |path| schema_body_from_legacy(element, context, dimensions, path),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        ValueKind::Set(element, size) => {
            let element = with_path(path, LegacyTypePathSegment::SetElement, |path| {
                schema_body_from_legacy(element, context, dimensions, path)
            })?;
            let cardinality = match size {
                Some(size) => DimensionExpr::Constant(checked_extent(*size)?),
                None => require_cardinality(
                    context,
                    dimensions,
                    LegacyTypeSource::ValueKind,
                    path,
                    LegacyExtentRole::SetCardinality,
                )?,
            };
            SchemaBody::Set {
                element: Box::new(element),
                cardinality,
            }
        }
        ValueKind::Option(element) => SchemaBody::Option(Box::new(with_path(
            path,
            LegacyTypePathSegment::OptionElement,
            |path| schema_body_from_legacy(element, context, dimensions, path),
        )?)),
        ValueKind::Kind(_) => SchemaBody::ReifiedType,
    })
}

fn schema_fields_from_legacy(
    fields: &[(String, ValueKind)],
    segment: fn(u32) -> LegacyTypePathSegment,
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    path: &mut Vec<LegacyTypePathSegment>,
) -> Result<Box<[SchemaField]>, SemanticModelError> {
    fields
        .iter()
        .enumerate()
        .map(|(index, (name, kind))| {
            Ok(SchemaField {
                name: name.clone(),
                schema: with_path(path, segment(checked_index(index)?), |path| {
                    schema_body_from_legacy(kind, context, dimensions, path)
                })?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn require_dimensions(
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    source: LegacyTypeSource,
    path: &[LegacyTypePathSegment],
) -> Result<Box<[DimensionExpr]>, SemanticModelError> {
    let site = LegacyExtentSite {
        source,
        path: path.to_vec().into_boxed_slice(),
        role: LegacyExtentRole::MatrixDimensions,
    };
    match context.resolve_unspecified_extent(&site, dimensions)? {
        LegacyResolvedExtent::Dimensions(values) if !values.is_empty() => Ok(values),
        LegacyResolvedExtent::Dimensions(_) | LegacyResolvedExtent::Cardinality(_) => {
            Err(SemanticModelError::LegacyExtentResolutionKindMismatch)
        }
    }
}

fn require_cardinality(
    context: &mut dyn LegacySemanticContext,
    dimensions: &mut DimensionEnvironmentBuilder,
    source: LegacyTypeSource,
    path: &[LegacyTypePathSegment],
    role: LegacyExtentRole,
) -> Result<DimensionExpr, SemanticModelError> {
    let site = LegacyExtentSite {
        source,
        path: path.to_vec().into_boxed_slice(),
        role,
    };
    match context.resolve_unspecified_extent(&site, dimensions)? {
        LegacyResolvedExtent::Cardinality(value) => Ok(value),
        LegacyResolvedExtent::Dimensions(_) => {
            Err(SemanticModelError::LegacyExtentResolutionKindMismatch)
        }
    }
}

fn with_path<T>(
    path: &mut Vec<LegacyTypePathSegment>,
    segment: LegacyTypePathSegment,
    f: impl FnOnce(&mut Vec<LegacyTypePathSegment>) -> Result<T, SemanticModelError>,
) -> Result<T, SemanticModelError> {
    path.push(segment);
    let result = f(path);
    path.pop();
    result
}

fn checked_extent(value: usize) -> Result<u64, SemanticModelError> {
    checked_extent_value(value as u128)
}

fn checked_extent_value(value: u128) -> Result<u64, SemanticModelError> {
    u64::try_from(value).map_err(|_| SemanticModelError::LegacyExtentOutOfRange { value })
}

fn checked_index(value: usize) -> Result<u32, SemanticModelError> {
    u32::try_from(value).map_err(|_| SemanticModelError::LegacyExtentOutOfRange {
        value: value as u128,
    })
}

fn legacy_value_kind_tag(kind: &ValueKind) -> LegacyValueKindTag {
    match kind {
        ValueKind::U8 => LegacyValueKindTag::U8,
        ValueKind::U16 => LegacyValueKindTag::U16,
        ValueKind::U32 => LegacyValueKindTag::U32,
        ValueKind::U64 => LegacyValueKindTag::U64,
        ValueKind::U128 => LegacyValueKindTag::U128,
        ValueKind::I8 => LegacyValueKindTag::I8,
        ValueKind::I16 => LegacyValueKindTag::I16,
        ValueKind::I32 => LegacyValueKindTag::I32,
        ValueKind::I64 => LegacyValueKindTag::I64,
        ValueKind::I128 => LegacyValueKindTag::I128,
        ValueKind::F32 => LegacyValueKindTag::F32,
        ValueKind::F64 => LegacyValueKindTag::F64,
        ValueKind::C64 => LegacyValueKindTag::C64,
        ValueKind::R64 => LegacyValueKindTag::R64,
        ValueKind::String => LegacyValueKindTag::String,
        ValueKind::Bool => LegacyValueKindTag::Bool,
        ValueKind::Id => LegacyValueKindTag::Id,
        ValueKind::Index => LegacyValueKindTag::Index,
        ValueKind::Empty => LegacyValueKindTag::Empty,
        ValueKind::Any => LegacyValueKindTag::Any,
        ValueKind::None => LegacyValueKindTag::None,
        ValueKind::Matrix(_, _) => LegacyValueKindTag::Matrix,
        ValueKind::Enum(_, _) => LegacyValueKindTag::Enum,
        ValueKind::Record(_) => LegacyValueKindTag::Record,
        ValueKind::Map(_, _) => LegacyValueKindTag::Map,
        ValueKind::Atom(_, _) => LegacyValueKindTag::Atom,
        ValueKind::Table(_, _) => LegacyValueKindTag::Table,
        ValueKind::Tuple(_) => LegacyValueKindTag::Tuple,
        ValueKind::Reference(_) => LegacyValueKindTag::Reference,
        ValueKind::Set(_, _) => LegacyValueKindTag::Set,
        ValueKind::Option(_) => LegacyValueKindTag::Option,
        ValueKind::Kind(_) => LegacyValueKindTag::Kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_extent_conversion_is_checked_before_narrowing() {
        assert_eq!(checked_extent_value(u64::MAX as u128).unwrap(), u64::MAX);
        assert!(matches!(
            checked_extent_value(u64::MAX as u128 + 1),
            Err(SemanticModelError::LegacyExtentOutOfRange { .. })
        ));
    }
}
