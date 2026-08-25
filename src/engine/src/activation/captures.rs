use super::ActivationPatternCaptureKindUnsupported;
#[cfg(feature = "complex")]
use crate::C64;
#[cfg(feature = "atom")]
use crate::MechAtom;
#[cfg(feature = "enum")]
use crate::MechEnum;
#[cfg(feature = "map")]
use crate::MechMap;
#[cfg(feature = "record")]
use crate::MechRecord;
#[cfg(feature = "set")]
use crate::MechSet;
#[cfg(feature = "table")]
use crate::MechTable;
#[cfg(feature = "tuple")]
use crate::MechTuple;
#[cfg(feature = "rational")]
use crate::R64;
use crate::{
    Interpreter, LegacyValue, MResult, MechError, PatternBindingSink, PatternMatch, Ref, ValueKind,
    hash_str,
};
#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;

pub(super) fn generation() -> (Ref<usize>, LegacyValue) {
    let generation = Ref::new(0);
    (generation.clone(), LegacyValue::Index(generation))
}

pub(super) fn transaction_bool_state(
    #[cfg(any(feature = "bool", feature = "variable_define"))] value: &Ref<bool>,
    #[cfg(not(any(feature = "bool", feature = "variable_define")))] _: &Ref<bool>,
) -> MResult<LegacyValue> {
    #[cfg(any(feature = "bool", feature = "variable_define"))]
    {
        Ok(LegacyValue::Bool(value.clone()))
    }
    #[cfg(not(any(feature = "bool", feature = "variable_define")))]
    {
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
    pub(super) proposed: LegacyValue,
    pub(super) committed: LegacyValue,
}
pub(super) fn detached(v: &LegacyValue) -> LegacyValue {
    match v {
        LegacyValue::MutableReference(r) => detached(&r.borrow()),
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

pub(super) fn create_capture_slot_for_kind(
    kind: &ValueKind,
    interpreter: &Interpreter,
) -> MResult<LegacyValue> {
    match kind.deref_kind() {
        #[cfg(feature = "u8")]
        ValueKind::U8 => Ok(LegacyValue::U8(Ref::new(0))),
        #[cfg(feature = "u16")]
        ValueKind::U16 => Ok(LegacyValue::U16(Ref::new(0))),
        #[cfg(feature = "u32")]
        ValueKind::U32 => Ok(LegacyValue::U32(Ref::new(0))),
        #[cfg(feature = "u64")]
        ValueKind::U64 => Ok(LegacyValue::U64(Ref::new(0))),
        #[cfg(feature = "u128")]
        ValueKind::U128 => Ok(LegacyValue::U128(Ref::new(0))),
        #[cfg(feature = "i8")]
        ValueKind::I8 => Ok(LegacyValue::I8(Ref::new(0))),
        #[cfg(feature = "i16")]
        ValueKind::I16 => Ok(LegacyValue::I16(Ref::new(0))),
        #[cfg(feature = "i32")]
        ValueKind::I32 => Ok(LegacyValue::I32(Ref::new(0))),
        #[cfg(feature = "i64")]
        ValueKind::I64 => Ok(LegacyValue::I64(Ref::new(0))),
        #[cfg(feature = "i128")]
        ValueKind::I128 => Ok(LegacyValue::I128(Ref::new(0))),
        #[cfg(feature = "f64")]
        ValueKind::F64 => Ok(LegacyValue::F64(Ref::new(0.0))),
        #[cfg(feature = "f32")]
        ValueKind::F32 => Ok(LegacyValue::F32(Ref::new(0.0))),
        #[cfg(feature = "complex")]
        ValueKind::C64 => Ok(LegacyValue::C64(Ref::new(C64::default()))),
        #[cfg(feature = "rational")]
        ValueKind::R64 => Ok(LegacyValue::R64(Ref::new(R64::default()))),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        ValueKind::Bool => Ok(LegacyValue::Bool(Ref::new(false))),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        ValueKind::String => Ok(LegacyValue::String(Ref::new(String::new()))),
        ValueKind::Index => Ok(LegacyValue::Index(Ref::new(0))),
        #[cfg(feature = "atom")]
        ValueKind::Atom(id, _) => Ok(LegacyValue::Atom(Ref::new(MechAtom::new(id)))),
        #[cfg(feature = "tuple")]
        ValueKind::Tuple(kinds) => Ok(LegacyValue::Tuple(Ref::new(MechTuple::from_vec(
            kinds
                .iter()
                .map(|kind| create_capture_slot_for_kind(kind, interpreter))
                .collect::<MResult<Vec<_>>>()?,
        )))),
        #[cfg(feature = "enum")]
        ValueKind::Enum(id, _) => Ok(LegacyValue::Enum(Ref::new(MechEnum {
            id,
            variants: Vec::new(),
            names: interpreter.dictionary(),
        }))),
        #[cfg(feature = "record")]
        ValueKind::Record(fields) => {
            let values = fields
                .iter()
                .map(|(name, kind)| {
                    Ok((
                        (hash_str(name), name.clone()),
                        create_capture_slot_for_kind(kind, interpreter)?,
                    ))
                })
                .collect::<MResult<Vec<_>>>()?;
            Ok(LegacyValue::Record(Ref::new(MechRecord::from_vec(values))))
        }
        #[cfg(feature = "map")]
        ValueKind::Map(key_kind, value_kind) => Ok(LegacyValue::Map(Ref::new(MechMap {
            key_kind: *key_kind,
            value_kind: *value_kind,
            num_elements: 0,
            map: Default::default(),
        }))),
        #[cfg(feature = "set")]
        ValueKind::Set(element_kind, size) => Ok(LegacyValue::Set(Ref::new(MechSet::new(
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
            Ok(LegacyValue::Table(Ref::new(MechTable::new_table(
                names, kinds, values,
            ))))
        }
        #[cfg(feature = "matrix")]
        ValueKind::Matrix(element_kind, shape) => {
            let (rows, cols) = capture_matrix_dimensions(&shape)?;
            let count = rows.saturating_mul(cols);
            match *element_kind {
                ValueKind::Index => Ok(LegacyValue::MatrixIndex(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "bool")]
                ValueKind::Bool => Ok(LegacyValue::MatrixBool(Matrix::from_vec(
                    vec![false; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u8")]
                ValueKind::U8 => Ok(LegacyValue::MatrixU8(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u16")]
                ValueKind::U16 => Ok(LegacyValue::MatrixU16(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u32")]
                ValueKind::U32 => Ok(LegacyValue::MatrixU32(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u64")]
                ValueKind::U64 => Ok(LegacyValue::MatrixU64(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "u128")]
                ValueKind::U128 => Ok(LegacyValue::MatrixU128(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "i8")]
                ValueKind::I8 => Ok(LegacyValue::MatrixI8(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "i16")]
                ValueKind::I16 => Ok(LegacyValue::MatrixI16(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "i32")]
                ValueKind::I32 => Ok(LegacyValue::MatrixI32(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "i64")]
                ValueKind::I64 => Ok(LegacyValue::MatrixI64(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "i128")]
                ValueKind::I128 => Ok(LegacyValue::MatrixI128(Matrix::from_vec(
                    vec![0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "f32")]
                ValueKind::F32 => Ok(LegacyValue::MatrixF32(Matrix::from_vec(
                    vec![0.0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "f64")]
                ValueKind::F64 => Ok(LegacyValue::MatrixF64(Matrix::from_vec(
                    vec![0.0; count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "string")]
                ValueKind::String => Ok(LegacyValue::MatrixString(Matrix::from_vec(
                    vec![String::new(); count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "rational")]
                ValueKind::R64 => Ok(LegacyValue::MatrixR64(Matrix::from_vec(
                    vec![R64::default(); count],
                    rows,
                    cols,
                ))),
                #[cfg(feature = "complex")]
                ValueKind::C64 => Ok(LegacyValue::MatrixC64(Matrix::from_vec(
                    vec![C64::default(); count],
                    rows,
                    cols,
                ))),
                element_kind => {
                    let default = create_capture_slot_for_kind(&element_kind, interpreter)
                        .unwrap_or(LegacyValue::EmptyKind(element_kind));
                    Ok(LegacyValue::MatrixValue(Matrix::from_vec(
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

fn capture_slot_accepts_payload(destination: &LegacyValue, source: &LegacyValue) -> bool {
    let source = detached(source);
    match (destination, &source) {
        #[cfg(feature = "tuple")]
        (LegacyValue::Tuple(destination), LegacyValue::Tuple(source)) => {
            let destination = destination.borrow();
            let source = source.borrow();
            destination.elements.len() == source.elements.len()
                && destination
                    .elements
                    .iter()
                    .zip(&source.elements)
                    .all(|(destination, source)| capture_slot_accepts_payload(destination, source))
        }
        #[cfg(feature = "enum")]
        (LegacyValue::Enum(destination), LegacyValue::Enum(source)) => {
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
                    .all(|((destination_id, _), (source_id, _))| destination_id == source_id);
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
        (LegacyValue::Record(destination), LegacyValue::Record(source)) => {
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
        (LegacyValue::Map(destination), LegacyValue::Map(source)) => {
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
                    .all(|(destination, source)| capture_slot_accepts_payload(destination, source))
        }
        #[cfg(feature = "table")]
        (LegacyValue::Table(destination), LegacyValue::Table(source)) => {
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
        (LegacyValue::MatrixIndex(destination), LegacyValue::MatrixIndex(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        (LegacyValue::MatrixBool(destination), LegacyValue::MatrixBool(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u8"))]
        (LegacyValue::MatrixU8(destination), LegacyValue::MatrixU8(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u16"))]
        (LegacyValue::MatrixU16(destination), LegacyValue::MatrixU16(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u32"))]
        (LegacyValue::MatrixU32(destination), LegacyValue::MatrixU32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u64"))]
        (LegacyValue::MatrixU64(destination), LegacyValue::MatrixU64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "u128"))]
        (LegacyValue::MatrixU128(destination), LegacyValue::MatrixU128(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i8"))]
        (LegacyValue::MatrixI8(destination), LegacyValue::MatrixI8(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i16"))]
        (LegacyValue::MatrixI16(destination), LegacyValue::MatrixI16(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i32"))]
        (LegacyValue::MatrixI32(destination), LegacyValue::MatrixI32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i64"))]
        (LegacyValue::MatrixI64(destination), LegacyValue::MatrixI64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "i128"))]
        (LegacyValue::MatrixI128(destination), LegacyValue::MatrixI128(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "f32"))]
        (LegacyValue::MatrixF32(destination), LegacyValue::MatrixF32(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "f64"))]
        (LegacyValue::MatrixF64(destination), LegacyValue::MatrixF64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "string"))]
        (LegacyValue::MatrixString(destination), LegacyValue::MatrixString(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "rational"))]
        (LegacyValue::MatrixR64(destination), LegacyValue::MatrixR64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(all(feature = "matrix", feature = "complex"))]
        (LegacyValue::MatrixC64(destination), LegacyValue::MatrixC64(source)) => {
            destination.can_replace_payload_from(source)
        }
        #[cfg(feature = "matrix")]
        (LegacyValue::MatrixValue(destination), LegacyValue::MatrixValue(source)) => {
            destination.can_replace_payload_from(source)
        }
        (destination, source) => {
            std::mem::discriminant(destination) == std::mem::discriminant(source)
        }
    }
}

pub(super) fn commit_capture_slot(destination: &LegacyValue, source: &LegacyValue) -> MResult<()> {
    if !capture_slot_accepts_payload(destination, source) {
        return Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        ));
    }
    match (destination, &detached(source)) {
        #[cfg(feature = "u8")]
        (LegacyValue::U8(a), LegacyValue::U8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u16")]
        (LegacyValue::U16(a), LegacyValue::U16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u32")]
        (LegacyValue::U32(a), LegacyValue::U32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u64")]
        (LegacyValue::U64(a), LegacyValue::U64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u128")]
        (LegacyValue::U128(a), LegacyValue::U128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i8")]
        (LegacyValue::I8(a), LegacyValue::I8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i16")]
        (LegacyValue::I16(a), LegacyValue::I16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i32")]
        (LegacyValue::I32(a), LegacyValue::I32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i64")]
        (LegacyValue::I64(a), LegacyValue::I64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i128")]
        (LegacyValue::I128(a), LegacyValue::I128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f64")]
        (LegacyValue::F64(a), LegacyValue::F64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f32")]
        (LegacyValue::F32(a), LegacyValue::F32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "complex")]
        (LegacyValue::C64(a), LegacyValue::C64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "rational")]
        (LegacyValue::R64(a), LegacyValue::R64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        (LegacyValue::Bool(a), LegacyValue::Bool(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "string", feature = "variable_define"))]
        (LegacyValue::String(a), LegacyValue::String(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        (LegacyValue::Index(a), LegacyValue::Index(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "atom")]
        (LegacyValue::Atom(a), LegacyValue::Atom(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "tuple")]
        (LegacyValue::Tuple(a), LegacyValue::Tuple(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            for (destination, source) in a.elements.iter().zip(&b.elements) {
                commit_capture_slot(destination, source)?;
            }
            Ok(())
        }
        #[cfg(feature = "enum")]
        (LegacyValue::Enum(a), LegacyValue::Enum(b)) => {
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
        (LegacyValue::Record(a), LegacyValue::Record(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            for ((_, destination), (_, source)) in a.data.iter().zip(&b.data) {
                commit_capture_slot(destination, source)?;
            }
            Ok(())
        }
        #[cfg(feature = "map")]
        (LegacyValue::Map(a), LegacyValue::Map(b)) => {
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
        (LegacyValue::Set(a), LegacyValue::Set(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "table")]
        (LegacyValue::Table(a), LegacyValue::Table(b)) => {
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
        (LegacyValue::MatrixIndex(a), LegacyValue::MatrixIndex(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        (LegacyValue::MatrixBool(a), LegacyValue::MatrixBool(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "u8"))]
        (LegacyValue::MatrixU8(a), LegacyValue::MatrixU8(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        (LegacyValue::MatrixU16(a), LegacyValue::MatrixU16(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "u32"))]
        (LegacyValue::MatrixU32(a), LegacyValue::MatrixU32(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "u64"))]
        (LegacyValue::MatrixU64(a), LegacyValue::MatrixU64(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "u128"))]
        (LegacyValue::MatrixU128(a), LegacyValue::MatrixU128(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "i8"))]
        (LegacyValue::MatrixI8(a), LegacyValue::MatrixI8(b)) if a.replace_payload_from(b) => Ok(()),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        (LegacyValue::MatrixI16(a), LegacyValue::MatrixI16(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "i32"))]
        (LegacyValue::MatrixI32(a), LegacyValue::MatrixI32(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "i64"))]
        (LegacyValue::MatrixI64(a), LegacyValue::MatrixI64(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "i128"))]
        (LegacyValue::MatrixI128(a), LegacyValue::MatrixI128(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "f32"))]
        (LegacyValue::MatrixF32(a), LegacyValue::MatrixF32(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "f64"))]
        (LegacyValue::MatrixF64(a), LegacyValue::MatrixF64(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "string"))]
        (LegacyValue::MatrixString(a), LegacyValue::MatrixString(b))
            if a.replace_payload_from(b) =>
        {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "rational"))]
        (LegacyValue::MatrixR64(a), LegacyValue::MatrixR64(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(all(feature = "matrix", feature = "complex"))]
        (LegacyValue::MatrixC64(a), LegacyValue::MatrixC64(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
        #[cfg(feature = "matrix")]
        (LegacyValue::MatrixValue(a), LegacyValue::MatrixValue(b)) if a.replace_payload_from(b) => {
            Ok(())
        }
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
            let capture = self
                .captures
                .get(binding.index)
                .ok_or_else(|| MechError::new(ActivationPatternCaptureKindUnsupported, None))?;
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
