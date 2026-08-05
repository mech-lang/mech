// Canonical bytecode-v1 constant codecs.
mod composite;
pub(crate) mod inline_type;
mod kind;
mod limits;
mod matrix;
mod scalar;

use crate::{MResult, Ref, Value};

#[cfg(feature = "no_std")]
use alloc::{collections::BTreeSet, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::collections::BTreeSet;

use super::{ByteReader, MatrixStorage, RuntimeType, checked_usize, invalid};

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

struct ConstantCodecContext {
    active_references: BTreeSet<usize>,
    depth: usize,
}

impl ConstantCodecContext {
    fn new() -> Self {
        Self {
            active_references: BTreeSet::new(),
            depth: 0,
        }
    }

    fn decode_child(&mut self, ty: &RuntimeType, bytes: &[u8]) -> MResult<Value> {
        if self.depth >= MAX_CONSTANT_NESTING {
            return Err(super::depth_exceeded(MAX_CONSTANT_NESTING));
        }
        self.depth += 1;
        let value = decode_value_payload(ty, bytes, self);
        self.depth -= 1;
        value
    }
}

pub fn decode_constants(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<Value>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(entries.len())
        .map_err(|_| invalid::<()>("unable to allocate decoded constants").unwrap_err())?;
    for entry in entries {
        values.push(decode_constant(types, entry, blob)?);
    }
    Ok(values)
}

pub(crate) fn referenced_runtime_types(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<RuntimeType>> {
    let mut referenced = types.iter().cloned().collect::<BTreeSet<_>>();
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

fn collect_inline_runtime_types(
    ty: &RuntimeType,
    bytes: &[u8],
    depth: usize,
    referenced: &mut BTreeSet<RuntimeType>,
) -> MResult<()> {
    if depth > MAX_CONSTANT_NESTING {
        return Err(super::depth_exceeded(MAX_CONSTANT_NESTING));
    }

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
                        referenced.insert(payload_type.clone());
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

fn decode_value_payload(
    ty: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    if let Some(value) = scalar::decode(ty, bytes) {
        return value;
    }
    match ty {
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => decode_matrix_constant(element, *storage, *rows, *cols, bytes),
        RuntimeType::Tuple(types) => decode_tuple_constant(types, bytes, context),
        RuntimeType::Record(fields) => decode_record_constant(fields, bytes, context),
        RuntimeType::Map { key, value } => decode_map_constant(key, value, bytes, context),
        RuntimeType::Set { element, max_len } => {
            decode_set_constant(element, *max_len, bytes, context)
        }
        RuntimeType::Table {
            columns,
            primary_key,
        } => decode_table_constant(columns, *primary_key, bytes, context),
        RuntimeType::Reference(child_type) => decode_reference_constant(child_type, bytes, context),
        RuntimeType::Option(inner) => decode_option_constant(inner, bytes, context),
        RuntimeType::Atom { id, name } => decode_atom_constant(*id, name, bytes),
        RuntimeType::Kind(kind) => {
            if bytes.is_empty() {
                Ok(Value::Kind(kind::value_kind_from_semantic_kind(kind)?))
            } else {
                invalid("Kind constants must have zero payload bytes")
            }
        }
        RuntimeType::Any => {
            if bytes.is_empty() {
                Ok(Value::EmptyKind(crate::ValueKind::Any))
            } else {
                invalid("Any constants must have zero payload bytes")
            }
        }
        RuntimeType::None => {
            if bytes.is_empty() {
                Ok(Value::EmptyKind(crate::ValueKind::None))
            } else {
                invalid("None constants must have zero payload bytes")
            }
        }
        RuntimeType::Enum { id, name } => decode_enum_constant(*id, name, bytes, context),
        _ => unreachable!("scalar runtime types are handled by scalar::decode"),
    }
}

pub(crate) fn validate_constant_value_payload(ty: &RuntimeType, bytes: &[u8]) -> MResult<()> {
    let mut context = ConstantCodecContext::new();
    decode_value_payload(ty, bytes, &mut context).map(|_| ())
}

fn runtime_type_to_value_kind(ty: &RuntimeType) -> MResult<crate::ValueKind> {
    use crate::ValueKind;

    Ok(match ty {
        RuntimeType::U8 => ValueKind::U8,
        RuntimeType::U16 => ValueKind::U16,
        RuntimeType::U32 => ValueKind::U32,
        RuntimeType::U64 => ValueKind::U64,
        RuntimeType::U128 => ValueKind::U128,
        RuntimeType::I8 => ValueKind::I8,
        RuntimeType::I16 => ValueKind::I16,
        RuntimeType::I32 => ValueKind::I32,
        RuntimeType::I64 => ValueKind::I64,
        RuntimeType::I128 => ValueKind::I128,
        RuntimeType::F32 => ValueKind::F32,
        RuntimeType::F64 => ValueKind::F64,
        RuntimeType::C64 => ValueKind::C64,
        RuntimeType::R64 => ValueKind::R64,
        RuntimeType::String => ValueKind::String,
        RuntimeType::Bool => ValueKind::Bool,
        RuntimeType::Id => ValueKind::Id,
        RuntimeType::Index => ValueKind::Index,
        RuntimeType::Empty => ValueKind::Empty,
        RuntimeType::Any => ValueKind::Any,
        RuntimeType::None => ValueKind::None,
        RuntimeType::Matrix {
            element,
            rows,
            cols,
            ..
        } => ValueKind::Matrix(
            Box::new(runtime_type_to_value_kind(element)?),
            vec![
                usize::try_from(*rows)
                    .map_err(|_| invalid::<()>("matrix row count exceeds usize").unwrap_err())?,
                usize::try_from(*cols)
                    .map_err(|_| invalid::<()>("matrix column count exceeds usize").unwrap_err())?,
            ],
        ),
        RuntimeType::Enum { id, name } => ValueKind::Enum(*id, name.clone()),
        RuntimeType::Record(fields) => ValueKind::Record(
            fields
                .iter()
                .map(|(name, child)| Ok((name.clone(), runtime_type_to_value_kind(child)?)))
                .collect::<MResult<_>>()?,
        ),
        RuntimeType::Map { key, value } => ValueKind::Map(
            Box::new(runtime_type_to_value_kind(key)?),
            Box::new(runtime_type_to_value_kind(value)?),
        ),
        RuntimeType::Atom { id, name } => ValueKind::Atom(*id, name.clone()),
        RuntimeType::Table {
            columns,
            primary_key,
        } => ValueKind::Table(
            columns
                .iter()
                .map(|(name, child)| Ok((name.clone(), runtime_type_to_value_kind(child)?)))
                .collect::<MResult<_>>()?,
            usize::try_from(*primary_key)
                .map_err(|_| invalid::<()>("table primary key exceeds usize").unwrap_err())?,
        ),
        RuntimeType::Tuple(types) => ValueKind::Tuple(
            types
                .iter()
                .map(runtime_type_to_value_kind)
                .collect::<MResult<_>>()?,
        ),
        RuntimeType::Reference(child) => {
            ValueKind::Reference(Box::new(runtime_type_to_value_kind(child)?))
        }
        RuntimeType::Set { element, max_len } => ValueKind::Set(
            Box::new(runtime_type_to_value_kind(element)?),
            max_len
                .map(|value| {
                    usize::try_from(value)
                        .map_err(|_| invalid::<()>("set maximum length exceeds usize").unwrap_err())
                })
                .transpose()?,
        ),
        RuntimeType::Option(inner) => {
            ValueKind::Option(Box::new(runtime_type_to_value_kind(inner)?))
        }
        RuntimeType::Kind(kind) => {
            ValueKind::Kind(Box::new(kind::value_kind_from_semantic_kind(kind)?))
        }
    })
}

#[cfg(feature = "tuple")]
fn decode_tuple_constant(
    types: &[RuntimeType],
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("tuple element count")?),
        "tuple element count",
    )?;
    if count != types.len() {
        return invalid("tuple element count does not match RuntimeType");
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| invalid::<()>("unable to allocate tuple values").unwrap_err())?;
    for ty in types {
        values.push(context.decode_child(ty, read_child_payload(&mut reader, "tuple element")?)?);
    }
    if !reader.is_empty() {
        return invalid("tuple constant has trailing bytes");
    }
    Ok(Value::Tuple(Ref::new(crate::MechTuple::from_vec(values))))
}

#[cfg(not(feature = "tuple"))]
fn decode_tuple_constant(
    _types: &[RuntimeType],
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Tuple constants are unavailable in this runtime")
}

#[cfg(feature = "record")]
fn decode_record_constant(
    fields: &[(String, RuntimeType)],
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("record field count")?),
        "record field count",
    )?;
    if count != fields.len() {
        return invalid("record field count does not match RuntimeType");
    }
    let mut values = Vec::new();
    for (name, ty) in fields {
        let id = crate::hash_str(name);
        if name.is_empty() {
            return invalid("record field name must not be empty");
        }
        values.push((
            (id, name.clone()),
            context.decode_child(ty, read_child_payload(&mut reader, "record field")?)?,
        ));
    }
    if !reader.is_empty() {
        return invalid("record constant has trailing bytes");
    }
    Ok(Value::Record(Ref::new(crate::MechRecord::from_vec(values))))
}

