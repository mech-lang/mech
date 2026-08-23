#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeMap, BTreeSet};

use crate::{MResult, hash_str, kind::Kind};

use super::{
    ByteReader, MAX_TYPE_RECURSION, checked_usize, invalid, write_string, write_u32, write_u64,
};

/// Maximum number of tree nodes materialized while resolving the compact
/// bytecode type DAG. `RuntimeType` is intentionally a tree, so shared raw
/// children must be charged once for every expanded occurrence before any
/// cloning or allocation takes place.
const MAX_EXPANDED_RUNTIME_TYPE_NODES: usize = 1_000_000;
const MAX_INLINE_RUNTIME_TYPE_NODES: usize = MAX_EXPANDED_RUNTIME_TYPE_NODES;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    C64,
    R64,
    String,
    Bool,
    Id,
    Index,
    Empty,
    Any,
    None,
    Matrix {
        element: Box<RuntimeType>,
        storage: MatrixStorage,
        rows: u32,
        cols: u32,
    },
    Enum {
        id: u64,
        name: String,
    },
    Record(Vec<(String, RuntimeType)>),
    Map {
        key: Box<RuntimeType>,
        value: Box<RuntimeType>,
    },
    Atom {
        id: u64,
        name: String,
    },
    Table {
        columns: Vec<(String, RuntimeType)>,
        primary_key: u32,
    },
    Tuple(Vec<RuntimeType>),
    Reference(Box<RuntimeType>),
    Set {
        element: Box<RuntimeType>,
        max_len: Option<u32>,
    },
    Option(Box<RuntimeType>),
    Kind(Kind),
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeTypeTag {
    U8 = 1,
    U16 = 2,
    U32 = 3,
    U64 = 4,
    U128 = 5,
    I8 = 6,
    I16 = 7,
    I32 = 8,
    I64 = 9,
    I128 = 10,
    F32 = 11,
    F64 = 12,
    C64 = 13,
    R64 = 14,
    String = 15,
    Bool = 16,
    Id = 17,
    Index = 18,
    Empty = 19,
    Any = 20,
    None = 21,
    Matrix = 22,
    Enum = 23,
    Record = 24,
    Map = 25,
    Atom = 26,
    Table = 27,
    Tuple = 28,
    Reference = 29,
    Set = 30,
    Option = 31,
    Kind = 32,
}

impl RuntimeTypeTag {
    pub(crate) fn from_u16(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::U8,
            2 => Self::U16,
            3 => Self::U32,
            4 => Self::U64,
            5 => Self::U128,
            6 => Self::I8,
            7 => Self::I16,
            8 => Self::I32,
            9 => Self::I64,
            10 => Self::I128,
            11 => Self::F32,
            12 => Self::F64,
            13 => Self::C64,
            14 => Self::R64,
            15 => Self::String,
            16 => Self::Bool,
            17 => Self::Id,
            18 => Self::Index,
            19 => Self::Empty,
            20 => Self::Any,
            21 => Self::None,
            22 => Self::Matrix,
            23 => Self::Enum,
            24 => Self::Record,
            25 => Self::Map,
            26 => Self::Atom,
            27 => Self::Table,
            28 => Self::Tuple,
            29 => Self::Reference,
            30 => Self::Set,
            31 => Self::Option,
            32 => Self::Kind,
            _ => return None,
        })
    }
}

#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatrixStorage {
    Matrix1 = 1,
    Matrix2 = 2,
    Matrix3 = 3,
    Matrix4 = 4,
    Matrix2x3 = 5,
    Matrix3x2 = 6,
    RowVector2 = 7,
    RowVector3 = 8,
    RowVector4 = 9,
    Vector2 = 10,
    Vector3 = 11,
    Vector4 = 12,
    RowVectorD = 13,
    VectorD = 14,
    MatrixD = 15,
}

