use crate::{
    Interpreter, MResult, MechError, PatternBindingSink, PatternMatch, Ref, Value, ValueKind,
    hash_str,
};
#[cfg(feature = "tuple")]
use crate::MechTuple;
#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;
#[cfg(feature = "atom")]
use crate::MechAtom;
#[cfg(feature = "complex")]
use crate::C64;
#[cfg(feature = "enum")]
use crate::MechEnum;
#[cfg(feature = "map")]
use crate::MechMap;
#[cfg(feature = "rational")]
use crate::R64;
#[cfg(feature = "record")]
use crate::MechRecord;
#[cfg(feature = "set")]
use crate::MechSet;
#[cfg(feature = "table")]
use crate::MechTable;
use super::{
    ActivationPatternCaptureKindUnsupported, ActivationPatternTransactionBoolStateUnsupported,
};

pub(super) fn generation() -> (Ref<usize>, Value) {
    let generation = Ref::new(0);
    (generation.clone(), Value::Index(generation))
}

pub(super) fn transaction_bool_state(value: &Ref<bool>) -> MResult<Value> {
    #[cfg(any(feature = "bool", feature = "variable_define"))]
    {
        Ok(Value::Bool(value.clone()))
    }
    #[cfg(not(any(feature = "bool", feature = "variable_define")))]
    {
        let _ = value;
        Err(MechError::new(
            ActivationPatternTransactionBoolStateUnsupported,
            None,
        ))
    }
}