#[cfg(not(feature = "record"))]
fn decode_record_constant(
    _fields: &[(String, RuntimeType)],
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Record constants are unavailable in this runtime")
}

#[cfg(feature = "map")]
fn decode_map_constant(
    key_type: &RuntimeType,
    value_type: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("map entry count")?),
        "map entry count",
    )?;
    let mut map = indexmap::IndexMap::new();
    let mut previous_key = None::<Vec<u8>>;
    for _ in 0..count {
        let key_payload = read_child_payload(&mut reader, "map key")?;
        let value_payload = read_child_payload(&mut reader, "map value")?;
        if previous_key.as_deref() >= Some(key_payload) {
            return invalid("map keys are not in strict canonical payload order");
        }
        previous_key = Some(key_payload.to_vec());
        let key = context.decode_child(key_type, key_payload)?;
        let value = context.decode_child(value_type, value_payload)?;
        if map.insert(key, value).is_some() {
            return invalid("map contains duplicate keys");
        }
    }
    if !reader.is_empty() {
        return invalid("map constant has trailing bytes");
    }
    Ok(Value::Map(Ref::new(crate::MechMap {
        key_kind: runtime_type_to_value_kind(key_type)?,
        value_kind: runtime_type_to_value_kind(value_type)?,
        num_elements: count,
        map,
    })))
}