impl MatrixStorage {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Matrix1,
            2 => Self::Matrix2,
            3 => Self::Matrix3,
            4 => Self::Matrix4,
            5 => Self::Matrix2x3,
            6 => Self::Matrix3x2,
            7 => Self::RowVector2,
            8 => Self::RowVector3,
            9 => Self::RowVector4,
            10 => Self::Vector2,
            11 => Self::Vector3,
            12 => Self::Vector4,
            13 => Self::RowVectorD,
            14 => Self::VectorD,
            15 => Self::MatrixD,
            _ => return None,
        })
    }

    pub fn validate_dimensions(self, rows: u32, cols: u32) -> bool {
        match self {
            Self::Matrix1 => (rows, cols) == (1, 1),
            Self::Matrix2 => (rows, cols) == (2, 2),
            Self::Matrix3 => (rows, cols) == (3, 3),
            Self::Matrix4 => (rows, cols) == (4, 4),
            Self::Matrix2x3 => (rows, cols) == (2, 3),
            Self::Matrix3x2 => (rows, cols) == (3, 2),
            Self::RowVector2 => (rows, cols) == (1, 2),
            Self::RowVector3 => (rows, cols) == (1, 3),
            Self::RowVector4 => (rows, cols) == (1, 4),
            Self::Vector2 => (rows, cols) == (2, 1),
            Self::Vector3 => (rows, cols) == (3, 1),
            Self::Vector4 => (rows, cols) == (4, 1),
            Self::RowVectorD => rows == 1 && cols > 0,
            Self::VectorD => rows > 0 && cols == 1,
            // A dynamic matrix owns its dimensions, including canonical empty
            // shapes such as 0x0. Fixed and vector storage classes retain
            // their stricter dimensional contracts above.
            Self::MatrixD => true,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodedKindTag {
    Any = 1,
    None = 2,
    Atom = 3,
    Empty = 4,
    Enum = 5,
    Id = 6,
    Index = 7,
    Map = 8,
    Matrix = 9,
    Option = 10,
    Record = 11,
    Reference = 12,
    Scalar = 13,
    Set = 14,
    Table = 15,
    Tuple = 16,
    Kind = 17,
}

impl RuntimeType {
    pub(crate) fn tag(&self) -> RuntimeTypeTag {
        match self {
            Self::U8 => RuntimeTypeTag::U8,
            Self::U16 => RuntimeTypeTag::U16,
            Self::U32 => RuntimeTypeTag::U32,
            Self::U64 => RuntimeTypeTag::U64,
            Self::U128 => RuntimeTypeTag::U128,
            Self::I8 => RuntimeTypeTag::I8,
            Self::I16 => RuntimeTypeTag::I16,
            Self::I32 => RuntimeTypeTag::I32,
            Self::I64 => RuntimeTypeTag::I64,
            Self::I128 => RuntimeTypeTag::I128,
            Self::F32 => RuntimeTypeTag::F32,
            Self::F64 => RuntimeTypeTag::F64,
            Self::C64 => RuntimeTypeTag::C64,
            Self::R64 => RuntimeTypeTag::R64,
            Self::String => RuntimeTypeTag::String,
            Self::Bool => RuntimeTypeTag::Bool,
            Self::Id => RuntimeTypeTag::Id,
            Self::Index => RuntimeTypeTag::Index,
            Self::Empty => RuntimeTypeTag::Empty,
            Self::Any => RuntimeTypeTag::Any,
            Self::None => RuntimeTypeTag::None,
            Self::Matrix { .. } => RuntimeTypeTag::Matrix,
            Self::Enum { .. } => RuntimeTypeTag::Enum,
            Self::Record(_) => RuntimeTypeTag::Record,
            Self::Map { .. } => RuntimeTypeTag::Map,
            Self::Atom { .. } => RuntimeTypeTag::Atom,
            Self::Table { .. } => RuntimeTypeTag::Table,
            Self::Tuple(_) => RuntimeTypeTag::Tuple,
            Self::Reference(_) => RuntimeTypeTag::Reference,
            Self::Set { .. } => RuntimeTypeTag::Set,
            Self::Option(_) => RuntimeTypeTag::Option,
            Self::Kind(_) => RuntimeTypeTag::Kind,
        }
    }

    fn children(&self) -> Vec<&RuntimeType> {
        match self {
            Self::Matrix { element, .. }
            | Self::Reference(element)
            | Self::Option(element)
            | Self::Set { element, .. } => vec![element],
            Self::Map { key, value } => vec![key, value],
            Self::Record(fields) => fields.iter().map(|(_, ty)| ty).collect(),
            Self::Table { columns, .. } => columns.iter().map(|(_, ty)| ty).collect(),
            Self::Tuple(types) => types.iter().collect(),
            _ => Vec::new(),
        }
    }
}

fn canonical_key(ty: &RuntimeType, depth: usize) -> MResult<Vec<u8>> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("runtime type recursion exceeds bytecode v1 limit");
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(ty.tag() as u16).to_le_bytes());
    match ty {
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => {
            out.push(*storage as u8);
            write_u32(&mut out, *rows);
            write_u32(&mut out, *cols);
            let child = canonical_key(element, depth + 1)?;
            write_u32(
                &mut out,
                child.len().try_into().map_err(|_| {
                    invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                })?,
            );
            out.extend(child);
        }
        RuntimeType::Enum { id, name } | RuntimeType::Atom { id, name } => {
            write_u64(&mut out, *id);
            write_string(&mut out, name)?;
        }
        RuntimeType::Record(fields) => {
            write_u32(
                &mut out,
                fields
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("record field count exceeds u32").unwrap_err())?,
            );
            for (name, child) in fields {
                write_string(&mut out, name)?;
                let key = canonical_key(child, depth + 1)?;
                write_u32(
                    &mut out,
                    key.len().try_into().map_err(|_| {
                        invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                    })?,
                );
                out.extend(key);
            }
        }
        RuntimeType::Map { key, value } => {
            for child in [key.as_ref(), value.as_ref()] {
                let key = canonical_key(child, depth + 1)?;
                write_u32(
                    &mut out,
                    key.len().try_into().map_err(|_| {
                        invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                    })?,
                );
                out.extend(key);
            }
        }
        RuntimeType::Table {
            columns,
            primary_key,
        } => {
            write_u32(
                &mut out,
                columns
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("table column count exceeds u32").unwrap_err())?,
            );
            for (name, child) in columns {
                write_string(&mut out, name)?;
                let key = canonical_key(child, depth + 1)?;
                write_u32(
                    &mut out,
                    key.len().try_into().map_err(|_| {
                        invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                    })?,
                );
                out.extend(key);
            }
            write_u32(&mut out, *primary_key);
        }
        RuntimeType::Tuple(types) => {
            write_u32(
                &mut out,
                types
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("tuple type count exceeds u32").unwrap_err())?,
            );
            for child in types {
                let key = canonical_key(child, depth + 1)?;
                write_u32(
                    &mut out,
                    key.len().try_into().map_err(|_| {
                        invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                    })?,
                );
                out.extend(key);
            }
        }
        RuntimeType::Reference(child) | RuntimeType::Option(child) => {
            let key = canonical_key(child, depth + 1)?;
            write_u32(
                &mut out,
                key.len().try_into().map_err(|_| {
                    invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                })?,
            );
            out.extend(key);
        }
        RuntimeType::Set { element, max_len } => {
            let key = canonical_key(element, depth + 1)?;
            write_u32(
                &mut out,
                key.len().try_into().map_err(|_| {
                    invalid::<()>("canonical runtime type key exceeds u32").unwrap_err()
                })?,
            );
            out.extend(key);
            match max_len {
                Some(value) => {
                    out.push(1);
                    write_u32(&mut out, *value);
                }
                None => out.push(0),
            }
        }
        RuntimeType::Kind(kind) => encode_kind(kind, &mut out, depth + 1)?,
        _ => {}
    }
    Ok(out)
}

pub(crate) fn canonical_runtime_type_key(ty: &RuntimeType) -> MResult<Vec<u8>> {
    validate_runtime_type(ty, 0)?;
    canonical_key(ty, 0)
}