#[derive(Clone)]
pub(super) struct ActivationPatternCapture {
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) kind: ValueKind,
    pub(super) proposed: Value,
    pub(super) committed: Value,
}
pub(super) fn detached(v: &Value) -> Value {
    match v {
        Value::MutableReference(r) => detached(&r.borrow()),
        _ => v.clone(),
    }
}
fn clone_ref_value<T: Clone>(destination: &Ref<T>, source: &Ref<T>) {
    destination.borrow_mut().clone_from(&*source.borrow())
}
#[cfg(feature = "matrix")]
fn capture_matrix_dimensions(shape: &[usize]) -> MResult<(usize, usize)> {
    match shape {
        [] => Ok((1, 0)),
        [rows, cols] => Ok((*rows, *cols)),
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}

pub(super) fn create_capture_slot_for_kind(kind: &ValueKind, interpreter: &Interpreter) -> MResult<Value> {
    match kind.deref_kind() {
        #[cfg(feature = "u8")]
        ValueKind::U8 => Ok(Value::U8(Ref::new(0))),
        #[cfg(feature = "u16")]
        ValueKind::U16 => Ok(Value::U16(Ref::new(0))),
        #[cfg(feature = "u32")]
        ValueKind::U32 => Ok(Value::U32(Ref::new(0))),
        #[cfg(feature = "u64")]
        ValueKind::U64 => Ok(Value::U64(Ref::new(0))),
        #[cfg(feature = "u128")]
        ValueKind::U128 => Ok(Value::U128(Ref::new(0))),
        #[cfg(feature = "i8")]
        ValueKind::I8 => Ok(Value::I8(Ref::new(0))),
        #[cfg(feature = "i16")]
        ValueKind::I16 => Ok(Value::I16(Ref::new(0))),
        #[cfg(feature = "i32")]
        ValueKind::I32 => Ok(Value::I32(Ref::new(0))),
        #[cfg(feature = "i64")]
        ValueKind::I64 => Ok(Value::I64(Ref::new(0))),
        #[cfg(feature = "i128")]
        ValueKind::I128 => Ok(Value::I128(Ref::new(0))),
        #[cfg(feature = "f64")]
        ValueKind::F64 => Ok(Value::F64(Ref::new(0.0))),
        #[cfg(feature = "f32")]
        ValueKind::F32 => Ok(Value::F32(Ref::new(0.0))),
        #[cfg(feature = "complex")]
        ValueKind::C64 => Ok(Value::C64(Ref::new(C64::default()))),
        #[cfg(feature = "rational")]
        ValueKind::R64 => Ok(Value::R64(Ref::new(R64::default()))),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        ValueKind::Bool => Ok(Value::Bool(Ref::new(false))),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        ValueKind::String => Ok(Value::String(Ref::new(String::new()))),
        ValueKind::Index => Ok(Value::Index(Ref::new(0))),
        #[cfg(feature = "atom")]
        ValueKind::Atom(id, _) => Ok(Value::Atom(Ref::new(MechAtom::new(id)))),
        #[cfg(feature = "tuple")]
        ValueKind::Tuple(kinds) => Ok(Value::Tuple(Ref::new(MechTuple::from_vec(
            kinds
                .iter()
                .map(|kind| create_capture_slot_for_kind(kind, interpreter))
                .collect::<MResult<Vec<_>>>()?,
        )))),
        #[cfg(feature = "enum")]
        ValueKind::Enum(id, _) => Ok(Value::Enum(Ref::new(MechEnum {
            id,
            variants: Vec::new(),
            names: interpreter.dictionary(),
        }))),
        #[cfg(feature = "record")]
        ValueKind::Record(fields) => {
            let values = fields
                .iter()
                .map(|(name, kind)| {
                    Ok(((hash_str(name), name.clone()), create_capture_slot_for_kind(kind, interpreter)?))
                })
                .collect::<MResult<Vec<_>>>()?;
            Ok(Value::Record(Ref::new(MechRecord::from_vec(values))))
        }
        #[cfg(feature = "map")]
        ValueKind::Map(key_kind, value_kind) => Ok(Value::Map(Ref::new(MechMap {
            key_kind: *key_kind,
            value_kind: *value_kind,
            num_elements: 0,
            map: Default::default(),
        }))),
        #[cfg(feature = "set")]
        ValueKind::Set(element_kind, size) => Ok(Value::Set(Ref::new(MechSet::new(
            *element_kind,
            size.unwrap_or(0),
        )))),
        #[cfg(feature = "table")]
        ValueKind::Table(columns, rows) => {
            let mut names = Vec::with_capacity(columns.len());
            let mut kinds = Vec::with_capacity(columns.len());
            let mut values = Vec::with_capacity(columns.len());
            for (name, kind) in columns {
                names.push(name);
                kinds.push(kind.clone());
                let default = create_capture_slot_for_kind(&kind, interpreter)?;
                values.push(vec![default; rows]);
            }
            Ok(Value::Table(Ref::new(MechTable::new_table(
                names, kinds, values,
            ))))
        }
        #[cfg(feature = "matrix")]
        ValueKind::Matrix(element_kind, shape) => {
            let (rows, cols) = capture_matrix_dimensions(&shape)?;
            let count = rows.saturating_mul(cols);
            match *element_kind {
                ValueKind::Index => Ok(Value::MatrixIndex(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "bool")]
                ValueKind::Bool => Ok(Value::MatrixBool(Matrix::from_vec(
                    vec![false; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u8")]
                ValueKind::U8 => Ok(Value::MatrixU8(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "u16")]
                ValueKind::U16 => Ok(Value::MatrixU16(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "u32")]
                ValueKind::U32 => Ok(Value::MatrixU32(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "u64")]
                ValueKind::U64 => Ok(Value::MatrixU64(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "u128")]
                ValueKind::U128 => Ok(Value::MatrixU128(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "i8")]
                ValueKind::I8 => Ok(Value::MatrixI8(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "i16")]
                ValueKind::I16 => Ok(Value::MatrixI16(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "i32")]
                ValueKind::I32 => Ok(Value::MatrixI32(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "i64")]
                ValueKind::I64 => Ok(Value::MatrixI64(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "i128")]
                ValueKind::I128 => Ok(Value::MatrixI128(Matrix::from_vec(vec![0; count], rows, cols))),
                #[cfg(feature = "f32")]
                ValueKind::F32 => Ok(Value::MatrixF32(Matrix::from_vec(vec![0.0; count], rows, cols))),
                #[cfg(feature = "f64")]
                ValueKind::F64 => Ok(Value::MatrixF64(Matrix::from_vec(vec![0.0; count], rows, cols))),
                #[cfg(feature = "string")]
                ValueKind::String => Ok(Value::MatrixString(Matrix::from_vec(
                    vec![String::new(); count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "rational")]
                ValueKind::R64 => Ok(Value::MatrixR64(Matrix::from_vec(
                    vec![R64::default(); count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "complex")]
                ValueKind::C64 => Ok(Value::MatrixC64(Matrix::from_vec(
                    vec![C64::default(); count],
                    rows,
                    cols,
                ))),
                element_kind => {
                    let default = create_capture_slot_for_kind(&element_kind, interpreter)
                        .unwrap_or(Value::EmptyKind(element_kind));
                    Ok(Value::MatrixValue(Matrix::from_vec(
                        vec![default; count],
                        rows,
                        cols,
                    )))
                }
            }
        }
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}

fn capture_slot_accepts_payload(destination: &Value, source: &Value) -> bool {
    let source = detached(source);
    match (destination, &source) {
        #[cfg(feature = "tuple")]
        (Value::Tuple(destination), Value::Tuple(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            destination.elements.len() == source.elements.len()
                && destination
                    .elements
                    .iter()
                    .zip(&source.elements)
                    .all(|(destination, source)| {
                        capture_slot_accepts_payload(destination, source)
                    })
        }
        #[cfg(feature = "enum")]
        (Value::Enum(destination), Value::Enum(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            if destination.id != source.id || destination.variants.is_empty() {
                return destination.id == source.id;
            }
            let same_variants = destination.variants.len() == source.variants.len()
                && destination
                    .variants
                    .iter()
                    .zip(&source.variants)
                    .all(|((destination_id, _), (source_id, _))| {
                        destination_id == source_id
                    });
            !same_variants
                || destination.variants.iter().zip(&source.variants).all(
                    |((_, destination), (_, source))| match (destination, source) {
                        (Some(destination), Some(source)) => {
                            capture_slot_accepts_payload(destination, source)
                        }
                        (None, None) => true,
                        _ => false,
                    },
                )
        }
        #[cfg(feature = "record")]
        (Value::Record(destination), Value::Record(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            destination.data.len() == source.data.len()
                && destination.data.iter().zip(&source.data).all(
                    |((destination_id, destination), (source_id, source))| {
                        destination_id == source_id
                            && capture_slot_accepts_payload(destination, source)
                    },
                )
        }
        #[cfg(feature = "map")]
        (Value::Map(destination), Value::Map(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            if destination.map.is_empty() || destination.map.len() != source.map.len() {
                return true;
            }
            let same_keys = destination
                .map
                .keys()
                .zip(source.map.keys())
                .all(|(destination, source)| destination == source);
            !same_keys
                || destination
                    .map
                    .values()
                    .zip(source.map.values())
                    .all(|(destination, source)| {
                        capture_slot_accepts_payload(destination, source)
                    })
        }
        #[cfg(feature = "table")]
        (Value::Table(destination), Value::Table(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            destination.rows == source.rows
                && destination.data.len() == source.data.len()
                && destination.data.iter().zip(&source.data).all(
                    |(
                        (destination_id, (destination_kind, destination)),
                        (source_id, (source_kind, source)),
                    )| {
                        destination_id == source_id
                            && destination_kind == source_kind
                            && destination.can_replace_payload_from(source)
                    },
                )
        }
        #[cfg(feature = "matrix")]
        (Value::MatrixIndex(destination), Value::MatrixIndex(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        (Value::MatrixBool(destination), Value::MatrixBool(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u8"))]
        (Value::MatrixU8(destination), Value::MatrixU8(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u16"))]
        (Value::MatrixU16(destination), Value::MatrixU16(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u32"))]
        (Value::MatrixU32(destination), Value::MatrixU32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u64"))]
        (Value::MatrixU64(destination), Value::MatrixU64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u128"))]
        (Value::MatrixU128(destination), Value::MatrixU128(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i8"))]
        (Value::MatrixI8(destination), Value::MatrixI8(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i16"))]
        (Value::MatrixI16(destination), Value::MatrixI16(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i32"))]
        (Value::MatrixI32(destination), Value::MatrixI32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i64"))]
        (Value::MatrixI64(destination), Value::MatrixI64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i128"))]
        (Value::MatrixI128(destination), Value::MatrixI128(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "f32"))]
        (Value::MatrixF32(destination), Value::MatrixF32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "f64"))]
        (Value::MatrixF64(destination), Value::MatrixF64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "string"))]
        (Value::MatrixString(destination), Value::MatrixString(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "rational"))]
        (Value::MatrixR64(destination), Value::MatrixR64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "complex"))]
        (Value::MatrixC64(destination), Value::MatrixC64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(feature = "matrix")]
        (Value::MatrixValue(destination), Value::MatrixValue(source)) => {
            destination.can_replace_payload_from(source)
        }
        (destination, source) => {
            std::mem::discriminant(destination) == std::mem::discriminant(source)
        }
    }
}

pub(super) fn commit_capture_slot(destination: &Value, source: &Value) -> MResult<()> {
    if !capture_slot_accepts_payload(destination, source) {
        return Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        ));
    }
    match (destination, &detached(source)) {
        #[cfg(feature = "u8")]
        (Value::U8(a), Value::U8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u16")]
        (Value::U16(a), Value::U16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u32")]
        (Value::U32(a), Value::U32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u64")]
        (Value::U64(a), Value::U64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u128")]
        (Value::U128(a), Value::U128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i8")]
        (Value::I8(a), Value::I8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i16")]
        (Value::I16(a), Value::I16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i32")]
        (Value::I32(a), Value::I32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i64")]
        (Value::I64(a), Value::I64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i128")]
        (Value::I128(a), Value::I128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f64")]
        (Value::F64(a), Value::F64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f32")]
        (Value::F32(a), Value::F32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "complex")]
        (Value::C64(a), Value::C64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "rational")]
        (Value::R64(a), Value::R64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        (Value::Bool(a), Value::Bool(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "string", feature = "variable_define"))]
        (Value::String(a), Value::String(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        (Value::Index(a), Value::Index(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "atom")]
        (Value::Atom(a), Value::Atom(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "tuple")]
        (Value::Tuple(a), Value::Tuple(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            for (destination, source) in a.elements.iter().zip(&b.elements) {
                commit_capture_slot(destination, source)?;
            }
            Ok(())
        }
        #[cfg(feature = "enum")]
        (Value::Enum(a), Value::Enum(b)) => {
            let preserve_payload_cells = {
                let a = a.borrow();
                let b = b.borrow();
                !a.variants.is_empty()
                    && a.variants.len() == b.variants.len()
                    && a.variants
                        .iter()
                        .zip(&b.variants)
                        .all(|((a, _), (b, _))| a == b)
            };
            if preserve_payload_cells {
                let a = a.borrow();
                let b = b.borrow();
                for ((_, destination), (_, source)) in a.variants.iter().zip(&b.variants) {
                    if let (Some(destination), Some(source)) = (destination, source) {
                        commit_capture_slot(destination, source)?;
                    }
                }
            } else {
                clone_ref_value(a, b);
            }
            Ok(())
        }
        #[cfg(feature = "record")]
        (Value::Record(a), Value::Record(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            for ((_, destination), (_, source)) in a.data.iter().zip(&b.data) {
                commit_capture_slot(destination, source)?;
            }
            Ok(())
        }
        #[cfg(feature = "map")]
        (Value::Map(a), Value::Map(b)) => {
            let preserve_value_cells = {
                let a = a.borrow();
                let b = b.borrow();
                !a.map.is_empty()
                    && a.map.len() == b.map.len()
                    && a.map.keys().zip(b.map.keys()).all(|(a, b)| a == b)
            };
            if preserve_value_cells {
                let a = a.borrow();
                let b = b.borrow();
                for ((_, destination), (_, source)) in a.map.iter().zip(&b.map) {
                    commit_capture_slot(destination, source)?;
                }
            } else {
                clone_ref_value(a, b);
            }
            Ok(())
        }
        #[cfg(feature = "set")]
        (Value::Set(a), Value::Set(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "table")]
        (Value::Table(a), Value::Table(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            for ((_, (_, destination)), (_, (_, source))) in a.data.iter().zip(&b.data) {
                if !destination.replace_payload_from(source) {
                    return Err(MechError::new(
                        ActivationPatternCaptureKindUnsupported,
                        None,
                    ));
                }
            }
            Ok(())
        }
        #[cfg(feature = "matrix")]
        (Value::MatrixIndex(a), Value::MatrixIndex(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        (Value::MatrixBool(a), Value::MatrixBool(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        (Value::MatrixU8(a), Value::MatrixU8(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        (Value::MatrixU16(a), Value::MatrixU16(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        (Value::MatrixU32(a), Value::MatrixU32(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        (Value::MatrixU64(a), Value::MatrixU64(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        (Value::MatrixU128(a), Value::MatrixU128(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        (Value::MatrixI8(a), Value::MatrixI8(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        (Value::MatrixI16(a), Value::MatrixI16(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        (Value::MatrixI32(a), Value::MatrixI32(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        (Value::MatrixI64(a), Value::MatrixI64(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        (Value::MatrixI128(a), Value::MatrixI128(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        (Value::MatrixF32(a), Value::MatrixF32(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        (Value::MatrixF64(a), Value::MatrixF64(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "string"))]
        (Value::MatrixString(a), Value::MatrixString(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        (Value::MatrixR64(a), Value::MatrixR64(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        (Value::MatrixC64(a), Value::MatrixC64(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(feature = "matrix")]
        (Value::MatrixValue(a), Value::MatrixValue(b)) if a.replace_payload_from(b) => Ok(()),
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}

fn capture_kinds_are_storage_compatible(destination: &ValueKind, source: &ValueKind) -> bool {
    let destination = destination.deref_kind();
    let source = source.deref_kind();
    #[cfg(feature = "atom")]
    if matches!(
        (&destination, &source),
        (ValueKind::Atom(_, _), ValueKind::Atom(_, _))
    ) {
        return true;
    }
    #[cfg(feature = "enum")]
    if matches!(
        (&destination, &source),
        (ValueKind::Enum(destination, _), ValueKind::Enum(source, _)) if destination == source
    ) {
        return true;
    }
    #[cfg(feature = "matrix")]
    if matches!(
        (&destination, &source),
        (
            ValueKind::Matrix(destination_element, destination_shape),
            ValueKind::Matrix(source_element, _)
        ) if destination_shape.is_empty() && destination_element == source_element
    ) {
        return true;
    }
    destination == source
}

pub(super) struct ReactiveBindingSink<'a> {
    pub(super) captures: &'a [ActivationPatternCapture],
}

impl PatternBindingSink for ReactiveBindingSink<'_> {
    fn commit(&mut self, pattern_match: &PatternMatch) -> MResult<()> {
        if !pattern_match.matched {
            return Ok(());
        }

        // Validate every destination before mutating any stable capture cell.
        for binding in &pattern_match.bindings {
            let capture = self.captures.get(binding.index).ok_or_else(|| {
                MechError::new(ActivationPatternCaptureKindUnsupported, None)
            })?;
            let source = detached(&binding.value);
            if capture.id != binding.id
                || !capture_kinds_are_storage_compatible(&capture.kind, &binding.kind)
                || !capture_slot_accepts_payload(&capture.proposed, &source)
            {
                return Err(MechError::new(
                    ActivationPatternCaptureKindUnsupported,
                    None,
                ));
            }
        }

        for binding in &pattern_match.bindings {
            commit_capture_slot(&self.captures[binding.index].proposed, &binding.value)?;
        }
        Ok(())
    }
}

pub(super) fn commit_proposed_captures(captures: &[ActivationPatternCapture]) -> MResult<()> {
    // Preflight the complete batch so a later incompatible destination cannot
    // leave an arm body observing only some captures from the current trigger.
    for capture in captures {
        let proposed = detached(&capture.proposed);
        if !capture_kinds_are_storage_compatible(&capture.kind, &proposed.kind())
            || !capture_slot_accepts_payload(&capture.committed, &proposed)
        {
            return Err(MechError::new(
                ActivationPatternCaptureKindUnsupported,
                None,
            ));
        }
    }

    for capture in captures {
        commit_capture_slot(&capture.committed, &capture.proposed)?;
    }
    Ok(())
}