#[cfg(not(feature = "map"))]
fn decode_map_constant(
    _key_type: &RuntimeType,
    _value_type: &RuntimeType,
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Map constants are unavailable in this runtime")
}

#[cfg(feature = "set")]
fn decode_set_constant(
    element_type: &RuntimeType,
    max_len: Option<u32>,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("set element count")?),
        "set element count",
    )?;
    if max_len.is_some_and(|limit| count > limit as usize) {
        return invalid("set element count exceeds its RuntimeType limit");
    }
    let mut set = indexmap::IndexSet::new();
    let mut previous = None::<Vec<u8>>;
    for _ in 0..count {
        let payload = read_child_payload(&mut reader, "set element")?;
        if previous.as_deref() >= Some(payload) {
            return invalid("set elements are not in strict canonical payload order");
        }
        previous = Some(payload.to_vec());
        if !set.insert(context.decode_child(element_type, payload)?) {
            return invalid("set contains duplicate elements");
        }
    }
    if !reader.is_empty() {
        return invalid("set constant has trailing bytes");
    }
    let max_elements = max_len
        .map(|limit| checked_usize(u64::from(limit), "set maximum length"))
        .transpose()?;
    Ok(Value::Set(Ref::new(crate::MechSet {
        kind: runtime_type_to_value_kind(element_type)?,
        max_elements,
        num_elements: max_elements.unwrap_or(0),
        set,
    })))
}

#[cfg(not(feature = "set"))]
fn decode_set_constant(
    _element_type: &RuntimeType,
    _max_len: Option<u32>,
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Set constants are unavailable in this runtime")
}