pub(crate) fn decode_canonical_runtime_type_key(bytes: &[u8]) -> MResult<RuntimeType> {
    fn decode(
        reader: &mut ByteReader<'_>,
        depth: usize,
        remaining_nodes: &mut usize,
    ) -> MResult<RuntimeType> {
        if depth > MAX_TYPE_RECURSION {
            return invalid("canonical runtime type key exceeds bytecode v1 recursion limit");
        }
        *remaining_nodes = remaining_nodes.checked_sub(1).ok_or_else(|| {
            invalid::<()>("canonical runtime type key exceeds inline node limit").unwrap_err()
        })?;
        let tag = RuntimeTypeTag::from_u16(reader.read_u16("canonical runtime type tag")?)
            .ok_or_else(|| invalid::<()>("unknown canonical runtime type tag").unwrap_err())?;
        let child = |reader: &mut ByteReader<'_>,
                     depth: usize,
                     remaining_nodes: &mut usize,
                     what: &str|
         -> MResult<RuntimeType> {
            let length = checked_usize(
                u64::from(reader.read_u32(&format!("{what} length"))?),
                &format!("{what} length"),
            )?;
            let bytes = reader.read_exact(length, what)?;
            let mut nested = ByteReader::new(bytes);
            let ty = decode(&mut nested, depth + 1, remaining_nodes)?;
            if !nested.is_empty() {
                return invalid(format!("{what} has trailing bytes"));
            }
            Ok(ty)
        };
        Ok(match tag {
            RuntimeTypeTag::U8 => RuntimeType::U8,
            RuntimeTypeTag::U16 => RuntimeType::U16,
            RuntimeTypeTag::U32 => RuntimeType::U32,
            RuntimeTypeTag::U64 => RuntimeType::U64,
            RuntimeTypeTag::U128 => RuntimeType::U128,
            RuntimeTypeTag::I8 => RuntimeType::I8,
            RuntimeTypeTag::I16 => RuntimeType::I16,
            RuntimeTypeTag::I32 => RuntimeType::I32,
            RuntimeTypeTag::I64 => RuntimeType::I64,
            RuntimeTypeTag::I128 => RuntimeType::I128,
            RuntimeTypeTag::F32 => RuntimeType::F32,
            RuntimeTypeTag::F64 => RuntimeType::F64,
            RuntimeTypeTag::C64 => RuntimeType::C64,
            RuntimeTypeTag::R64 => RuntimeType::R64,
            RuntimeTypeTag::String => RuntimeType::String,
            RuntimeTypeTag::Bool => RuntimeType::Bool,
            RuntimeTypeTag::Id => RuntimeType::Id,
            RuntimeTypeTag::Index => RuntimeType::Index,
            RuntimeTypeTag::Empty => RuntimeType::Empty,
            RuntimeTypeTag::Any => RuntimeType::Any,
            RuntimeTypeTag::None => RuntimeType::None,
            RuntimeTypeTag::Matrix => {
                let storage = MatrixStorage::from_u8(reader.read_u8("canonical matrix storage")?)
                    .ok_or_else(|| {
                    invalid::<()>("unknown canonical matrix storage").unwrap_err()
                })?;
                let rows = reader.read_u32("canonical matrix rows")?;
                let cols = reader.read_u32("canonical matrix columns")?;
                if !storage.validate_dimensions(rows, cols) {
                    return invalid("canonical matrix storage and dimensions disagree");
                }
                RuntimeType::Matrix {
                    storage,
                    rows,
                    cols,
                    element: Box::new(child(
                        reader,
                        depth,
                        remaining_nodes,
                        "canonical matrix element type",
                    )?),
                }
            }
            RuntimeTypeTag::Enum => {
                let id = reader.read_u64("canonical enum ID")?;
                let name = reader.read_string("canonical enum name")?;
                validate_named_id("runtime enum", id, &name)?;
                RuntimeType::Enum { id, name }
            }
            RuntimeTypeTag::Atom => {
                let id = reader.read_u64("canonical atom ID")?;
                let name = reader.read_string("canonical atom name")?;
                validate_named_id("runtime atom", id, &name)?;
                RuntimeType::Atom { id, name }
            }
            RuntimeTypeTag::Record => {
                let encoded_count = reader.read_u32("canonical record field count")?;
                let count = checked_inline_type_count(
                    reader,
                    encoded_count,
                    8,
                    0,
                    *remaining_nodes,
                    "canonical record field count",
                )?;
                let mut fields = try_vec_with_capacity(count, "canonical record fields")?;
                for _ in 0..count {
                    fields.push((
                        reader.read_string("canonical record field name")?,
                        child(
                            reader,
                            depth,
                            remaining_nodes,
                            "canonical record field type",
                        )?,
                    ));
                }
                RuntimeType::Record(fields)
            }
            RuntimeTypeTag::Map => RuntimeType::Map {
                key: Box::new(child(
                    reader,
                    depth,
                    remaining_nodes,
                    "canonical map key type",
                )?),
                value: Box::new(child(
                    reader,
                    depth,
                    remaining_nodes,
                    "canonical map value type",
                )?),
            },
            RuntimeTypeTag::Table => {
                let encoded_count = reader.read_u32("canonical table column count")?;
                let count = checked_inline_type_count(
                    reader,
                    encoded_count,
                    8,
                    4,
                    *remaining_nodes,
                    "canonical table column count",
                )?;
                let mut columns = try_vec_with_capacity(count, "canonical table columns")?;
                for _ in 0..count {
                    columns.push((
                        reader.read_string("canonical table column name")?,
                        child(
                            reader,
                            depth,
                            remaining_nodes,
                            "canonical table column type",
                        )?,
                    ));
                }
                let primary_key = reader.read_u32("canonical table primary key")?;
                if primary_key != 0 {
                    return invalid("canonical table primary keys other than zero are unsupported");
                }
                RuntimeType::Table {
                    columns,
                    primary_key,
                }
            }
            RuntimeTypeTag::Tuple => {
                let encoded_count = reader.read_u32("canonical tuple type count")?;
                let count = checked_inline_type_count(
                    reader,
                    encoded_count,
                    4,
                    0,
                    *remaining_nodes,
                    "canonical tuple type count",
                )?;
                let mut types = try_vec_with_capacity(count, "canonical tuple types")?;
                for _ in 0..count {
                    types.push(child(
                        reader,
                        depth,
                        remaining_nodes,
                        "canonical tuple child type",
                    )?);
                }
                RuntimeType::Tuple(types)
            }
            RuntimeTypeTag::Reference => RuntimeType::Reference(Box::new(child(
                reader,
                depth,
                remaining_nodes,
                "canonical reference child type",
            )?)),
            RuntimeTypeTag::Set => {
                let element = Box::new(child(
                    reader,
                    depth,
                    remaining_nodes,
                    "canonical set element type",
                )?);
                let max_len = match reader.read_u8("canonical set limit presence")? {
                    0 => None,
                    1 => Some(reader.read_u32("canonical set limit")?),
                    _ => return invalid("invalid canonical set limit presence"),
                };
                RuntimeType::Set { element, max_len }
            }
            RuntimeTypeTag::Option => RuntimeType::Option(Box::new(child(
                reader,
                depth,
                remaining_nodes,
                "canonical option child type",
            )?)),
            RuntimeTypeTag::Kind => RuntimeType::Kind(decode_kind(reader, depth + 1)?),
        })
    }

    let mut reader = ByteReader::new(bytes);
    let mut remaining_nodes = MAX_INLINE_RUNTIME_TYPE_NODES;
    let ty = decode(&mut reader, 0, &mut remaining_nodes)?;
    if !reader.is_empty() {
        return invalid("canonical runtime type key has trailing bytes");
    }
    validate_runtime_type(&ty, 0)?;
    Ok(ty)
}

