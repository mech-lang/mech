//! Semantic kind expressions and the closed-kind canonical encoder.

use crate::dimension::{
    canonicalize_dimension_environment, collect_dimension_references, encode_dimension_parameters,
    normalize_dimension, rewrite_dimension_references,
};
use crate::{
    CanonicalNominalPath, DimensionExpr, DimensionParameterDeclaration, DimensionParameterId,
    KindId, KindNameCategory, KindParameterId, NominalKey, SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeSet, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum KindExpr {
    Wildcard,
    Never,
    Hole,
    Parameter(KindParameterId),
    Named(KindId),
    Id,
    Index,
    Atom(NominalKey),
    Enum(NominalKey),
    Matrix {
        element: Box<KindExpr>,
        dimensions: Box<[DimensionExpr]>,
    },
    Option(Box<KindExpr>),
    Tuple(Box<[KindExpr]>),
    Record(Box<[KindField]>),
    Table {
        columns: Box<[KindField]>,
        rows: DimensionExpr,
    },
    Set {
        element: Box<KindExpr>,
        cardinality: DimensionExpr,
    },
    Map {
        key: Box<KindExpr>,
        value: Box<KindExpr>,
        cardinality: DimensionExpr,
    },
    Reference(Box<KindExpr>),
    TypeOf(Box<KindExpr>),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KindField {
    pub name: String,
    pub kind: KindExpr,
}

pub trait NamedKindPathResolver {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath>;
}

/// Resolves the frozen legacy scalar identifier through the authoritative
/// built-in semantic kind registry. Wire codecs and compatibility adapters
/// share this registry rather than inventing context-specific nominal paths.
pub fn builtin_scalar_named_kind(
    legacy_id: u64,
) -> Result<(KindId, CanonicalNominalPath), SemanticModelError> {
    const NAMES: &[&str] = &[
        "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64", "c64",
        "r64", "string", "bool",
    ];
    let Some((index, name)) = NAMES
        .iter()
        .copied()
        .enumerate()
        .find(|(_, name)| crate::hash_str(name) == legacy_id)
    else {
        return Err(SemanticModelError::LegacyNamedKindUnresolved { legacy_id });
    };
    Ok((
        KindId::new(index as u32),
        CanonicalNominalPath::new([
            "mech".to_owned(),
            "builtin".to_owned(),
            "scalar".to_owned(),
            name.to_owned(),
        ])?,
    ))
}

pub fn canonical_closed_kind_bytes(
    kind: &KindExpr,
    dimension_parameters: &[DimensionParameterDeclaration],
    named_kinds: &dyn NamedKindPathResolver,
) -> Result<Box<[u8]>, SemanticModelError> {
    validate_kind_structure(kind)?;
    let kind = normalize_kind_dimensions(kind.clone(), dimension_parameters.len())?;
    let mut references = Vec::new();
    collect_kind_dimension_references(&kind, &mut references);
    let environment = canonicalize_dimension_environment(dimension_parameters, &references)?;
    let rewritten = rewrite_kind_dimensions(&kind, &environment.old_to_new)?;
    let rewritten = normalize_kind_dimensions(rewritten, environment.parameters.len())?;

    let mut bytes = Vec::new();
    bytes.push(0x01);
    bytes.extend_from_slice(&(environment.parameters.len() as u32).to_le_bytes());
    encode_dimension_parameters(&environment.parameters, &mut bytes);
    let body = encode_kind_body(&rewritten, named_kinds)?;
    push_node(&mut bytes, &body);
    Ok(bytes.into_boxed_slice())
}

pub(crate) fn validate_kind_structure(kind: &KindExpr) -> Result<(), SemanticModelError> {
    match kind {
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => validate_kind_structure(element)?,
        KindExpr::Tuple(elements) => {
            for element in elements {
                validate_kind_structure(element)?;
            }
        }
        KindExpr::Record(fields) => {
            validate_kind_fields(fields, KindNameCategory::RecordField)?;
        }
        KindExpr::Table { columns, .. } => {
            validate_kind_fields(columns, KindNameCategory::TableColumn)?;
        }
        KindExpr::Map { key, value, .. } => {
            validate_kind_structure(key)?;
            validate_kind_structure(value)?;
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => {}
    }
    Ok(())
}

fn validate_kind_fields(
    fields: &[KindField],
    category: KindNameCategory,
) -> Result<(), SemanticModelError> {
    let mut names = BTreeSet::new();
    for field in fields {
        if !names.insert(&field.name) {
            return Err(SemanticModelError::DuplicateKindName {
                category,
                name: field.name.clone(),
            });
        }
        validate_kind_structure(&field.kind)?;
    }
    Ok(())
}

pub(crate) fn collect_kind_dimension_references(
    kind: &KindExpr,
    references: &mut Vec<DimensionParameterId>,
) {
    match kind {
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => {}
        KindExpr::Matrix {
            element,
            dimensions,
        } => {
            collect_kind_dimension_references(element, references);
            for dimension in dimensions {
                collect_dimension_references(dimension, references);
            }
        }
        KindExpr::Option(element) | KindExpr::Reference(element) | KindExpr::TypeOf(element) => {
            collect_kind_dimension_references(element, references);
        }
        KindExpr::Tuple(elements) => {
            for element in elements {
                collect_kind_dimension_references(element, references);
            }
        }
        KindExpr::Record(fields) => collect_field_dimension_references(fields, references),
        KindExpr::Table { columns, rows } => {
            collect_field_dimension_references(columns, references);
            collect_dimension_references(rows, references);
        }
        KindExpr::Set {
            element,
            cardinality,
        } => {
            collect_kind_dimension_references(element, references);
            collect_dimension_references(cardinality, references);
        }
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => {
            collect_kind_dimension_references(key, references);
            collect_kind_dimension_references(value, references);
            collect_dimension_references(cardinality, references);
        }
    }
}

fn collect_field_dimension_references(
    fields: &[KindField],
    references: &mut Vec<DimensionParameterId>,
) {
    for field in fields {
        collect_kind_dimension_references(&field.kind, references);
    }
}

pub(crate) fn visit_kind_parameters(
    kind: &KindExpr,
    visit: &mut impl FnMut(KindParameterId) -> Result<(), SemanticModelError>,
) -> Result<(), SemanticModelError> {
    match kind {
        KindExpr::Hole => return Err(SemanticModelError::UnresolvedKindHole),
        KindExpr::Parameter(id) => visit(*id)?,
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => visit_kind_parameters(element, visit)?,
        KindExpr::Tuple(elements) => {
            for element in elements {
                visit_kind_parameters(element, visit)?;
            }
        }
        KindExpr::Record(fields)
        | KindExpr::Table {
            columns: fields, ..
        } => {
            for field in fields {
                visit_kind_parameters(&field.kind, visit)?;
            }
        }
        KindExpr::Map { key, value, .. } => {
            visit_kind_parameters(key, visit)?;
            visit_kind_parameters(value, visit)?;
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => {}
    }
    Ok(())
}

pub(crate) fn visit_kind_dimensions(
    kind: &KindExpr,
    visit: &mut impl FnMut(&DimensionExpr) -> Result<(), SemanticModelError>,
) -> Result<(), SemanticModelError> {
    match kind {
        KindExpr::Matrix {
            element,
            dimensions,
        } => {
            visit_kind_dimensions(element, visit)?;
            for dimension in dimensions {
                visit(dimension)?;
            }
        }
        KindExpr::Option(element) | KindExpr::Reference(element) | KindExpr::TypeOf(element) => {
            visit_kind_dimensions(element, visit)?;
        }
        KindExpr::Tuple(elements) => {
            for element in elements {
                visit_kind_dimensions(element, visit)?;
            }
        }
        KindExpr::Record(fields) => {
            for field in fields {
                visit_kind_dimensions(&field.kind, visit)?;
            }
        }
        KindExpr::Table { columns, rows } => {
            for field in columns {
                visit_kind_dimensions(&field.kind, visit)?;
            }
            visit(rows)?;
        }
        KindExpr::Set {
            element,
            cardinality,
        } => {
            visit_kind_dimensions(element, visit)?;
            visit(cardinality)?;
        }
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => {
            visit_kind_dimensions(key, visit)?;
            visit_kind_dimensions(value, visit)?;
            visit(cardinality)?;
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => {}
    }
    Ok(())
}

fn rewrite_kind_dimensions(
    kind: &KindExpr,
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<KindExpr, SemanticModelError> {
    Ok(match kind {
        KindExpr::Wildcard => KindExpr::Wildcard,
        KindExpr::Never => KindExpr::Never,
        KindExpr::Hole => KindExpr::Hole,
        KindExpr::Parameter(id) => KindExpr::Parameter(*id),
        KindExpr::Named(id) => KindExpr::Named(*id),
        KindExpr::Id => KindExpr::Id,
        KindExpr::Index => KindExpr::Index,
        KindExpr::Atom(key) => KindExpr::Atom(*key),
        KindExpr::Enum(key) => KindExpr::Enum(*key),
        KindExpr::Matrix {
            element,
            dimensions,
        } => KindExpr::Matrix {
            element: Box::new(rewrite_kind_dimensions(element, old_to_new)?),
            dimensions: dimensions
                .iter()
                .map(|value| rewrite_dimension_references(value, old_to_new))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        KindExpr::Option(element) => {
            KindExpr::Option(Box::new(rewrite_kind_dimensions(element, old_to_new)?))
        }
        KindExpr::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .map(|value| rewrite_kind_dimensions(value, old_to_new))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindExpr::Record(fields) => KindExpr::Record(rewrite_fields(fields, old_to_new)?),
        KindExpr::Table { columns, rows } => KindExpr::Table {
            columns: rewrite_fields(columns, old_to_new)?,
            rows: rewrite_dimension_references(rows, old_to_new)?,
        },
        KindExpr::Set {
            element,
            cardinality,
        } => KindExpr::Set {
            element: Box::new(rewrite_kind_dimensions(element, old_to_new)?),
            cardinality: rewrite_dimension_references(cardinality, old_to_new)?,
        },
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => KindExpr::Map {
            key: Box::new(rewrite_kind_dimensions(key, old_to_new)?),
            value: Box::new(rewrite_kind_dimensions(value, old_to_new)?),
            cardinality: rewrite_dimension_references(cardinality, old_to_new)?,
        },
        KindExpr::Reference(element) => {
            KindExpr::Reference(Box::new(rewrite_kind_dimensions(element, old_to_new)?))
        }
        KindExpr::TypeOf(element) => {
            KindExpr::TypeOf(Box::new(rewrite_kind_dimensions(element, old_to_new)?))
        }
    })
}

fn rewrite_fields(
    fields: &[KindField],
    old_to_new: &[Option<DimensionParameterId>],
) -> Result<Box<[KindField]>, SemanticModelError> {
    fields
        .iter()
        .map(|field| {
            Ok(KindField {
                name: field.name.clone(),
                kind: rewrite_kind_dimensions(&field.kind, old_to_new)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn normalize_kind_dimensions(
    kind: KindExpr,
    parameter_count: usize,
) -> Result<KindExpr, SemanticModelError> {
    Ok(match kind {
        KindExpr::Matrix {
            element,
            dimensions,
        } => KindExpr::Matrix {
            element: Box::new(normalize_kind_dimensions(*element, parameter_count)?),
            dimensions: dimensions
                .iter()
                .map(|value| normalize_dimension(value, parameter_count))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        KindExpr::Option(element) => KindExpr::Option(Box::new(normalize_kind_dimensions(
            *element,
            parameter_count,
        )?)),
        KindExpr::Tuple(elements) => KindExpr::Tuple(
            elements
                .into_vec()
                .into_iter()
                .map(|value| normalize_kind_dimensions(value, parameter_count))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindExpr::Record(fields) => KindExpr::Record(normalize_fields(fields, parameter_count)?),
        KindExpr::Table { columns, rows } => KindExpr::Table {
            columns: normalize_fields(columns, parameter_count)?,
            rows: normalize_dimension(&rows, parameter_count)?,
        },
        KindExpr::Set {
            element,
            cardinality,
        } => KindExpr::Set {
            element: Box::new(normalize_kind_dimensions(*element, parameter_count)?),
            cardinality: normalize_dimension(&cardinality, parameter_count)?,
        },
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => KindExpr::Map {
            key: Box::new(normalize_kind_dimensions(*key, parameter_count)?),
            value: Box::new(normalize_kind_dimensions(*value, parameter_count)?),
            cardinality: normalize_dimension(&cardinality, parameter_count)?,
        },
        KindExpr::Reference(element) => KindExpr::Reference(Box::new(normalize_kind_dimensions(
            *element,
            parameter_count,
        )?)),
        KindExpr::TypeOf(element) => KindExpr::TypeOf(Box::new(normalize_kind_dimensions(
            *element,
            parameter_count,
        )?)),
        other => other,
    })
}

fn normalize_fields(
    fields: Box<[KindField]>,
    parameter_count: usize,
) -> Result<Box<[KindField]>, SemanticModelError> {
    fields
        .into_vec()
        .into_iter()
        .map(|field| {
            Ok(KindField {
                name: field.name,
                kind: normalize_kind_dimensions(field.kind, parameter_count)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn encode_kind_body(
    kind: &KindExpr,
    named_kinds: &dyn NamedKindPathResolver,
) -> Result<Vec<u8>, SemanticModelError> {
    let mut bytes = Vec::new();
    match kind {
        KindExpr::Wildcard => bytes.push(0x01),
        KindExpr::Never => bytes.push(0x02),
        KindExpr::Hole => return Err(SemanticModelError::UnresolvedKindHole),
        KindExpr::Parameter(id) => {
            return Err(SemanticModelError::KindParameterNotClosed { id: *id });
        }
        KindExpr::Named(id) => {
            bytes.push(0x04);
            let path = named_kinds
                .canonical_path(*id)
                .ok_or(SemanticModelError::UnknownNamedKind { id: *id })?;
            bytes.extend_from_slice(&path.canonical_bytes());
        }
        KindExpr::Id => bytes.push(0x05),
        KindExpr::Index => bytes.push(0x06),
        KindExpr::Atom(key) => {
            bytes.push(0x07);
            bytes.extend_from_slice(key.as_bytes());
        }
        KindExpr::Enum(key) => {
            bytes.push(0x08);
            bytes.extend_from_slice(key.as_bytes());
        }
        KindExpr::Matrix {
            element,
            dimensions,
        } => {
            bytes.push(0x09);
            push_encoded_kind(&mut bytes, element, named_kinds)?;
            bytes.extend_from_slice(&(dimensions.len() as u32).to_le_bytes());
            for dimension in dimensions {
                let encoded = crate::dimension::encode_normalized_dimension(dimension);
                push_node(&mut bytes, &encoded);
            }
        }
        KindExpr::Option(element) => {
            bytes.push(0x0a);
            push_encoded_kind(&mut bytes, element, named_kinds)?;
        }
        KindExpr::Tuple(elements) => {
            bytes.push(0x0b);
            bytes.extend_from_slice(&(elements.len() as u32).to_le_bytes());
            for element in elements {
                push_encoded_kind(&mut bytes, element, named_kinds)?;
            }
        }
        KindExpr::Record(fields) => {
            bytes.push(0x0c);
            encode_fields(&mut bytes, fields, named_kinds)?;
        }
        KindExpr::Table { columns, rows } => {
            bytes.push(0x0d);
            encode_fields(&mut bytes, columns, named_kinds)?;
            let rows = crate::dimension::encode_normalized_dimension(rows);
            push_node(&mut bytes, &rows);
        }
        KindExpr::Set {
            element,
            cardinality,
        } => {
            bytes.push(0x0e);
            push_encoded_kind(&mut bytes, element, named_kinds)?;
            let cardinality = crate::dimension::encode_normalized_dimension(cardinality);
            push_node(&mut bytes, &cardinality);
        }
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => {
            bytes.push(0x0f);
            push_encoded_kind(&mut bytes, key, named_kinds)?;
            push_encoded_kind(&mut bytes, value, named_kinds)?;
            let cardinality = crate::dimension::encode_normalized_dimension(cardinality);
            push_node(&mut bytes, &cardinality);
        }
        KindExpr::Reference(element) => {
            bytes.push(0x10);
            push_encoded_kind(&mut bytes, element, named_kinds)?;
        }
        KindExpr::TypeOf(element) => {
            bytes.push(0x11);
            push_encoded_kind(&mut bytes, element, named_kinds)?;
        }
    }
    Ok(bytes)
}

fn push_encoded_kind(
    bytes: &mut Vec<u8>,
    kind: &KindExpr,
    named_kinds: &dyn NamedKindPathResolver,
) -> Result<(), SemanticModelError> {
    let encoded = encode_kind_body(kind, named_kinds)?;
    push_node(bytes, &encoded);
    Ok(())
}

fn encode_fields(
    bytes: &mut Vec<u8>,
    fields: &[KindField],
    named_kinds: &dyn NamedKindPathResolver,
) -> Result<(), SemanticModelError> {
    bytes.extend_from_slice(&(fields.len() as u32).to_le_bytes());
    for field in fields {
        push_utf8(bytes, &field.name);
        push_encoded_kind(bytes, &field.kind, named_kinds)?;
    }
    Ok(())
}

fn push_utf8(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_node(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
}