#[cfg(all(feature = "table", feature = "vectord"))]
fn decode_table_constant(
    columns: &[(String, RuntimeType)],
    primary_key: u32,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let rows = checked_usize(
        u64::from(reader.read_u32("table row count")?),
        "table row count",
    )?;
    let count = checked_usize(
        u64::from(reader.read_u32("table column count")?),
        "table column count",
    )?;
    if count != columns.len()
        || (count == 0 && primary_key != 0)
        || (count > 0 && primary_key as usize >= count)
    {
        return invalid("table schema does not match RuntimeType");
    }
    let mut cell_columns = (0..count)
        .map(|_| Vec::with_capacity(rows))
        .collect::<Vec<_>>();
    for _ in 0..rows {
        for (index, (_, ty)) in columns.iter().enumerate() {
            cell_columns[index]
                .push(context.decode_child(ty, read_child_payload(&mut reader, "table cell")?)?);
        }
    }
    if !reader.is_empty() {
        return invalid("table constant has trailing bytes");
    }
    let mut data = indexmap::IndexMap::new();
    let mut names = std::collections::HashMap::new();
    for ((name, ty), cells) in columns.iter().zip(cell_columns) {
        let id = crate::hash_str(name);
        data.insert(
            id,
            (
                runtime_type_to_value_kind(ty)?,
                crate::matrix::Matrix::DVector(Ref::new(na::DVector::from_vec(cells))),
            ),
        );
        names.insert(id, name.clone());
    }
    Ok(Value::Table(Ref::new(crate::MechTable::new(
        rows, count, data, names,
    ))))
}

#[cfg(not(all(feature = "table", feature = "vectord")))]
fn decode_table_constant(
    _columns: &[(String, RuntimeType)],
    _primary_key: u32,
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Table constants require the dynamic vector feature")
}

fn decode_reference_constant(
    child_type: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let child = context.decode_child(
        child_type,
        read_child_payload(&mut reader, "reference child")?,
    )?;
    if !reader.is_empty() {
        return invalid("reference constant has trailing bytes");
    }
    Ok(Value::MutableReference(Ref::new(child)))
}

fn decode_option_constant(
    inner: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    let mut reader = ByteReader::new(bytes);
    let kind = crate::ValueKind::Option(Box::new(runtime_type_to_value_kind(inner)?));
    let value = match reader.read_u8("option presence")? {
        0 => Value::EmptyKind(kind),
        1 => {
            let child =
                context.decode_child(inner, read_child_payload(&mut reader, "option child")?)?;
            Value::Typed(Box::new(child), kind)
        }
        _ => return invalid("option presence must be exactly 0x00 or 0x01"),
    };
    if !reader.is_empty() {
        return invalid("option constant has trailing bytes");
    }
    Ok(value)
}

#[cfg(feature = "atom")]
fn decode_atom_constant(id: u64, name: &str, bytes: &[u8]) -> MResult<Value> {
    if !bytes.is_empty() {
        return invalid("Atom constants must have zero payload bytes");
    }
    if crate::hash_str(name) != id {
        return invalid("Atom RuntimeType name does not match its stable ID");
    }
    Ok(Value::Atom(Ref::new(crate::MechAtom::from_name(name))))
}

#[cfg(not(feature = "atom"))]
fn decode_atom_constant(_id: u64, _name: &str, _bytes: &[u8]) -> MResult<Value> {
    invalid("Atom constants are unavailable in this runtime")
}

