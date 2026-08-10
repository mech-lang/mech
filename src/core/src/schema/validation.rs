use super::{EnumVariantSchema, Schema, SchemaBody, SchemaDraft, SchemaField};
use crate::dimension::{
    canonicalize_dimension_environment, collect_dimension_references, normalize_dimension,
    rewrite_dimension_references,
};
use crate::{DimensionParameterId, SchemaNameCategory, SemanticModelError};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};

pub(super) fn finalize_schema(draft: SchemaDraft) -> Result<Schema, SemanticModelError> {
    validate_names_and_keyability(&draft.body)?;
    let body = normalize_body_dimensions(draft.body, draft.dimension_parameters.len())?;
    let mut references = Vec::new();
    collect_body_dimension_references(&body, &mut references);
    let environment = canonicalize_dimension_environment(&draft.dimension_parameters, &references)?;
    let body = rewrite_body_dimensions(&body, &environment.old_to_new)?;
    let body = normalize_body_dimensions(body, environment.parameters.len())?;
    validate_names_and_keyability(&body)?;
    Ok(Schema {
        dimension_parameters: environment.parameters,
        body,
    })
}

fn validate_names_and_keyability(body: &SchemaBody) -> Result<(), SemanticModelError> {
    match body {
        SchemaBody::Enum { variants, .. } => {
            validate_unique_names(
                variants.iter().map(|variant| &variant.name),
                SchemaNameCategory::EnumVariant,
            )?;
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    validate_names_and_keyability(payload)?;
                }
            }
        }
        SchemaBody::Option(element) => validate_names_and_keyability(element)?,
        SchemaBody::Tuple(elements) => {
            for element in elements {
                validate_names_and_keyability(element)?;
            }
        }
        SchemaBody::Record(fields) => {
            validate_unique_names(
                fields.iter().map(|field| &field.name),
                SchemaNameCategory::RecordField,
            )?;
            for field in fields {
                validate_names_and_keyability(&field.schema)?;
            }
        }
        SchemaBody::Matrix { element, .. } => validate_names_and_keyability(element)?,
        SchemaBody::Table { columns, .. } => {
            validate_unique_names(
                columns.iter().map(|field| &field.name),
                SchemaNameCategory::TableColumn,
            )?;
            for column in columns {
                validate_names_and_keyability(&column.schema)?;
            }
        }
        SchemaBody::Set { element, .. } => {
            validate_names_and_keyability(element)?;
            if !is_body_keyable(element) {
                return Err(SemanticModelError::SchemaNotKeyableV1);
            }
        }
        SchemaBody::Map { key, value, .. } => {
            validate_names_and_keyability(key)?;
            if !is_body_keyable(key) {
                return Err(SemanticModelError::SchemaNotKeyableV1);
            }
            validate_names_and_keyability(value)?;
        }
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
        | SchemaBody::ReifiedType => {}
    }
    Ok(())
}

fn validate_unique_names<'a>(
    names: impl IntoIterator<Item = &'a String>,
    category: SchemaNameCategory,
) -> Result<(), SemanticModelError> {
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(SemanticModelError::DuplicateSchemaNameV1 {
                category,
                name: name.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn is_body_keyable(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_) => true,
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .all(|variant| variant.payload.as_ref().is_none_or(is_body_keyable)),
        SchemaBody::Option(element) => is_body_keyable(element),
        SchemaBody::Tuple(elements) => elements.iter().all(is_body_keyable),
        SchemaBody::Record(fields) => fields.iter().all(|field| is_body_keyable(&field.schema)),
        SchemaBody::Complex(_)
        | SchemaBody::Matrix { .. }
        | SchemaBody::Table { .. }
        | SchemaBody::Set { .. }
        | SchemaBody::Map { .. }
        | SchemaBody::ReifiedType => false,
    }
}

pub(super) fn collect_body_dimension_references(
    body: &SchemaBody,
    references: &mut Vec<DimensionParameterId>,
) {
    match body {
        SchemaBody::Enum { variants, .. } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    collect_body_dimension_references(payload, references);
                }
            }
        }
        SchemaBody::Option(element) => collect_body_dimension_references(element, references),
        SchemaBody::Tuple(elements) => {
            for element in elements {
                collect_body_dimension_references(element, references);
            }
        }
        SchemaBody::Record(fields) => collect_field_dimension_references(fields, references),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            collect_body_dimension_references(element, references);
            for dimension in dimensions {
                collect_dimension_references(dimension, references);
            }
        }
        SchemaBody::Table { columns, rows } => {
            collect_field_dimension_references(columns, references);
            collect_dimension_references(rows, references);
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            collect_body_dimension_references(element, references);
            collect_dimension_references(cardinality, references);
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            collect_body_dimension_references(key, references);
            collect_body_dimension_references(value, references);
            collect_dimension_references(cardinality, references);
        }
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
        | SchemaBody::ReifiedType => {}
    }
}