fn dependency_depth(ty: &RuntimeType, depth: usize) -> MResult<usize> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("runtime type recursion exceeds bytecode v1 limit");
    }
    let mut result = 0usize;
    for child in ty.children() {
        result = result.max(
            dependency_depth(child, depth + 1)?
                .checked_add(1)
                .ok_or_else(|| invalid::<()>("runtime type depth overflow").unwrap_err())?,
        );
    }
    Ok(result)
}

fn collect_type(ty: &RuntimeType, types: &mut BTreeSet<RuntimeType>, depth: usize) -> MResult<()> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("runtime type recursion exceeds bytecode v1 limit");
    }
    for child in ty.children() {
        collect_type(child, types, depth + 1)?;
    }
    types.insert(ty.clone());
    Ok(())
}

pub fn finalize_runtime_types<'a>(
    roots: impl IntoIterator<Item = &'a RuntimeType>,
) -> MResult<(Vec<RuntimeType>, BTreeMap<RuntimeType, u32>)> {
    let mut collected = BTreeSet::new();
    for root in roots {
        validate_runtime_type(root, 0)?;
        collect_type(root, &mut collected, 0)?;
    }
    let mut sortable = collected
        .into_iter()
        .map(|ty| Ok((dependency_depth(&ty, 0)?, canonical_key(&ty, 0)?, ty)))
        .collect::<MResult<Vec<_>>>()?;
    sortable.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let types = sortable
        .into_iter()
        .map(|(_, _, ty)| ty)
        .collect::<Vec<_>>();
    if types.len() > u32::MAX as usize {
        return invalid("runtime type count exceeds u32");
    }
    let ids = types
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, ty)| (ty, id as u32))
        .collect();
    Ok((types, ids))
}

fn validate_named_schema<'a>(
    category: &'static str,
    names: impl IntoIterator<Item = &'a str>,
) -> MResult<()> {
    validate_named_schema_with_hash(category, names, hash_str)
}

fn validate_named_schema_with_hash<'a>(
    category: &'static str,
    names: impl IntoIterator<Item = &'a str>,
    hash: impl Fn(&str) -> u64,
) -> MResult<()> {
    let mut exact_names = BTreeSet::new();
    let mut names_by_id = BTreeMap::new();
    for incoming in names {
        if incoming.is_empty() {
            return invalid(format!("{category} name must not be empty"));
        }
        let stable_id = hash(incoming);
        if !exact_names.insert(incoming) {
            return invalid(format!(
                "{category} schema has duplicate name `{incoming}` (stable ID {stable_id})"
            ));
        }
        if let Some(existing) = names_by_id.insert(stable_id, incoming) {
            return invalid(format!(
                "{category} schema name collision: existing `{existing}`, incoming `{incoming}`, stable ID {stable_id}"
            ));
        }
    }
    Ok(())
}

fn validate_named_id(category: &'static str, id: u64, name: &str) -> MResult<()> {
    if name.is_empty() {
        return invalid(format!("{category} name must not be empty"));
    }

    let expected = hash_str(name);
    if id != expected {
        return invalid(format!(
            "{category} ID 0x{id:016x} does not match the stable hash of name {name:?}; expected 0x{expected:016x}"
        ));
    }

    Ok(())
}

fn validate_scalar_kind_id(id: u64) -> MResult<()> {
    if [
        "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64", "c64",
        "r64", "string", "bool",
    ]
    .into_iter()
    .any(|name| hash_str(name) == id)
    {
        Ok(())
    } else {
        invalid("Kind scalar ID does not identify a canonical runtime scalar")
    }
}

fn validate_matrix_element_type(element: &RuntimeType) -> MResult<()> {
    match element {
        RuntimeType::Bool
        | RuntimeType::String
        | RuntimeType::U8
        | RuntimeType::U16
        | RuntimeType::U32
        | RuntimeType::U64
        | RuntimeType::U128
        | RuntimeType::I8
        | RuntimeType::I16
        | RuntimeType::I32
        | RuntimeType::I64
        | RuntimeType::I128
        | RuntimeType::F32
        | RuntimeType::F64
        | RuntimeType::C64
        | RuntimeType::R64
        | RuntimeType::Index => Ok(()),
        _ => invalid("matrix element type is not supported by bytecode v1"),
    }
}

fn validate_kind(kind: &Kind, depth: usize) -> MResult<()> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("semantic kind recursion exceeds bytecode v1 limit");
    }
    match kind {
        Kind::Atom(id, name) => validate_named_id("kind atom", *id, name)?,
        Kind::Enum(id, name) => validate_named_id("kind enum", *id, name)?,
        Kind::Map(key, value) => {
            validate_kind(key, depth + 1)?;
            validate_kind(value, depth + 1)?;
        }
        Kind::Matrix(element, dimensions) => {
            let _: u32 = dimensions
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("too many kind dimensions").unwrap_err())?;
            for dimension in dimensions {
                let _: u32 = (*dimension)
                    .try_into()
                    .map_err(|_| invalid::<()>("kind dimension exceeds u32").unwrap_err())?;
            }
            validate_kind(element, depth + 1)?;
        }
        Kind::Option(inner) | Kind::Reference(inner) | Kind::Kind(inner) => {
            validate_kind(inner, depth + 1)?;
        }
        Kind::Record(fields) => {
            let _: u32 = fields
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("too many kind fields").unwrap_err())?;
            validate_named_schema(
                "kind record field",
                fields.iter().map(|(name, _)| name.as_str()),
            )?;
            for (_, field) in fields {
                validate_kind(field, depth + 1)?;
            }
        }
        Kind::Set(element, max_len) => {
            if let Some(max_len) = max_len {
                let _: u32 = (*max_len)
                    .try_into()
                    .map_err(|_| invalid::<()>("kind set limit exceeds u32").unwrap_err())?;
            }
            validate_kind(element, depth + 1)?;
        }
        Kind::Table(columns, primary_key) => {
            let _: u32 = columns
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("too many kind columns").unwrap_err())?;
            let primary_key: u32 = (*primary_key)
                .try_into()
                .map_err(|_| invalid::<()>("kind primary key exceeds u32").unwrap_err())?;
            if primary_key != 0 {
                return invalid("kind table primary keys other than zero are unsupported");
            }
            validate_named_schema(
                "kind table column",
                columns.iter().map(|(name, _)| name.as_str()),
            )?;
            for (_, column) in columns {
                validate_kind(column, depth + 1)?;
            }
        }
        Kind::Tuple(types) => {
            let _: u32 = types
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("too many kind tuple entries").unwrap_err())?;
            for ty in types {
                validate_kind(ty, depth + 1)?;
            }
        }
        Kind::Scalar(id) => validate_scalar_kind_id(*id)?,
        Kind::Any | Kind::None | Kind::Empty | Kind::Id | Kind::Index => {}
    }
    Ok(())
}