#[cfg(feature = "enum")]
fn decode_enum_constant(
    id: u64,
    name: &str,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<Value> {
    if crate::hash_str(name) != id {
        return invalid("Enum RuntimeType name does not match its stable ID");
    }
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("enum variant count")?),
        "enum variant count",
    )?;
    let names = Ref::new(crate::Dictionary::new());
    names.borrow_mut().insert(id, name.to_string());
    let mut variants = Vec::new();
    let mut previous = None;
    for _ in 0..count {
        let variant_id = reader.read_u64("enum variant ID")?;
        if previous >= Some(variant_id) {
            return invalid("enum variants are duplicate or not sorted by ID");
        }
        let variant_name = reader.read_string("enum variant name")?;
        if crate::hash_str(&variant_name) != variant_id {
            return invalid("enum variant name does not match its stable ID");
        }
        let payload = match reader.read_u8("enum variant payload presence")? {
            0 => None,
            1 => {
                let type_key = read_child_payload(&mut reader, "enum variant inline type")?;
                let payload_type = inline_type::decode(type_key)?;
                let payload = context.decode_child(
                    &payload_type,
                    read_child_payload(&mut reader, "enum variant payload")?,
                )?;
                Some(payload)
            }
            _ => return invalid("enum variant payload presence must be exactly 0x00 or 0x01"),
        };
        names.borrow_mut().insert(variant_id, variant_name);
        variants.push((variant_id, payload));
        previous = Some(variant_id);
    }
    if !reader.is_empty() {
        return invalid("enum constant has trailing bytes");
    }
    Ok(Value::Enum(Ref::new(crate::MechEnum {
        id,
        variants,
        names,
    })))
}

#[cfg(not(feature = "enum"))]
fn decode_enum_constant(
    _id: u64,
    _name: &str,
    _bytes: &[u8],
    _context: &mut ConstantCodecContext,
) -> MResult<Value> {
    invalid("Enum constants are unavailable in this runtime")
}

fn decode_constant(types: &[RuntimeType], entry: &ConstantEntry, blob: &[u8]) -> MResult<Value> {
    if entry.encoding != 1 {
        return invalid("unsupported bytecode constant encoding");
    }
    if entry.flags != 0 {
        return invalid("constant entry flags must be zero");
    }
    if !matches!(entry.alignment, 1 | 2 | 4 | 8 | 16) {
        return invalid("invalid constant alignment");
    }
    if entry.offset % u64::from(entry.alignment) != 0 {
        return invalid("misaligned constant entry");
    }
    let start = usize::try_from(entry.offset)
        .map_err(|_| invalid::<()>("constant offset exceeds address space").unwrap_err())?;
    let length = usize::try_from(entry.length)
        .map_err(|_| invalid::<()>("constant length exceeds address space").unwrap_err())?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
    let bytes = blob
        .get(start..end)
        .ok_or_else(|| invalid::<()>("constant entry is outside ConstantBlob").unwrap_err())?;
    let type_id = checked_usize(u64::from(entry.type_id), "constant type ID")?;
    let ty = types
        .get(type_id)
        .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
    let mut context = ConstantCodecContext::new();
    decode_value_payload(ty, bytes, &mut context)
}

