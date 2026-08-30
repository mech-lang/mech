// Canonical bytecode-v1 constant codecs.
mod canonical;
mod composite;
pub(crate) mod inline_type;
mod limits;
mod matrix;

#[cfg(feature = "semantic-compiler")]
use crate::FunctionValueRepresentation;
use crate::{MResult, Value, ValueCell};

#[cfg(feature = "no_std")]
use alloc::{collections::BTreeSet, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::collections::BTreeSet;

#[cfg(any(test, feature = "semantic-compiler"))]
use super::MatrixStorage;
use super::{ByteReader, RuntimeType, checked_usize, invalid};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedConstant {
    pub runtime_type: RuntimeType,
    pub alignment: u8,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantEntry {
    pub type_id: u32,
    pub encoding: u8,
    pub alignment: u8,
    pub flags: u16,
    pub offset: u64,
    pub length: u64,
}

use composite::read_child_payload;
use limits::MAX_CONSTANT_NESTING;

pub(crate) const MAX_TABLE_CONSTANT_ROWS: usize = 1_000_000;
pub(crate) const MAX_TABLE_CONSTANT_CELLS: usize = 1_000_000;

struct ConstantCodecContext {
    depth: usize,
}

impl ConstantCodecContext {
    fn new() -> Self {
        Self { depth: 0 }
    }
}

fn validate_table_payload_shape(rows: usize, columns: usize, remaining: usize) -> MResult<()> {
    if rows > MAX_TABLE_CONSTANT_ROWS {
        return invalid("table row count exceeds bytecode v1 limit");
    }
    let cells = rows
        .checked_mul(columns)
        .ok_or_else(|| invalid::<()>("table cell count overflow").unwrap_err())?;
    if cells > MAX_TABLE_CONSTANT_CELLS {
        return invalid("table cell count exceeds bytecode v1 limit");
    }
    let minimum_payload = cells
        .checked_mul(4)
        .ok_or_else(|| invalid::<()>("table cell framing length overflow").unwrap_err())?;
    if minimum_payload > remaining {
        return invalid("table row count exceeds the feasible framed cell payload");
    }
    Ok(())
}

fn validate_matrix_payload_feasibility(
    element: &RuntimeType,
    rows: u32,
    cols: u32,
    bytes: &[u8],
) -> MResult<()> {
    let minimum_element_bytes = match element {
        RuntimeType::Bool | RuntimeType::U8 | RuntimeType::I8 => 1,
        RuntimeType::U16 | RuntimeType::I16 => 2,
        RuntimeType::U32 | RuntimeType::I32 | RuntimeType::F32 | RuntimeType::String => 4,
        RuntimeType::U64 | RuntimeType::I64 | RuntimeType::F64 | RuntimeType::Index => 8,
        RuntimeType::U128 | RuntimeType::I128 | RuntimeType::C64 | RuntimeType::R64 => 16,
        _ => return Ok(()),
    };
    let (_, _, element_count) = matrix::element_count(rows, cols)?;
    let element_payload_bytes = bytes.len().checked_sub(8).ok_or_else(|| {
        invalid::<()>("matrix constant is shorter than its shape prefix").unwrap_err()
    })?;
    let minimum_payload_bytes = element_count
        .checked_mul(minimum_element_bytes)
        .ok_or_else(|| invalid::<()>("matrix element payload length overflow").unwrap_err())?;
    if minimum_payload_bytes > element_payload_bytes {
        return invalid("matrix element count exceeds the feasible remaining payload");
    }
    Ok(())
}

pub fn decode_constants(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<Value>> {
    canonical::decode_constants(types, entries, blob)
}

/// Decodes compiler-owned constants before bytecode section framing.
///
/// This keeps semantic artifact construction independent from file-size read
/// limits. The final bytecode writer still applies all framing and read limits.
pub fn decode_encoded_constants(constants: &[EncodedConstant]) -> MResult<Vec<Value>> {
    canonical::decode_encoded_constants(constants)
}

#[cfg(feature = "semantic-compiler")]
pub(crate) fn encode_canonical_constant(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    canonical::encode_value(value, representation)
}

#[cfg(feature = "semantic-compiler")]
pub(crate) fn encode_canonical_exact_backing(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    canonical::encode_exact_backing(value, representation)
}

#[cfg(feature = "semantic-compiler")]
pub(crate) fn encode_canonical_composite_template(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    canonical::encode_composite_template(value, representation)
}

#[cfg(feature = "semantic-compiler")]
pub(crate) use canonical::runtime_schema_body;

pub(crate) fn validate_constant_value_payload(ty: &RuntimeType, bytes: &[u8]) -> MResult<()> {
    canonical::validate_payload(ty, bytes)
}

/// Decodes compiler-owned constants directly into canonical runtime cells.
pub fn decode_encoded_constant_cells(constants: &[EncodedConstant]) -> MResult<Vec<ValueCell>> {
    canonical::decode_encoded_constant_cells(constants)
}

pub(crate) fn decode_constant_cells(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<ValueCell>> {
    canonical::decode_constant_cells(types, entries, blob)
}

pub(crate) fn referenced_runtime_types(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<RuntimeType>> {
    let mut referenced = BTreeSet::new();
    for entry in entries {
        let type_id = checked_usize(u64::from(entry.type_id), "constant type ID")?;
        let ty = types
            .get(type_id)
            .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
        let start = checked_usize(entry.offset, "constant offset")?;
        let length = checked_usize(entry.length, "constant length")?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
        let payload = blob
            .get(start..end)
            .ok_or_else(|| invalid::<()>("constant exceeds ConstantBlob").unwrap_err())?;
        collect_inline_runtime_types(ty, payload, 0, &mut referenced)?;
    }
    Ok(referenced.into_iter().collect())
}

/// Return the largest canonical `Index` payload without converting it through
/// the build machine's `usize`. Native cross-compilation uses this to validate
/// the bytecode against the requested target's pointer width before emitting a
/// project that would fail while decoding on that target.
pub fn maximum_index_constant(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Option<u64>> {
    let mut maximum = None;
    for entry in entries {
        let type_id = checked_usize(u64::from(entry.type_id), "constant type ID")?;
        let ty = types
            .get(type_id)
            .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
        let start = checked_usize(entry.offset, "constant offset")?;
        let length = checked_usize(entry.length, "constant length")?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
        let payload = blob
            .get(start..end)
            .ok_or_else(|| invalid::<()>("constant exceeds ConstantBlob").unwrap_err())?;
        collect_maximum_index(ty, payload, 0, &mut maximum)?;
    }
    Ok(maximum)
}

fn record_index_maximum(maximum: &mut Option<u64>, value: u64) {
    *maximum = Some(maximum.map_or(value, |current| current.max(value)));
}

fn collect_maximum_index(
    ty: &RuntimeType,
    bytes: &[u8],
    depth: usize,
    maximum: &mut Option<u64>,
) -> MResult<()> {
    if depth > MAX_CONSTANT_NESTING {
        return Err(super::depth_exceeded(MAX_CONSTANT_NESTING));
    }

    let mut reader = ByteReader::new(bytes);
    match ty {
        RuntimeType::Index => {
            record_index_maximum(maximum, reader.read_u64("Index constant")?);
        }
        RuntimeType::Matrix {
            element,
            rows,
            cols,
            ..
        } if element.as_ref() == &RuntimeType::Index => {
            if (
                reader.read_u32("matrix constant rows")?,
                reader.read_u32("matrix constant columns")?,
            ) != (*rows, *cols)
            {
                return invalid("matrix constant shape disagrees with RuntimeType");
            }
            let (_, _, elements) = matrix::element_count(*rows, *cols)?;
            for _ in 0..elements {
                record_index_maximum(maximum, reader.read_u64("Index matrix element")?);
            }
        }
        RuntimeType::Tuple(types) => {
            let count = checked_usize(
                u64::from(reader.read_u32("tuple element count")?),
                "tuple element count",
            )?;
            if count != types.len() {
                return invalid("tuple element count does not match RuntimeType");
            }
            for child in types {
                collect_maximum_index(
                    child,
                    read_child_payload(&mut reader, "tuple element")?,
                    depth + 1,
                    maximum,
                )?;
            }
        }
        RuntimeType::Record(fields) => {
            let count = checked_usize(
                u64::from(reader.read_u32("record field count")?),
                "record field count",
            )?;
            if count != fields.len() {
                return invalid("record field count does not match RuntimeType");
            }
            for (_, child) in fields {
                collect_maximum_index(
                    child,
                    read_child_payload(&mut reader, "record field")?,
                    depth + 1,
                    maximum,
                )?;
            }
        }
        RuntimeType::Map { key, value } => {
            let count = checked_usize(
                u64::from(reader.read_u32("map entry count")?),
                "map entry count",
            )?;
            for _ in 0..count {
                collect_maximum_index(
                    key,
                    read_child_payload(&mut reader, "map key")?,
                    depth + 1,
                    maximum,
                )?;
                collect_maximum_index(
                    value,
                    read_child_payload(&mut reader, "map value")?,
                    depth + 1,
                    maximum,
                )?;
            }
        }
        RuntimeType::Set { element, .. } => {
            let count = checked_usize(
                u64::from(reader.read_u32("set element count")?),
                "set element count",
            )?;
            for _ in 0..count {
                collect_maximum_index(
                    element,
                    read_child_payload(&mut reader, "set element")?,
                    depth + 1,
                    maximum,
                )?;
            }
        }
        RuntimeType::Table { columns, .. } => {
            let rows = checked_usize(
                u64::from(reader.read_u32("table row count")?),
                "table row count",
            )?;
            let count = checked_usize(
                u64::from(reader.read_u32("table column count")?),
                "table column count",
            )?;
            if count != columns.len() {
                return invalid("table column count does not match RuntimeType");
            }
            validate_table_payload_shape(rows, count, reader.remaining())?;
            for _ in 0..rows {
                for (_, child) in columns {
                    collect_maximum_index(
                        child,
                        read_child_payload(&mut reader, "table cell")?,
                        depth + 1,
                        maximum,
                    )?;
                }
            }
        }
        RuntimeType::Reference(child) => collect_maximum_index(
            child,
            read_child_payload(&mut reader, "reference child")?,
            depth + 1,
            maximum,
        )?,
        RuntimeType::Option(child) => match reader.read_u8("option presence")? {
            0 => {}
            1 => collect_maximum_index(
                child,
                read_child_payload(&mut reader, "option child")?,
                depth + 1,
                maximum,
            )?,
            _ => return invalid("option presence must be exactly 0x00 or 0x01"),
        },
        RuntimeType::Enum { .. } => {
            let count = checked_usize(
                u64::from(reader.read_u32("enum variant count")?),
                "enum variant count",
            )?;
            for _ in 0..count {
                reader.read_u64("enum variant ID")?;
                reader.read_string("enum variant name")?;
                match reader.read_u8("enum variant payload presence")? {
                    0 => {}
                    1 => {
                        let payload_type = inline_type::decode(read_child_payload(
                            &mut reader,
                            "enum variant inline type",
                        )?)?;
                        collect_maximum_index(
                            &payload_type,
                            read_child_payload(&mut reader, "enum variant payload")?,
                            depth + 1,
                            maximum,
                        )?;
                    }
                    _ => {
                        return invalid(
                            "enum variant payload presence must be exactly 0x00 or 0x01",
                        );
                    }
                }
            }
        }
        RuntimeType::Matrix { .. }
        | RuntimeType::Empty
        | RuntimeType::Bool
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
        | RuntimeType::Id
        | RuntimeType::Atom { .. }
        | RuntimeType::Kind(_)
        | RuntimeType::Any
        | RuntimeType::None => return Ok(()),
    }

    if !reader.is_empty() {
        return invalid("constant has trailing bytes while scanning Index payloads");
    }
    Ok(())
}

fn collect_inline_runtime_types(
    ty: &RuntimeType,
    bytes: &[u8],
    depth: usize,
    referenced: &mut BTreeSet<RuntimeType>,
) -> MResult<()> {
    if depth > MAX_CONSTANT_NESTING {
        return Err(super::depth_exceeded(MAX_CONSTANT_NESTING));
    }

    collect_runtime_type_closure(ty, referenced);
    let mut reader = ByteReader::new(bytes);
    match ty {
        RuntimeType::Tuple(types) => {
            let count = checked_usize(
                u64::from(reader.read_u32("tuple element count")?),
                "tuple element count",
            )?;
            if count != types.len() {
                return invalid("tuple element count does not match RuntimeType");
            }
            for child in types {
                collect_inline_runtime_types(
                    child,
                    read_child_payload(&mut reader, "tuple element")?,
                    depth + 1,
                    referenced,
                )?;
            }
        }
        RuntimeType::Record(fields) => {
            let count = checked_usize(
                u64::from(reader.read_u32("record field count")?),
                "record field count",
            )?;
            if count != fields.len() {
                return invalid("record field count does not match RuntimeType");
            }
            for (_, child) in fields {
                collect_inline_runtime_types(
                    child,
                    read_child_payload(&mut reader, "record field")?,
                    depth + 1,
                    referenced,
                )?;
            }
        }
        RuntimeType::Map { key, value } => {
            let count = checked_usize(
                u64::from(reader.read_u32("map entry count")?),
                "map entry count",
            )?;
            for _ in 0..count {
                collect_inline_runtime_types(
                    key,
                    read_child_payload(&mut reader, "map key")?,
                    depth + 1,
                    referenced,
                )?;
                collect_inline_runtime_types(
                    value,
                    read_child_payload(&mut reader, "map value")?,
                    depth + 1,
                    referenced,
                )?;
            }
        }
        RuntimeType::Set { element, .. } => {
            let count = checked_usize(
                u64::from(reader.read_u32("set element count")?),
                "set element count",
            )?;
            for _ in 0..count {
                collect_inline_runtime_types(
                    element,
                    read_child_payload(&mut reader, "set element")?,
                    depth + 1,
                    referenced,
                )?;
            }
        }
        RuntimeType::Table { columns, .. } => {
            let rows = checked_usize(
                u64::from(reader.read_u32("table row count")?),
                "table row count",
            )?;
            let count = checked_usize(
                u64::from(reader.read_u32("table column count")?),
                "table column count",
            )?;
            if count != columns.len() {
                return invalid("table column count does not match RuntimeType");
            }
            validate_table_payload_shape(rows, count, reader.remaining())?;
            for _ in 0..rows {
                for (_, child) in columns {
                    collect_inline_runtime_types(
                        child,
                        read_child_payload(&mut reader, "table cell")?,
                        depth + 1,
                        referenced,
                    )?;
                }
            }
        }
        RuntimeType::Reference(child) => collect_inline_runtime_types(
            child,
            read_child_payload(&mut reader, "reference child")?,
            depth + 1,
            referenced,
        )?,
        RuntimeType::Option(child) => match reader.read_u8("option presence")? {
            0 => {}
            1 => collect_inline_runtime_types(
                child,
                read_child_payload(&mut reader, "option child")?,
                depth + 1,
                referenced,
            )?,
            _ => return invalid("option presence must be exactly 0x00 or 0x01"),
        },
        RuntimeType::Enum { .. } => {
            let count = checked_usize(
                u64::from(reader.read_u32("enum variant count")?),
                "enum variant count",
            )?;
            let mut previous = None;
            for _ in 0..count {
                let variant_id = reader.read_u64("enum variant ID")?;
                if previous >= Some(variant_id) {
                    return invalid("enum variants are duplicate or not sorted by ID");
                }
                let variant_name = reader.read_string("enum variant name")?;
                if variant_name.is_empty() {
                    return invalid("enum variant name must not be empty");
                }
                if crate::hash_str(&variant_name) != variant_id {
                    return invalid("enum variant name does not match its stable ID");
                }
                match reader.read_u8("enum variant payload presence")? {
                    0 => {}
                    1 => {
                        let payload_type = inline_type::decode(read_child_payload(
                            &mut reader,
                            "enum variant inline type",
                        )?)?;
                        let payload = read_child_payload(&mut reader, "enum variant payload")?;
                        collect_inline_runtime_types(
                            &payload_type,
                            payload,
                            depth + 1,
                            referenced,
                        )?;
                    }
                    _ => {
                        return invalid(
                            "enum variant payload presence must be exactly 0x00 or 0x01",
                        );
                    }
                }
                previous = Some(variant_id);
            }
        }
        RuntimeType::Empty
        | RuntimeType::Bool
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
        | RuntimeType::Id
        | RuntimeType::Index
        | RuntimeType::Matrix { .. }
        | RuntimeType::Atom { .. }
        | RuntimeType::Kind(_)
        | RuntimeType::Any
        | RuntimeType::None => return Ok(()),
    }
    if !reader.is_empty() {
        return invalid("constant has trailing bytes while collecting inline runtime types");
    }
    Ok(())
}

pub(crate) fn collect_runtime_type_closure(
    runtime_type: &RuntimeType,
    referenced: &mut BTreeSet<RuntimeType>,
) {
    if !referenced.insert(runtime_type.clone()) {
        return;
    }
    match runtime_type {
        RuntimeType::Matrix { element, .. }
        | RuntimeType::Reference(element)
        | RuntimeType::Set { element, .. }
        | RuntimeType::Option(element) => collect_runtime_type_closure(element, referenced),
        RuntimeType::Record(fields)
        | RuntimeType::Table {
            columns: fields, ..
        } => {
            for (_, child) in fields {
                collect_runtime_type_closure(child, referenced);
            }
        }
        RuntimeType::Map { key, value } => {
            collect_runtime_type_closure(key, referenced);
            collect_runtime_type_closure(value, referenced);
        }
        RuntimeType::Tuple(types) => {
            for child in types {
                collect_runtime_type_closure(child, referenced);
            }
        }
        RuntimeType::Empty
        | RuntimeType::Bool
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
        | RuntimeType::Id
        | RuntimeType::Index
        | RuntimeType::Enum { .. }
        | RuntimeType::Atom { .. }
        | RuntimeType::Kind(_)
        | RuntimeType::Any
        | RuntimeType::None => {}
    }
}