fn validate_runtime_type(ty: &RuntimeType, depth: usize) -> MResult<()> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("runtime type recursion exceeds bytecode v1 limit");
    }
    match ty {
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => {
            if !storage.validate_dimensions(*rows, *cols) {
                return invalid("matrix storage and dimensions disagree");
            }
            if !(matches!(element.as_ref(), RuntimeType::Any) && (*rows == 0 || *cols == 0)) {
                validate_matrix_element_type(element)?;
            }
        }
        RuntimeType::Enum { id, name } => validate_named_id("runtime enum", *id, name)?,
        RuntimeType::Atom { id, name } => validate_named_id("runtime atom", *id, name)?,
        RuntimeType::Record(fields) => {
            let _: u32 = fields
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("record field count exceeds u32").unwrap_err())?;
            validate_named_schema("record field", fields.iter().map(|(name, _)| name.as_str()))?;
        }
        RuntimeType::Table {
            columns,
            primary_key,
        } => {
            let _: u32 = columns
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("table column count exceeds u32").unwrap_err())?;
            if *primary_key != 0 {
                return invalid("table primary keys other than zero are unsupported");
            }
            validate_named_schema(
                "table column",
                columns.iter().map(|(name, _)| name.as_str()),
            )?;
        }
        RuntimeType::Tuple(types) => {
            let _: u32 = types
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("tuple type count exceeds u32").unwrap_err())?;
        }
        RuntimeType::Kind(kind) => validate_kind(kind, depth + 1)?,
        _ => {}
    }
    for child in ty.children() {
        validate_runtime_type(child, depth + 1)?;
    }
    Ok(())
}

pub(crate) fn encode_type_payload(
    ty: &RuntimeType,
    ids: &BTreeMap<RuntimeType, u32>,
) -> MResult<Vec<u8>> {
    let id = |child: &RuntimeType| {
        ids.get(child)
            .copied()
            .ok_or_else(|| invalid::<()>("missing child runtime type ID").unwrap_err())
    };
    let mut out = Vec::new();
    match ty {
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => {
            write_u32(&mut out, id(element)?);
            out.push(*storage as u8);
            write_u32(&mut out, *rows);
            write_u32(&mut out, *cols);
        }
        RuntimeType::Enum { id, name } | RuntimeType::Atom { id, name } => {
            write_u64(&mut out, *id);
            write_string(&mut out, name)?;
        }
        RuntimeType::Record(fields) => {
            write_u32(
                &mut out,
                fields
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("record field count exceeds u32").unwrap_err())?,
            );
            for (name, child) in fields {
                write_string(&mut out, name)?;
                write_u32(&mut out, id(child)?);
            }
        }
        RuntimeType::Map { key, value } => {
            write_u32(&mut out, id(key)?);
            write_u32(&mut out, id(value)?);
        }
        RuntimeType::Table {
            columns,
            primary_key,
        } => {
            write_u32(
                &mut out,
                columns
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("table column count exceeds u32").unwrap_err())?,
            );
            for (name, child) in columns {
                write_string(&mut out, name)?;
                write_u32(&mut out, id(child)?);
            }
            write_u32(&mut out, *primary_key);
        }
        RuntimeType::Tuple(types) => {
            write_u32(
                &mut out,
                types
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("tuple type count exceeds u32").unwrap_err())?,
            );
            for child in types {
                write_u32(&mut out, id(child)?);
            }
        }
        RuntimeType::Reference(child) | RuntimeType::Option(child) => {
            write_u32(&mut out, id(child)?)
        }
        RuntimeType::Set { element, max_len } => {
            write_u32(&mut out, id(element)?);
            match max_len {
                Some(value) => {
                    out.push(1);
                    write_u32(&mut out, *value);
                }
                None => out.push(0),
            }
        }
        RuntimeType::Kind(kind) => encode_kind(kind, &mut out, 0)?,
        _ => {}
    }
    Ok(out)
}