fn collect_field_dimension_references(
    fields: &[SchemaField],
    references: &mut Vec<DimensionParameterId>,
) {
    for field in fields {
        collect_body_dimension_references(&field.schema, references);
    }
}

fn rewrite_body_dimensions(
    body: &SchemaBody,
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<SchemaBody, SemanticModelError> {
    Ok(match body {
        SchemaBody::Bool => SchemaBody::Bool,
        SchemaBody::UnsignedInteger(width) => SchemaBody::UnsignedInteger(*width),
        SchemaBody::SignedInteger(width) => SchemaBody::SignedInteger(*width),
        SchemaBody::FloatingPoint(width) => SchemaBody::FloatingPoint(*width),
        SchemaBody::Complex(width) => SchemaBody::Complex(*width),
        SchemaBody::Rational64 => SchemaBody::Rational64,
        SchemaBody::String => SchemaBody::String,
        SchemaBody::Id => SchemaBody::Id,
        SchemaBody::Index => SchemaBody::Index,
        SchemaBody::Atom(key) => SchemaBody::Atom(*key),
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key: *key,
            variants: variants
                .iter()
                .map(|variant| {
                    Ok(EnumVariantSchema {
                        name: variant.name.clone(),
                        payload: variant
                            .payload
                            .as_ref()
                            .map(|payload| rewrite_body_dimensions(payload, old_to_new))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => {
            SchemaBody::Option(Box::new(rewrite_body_dimensions(element, old_to_new)?))
        }
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| rewrite_body_dimensions(element, old_to_new))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(rewrite_fields(fields, old_to_new)?),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(rewrite_body_dimensions(element, old_to_new)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| rewrite_dimension_references(dimension, old_to_new))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: rewrite_fields(columns, old_to_new)?,
            rows: rewrite_dimension_references(rows, old_to_new)?,
        },
        SchemaBody::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(rewrite_body_dimensions(element, old_to_new)?),
            cardinality: rewrite_dimension_references(cardinality, old_to_new)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(rewrite_body_dimensions(key, old_to_new)?),
            value: Box::new(rewrite_body_dimensions(value, old_to_new)?),
            cardinality: rewrite_dimension_references(cardinality, old_to_new)?,
        },
        SchemaBody::ReifiedType => SchemaBody::ReifiedType,
    })
}

fn rewrite_fields(
    fields: &[SchemaField],
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<Box<[SchemaField]>, SemanticModelError> {
    fields
        .iter()
        .map(|field| {
            Ok(SchemaField {
                name: field.name.clone(),
                schema: rewrite_body_dimensions(&field.schema, old_to_new)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn normalize_body_dimensions(
    body: SchemaBody,
    parameter_count: usize,
) -> Result<SchemaBody, SemanticModelError> {
    Ok(match body {
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key,
            variants: variants
                .into_vec()
                .into_iter()
                .map(|variant| {
                    Ok(EnumVariantSchema {
                        name: variant.name,
                        payload: variant
                            .payload
                            .map(|payload| normalize_body_dimensions(payload, parameter_count))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => SchemaBody::Option(Box::new(normalize_body_dimensions(
            *element,
            parameter_count,
        )?)),
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .into_vec()
                .into_iter()
                .map(|element| normalize_body_dimensions(element, parameter_count))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => {
            SchemaBody::Record(normalize_fields(fields, parameter_count)?)
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(normalize_body_dimensions(*element, parameter_count)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| normalize_dimension(dimension, parameter_count))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: normalize_fields(columns, parameter_count)?,
            rows: normalize_dimension(&rows, parameter_count)?,
        },
        SchemaBody::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(normalize_body_dimensions(*element, parameter_count)?),
            cardinality: normalize_dimension(&cardinality, parameter_count)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(normalize_body_dimensions(*key, parameter_count)?),
            value: Box::new(normalize_body_dimensions(*value, parameter_count)?),
            cardinality: normalize_dimension(&cardinality, parameter_count)?,
        },
        other => other,
    })
}

fn normalize_fields(
    fields: Box<[SchemaField]>,
    parameter_count: usize,
) -> Result<Box<[SchemaField]>, SemanticModelError> {
    fields
        .into_vec()
        .into_iter()
        .map(|field| {
            Ok(SchemaField {
                name: field.name,
                schema: normalize_body_dimensions(field.schema, parameter_count)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}