fn decode_matrix_constant(
    element: &RuntimeType,
    storage: MatrixStorage,
    rows: u32,
    cols: u32,
    bytes: &[u8],
) -> MResult<Value> {
    #[cfg(feature = "matrix")]
    match element {
        #[cfg(feature = "bool")]
        RuntimeType::Bool => decode_matrix(storage, rows, cols, bytes, |reader| {
            match reader.read_u8("Bool matrix element")? {
                0 => Ok(false),
                1 => Ok(true),
                _ => invalid("Bool matrix elements must be exactly 0x00 or 0x01"),
            }
        })
        .map(Value::MatrixBool),
        #[cfg(feature = "u8")]
        RuntimeType::U8 => decode_matrix(storage, rows, cols, bytes, |reader| {
            reader.read_u8("U8 matrix element")
        })
        .map(Value::MatrixU8),
        #[cfg(feature = "u16")]
        RuntimeType::U16 => decode_matrix(storage, rows, cols, bytes, |reader| {
            reader.read_u16("U16 matrix element")
        })
        .map(Value::MatrixU16),
        #[cfg(feature = "u32")]
        RuntimeType::U32 => decode_matrix(storage, rows, cols, bytes, |reader| {
            reader.read_u32("U32 matrix element")
        })
        .map(Value::MatrixU32),
        #[cfg(feature = "u64")]
        RuntimeType::U64 => decode_matrix(storage, rows, cols, bytes, |reader| {
            reader.read_u64("U64 matrix element")
        })
        .map(Value::MatrixU64),
        #[cfg(feature = "u128")]
        RuntimeType::U128 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(u128::from_le_bytes(
                reader
                    .read_exact(16, "U128 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixU128),
        #[cfg(feature = "i8")]
        RuntimeType::I8 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(i8::from_le_bytes(
                reader
                    .read_exact(1, "I8 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixI8),
        #[cfg(feature = "i16")]
        RuntimeType::I16 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(i16::from_le_bytes(
                reader
                    .read_exact(2, "I16 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixI16),
        #[cfg(feature = "i32")]
        RuntimeType::I32 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(i32::from_le_bytes(
                reader
                    .read_exact(4, "I32 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixI32),
        #[cfg(feature = "i64")]
        RuntimeType::I64 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(i64::from_le_bytes(
                reader
                    .read_exact(8, "I64 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixI64),
        #[cfg(feature = "i128")]
        RuntimeType::I128 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(i128::from_le_bytes(
                reader
                    .read_exact(16, "I128 matrix element")?
                    .try_into()
                    .unwrap(),
            ))
        })
        .map(Value::MatrixI128),
        #[cfg(feature = "f32")]
        RuntimeType::F32 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(f32::from_bits(reader.read_u32("F32 matrix element")?))
        })
        .map(Value::MatrixF32),
        #[cfg(feature = "f64")]
        RuntimeType::F64 => decode_matrix(storage, rows, cols, bytes, |reader| {
            Ok(f64::from_bits(reader.read_u64("F64 matrix element")?))
        })
        .map(Value::MatrixF64),
        #[cfg(feature = "string")]
        RuntimeType::String => decode_matrix(storage, rows, cols, bytes, |reader| {
            let length = checked_usize(
                u64::from(reader.read_u32("String matrix element length")?),
                "String matrix element length",
            )?;
            reader.read_utf8(length, "String matrix element")
        })
        .map(Value::MatrixString),
        #[cfg(feature = "rational")]
        RuntimeType::R64 => decode_matrix(storage, rows, cols, bytes, |reader| {
            let raw = reader.read_exact(16, "R64 matrix element")?;
            let numerator = i64::from_le_bytes(raw[..8].try_into().unwrap());
            let denominator = i64::from_le_bytes(raw[8..].try_into().unwrap());
            if denominator <= 0 {
                return invalid("R64 matrix element denominator must be positive and nonzero");
            }
            let value = crate::R64::new(numerator, denominator);
            if *value.numer() != numerator || *value.denom() != denominator {
                return invalid("R64 matrix element is not reduced");
            }
            Ok(value)
        })
        .map(Value::MatrixR64),
        #[cfg(feature = "complex")]
        RuntimeType::C64 => decode_matrix(storage, rows, cols, bytes, |reader| {
            let raw = reader.read_exact(16, "C64 matrix element")?;
            Ok(crate::C64::new(
                f64::from_bits(u64::from_le_bytes(raw[..8].try_into().unwrap())),
                f64::from_bits(u64::from_le_bytes(raw[8..].try_into().unwrap())),
            ))
        })
        .map(Value::MatrixC64),
        RuntimeType::Index => decode_matrix(storage, rows, cols, bytes, |reader| {
            let value = reader.read_u64("Index matrix element")?;
            usize::try_from(value)
                .map_err(|_| invalid::<()>("Index matrix element exceeds usize").unwrap_err())
        })
        .map(Value::MatrixIndex),
        _ => invalid(format!(
            "matrix constants do not support element type {element:?} in this runtime"
        )),
    }
    #[cfg(not(feature = "matrix"))]
    {
        let _ = (element, storage, rows, cols, bytes);
        invalid("matrix constants are unavailable in this runtime")
    }
}

#[cfg(feature = "matrix")]
fn decode_matrix<T, F>(
    storage: MatrixStorage,
    rows: u32,
    cols: u32,
    bytes: &[u8],
    mut decode_element: F,
) -> MResult<crate::matrix::Matrix<T>>
where
    T: na::Scalar,
    F: FnMut(&mut ByteReader<'_>) -> MResult<T>,
{
    let mut reader = ByteReader::new(bytes);
    if (
        reader.read_u32("matrix constant rows")?,
        reader.read_u32("matrix constant columns")?,
    ) != (rows, cols)
        || !storage.validate_dimensions(rows, cols)
    {
        return invalid("matrix constant shape disagrees with RuntimeType");
    }
    let (row_count, column_count, element_count) = matrix::element_count(rows, cols)?;
    let mut elements = Vec::new();
    elements
        .try_reserve_exact(element_count)
        .map_err(|_| invalid::<()>("unable to allocate matrix constant elements").unwrap_err())?;
    for _ in 0..element_count {
        elements.push(decode_element(&mut reader)?);
    }
    if !reader.is_empty() {
        return invalid("matrix constant has trailing bytes");
    }

    match storage {
        #[cfg(feature = "matrix1")]
        MatrixStorage::Matrix1 => Ok(crate::matrix::Matrix::Matrix1(Ref::new(
            na::Matrix1::from_row_slice(&elements),
        ))),
        #[cfg(feature = "matrix2")]
        MatrixStorage::Matrix2 => Ok(crate::matrix::Matrix::Matrix2(Ref::new(
            na::Matrix2::from_row_slice(&elements),
        ))),
        #[cfg(feature = "matrix3")]
        MatrixStorage::Matrix3 => Ok(crate::matrix::Matrix::Matrix3(Ref::new(
            na::Matrix3::from_row_slice(&elements),
        ))),
        #[cfg(feature = "matrix4")]
        MatrixStorage::Matrix4 => Ok(crate::matrix::Matrix::Matrix4(Ref::new(
            na::Matrix4::from_row_slice(&elements),
        ))),
        #[cfg(feature = "matrix2x3")]
        MatrixStorage::Matrix2x3 => Ok(crate::matrix::Matrix::Matrix2x3(Ref::new(
            na::Matrix2x3::from_row_slice(&elements),
        ))),
        #[cfg(feature = "matrix3x2")]
        MatrixStorage::Matrix3x2 => Ok(crate::matrix::Matrix::Matrix3x2(Ref::new(
            na::Matrix3x2::from_row_slice(&elements),
        ))),
        #[cfg(feature = "row_vector2")]
        MatrixStorage::RowVector2 => Ok(crate::matrix::Matrix::RowVector2(Ref::new(
            na::RowVector2::from_row_slice(&elements),
        ))),
        #[cfg(feature = "row_vector3")]
        MatrixStorage::RowVector3 => Ok(crate::matrix::Matrix::RowVector3(Ref::new(
            na::RowVector3::from_row_slice(&elements),
        ))),
        #[cfg(feature = "row_vector4")]
        MatrixStorage::RowVector4 => Ok(crate::matrix::Matrix::RowVector4(Ref::new(
            na::RowVector4::from_row_slice(&elements),
        ))),
        #[cfg(feature = "vector2")]
        MatrixStorage::Vector2 => Ok(crate::matrix::Matrix::Vector2(Ref::new(
            na::Vector2::from_column_slice(&elements),
        ))),
        #[cfg(feature = "vector3")]
        MatrixStorage::Vector3 => Ok(crate::matrix::Matrix::Vector3(Ref::new(
            na::Vector3::from_column_slice(&elements),
        ))),
        #[cfg(feature = "vector4")]
        MatrixStorage::Vector4 => Ok(crate::matrix::Matrix::Vector4(Ref::new(
            na::Vector4::from_column_slice(&elements),
        ))),
        #[cfg(feature = "row_vectord")]
        MatrixStorage::RowVectorD => Ok(crate::matrix::Matrix::RowDVector(Ref::new(
            na::RowDVector::from_row_slice(&elements),
        ))),
        #[cfg(feature = "vectord")]
        MatrixStorage::VectorD => Ok(crate::matrix::Matrix::DVector(Ref::new(
            na::DVector::from_column_slice(&elements),
        ))),
        #[cfg(feature = "matrixd")]
        MatrixStorage::MatrixD => Ok(crate::matrix::Matrix::DMatrix(Ref::new(
            na::DMatrix::from_row_slice(row_count, column_count, &elements),
        ))),
        _ => invalid(format!(
            "matrix storage {storage:?} is unavailable in this runtime"
        )),
    }
}