fn encode_kind(kind: &Kind, out: &mut Vec<u8>, depth: usize) -> MResult<()> {
    validate_kind(kind, depth)?;
    let tag = match kind {
        Kind::Any => EncodedKindTag::Any,
        Kind::None => EncodedKindTag::None,
        Kind::Atom(..) => EncodedKindTag::Atom,
        Kind::Empty => EncodedKindTag::Empty,
        Kind::Enum(..) => EncodedKindTag::Enum,
        Kind::Id => EncodedKindTag::Id,
        Kind::Index => EncodedKindTag::Index,
        Kind::Map(..) => EncodedKindTag::Map,
        Kind::Matrix(..) => EncodedKindTag::Matrix,
        Kind::Option(..) => EncodedKindTag::Option,
        Kind::Record(..) => EncodedKindTag::Record,
        Kind::Reference(..) => EncodedKindTag::Reference,
        Kind::Scalar(..) => EncodedKindTag::Scalar,
        Kind::Set(..) => EncodedKindTag::Set,
        Kind::Table(..) => EncodedKindTag::Table,
        Kind::Tuple(..) => EncodedKindTag::Tuple,
        Kind::Kind(..) => EncodedKindTag::Kind,
    };
    out.push(tag as u8);
    match kind {
        Kind::Atom(id, name) | Kind::Enum(id, name) => {
            write_u64(out, *id);
            write_string(out, name)?;
        }
        Kind::Map(key, value) => {
            encode_kind(key, out, depth + 1)?;
            encode_kind(value, out, depth + 1)?;
        }
        Kind::Matrix(element, dimensions) => {
            encode_kind(element, out, depth + 1)?;
            write_u32(
                out,
                dimensions
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("too many kind dimensions").unwrap_err())?,
            );
            for dimension in dimensions {
                write_u32(
                    out,
                    (*dimension)
                        .try_into()
                        .map_err(|_| invalid::<()>("kind dimension exceeds u32").unwrap_err())?,
                );
            }
        }
        Kind::Option(inner) | Kind::Reference(inner) | Kind::Kind(inner) => {
            encode_kind(inner, out, depth + 1)?
        }
        Kind::Record(fields) => {
            write_u32(
                out,
                fields
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("too many kind fields").unwrap_err())?,
            );
            for (name, field) in fields {
                write_string(out, name)?;
                encode_kind(field, out, depth + 1)?;
            }
        }
        Kind::Scalar(id) => write_u64(out, *id),
        Kind::Set(element, max) => {
            encode_kind(element, out, depth + 1)?;
            match max {
                Some(value) => {
                    out.push(1);
                    write_u32(
                        out,
                        (*value).try_into().map_err(|_| {
                            invalid::<()>("kind set limit exceeds u32").unwrap_err()
                        })?,
                    );
                }
                None => out.push(0),
            }
        }
        Kind::Table(columns, primary_key) => {
            write_u32(
                out,
                columns
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("too many kind columns").unwrap_err())?,
            );
            for (name, column) in columns {
                write_string(out, name)?;
                encode_kind(column, out, depth + 1)?;
            }
            write_u32(
                out,
                (*primary_key)
                    .try_into()
                    .map_err(|_| invalid::<()>("kind primary key exceeds u32").unwrap_err())?,
            );
        }
        Kind::Tuple(types) => {
            write_u32(
                out,
                types
                    .len()
                    .try_into()
                    .map_err(|_| invalid::<()>("too many kind tuple entries").unwrap_err())?,
            );
            for ty in types {
                encode_kind(ty, out, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) enum RawRuntimeType {
    Complete(RuntimeType),
    Matrix {
        element: u32,
        storage: MatrixStorage,
        rows: u32,
        cols: u32,
    },
    Record(Vec<(String, u32)>),
    Map {
        key: u32,
        value: u32,
    },
    Table {
        columns: Vec<(String, u32)>,
        primary_key: u32,
    },
    Tuple(Vec<u32>),
    Reference(u32),
    Set {
        element: u32,
        max_len: Option<u32>,
    },
    Option(u32),
}

fn checked_embedded_count(
    reader: &ByteReader<'_>,
    count: u32,
    minimum_item_bytes: usize,
    trailing_bytes: usize,
    what: &str,
) -> MResult<usize> {
    let count = usize::try_from(count)
        .map_err(|_| invalid::<()>(format!("{what} exceeds address space")).unwrap_err())?;
    let minimum_bytes = count
        .checked_mul(minimum_item_bytes)
        .and_then(|bytes| bytes.checked_add(trailing_bytes))
        .ok_or_else(|| invalid::<()>(format!("{what} byte length overflow")).unwrap_err())?;
    if minimum_bytes > reader.remaining() {
        return invalid(format!("{what} exceeds the remaining payload"));
    }
    Ok(count)
}

fn checked_inline_type_count(
    reader: &ByteReader<'_>,
    count: u32,
    minimum_item_bytes: usize,
    trailing_bytes: usize,
    remaining_nodes: usize,
    what: &str,
) -> MResult<usize> {
    let count = checked_embedded_count(reader, count, minimum_item_bytes, trailing_bytes, what)?;
    if count > remaining_nodes {
        return invalid(format!("{what} exceeds the inline type node limit"));
    }
    Ok(count)
}

fn try_vec_with_capacity<T>(capacity: usize, what: &str) -> MResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| invalid::<()>(format!("unable to allocate {what}")).unwrap_err())?;
    Ok(values)
}

pub(crate) fn decode_raw_type(tag: RuntimeTypeTag, payload: &[u8]) -> MResult<RawRuntimeType> {
    let mut r = ByteReader::new(payload);
    let complete = |ty| -> MResult<RawRuntimeType> { Ok(RawRuntimeType::Complete(ty)) };
    let raw = match tag {
        RuntimeTypeTag::U8 => complete(RuntimeType::U8)?,
        RuntimeTypeTag::U16 => complete(RuntimeType::U16)?,
        RuntimeTypeTag::U32 => complete(RuntimeType::U32)?,
        RuntimeTypeTag::U64 => complete(RuntimeType::U64)?,
        RuntimeTypeTag::U128 => complete(RuntimeType::U128)?,
        RuntimeTypeTag::I8 => complete(RuntimeType::I8)?,
        RuntimeTypeTag::I16 => complete(RuntimeType::I16)?,
        RuntimeTypeTag::I32 => complete(RuntimeType::I32)?,
        RuntimeTypeTag::I64 => complete(RuntimeType::I64)?,
        RuntimeTypeTag::I128 => complete(RuntimeType::I128)?,
        RuntimeTypeTag::F32 => complete(RuntimeType::F32)?,
        RuntimeTypeTag::F64 => complete(RuntimeType::F64)?,
        RuntimeTypeTag::C64 => complete(RuntimeType::C64)?,
        RuntimeTypeTag::R64 => complete(RuntimeType::R64)?,
        RuntimeTypeTag::String => complete(RuntimeType::String)?,
        RuntimeTypeTag::Bool => complete(RuntimeType::Bool)?,
        RuntimeTypeTag::Id => complete(RuntimeType::Id)?,
        RuntimeTypeTag::Index => complete(RuntimeType::Index)?,
        RuntimeTypeTag::Empty => complete(RuntimeType::Empty)?,
        RuntimeTypeTag::Any => complete(RuntimeType::Any)?,
        RuntimeTypeTag::None => complete(RuntimeType::None)?,
        RuntimeTypeTag::Matrix => {
            let element = r.read_u32("matrix element type")?;
            let storage = MatrixStorage::from_u8(r.read_u8("matrix storage")?)
                .ok_or_else(|| invalid::<()>("unknown matrix storage").unwrap_err())?;
            let rows = r.read_u32("matrix rows")?;
            let cols = r.read_u32("matrix columns")?;
            if !storage.validate_dimensions(rows, cols) {
                return invalid("matrix storage and dimensions disagree");
            }
            RawRuntimeType::Matrix {
                element,
                storage,
                rows,
                cols,
            }
        }
        RuntimeTypeTag::Enum | RuntimeTypeTag::Atom => {
            let id = r.read_u64("named type ID")?;
            let name = r.read_string("named type name")?;
            let category = if tag == RuntimeTypeTag::Enum {
                "runtime enum"
            } else {
                "runtime atom"
            };
            validate_named_id(category, id, &name)?;
            complete(if tag == RuntimeTypeTag::Enum {
                RuntimeType::Enum { id, name }
            } else {
                RuntimeType::Atom { id, name }
            })?
        }
        RuntimeTypeTag::Record => {
            let encoded_count = r.read_u32("record field count")?;
            let count = checked_embedded_count(&r, encoded_count, 8, 0, "record field count")?;
            let mut fields = try_vec_with_capacity(count, "record fields")?;
            for _ in 0..count {
                fields.push((
                    r.read_string("record field name")?,
                    r.read_u32("record field type")?,
                ));
            }
            RawRuntimeType::Record(fields)
        }
        RuntimeTypeTag::Map => RawRuntimeType::Map {
            key: r.read_u32("map key type")?,
            value: r.read_u32("map value type")?,
        },
        RuntimeTypeTag::Table => {
            let encoded_count = r.read_u32("table column count")?;
            let count = checked_embedded_count(&r, encoded_count, 8, 4, "table column count")?;
            let mut columns = try_vec_with_capacity(count, "table columns")?;
            for _ in 0..count {
                columns.push((
                    r.read_string("table column name")?,
                    r.read_u32("table column type")?,
                ));
            }
            let primary_key = r.read_u32("table primary key")?;
            if primary_key != 0 {
                return invalid("table primary keys other than zero are unsupported");
            }
            RawRuntimeType::Table {
                columns,
                primary_key,
            }
        }
        RuntimeTypeTag::Tuple => {
            let encoded_count = r.read_u32("tuple type count")?;
            let count = checked_embedded_count(&r, encoded_count, 4, 0, "tuple type count")?;
            let mut types = try_vec_with_capacity(count, "tuple child types")?;
            for _ in 0..count {
                types.push(r.read_u32("tuple child type")?);
            }
            RawRuntimeType::Tuple(types)
        }
        RuntimeTypeTag::Reference => RawRuntimeType::Reference(r.read_u32("reference child type")?),
        RuntimeTypeTag::Set => {
            let element = r.read_u32("set element type")?;
            let max_len = match r.read_u8("set limit presence")? {
                0 => None,
                1 => Some(r.read_u32("set limit")?),
                _ => return invalid("invalid set limit presence tag"),
            };
            RawRuntimeType::Set { element, max_len }
        }
        RuntimeTypeTag::Option => RawRuntimeType::Option(r.read_u32("option child type")?),
        RuntimeTypeTag::Kind => complete(RuntimeType::Kind(decode_kind(&mut r, 0)?))?,
    };
    if !r.is_empty() {
        return invalid("runtime type payload has trailing bytes");
    }
    Ok(raw)
}

pub(crate) fn resolve_raw_types(raw: &[RawRuntimeType]) -> MResult<Vec<RuntimeType>> {
    fn expanded_nodes(
        index: usize,
        raw: &[RawRuntimeType],
        states: &mut [u8],
        counts: &mut [Option<usize>],
        depth: usize,
    ) -> MResult<usize> {
        if depth > MAX_TYPE_RECURSION {
            return invalid("runtime type graph exceeds recursion limit");
        }
        if index >= raw.len() {
            return invalid("runtime type references an out-of-range child");
        }
        if states[index] == 1 {
            return invalid("cyclic runtime type graph");
        }
        if let Some(count) = counts[index] {
            return Ok(count);
        }
        states[index] = 1;
        let mut count = 1usize;
        let mut add_child = |id: u32| -> MResult<()> {
            let child = checked_usize(u64::from(id), "runtime child type ID")?;
            count = count
                .checked_add(expanded_nodes(child, raw, states, counts, depth + 1)?)
                .ok_or_else(|| {
                    invalid::<()>("expanded runtime type node count overflow").unwrap_err()
                })?;
            if count > MAX_EXPANDED_RUNTIME_TYPE_NODES {
                return invalid("expanded runtime type graph exceeds bytecode v1 node limit");
            }
            Ok(())
        };
        match &raw[index] {
            RawRuntimeType::Complete(_) => {}
            RawRuntimeType::Matrix { element, .. }
            | RawRuntimeType::Reference(element)
            | RawRuntimeType::Set { element, .. }
            | RawRuntimeType::Option(element) => add_child(*element)?,
            RawRuntimeType::Record(fields)
            | RawRuntimeType::Table {
                columns: fields, ..
            } => {
                for (_, child) in fields {
                    add_child(*child)?;
                }
            }
            RawRuntimeType::Map { key, value } => {
                add_child(*key)?;
                add_child(*value)?;
            }
            RawRuntimeType::Tuple(children) => {
                for child in children {
                    add_child(*child)?;
                }
            }
        }
        states[index] = 2;
        counts[index] = Some(count);
        Ok(count)
    }

    // Charge the complete materialized output, not merely each individual
    // root. A bytecode file can otherwise repeat many individually acceptable
    // expanded roots and still exhaust memory while filling `out`.
    let mut count_states = vec![0; raw.len()];
    let mut counts = vec![None; raw.len()];
    let mut total = 0usize;
    for index in 0..raw.len() {
        total = total
            .checked_add(expanded_nodes(
                index,
                raw,
                &mut count_states,
                &mut counts,
                0,
            )?)
            .ok_or_else(|| {
                invalid::<()>("total expanded runtime type node count overflow").unwrap_err()
            })?;
        if total > MAX_EXPANDED_RUNTIME_TYPE_NODES {
            return invalid("expanded runtime type graph exceeds bytecode v1 node limit");
        }
    }

    fn resolve(
        index: usize,
        raw: &[RawRuntimeType],
        states: &mut [u8],
        out: &mut [Option<RuntimeType>],
        depth: usize,
    ) -> MResult<RuntimeType> {
        if depth > MAX_TYPE_RECURSION {
            return invalid("runtime type graph exceeds recursion limit");
        }
        if index >= raw.len() {
            return invalid("runtime type references an out-of-range child");
        }
        if states[index] == 1 {
            return invalid("cyclic runtime type graph");
        }
        if let Some(ty) = &out[index] {
            return Ok(ty.clone());
        }
        states[index] = 1;
        let child = |id: u32, states: &mut [u8], out: &mut [Option<RuntimeType>]| {
            resolve(
                checked_usize(u64::from(id), "runtime child type ID")?,
                raw,
                states,
                out,
                depth + 1,
            )
        };
        let ty = match &raw[index] {
            RawRuntimeType::Complete(ty) => ty.clone(),
            RawRuntimeType::Matrix {
                element,
                storage,
                rows,
                cols,
            } => RuntimeType::Matrix {
                element: Box::new(child(*element, states, out)?),
                storage: *storage,
                rows: *rows,
                cols: *cols,
            },
            RawRuntimeType::Record(fields) => RuntimeType::Record(
                fields
                    .iter()
                    .map(|(name, id)| Ok((name.clone(), child(*id, states, out)?)))
                    .collect::<MResult<_>>()?,
            ),
            RawRuntimeType::Map { key, value } => RuntimeType::Map {
                key: Box::new(child(*key, states, out)?),
                value: Box::new(child(*value, states, out)?),
            },
            RawRuntimeType::Table {
                columns,
                primary_key,
            } => RuntimeType::Table {
                columns: columns
                    .iter()
                    .map(|(name, id)| Ok((name.clone(), child(*id, states, out)?)))
                    .collect::<MResult<_>>()?,
                primary_key: *primary_key,
            },
            RawRuntimeType::Tuple(types) => RuntimeType::Tuple(
                types
                    .iter()
                    .map(|id| child(*id, states, out))
                    .collect::<MResult<_>>()?,
            ),
            RawRuntimeType::Reference(id) => {
                RuntimeType::Reference(Box::new(child(*id, states, out)?))
            }
            RawRuntimeType::Set { element, max_len } => RuntimeType::Set {
                element: Box::new(child(*element, states, out)?),
                max_len: *max_len,
            },
            RawRuntimeType::Option(id) => RuntimeType::Option(Box::new(child(*id, states, out)?)),
        };
        states[index] = 2;
        out[index] = Some(ty.clone());
        Ok(ty)
    }
    let mut states = vec![0; raw.len()];
    let mut out = vec![None; raw.len()];
    for index in 0..raw.len() {
        resolve(index, raw, &mut states, &mut out, 0)?;
    }
    let types = out.into_iter().map(Option::unwrap).collect::<Vec<_>>();
    for ty in &types {
        validate_runtime_type(ty, 0)?;
    }
    Ok(types)
}

fn decode_kind(r: &mut ByteReader<'_>, depth: usize) -> MResult<Kind> {
    if depth > MAX_TYPE_RECURSION {
        return invalid("semantic kind recursion exceeds bytecode v1 limit");
    }
    let tag = r.read_u8("kind tag")?;
    let kind = match tag {
        1 => Kind::Any,
        2 => Kind::None,
        3 => Kind::Atom(
            r.read_u64("kind atom ID")?,
            r.read_string("kind atom name")?,
        ),
        4 => Kind::Empty,
        5 => Kind::Enum(
            r.read_u64("kind enum ID")?,
            r.read_string("kind enum name")?,
        ),
        6 => Kind::Id,
        7 => Kind::Index,
        8 => Kind::Map(
            Box::new(decode_kind(r, depth + 1)?),
            Box::new(decode_kind(r, depth + 1)?),
        ),
        9 => {
            let element = Box::new(decode_kind(r, depth + 1)?);
            let encoded_count = r.read_u32("kind matrix dimension count")?;
            let count =
                checked_embedded_count(r, encoded_count, 4, 0, "kind matrix dimension count")?;
            let mut dimensions = try_vec_with_capacity(count, "kind matrix dimensions")?;
            for _ in 0..count {
                dimensions.push(
                    usize::try_from(r.read_u32("kind matrix dimension")?).map_err(|_| {
                        invalid::<()>("kind matrix dimension exceeds address space").unwrap_err()
                    })?,
                );
            }
            Kind::Matrix(element, dimensions)
        }
        10 => Kind::Option(Box::new(decode_kind(r, depth + 1)?)),
        11 => {
            let encoded_count = r.read_u32("kind record count")?;
            let count = checked_embedded_count(r, encoded_count, 5, 0, "kind record count")?;
            let mut fields = try_vec_with_capacity(count, "kind record fields")?;
            for _ in 0..count {
                fields.push((
                    r.read_string("kind record name")?,
                    decode_kind(r, depth + 1)?,
                ));
            }
            Kind::Record(fields)
        }
        12 => Kind::Reference(Box::new(decode_kind(r, depth + 1)?)),
        13 => Kind::Scalar(r.read_u64("kind scalar ID")?),
        14 => {
            let element = Box::new(decode_kind(r, depth + 1)?);
            let max = match r.read_u8("kind set presence")? {
                0 => None,
                1 => Some(usize::try_from(r.read_u32("kind set limit")?).map_err(|_| {
                    invalid::<()>("kind set limit exceeds address space").unwrap_err()
                })?),
                _ => return invalid("invalid kind set presence"),
            };
            Kind::Set(element, max)
        }
        15 => {
            let encoded_count = r.read_u32("kind table count")?;
            let count = checked_embedded_count(r, encoded_count, 5, 4, "kind table count")?;
            let mut columns = try_vec_with_capacity(count, "kind table columns")?;
            for _ in 0..count {
                columns.push((
                    r.read_string("kind table name")?,
                    decode_kind(r, depth + 1)?,
                ));
            }
            let primary_key = r.read_u32("kind table primary key")?;
            if primary_key != 0 {
                return invalid("kind table primary keys other than zero are unsupported");
            }
            Kind::Table(
                columns,
                usize::try_from(primary_key).map_err(|_| {
                    invalid::<()>("kind table primary key exceeds address space").unwrap_err()
                })?,
            )
        }
        16 => {
            let encoded_count = r.read_u32("kind tuple count")?;
            let count = checked_embedded_count(r, encoded_count, 1, 0, "kind tuple count")?;
            let mut types = try_vec_with_capacity(count, "kind tuple entries")?;
            for _ in 0..count {
                types.push(decode_kind(r, depth + 1)?);
            }
            Kind::Tuple(types)
        }
        17 => Kind::Kind(Box::new(decode_kind(r, depth + 1)?)),
        _ => return invalid("unknown semantic kind tag"),
    };
    validate_kind(&kind, depth)?;
    Ok(kind)
}

#[cfg(test)]
mod named_schema_tests {
    use super::*;

    #[test]
    fn record_field_ids_reject_distinct_name_collisions() {
        let error = validate_named_schema_with_hash("record field", ["first", "second"], |_| 42)
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeValidation");
        let message = error.kind_message();
        assert!(message.contains("record field"));
        assert!(message.contains("first"));
        assert!(message.contains("second"));
        assert!(message.contains("42"));
    }

    #[test]
    fn table_column_ids_reject_distinct_name_collisions() {
        let error = validate_named_schema_with_hash("table column", ["first", "second"], |_| 99)
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeValidation");
        let message = error.kind_message();
        assert!(message.contains("table column"));
        assert!(message.contains("first"));
        assert!(message.contains("second"));
        assert!(message.contains("99"));
    }
}
