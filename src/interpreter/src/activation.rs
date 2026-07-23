#![cfg(all(feature = "functions", feature = "symbol_table"))]
//! Statically elaborated structural dispatch for patterned activation scopes.
use crate::*;
use std::collections::HashSet;

macro_rules! activation_error {
    ($n:ident,$m:expr) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $n;
        impl MechErrorKind for $n {
            fn name(&self) -> &str {
                stringify!($n)
            }
            fn message(&self) -> String {
                $m.into()
            }
        }
    };
}
activation_error!(
    ActivationPatternCaptureKindUnsupported,
    "The capture kind cannot be inferred from the activation trigger."
);
activation_error!(
    ActivationPatternArmsNonExhaustive,
    "Patterned activations require a final unguarded wildcard arm."
);
activation_error!(
    ActivationPatternWildcardMustBeLast,
    "The wildcard activation arm must be last."
);
activation_error!(
    ActivationPatternGuardMustBePure,
    "Patterned activation guards must elaborate to a static pure expression graph."
);
activation_error!(
    ActivationPatternGuardDependencyInvariant,
    "The activation guard graph could not be attached to its match pulse."
);
activation_error!(
    ActivationPatternBodyDependencyInvariant,
    "The activation arm body could not sample its committed captures."
);
activation_error!(
    ActivationPatternRegisterWriteUnsupported,
    "Patterned activation register writes are not supported."
);
activation_error!(
    ActivationPatternContextEffectUnsupported,
    "Patterned activation context effects are not supported."
);
activation_error!(
    ActivationPatternTriggerInvariant,
    "Activation trigger root cells disagree with the resolved trigger."
);

#[derive(Clone)]
struct ActivationPatternCapture {
    id: u64,
    name: String,
    kind: ValueKind,
    proposed: Value,
    committed: Value,
}
#[derive(Clone)]
struct PreflightActivationArm {
    pattern: CompiledPattern,
    captures: Vec<ActivationPatternCapture>,
}
struct PreflightPatternedActivation {
    trigger_kind: ValueKind,
    arms: Vec<PreflightActivationArm>,
}
#[derive(Debug, Clone)]
pub(crate) struct ActivationPatternDefinitionUnsupported;
impl MechErrorKind for ActivationPatternDefinitionUnsupported {
    fn name(&self) -> &str {
        "ActivationPatternDefinitionUnsupported"
    }
    fn message(&self) -> String {
        "This definition or declaration is not supported inside a patterned activation arm."
            .to_string()
    }
}
fn detached(v: &Value) -> Value {
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

fn create_capture_slot_for_kind(kind: &ValueKind, interpreter: &Interpreter) -> MResult<Value> {
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

fn commit_capture_slot(destination: &Value, source: &Value) -> MResult<()> {
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

struct ReactiveBindingSink<'a> {
    captures: &'a [ActivationPatternCapture],
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

fn commit_proposed_captures(captures: &[ActivationPatternCapture]) -> MResult<()> {
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

fn generation() -> (Ref<usize>, Value) {
    let r = Ref::new(0);
    (r.clone(), Value::Index(r))
}
struct ScopePulse {
    out: Ref<usize>,
}
impl MechFunctionImpl for ScopePulse {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_dependency_scopes(&self, _: usize) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Root])
    }
    fn to_string(&self) -> String {
        "ActivationPatternScopePulse".into()
    }
}
struct Matcher {
    pattern: CompiledPattern,
    trigger: Value,
    expression_values: Vec<Value>,
    captures: Vec<ActivationPatternCapture>,
    matched: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Matcher {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let pattern_match = match_compiled_pattern_with_values(
            &self.pattern,
            &self.trigger,
            &self.expression_values,
        )?;
        ReactiveBindingSink {
            captures: &self.captures,
        }
        .commit(&pattern_match)?;
        *self.matched.borrow_mut() = pattern_match.matched;
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        let mut outputs = vec![self.out()];
        outputs.extend(self.captures.iter().map(|capture| capture.proposed.clone()));
        outputs
    }
    fn reactive_dependency_kinds(&self, argument_count: usize) -> Option<Vec<ReactiveDependencyKind>> {
        let mut kinds = vec![ReactiveDependencyKind::Sampled; argument_count];
        if let Some(scope_pulse) = kinds.first_mut() {
            *scope_pulse = ReactiveDependencyKind::Reactive;
        }
        Some(kinds)
    }
    fn to_string(&self) -> String {
        "ActivationPatternMatcher".into()
    }
}
struct Finalize {
    matched: Ref<bool>,
    eligible: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Finalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.eligible.borrow_mut() = *self.matched.borrow();
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmFinalize".into()
    }
}
struct MatchGate {
    matched: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for MatchGate {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.matched.borrow() {
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardMatchGate".into()
    }
}
struct UnmatchedFinalize {
    matched: Ref<bool>,
    eligible: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for UnmatchedFinalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.matched.borrow() {
            Ok(ReactiveSolveStatus::Unchanged)
        } else {
            *self.eligible.borrow_mut() = false;
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardUnmatchedFinalize".into()
    }
}
struct GuardFinalize {
    guard: Ref<bool>,
    eligible: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for GuardFinalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.eligible.borrow_mut() = *self.guard.borrow();
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardFinalize".into()
    }
}
struct Select {
    eligible: Vec<Ref<bool>>,
    selected: Ref<usize>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Select {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.selected.borrow_mut() = self
            .eligible
            .iter()
            .position(|x| *x.borrow())
            .unwrap_or(usize::MAX);
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternSelectArm".into()
    }
}
struct Gate {
    arm: usize,
    selected: Ref<usize>,
    captures: Vec<ActivationPatternCapture>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Gate {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.selected.borrow() == self.arm {
            commit_proposed_captures(&self.captures)?;
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        let mut outputs = vec![self.out()];
        outputs.extend(
            self.captures
                .iter()
                .map(|capture| capture.committed.clone()),
        );
        outputs
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmGate".into()
    }
}

#[cfg(feature = "compiler")]
macro_rules! interpreter_only {
    ($t:ty) => {
        impl MechFunctionCompiler for $t {
            fn compile(&self, _: &mut CompileCtx) -> MResult<Register> {
                Err(MechError::new(
                    GenericError {
                        msg: "Activation pattern dispatch is interpreter-only.".into(),
                    },
                    None,
                ))
            }
        }
    };
}
#[cfg(feature = "compiler")]
interpreter_only!(ScopePulse);
#[cfg(feature = "compiler")]
interpreter_only!(Matcher);
#[cfg(feature = "compiler")]
interpreter_only!(Finalize);
#[cfg(feature = "compiler")]
interpreter_only!(MatchGate);
#[cfg(feature = "compiler")]
interpreter_only!(UnmatchedFinalize);
#[cfg(feature = "compiler")]
interpreter_only!(GuardFinalize);
#[cfg(feature = "compiler")]
interpreter_only!(Select);
#[cfg(feature = "compiler")]
interpreter_only!(Gate);

fn preflight_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: &Value,
    trigger_cells: &[ReactiveCellId],
    i: &Interpreter,
) -> MResult<PreflightPatternedActivation> {
    let last = arms.last().ok_or_else(|| {
        MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
    })?;
    if !matches!(last.pattern, Pattern::Wildcard) || last.guard.is_some() {
        return Err(
            MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
        );
    }
    if arms[..arms.len() - 1]
        .iter()
        .any(|a| matches!(a.pattern, Pattern::Wildcard))
    {
        return Err(
            MechError::new(ActivationPatternWildcardMustBeLast, None).with_tokens(scope.tokens())
        );
    }
    for arm in arms {
        if let Some(guard) = &arm.guard {
            validate_patterned_guard_expression(guard, i)?;
        }
        validate_patterned_arm_body(&arm.body)?;
    }
    if trigger.reactive_root_cell_ids() != trigger_cells {
        return Err(
            MechError::new(ActivationPatternTriggerInvariant, None).with_tokens(scope.tokens())
        );
    }
    let trigger_kind = trigger.kind().deref_kind();
    let mut compiled = Vec::new();
    for a in arms {
        let pattern = compile_pattern(&a.pattern, Some(&trigger_kind), i)?;
        let captures = pattern
            .binding_specs()
            .into_iter()
            .map(|binding| {
                let kind = binding.kind.ok_or_else(|| {
                    MechError::new(ActivationPatternCaptureKindUnsupported, None)
                        .with_tokens(a.pattern.tokens())
                })?;
                let proposed = create_capture_slot_for_kind(&kind, i)
                    .map_err(|error| error.with_tokens(a.pattern.tokens()))?;
                let committed = create_capture_slot_for_kind(&kind, i)
                    .map_err(|error| error.with_tokens(a.pattern.tokens()))?;
                Ok(ActivationPatternCapture {
                    id: binding.id,
                    name: binding.name,
                    kind,
                    proposed,
                    committed,
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        compiled.push(PreflightActivationArm { pattern, captures });
    }
    Ok(PreflightPatternedActivation {
        trigger_kind,
        arms: compiled,
    })
}

fn validation_error(kind: impl MechErrorKind + 'static, tokens: Vec<Token>) -> MResult<()> {
    Err(MechError::new(kind, None).with_tokens(tokens))
}

fn validate_patterned_arm_body(body: &ActivationArmBody) -> MResult<()> {
    match body {
        ActivationArmBody::Block(body) => {
            for (code, _) in body {
                validate_patterned_code(code)?;
            }
            Ok(())
        }
        ActivationArmBody::Expression(expression) => validate_patterned_expression(expression),
    }
}
fn validate_patterned_code(code: &MechCode) -> MResult<()> {
    match code {
        MechCode::Comment(_) => Ok(()),
        MechCode::Expression(expression) => validate_patterned_expression(expression),
        MechCode::Statement(statement) => validate_patterned_statement(statement),
        MechCode::ActivationScope(_)
        | MechCode::FunctionDefine(_)
        | MechCode::FsmSpecification(_)
        | MechCode::FsmImplementation(_)
        | MechCode::Import(_)
        | MechCode::Error(_, _) => {
            validation_error(ActivationPatternDefinitionUnsupported, code.tokens())
        }
    }
}
fn validate_patterned_statement(statement: &Statement) -> MResult<()> {
    match statement {
        Statement::VariableDefine(definition)
            if !definition.mutable && definition.var.context.is_none() =>
        {
            validate_patterned_expression(&definition.expression)
        }
        Statement::VariableDefine(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                statement.tokens(),
            )
        }
        Statement::VariableDefine(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, statement.tokens())
        }
        Statement::VariableAssign(assignment) if assignment.target.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                statement.tokens(),
            )
        }
        Statement::VariableAssign(_) => validation_error(
            ActivationPatternRegisterWriteUnsupported,
            statement.tokens(),
        ),
        Statement::OpAssign(assignment) if assignment.target.context.is_some() => validation_error(
            ActivationPatternContextEffectUnsupported,
            statement.tokens(),
        ),
        Statement::OpAssign(_) => validation_error(
            ActivationPatternRegisterWriteUnsupported,
            statement.tokens(),
        ),
        Statement::ContextSend(_) => validation_error(
            ActivationPatternContextEffectUnsupported,
            statement.tokens(),
        ),
        _ => validation_error(ActivationPatternDefinitionUnsupported, statement.tokens()),
    }
}
fn validate_patterned_expression(expression: &Expression) -> MResult<()> {
    match expression {
        Expression::Literal(_) | Expression::Var(_) => Ok(()),
        Expression::Slice(slice) => validate_patterned_slice(slice),
        Expression::Formula(factor) => validate_patterned_factor(factor),
        Expression::FunctionCall(call) => {
            for (_, expression) in &call.args {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Expression::Match(matched) => {
            validate_patterned_expression(&matched.source)?;
            for arm in &matched.arms {
                validate_patterned_pattern(&arm.pattern)?;
                if let Some(guard) = &arm.guard {
                    validate_patterned_expression(guard)?;
                }
                validate_patterned_expression(&arm.expression)?;
            }
            Ok(())
        }
        Expression::Range(range) => validate_patterned_range(range),
        Expression::Structure(structure) => validate_patterned_structure(structure),
        Expression::SetComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::MatrixComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::FsmPipe(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, expression.tokens())
        }
    }
}

fn validate_patterned_guard_expression(
    expression: &Expression,
    interpreter: &Interpreter,
) -> MResult<()> {
    validate_patterned_expression(expression)?;
    if guard_expression_is_not_static_pure(
        expression,
        interpreter,
        &mut HashSet::new(),
    ) {
        validation_error(ActivationPatternGuardMustBePure, expression.tokens())
    } else {
        Ok(())
    }
}

fn guard_expression_is_not_static_pure(
    expression: &Expression,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match expression {
        Expression::Literal(_) | Expression::Var(_) => false,
        Expression::Slice(slice) => slice
            .subscript
            .iter()
            .any(|subscript| {
                guard_subscript_is_not_static_pure(
                    subscript,
                    interpreter,
                    visiting_functions,
                )
            }),
        Expression::Formula(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Expression::FunctionCall(call) => {
            if call.args.iter().any(|(_, expression)| {
                guard_expression_is_not_static_pure(
                    expression,
                    interpreter,
                    visiting_functions,
                )
            }) {
                return true;
            }
            let function_id = call.name.hash();
            let functions = interpreter.functions();
            let functions = functions.borrow();
            let user_function = functions.user_functions.get(&function_id).cloned();
            let has_precompiled_function = functions.functions.contains_key(&function_id);
            let native_guard_safety = functions
                .function_compilers
                .get(&function_id)
                .map(|compiler| compiler.guard_safety());
            drop(functions);
            let Some(user_function) = user_function else {
                if has_precompiled_function {
                    return true;
                }
                return match native_guard_safety {
                    Some(GuardFunctionSafety::PureStatic) | None => false,
                    Some(GuardFunctionSafety::Unsupported) => true,
                };
            };
            if !visiting_functions.insert(function_id) {
                return true;
            }
            let eager = match user_function.code.match_arms.as_slice() {
                [arm] if matches!(arm.pattern, Pattern::Wildcard) => {
                    guard_expression_is_not_static_pure(
                        &arm.expression,
                        interpreter,
                        visiting_functions,
                    )
                }
                _ => true,
            };
            visiting_functions.remove(&function_id);
            eager
        }
        Expression::Match(_)
        | Expression::SetComprehension(_)
        | Expression::MatrixComprehension(_)
        | Expression::FsmPipe(_) => true,
        Expression::Range(range) => {
            guard_range_is_not_static_pure(range, interpreter, visiting_functions)
        }
        Expression::Structure(structure) => {
            guard_structure_is_not_static_pure(structure, interpreter, visiting_functions)
        }
    }
}

fn guard_factor_is_not_static_pure(
    factor: &Factor,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match factor {
        Factor::Expression(expression) => guard_expression_is_not_static_pure(
            expression,
            interpreter,
            visiting_functions,
        ),
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Factor::Term(term) => {
            guard_factor_is_not_static_pure(
                &term.lhs,
                interpreter,
                visiting_functions,
            ) || term.rhs.iter().any(|(_, factor)| {
                guard_factor_is_not_static_pure(
                    factor,
                    interpreter,
                    visiting_functions,
                )
            })
        }
    }
}

fn guard_range_is_not_static_pure(
    range: &RangeExpression,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    guard_factor_is_not_static_pure(&range.start, interpreter, visiting_functions)
        || range
            .increment
            .as_ref()
            .map_or(false, |(_, increment)| {
                guard_factor_is_not_static_pure(
                    increment,
                    interpreter,
                    visiting_functions,
                )
            })
        || guard_factor_is_not_static_pure(
            &range.terminal,
            interpreter,
            visiting_functions,
        )
}

fn guard_subscript_is_not_static_pure(
    subscript: &Subscript,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => subscripts
            .iter()
            .any(|subscript| {
                guard_subscript_is_not_static_pure(
                    subscript,
                    interpreter,
                    visiting_functions,
                )
            }),
        Subscript::Formula(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Subscript::Range(range) => {
            guard_range_is_not_static_pure(range, interpreter, visiting_functions)
        }
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => false,
    }
}

fn guard_structure_is_not_static_pure(
    structure: &Structure,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match structure {
        Structure::Empty => false,
        Structure::Map(map) => map.elements.iter().any(|mapping| {
            guard_expression_is_not_static_pure(
                &mapping.key,
                interpreter,
                visiting_functions,
            ) || guard_expression_is_not_static_pure(
                &mapping.value,
                interpreter,
                visiting_functions,
            )
        }),
        Structure::Matrix(matrix) => matrix.rows.iter().any(|row| {
            row.columns
                .iter()
                .any(|column| {
                    guard_expression_is_not_static_pure(
                        &column.element,
                        interpreter,
                        visiting_functions,
                    )
                })
        }),
        Structure::Record(record) => record
            .bindings
            .iter()
            .any(|binding| {
                guard_expression_is_not_static_pure(
                    &binding.value,
                    interpreter,
                    visiting_functions,
                )
            }),
        Structure::Set(set) => set
            .elements
            .iter()
            .any(|expression| {
                guard_expression_is_not_static_pure(
                    expression,
                    interpreter,
                    visiting_functions,
                )
            }),
        Structure::Table(table) => table.rows.iter().any(|row| {
            row.columns
                .iter()
                .any(|column| {
                    guard_expression_is_not_static_pure(
                        &column.element,
                        interpreter,
                        visiting_functions,
                    )
                })
        }),
        Structure::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|expression| {
                guard_expression_is_not_static_pure(
                    expression,
                    interpreter,
                    visiting_functions,
                )
            }),
        Structure::TupleStruct(tuple) => {
            guard_expression_is_not_static_pure(
                &tuple.value,
                interpreter,
                visiting_functions,
            )
        }
    }
}
fn validate_patterned_pattern(pattern: &Pattern) -> MResult<()> {
    match pattern {
        Pattern::Expression(expression) => validate_patterned_expression(expression),
        Pattern::Tuple(tuple) => {
            for pattern in &tuple.0 {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::TupleStruct(tuple) => {
            for pattern in &tuple.patterns {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::Array(array) => {
            for pattern in array.prefix.iter().chain(&array.suffix) {
                validate_patterned_pattern(pattern)?;
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    validate_patterned_pattern(binding)?;
                }
            }
            Ok(())
        }
        Pattern::Wildcard => Ok(()),
    }
}
fn validate_patterned_factor(factor: &Factor) -> MResult<()> {
    match factor {
        Factor::Expression(expression) => validate_patterned_expression(expression),
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => validate_patterned_factor(factor),
        Factor::Term(term) => {
            validate_patterned_factor(&term.lhs)?;
            for (_, factor) in &term.rhs {
                validate_patterned_factor(factor)?;
            }
            Ok(())
        }
    }
}
fn validate_patterned_range(range: &RangeExpression) -> MResult<()> {
    validate_patterned_factor(&range.start)?;
    if let Some((_, increment)) = &range.increment {
        validate_patterned_factor(increment)?;
    }
    validate_patterned_factor(&range.terminal)
}
fn validate_patterned_slice(slice: &Slice) -> MResult<()> {
    for subscript in &slice.subscript {
        validate_patterned_subscript(subscript)?;
    }
    Ok(())
}
fn validate_patterned_subscript(subscript: &Subscript) -> MResult<()> {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => {
            for subscript in subscripts {
                validate_patterned_subscript(subscript)?;
            }
            Ok(())
        }
        Subscript::Formula(factor) => validate_patterned_factor(factor),
        Subscript::Range(range) => validate_patterned_range(range),
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => Ok(()),
    }
}
fn validate_patterned_structure(structure: &Structure) -> MResult<()> {
    match structure {
        Structure::Empty => Ok(()),
        Structure::Map(map) => {
            for mapping in &map.elements {
                validate_patterned_expression(&mapping.key)?;
                validate_patterned_expression(&mapping.value)?;
            }
            Ok(())
        }
        Structure::Matrix(matrix) => {
            for row in &matrix.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Record(record) => {
            for binding in &record.bindings {
                validate_patterned_expression(&binding.value)?;
            }
            Ok(())
        }
        Structure::Set(set) => {
            for expression in &set.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::Table(table) => {
            for row in &table.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Tuple(tuple) => {
            for expression in &tuple.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::TupleStruct(tuple) => validate_patterned_expression(&tuple.value),
    }
}
fn validate_patterned_qualifier(qualifier: &ComprehensionQualifier) -> MResult<()> {
    match qualifier {
        ComprehensionQualifier::Generator((pattern, expression)) => {
            validate_patterned_pattern(pattern)?;
            validate_patterned_expression(expression)
        }
        ComprehensionQualifier::Filter(expression) => validate_patterned_expression(expression),
        ComprehensionQualifier::Let(definition) if definition.mutable => {
            validation_error(ActivationPatternDefinitionUnsupported, definition.tokens())
        }
        ComprehensionQualifier::Let(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                definition.tokens(),
            )
        }
        ComprehensionQualifier::Let(definition) => {
            validate_patterned_expression(&definition.expression)
        }
    }
}

struct ElaboratedPatternGuard {
    finalizer_node: ReactiveNodeId,
    node_start: usize,
    node_end: usize,
}

fn elaborate_patterned_arm_guard(
    guard: &Expression,
    captures: &[ActivationPatternCapture],
    pulse: &Value,
    eligible: &Ref<bool>,
    completion: Ref<usize>,
    interpreter: &Interpreter,
) -> MResult<ElaboratedPatternGuard> {
    let symbols = interpreter.symbols();
    let symbol_snapshot = symbols.borrow().snapshot();
    let plan = interpreter.plan();
    let original_scope_depth = plan.activation_registration_depth();
    {
        let mut symbols = symbols.borrow_mut();
        for capture in captures {
            symbols.mutable_variables.remove(&capture.id);
            symbols.insert(capture.id, capture.proposed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let node_start = plan.len();
    let pulse_cells = pulse.reactive_root_cell_ids();
    plan.push_activation_registration_scope(pulse_cells.clone());
    let result = (|| -> MResult<ElaboratedPatternGuard> {
        let _deferred_expression_solves =
            crate::expressions::DeferredExpressionSolveScope::enter(interpreter);
        let _persistent_user_function_plan =
            crate::functions::PersistentUserFunctionPlanScope::enter(interpreter);
        let guard_value = crate::expression(guard, None, interpreter)?;
        let guard_ref = crate::expressions::validate_guard_expression_result(
            guard_value.clone(),
            guard.tokens(),
        )?;
        let finalizer_node = plan.register_function(
            Box::new(GuardFinalize {
                guard: guard_ref,
                eligible: eligible.clone(),
                out: completion,
            }),
            &[guard_value],
        )?;
        let node_end = plan.len();
        {
            let plan_borrow = plan.borrow();
            if plan_borrow.nodes[node_start..node_end]
                .iter()
                .any(|node| node.kind != ReactiveNodeKind::Combinational)
            {
                return Err(
                    MechError::new(ActivationPatternGuardMustBePure, None)
                        .with_tokens(guard.tokens()),
                );
            }
        }
        {
            let Some(pulse_cell) = pulse_cells.first().copied() else {
                return Err(
                    MechError::new(ActivationPatternGuardDependencyInvariant, None)
                        .with_tokens(guard.tokens()),
                );
            };
            let mut plan_borrow = plan.borrow_mut();
            for node in node_start..node_end {
                if !plan_borrow.add_reactive_dependency(node, pulse_cell) {
                    return Err(
                        MechError::new(ActivationPatternGuardDependencyInvariant, None)
                            .with_tokens(guard.tokens()),
                    );
                }
                for capture in captures {
                    let capture_cell = capture.proposed.reactive_root_cell_ids()[0];
                    if !plan_borrow.add_sampled_dependency(node, capture_cell) {
                        return Err(
                            MechError::new(ActivationPatternGuardDependencyInvariant, None)
                                .with_tokens(guard.tokens()),
                        );
                    }
                }
            }
        }
        Ok(ElaboratedPatternGuard {
            finalizer_node,
            node_start,
            node_end,
        })
    })();
    while plan.activation_registration_depth() > original_scope_depth {
        plan.pop_activation_registration_scope();
    }
    symbols.borrow_mut().restore(symbol_snapshot);
    result
}

fn elaborate_patterned_arm_body(
    arm: &ActivationArm,
    captures: &[ActivationPatternCapture],
    pulse: &Value,
    interpreter: &Interpreter,
) -> MResult<(usize, usize)> {
    let symbols = interpreter.symbols();
    let symbol_snapshot = symbols.borrow().snapshot();
    let plan = interpreter.plan();
    let original_scope_depth = plan.activation_registration_depth();
    {
        let mut symbols = symbols.borrow_mut();
        for capture in captures {
            symbols.mutable_variables.remove(&capture.id);
            symbols.insert(capture.id, capture.committed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let body_node_start = plan.len();
    plan.push_activation_registration_scope(pulse.reactive_root_cell_ids());
    let body_result = (|| -> MResult<()> {
        match &arm.body {
            ActivationArmBody::Block(body) => {
                for (code, _) in body {
                    crate::mech_code(code, interpreter)?;
                }
                Ok(())
            }
            ActivationArmBody::Expression(expression) => {
                crate::expression(expression, None, interpreter)?;
                Ok(())
            }
        }
    })();
    while plan.activation_registration_depth() > original_scope_depth {
        plan.pop_activation_registration_scope();
    }
    symbols.borrow_mut().restore(symbol_snapshot);
    body_result?;
    let body_node_end = plan.len();
    {
        let mut plan = plan.borrow_mut();
        for node in body_node_start..body_node_end {
            for capture in captures {
                let cell = capture.committed.reactive_root_cell_ids()[0];
                if !plan.add_sampled_dependency(node, cell) {
                    return Err(MechError::new(
                        ActivationPatternBodyDependencyInvariant,
                        None,
                    ));
                }
            }
        }
    }
    Ok((body_node_start, body_node_end))
}

fn elaborate_patterned_activation_inner(
    arms: &[ActivationArm],
    trigger: Value,
    preflight: PreflightPatternedActivation,
    i: &Interpreter,
) -> MResult<Value> {
    if trigger.kind().deref_kind() != preflight.trigger_kind {
        return Err(MechError::new(ActivationPatternTriggerInvariant, None));
    }
    let compiled = preflight.arms;
    let plan = i.plan();
    let _persistent_user_function_plan =
        crate::functions::PersistentUserFunctionPlanScope::enter(i);
    let pattern_expression_values = compiled
        .iter()
        .map(|arm| {
            arm.pattern
                .expressions()
                .iter()
                .map(|expression| crate::expression(expression, None, i))
                .collect::<MResult<Vec<_>>>()
        })
        .collect::<MResult<Vec<_>>>()?;
    drop(_persistent_user_function_plan);
    // Seed proposal storage before guard graphs are elaborated. Composite
    // guard expressions may need the current proposal shape to compile, but
    // eligibility and selection are still determined by the runtime graph
    // initialization below.
    for (arm, expression_values) in compiled.iter().zip(&pattern_expression_values) {
        let pattern_match =
            match_compiled_pattern_with_values(&arm.pattern, &trigger, expression_values)?;
        ReactiveBindingSink {
            captures: &arm.captures,
        }
        .commit(&pattern_match)?;
    }
    let (scope_gen, scope_v) = generation();
    let scope_node = plan
        .borrow_mut()
        .register(Box::new(ScopePulse { out: scope_gen }), &[trigger.clone()])?;
    let (mut matcher_nodes, mut completions, mut matched) = (Vec::new(), Vec::new(), Vec::new());
    for (arm, expression_values) in compiled.iter().zip(&pattern_expression_values) {
        let (o, v) = generation();
        let f = Ref::new(false);
        let mut inputs = Vec::with_capacity(2 + expression_values.len());
        inputs.push(scope_v.clone());
        inputs.push(trigger.clone());
        inputs.extend(expression_values.iter().cloned());
        let n = plan.borrow_mut().register(
            Box::new(Matcher {
                pattern: arm.pattern.clone(),
                trigger: trigger.clone(),
                expression_values: expression_values.clone(),
                captures: arm.captures.clone(),
                matched: f.clone(),
                out: o,
            }),
            &inputs,
        )?;
        matcher_nodes.push(n);
        completions.push(v);
        matched.push(f);
    }
    let (mut finalizers, mut guards, mut eligible, mut done) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for n in 0..arms.len() {
        let e = Ref::new(false);
        if let Some(guard) = &arms[n].guard {
            let (match_gate_out, match_gate_pulse) = generation();
            let match_gate_node = plan.borrow_mut().register(
                Box::new(MatchGate {
                    matched: matched[n].clone(),
                    out: match_gate_out,
                }),
                &[completions[n].clone()],
            )?;
            let (unmatched_out, unmatched_done) = generation();
            let unmatched_finalizer = plan.borrow_mut().register(
                Box::new(UnmatchedFinalize {
                    matched: matched[n].clone(),
                    eligible: e.clone(),
                    out: unmatched_out,
                }),
                &[completions[n].clone()],
            )?;
            let (guard_out, guard_done) = generation();
            let elaborated = elaborate_patterned_arm_guard(
                guard,
                &compiled[n].captures,
                &match_gate_pulse,
                &e,
                guard_out,
                i,
            )?;
            finalizers.push(unmatched_finalizer);
            guards.push(Some(PatternActivationGuardRegistration {
                match_gate_node,
                guard_finalizer_node: elaborated.finalizer_node,
                guard_node_start: elaborated.node_start,
                guard_node_end: elaborated.node_end,
            }));
            done.push(unmatched_done);
            done.push(guard_done);
        } else {
            let (out, arm_done) = generation();
            finalizers.push(plan.borrow_mut().register(
                Box::new(Finalize {
                    matched: matched[n].clone(),
                    eligible: e.clone(),
                    out,
                }),
                &[completions[n].clone()],
            )?);
            guards.push(None);
            done.push(arm_done);
        }
        eligible.push(e);
    }
    let (o, selection) = generation();
    let selected = Ref::new(usize::MAX);
    let selector = plan.borrow_mut().register(
        Box::new(Select {
            eligible: eligible.clone(),
            selected: selected.clone(),
            out: o,
        }),
        &done,
    )?;
    let private_scope_cell = scope_v.reactive_root_cell_ids()[0];
    plan.solve_dirty_cells(&[private_scope_cell])?;
    let initially_selected = *selected.borrow();
    if initially_selected >= compiled.len() {
        return Err(MechError::new(ActivationPatternArmsNonExhaustive, None));
    }
    commit_proposed_captures(&compiled[initially_selected].captures)?;
    let (mut gates, mut pulses) = (Vec::new(), Vec::new());
    for arm in 0..arms.len() {
        let (o, v) = generation();
        gates.push(plan.borrow_mut().register(
            Box::new(Gate {
                arm,
                selected: selected.clone(),
                captures: compiled[arm].captures.clone(),
                out: o,
            }),
            &[selection.clone()],
        )?);
        pulses.push(v);
    }
    let mut ranges = Vec::new();
    for (arm, compiled_arm) in arms.iter().zip(&compiled) {
        ranges.push(elaborate_patterned_arm_body(
            arm,
            &compiled_arm.captures,
            &pulses[ranges.len()],
            i,
        )?);
    }
    let registration = PatternActivationRegistration {
        scope_pulse_node: scope_node,
        selector_node: selector,
        arms: (0..arms.len())
            .map(|n| PatternActivationArmRegistration {
                matcher_node: matcher_nodes[n],
                finalizer_node: finalizers[n],
                guard: guards[n].clone(),
                gate_node: gates[n],
                pulse_cell: pulses[n].reactive_root_cell_ids()[0],
                body_node_start: ranges[n].0,
                body_node_end: ranges[n].1,
                captures: compiled[n]
                    .captures
                    .iter()
                    .map(|c| PatternActivationCaptureRegistration {
                        id: c.id,
                        kind: c.kind.clone(),
                        cell: c.committed.reactive_root_cell_ids()[0],
                    })
                    .collect(),
            })
            .collect(),
    };
    plan.borrow_mut().register_pattern_activation(registration);
    Ok(Value::Empty)
}

pub(crate) fn elaborate_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: Value,
    trigger_cells: Vec<ReactiveCellId>,
    interpreter: &Interpreter,
) -> MResult<Value> {
    let preflight =
        preflight_patterned_activation(scope, arms, &trigger, &trigger_cells, interpreter)?;
    let plan = interpreter.plan();
    let checkpoint = plan.checkpoint();
    let program_dictionary = interpreter.state.borrow().dictionary.clone();
    let dictionary_snapshot = program_dictionary.borrow().clone();
    match elaborate_patterned_activation_inner(arms, trigger, preflight, interpreter) {
        Ok(value) => Ok(value),
        Err(error) => {
            *program_dictionary.borrow_mut() = dictionary_snapshot;
            match plan.rollback(checkpoint) {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rollback_error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct EagerGuardTestCompiler {
        compile_calls: Arc<AtomicUsize>,
    }

    impl NativeFunctionCompiler for EagerGuardTestCompiler {
        fn compile(&self, _arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
            self.compile_calls.fetch_add(1, Ordering::SeqCst);
            panic!("unsupported guard compiler must not run during preflight")
        }
    }

    fn scalar_capture_cases() -> Vec<(ValueKind, Value)> {
        let mut cases = Vec::new();
        #[cfg(feature = "u8")]
        cases.push((ValueKind::U8, Value::U8(Ref::new(8))));
        #[cfg(feature = "u16")]
        cases.push((ValueKind::U16, Value::U16(Ref::new(16))));
        #[cfg(feature = "u32")]
        cases.push((ValueKind::U32, Value::U32(Ref::new(32))));
        #[cfg(feature = "u64")]
        cases.push((ValueKind::U64, Value::U64(Ref::new(64))));
        #[cfg(feature = "u128")]
        cases.push((ValueKind::U128, Value::U128(Ref::new(128))));
        #[cfg(feature = "i8")]
        cases.push((ValueKind::I8, Value::I8(Ref::new(-8))));
        #[cfg(feature = "i16")]
        cases.push((ValueKind::I16, Value::I16(Ref::new(-16))));
        #[cfg(feature = "i32")]
        cases.push((ValueKind::I32, Value::I32(Ref::new(-32))));
        #[cfg(feature = "i64")]
        cases.push((ValueKind::I64, Value::I64(Ref::new(-64))));
        #[cfg(feature = "i128")]
        cases.push((ValueKind::I128, Value::I128(Ref::new(-128))));
        #[cfg(feature = "f32")]
        cases.push((ValueKind::F32, Value::F32(Ref::new(3.25))));
        #[cfg(feature = "f64")]
        cases.push((ValueKind::F64, Value::F64(Ref::new(6.5))));
        #[cfg(feature = "complex")]
        cases.push((ValueKind::C64, Value::C64(Ref::new(C64::new(3.0, 4.0)))));
        #[cfg(feature = "rational")]
        cases.push((ValueKind::R64, Value::R64(Ref::new(R64::new(3, 4)))));
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        cases.push((ValueKind::Bool, Value::Bool(Ref::new(true))));
        #[cfg(any(feature = "string", feature = "variable_define"))]
        cases.push((
            ValueKind::String,
            Value::String(Ref::new("captured".to_string())),
        ));
        cases.push((ValueKind::Index, Value::Index(Ref::new(42))));
        #[cfg(feature = "atom")]
        {
            let atom = MechAtom::from_name("captured");
            cases.push((
                ValueKind::Atom(atom.id(), atom.name()),
                Value::Atom(Ref::new(atom)),
            ));
        }
        cases
    }

    #[test]
    fn activation_capture_slot_supports_all_enabled_scalar_kinds() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        for (kind, source) in scalar_capture_cases() {
            let slot = create_capture_slot_for_kind(&kind, &interpreter).unwrap();
            let cells_before = slot.reactive_root_cell_ids();
            assert_eq!(cells_before.len(), 1);
            commit_capture_slot(&slot, &source).unwrap();
            assert_eq!(slot, source);
            assert_eq!(slot.reactive_root_cell_ids(), cells_before);
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    #[test]
    fn activation_capture_slot_preserves_identity_across_updates() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let slot = create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap();
        let cells = slot.reactive_root_cell_ids();
        commit_capture_slot(&slot, &Value::String(Ref::new("first".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("first".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
        commit_capture_slot(&slot, &Value::String(Ref::new("second".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("second".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
    }

    #[cfg(all(
        feature = "tuple",
        feature = "enum",
        feature = "record",
        feature = "map",
        feature = "set",
        feature = "table",
        feature = "string",
        feature = "f64"
    ))]
    #[test]
    fn activation_capture_slots_support_enabled_composite_value_kinds() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let enum_id = hash_str("capture-enum");
        let variant_id = hash_str("payload");
        let names = Ref::new(HashMap::from([
            (enum_id, "capture-enum".to_string()),
            (variant_id, "payload".to_string()),
        ]));
        let cases = vec![
            Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                Value::F64(Ref::new(1.0)),
                Value::String(Ref::new("tuple".to_string())),
            ]))),
            Value::Enum(Ref::new(MechEnum {
                id: enum_id,
                variants: vec![(variant_id, Some(Value::F64(Ref::new(2.0))))],
                names,
            })),
            Value::Record(Ref::new(MechRecord::new(vec![
                ("field", Value::F64(Ref::new(3.0))),
            ]))),
            Value::Map(Ref::new(MechMap::from_vec(vec![(
                Value::String(Ref::new("key".to_string())),
                Value::F64(Ref::new(4.0)),
            )]))),
            Value::Set(Ref::new(MechSet::from_vec(vec![Value::String(Ref::new(
                "member".to_string(),
            ))]))),
            Value::Table(Ref::new(MechTable::new_table(
                vec!["column".to_string()],
                vec![ValueKind::F64],
                vec![vec![Value::F64(Ref::new(5.0)), Value::F64(Ref::new(6.0))]],
            ))),
        ];

        for source in cases {
            let kind = source.kind();
            let slot = create_capture_slot_for_kind(&kind, &interpreter).unwrap();
            let cells = slot.reactive_root_cell_ids();
            assert_eq!(cells.len(), 1, "missing stable root for {kind}");
            commit_capture_slot(&slot, &source).unwrap();
            assert_eq!(slot, source);
            assert_eq!(slot.reactive_root_cell_ids(), cells);
        }
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn activation_capture_commit_validates_every_binding_before_mutation() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let number = ActivationPatternCapture {
            id: hash_str("number"),
            name: "number".to_string(),
            kind: ValueKind::F64,
            proposed: create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap(),
            committed: create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap(),
        };
        let text = ActivationPatternCapture {
            id: hash_str("text"),
            name: "text".to_string(),
            kind: ValueKind::String,
            proposed: create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap(),
            committed: create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap(),
        };
        let captures = vec![number, text];
        let attempted = PatternMatch {
            matched: true,
            bindings: vec![
                PatternBinding {
                    index: 0,
                    id: hash_str("number"),
                    name: "number".to_string(),
                    kind: ValueKind::F64,
                    value: Value::F64(Ref::new(9.0)),
                },
                PatternBinding {
                    index: 1,
                    id: hash_str("text"),
                    name: "text".to_string(),
                    kind: ValueKind::F64,
                    value: Value::F64(Ref::new(10.0)),
                },
            ],
        };

        let error = ReactiveBindingSink {
            captures: &captures,
        }
        .commit(&attempted)
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(captures[0].proposed, Value::F64(Ref::new(0.0)));
        assert_eq!(captures[1].proposed, Value::String(Ref::new(String::new())));
        assert_eq!(captures[0].committed, Value::F64(Ref::new(0.0)));
        assert_eq!(
            captures[1].committed,
            Value::String(Ref::new(String::new()))
        );
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn activation_capture_gate_validates_entire_commit_before_mutation_or_pulse() {
        let captures = vec![
            ActivationPatternCapture {
                id: hash_str("number"),
                name: "number".to_string(),
                kind: ValueKind::F64,
                proposed: Value::F64(Ref::new(9.0)),
                committed: Value::F64(Ref::new(1.0)),
            },
            ActivationPatternCapture {
                id: hash_str("text"),
                name: "text".to_string(),
                kind: ValueKind::String,
                proposed: Value::F64(Ref::new(10.0)),
                committed: Value::String(Ref::new("before".to_string())),
            },
        ];
        let selected = Ref::new(0);
        let pulse = Ref::new(0);
        let gate = Gate {
            arm: 0,
            selected,
            captures: captures.clone(),
            out: pulse.clone(),
        };

        let error = gate.solve_reactive().unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(captures[0].committed, Value::F64(Ref::new(1.0)));
        assert_eq!(
            captures[1].committed,
            Value::String(Ref::new("before".to_string()))
        );
        assert_eq!(
            *pulse.borrow(),
            0,
            "body pulse must follow a successful commit"
        );
    }

    #[cfg(feature = "atom")]
    #[test]
    fn activation_atom_capture_accepts_a_new_atom_value() {
        let mut interpreter = interpret(
            r#"
event := :first
~> event
  | captured => {
      selected := captured
    }
  | * => {
      selected := :fallback
    }
"#,
        );
        let trigger = root_cell(&interpreter, "event");
        let topology = plan_snapshot(&interpreter);
        let registration = registration(&interpreter);
        let Value::Atom(event) = symbol(&interpreter, "event") else {
            panic!("event is not an atom")
        };
        *event.borrow_mut() = MechAtom::from_name("second");

        let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&registration, &outcome), 0);
        let selected_atom = {
            let plan = interpreter.plan();
            let plan = plan.borrow();
            (registration.arms[0].body_node_start..registration.arms[0].body_node_end)
                .rev()
                .find_map(|node| match detached(&plan.node(node).unwrap().function.out()) {
                    Value::Atom(atom) => Some(atom.borrow().id()),
                    _ => None,
                })
                .expect("no atom output in selected arm body")
        };
        assert_eq!(selected_atom, hash_str("second"));
        assert_eq!(plan_snapshot(&interpreter), topology);
    }

    #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn activation_capture_slot_rejects_kind_mismatch() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let slot = create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap();
        let error =
            commit_capture_slot(&slot, &Value::String(Ref::new("wrong".to_string()))).unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
    }

    type PlanSnapshot = (
        usize,
        Vec<(
            ReactiveNodeId,
            usize,
            ReactiveNodeKind,
            Vec<u64>,
            Vec<(u64, ReactiveDependencyKind)>,
        )>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<PatternActivationRegistration>,
        usize,
    );

    fn interpret(source: &str) -> Interpreter {
        let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
        let mut interpreter = Interpreter::new_with_full_stdlib(0);
        interpreter.interpret(&tree).unwrap();
        interpreter
    }

    fn interpret_more(interpreter: &mut Interpreter, source: &str) -> MResult<Value> {
        let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
        interpreter.interpret(&tree)
    }

    fn symbol_ref(interpreter: &Interpreter, name: &str) -> ValRef {
        interpreter
            .symbols()
            .borrow()
            .get(hash_str(name))
            .unwrap_or_else(|| panic!("missing symbol `{name}`"))
    }
    fn symbol(interpreter: &Interpreter, name: &str) -> Value {
        symbol_ref(interpreter, name).borrow().clone()
    }
    fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId {
        symbol(interpreter, name).reactive_root_cell_ids()[0]
    }
    fn registration(interpreter: &Interpreter) -> PatternActivationRegistration {
        let plan = interpreter.plan();
        let registrations = plan.pattern_activation_registrations();
        assert_eq!(registrations.len(), 1);
        registrations[0].clone()
    }
    fn node_output_for_cell(
        interpreter: &Interpreter,
        node: ReactiveNodeId,
        cell: ReactiveCellId,
    ) -> Value {
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.node(node)
            .expect("missing activation dispatch node")
            .function
            .reactive_output_values()
            .into_iter()
            .find(|value| value.reactive_root_cell_ids().contains(&cell))
            .unwrap_or_else(|| panic!("node {node} does not expose cell {cell:?}"))
    }
    fn committed_capture_value(interpreter: &Interpreter, arm: usize, capture: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        node_output_for_cell(interpreter, arm.gate_node, arm.captures[capture].cell)
    }
    fn proposed_capture_value(interpreter: &Interpreter, arm: usize, capture: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        arm.captures
            .get(capture)
            .expect("missing capture registration");
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.node(arm.matcher_node)
            .expect("missing activation matcher")
            .function
            .reactive_output_values()
            .into_iter()
            .skip(1)
            .nth(capture)
            .expect("missing proposed capture output")
    }
    fn arm_pulse_generation(interpreter: &Interpreter, arm: usize) -> usize {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        let Value::Index(generation) =
            node_output_for_cell(interpreter, arm.gate_node, arm.pulse_cell)
        else {
            panic!("activation arm pulse is not an index")
        };
        let value = *generation.borrow();
        value
    }
    fn plan_snapshot(interpreter: &Interpreter) -> PlanSnapshot {
        let plan = interpreter.plan();
        let depth = plan.activation_registration_depth();
        let plan = plan.borrow();
        let nodes = plan
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id,
                    node.plan_index,
                    node.kind,
                    node.outputs.iter().map(|cell| cell.get()).collect(),
                    node.inputs
                        .iter()
                        .map(|dependency| (dependency.cell.get(), dependency.kind))
                        .collect(),
                )
            })
            .collect();
        let mut reactive = plan
            .reactive_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        reactive.sort_by_key(|(cell, _)| *cell);
        let mut sampled = plan
            .sampled_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        sampled.sort_by_key(|(cell, _)| *cell);
        (
            plan.len(),
            nodes,
            reactive,
            sampled,
            plan.pattern_activation_registrations().to_vec(),
            depth,
        )
    }
    fn turn_executed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .executed_nodes
            .iter()
            .chain(outcome.after_commit.executed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_changed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .changed_nodes
            .iter()
            .chain(outcome.after_commit.changed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_unchanged_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .unchanged_nodes
            .iter()
            .chain(outcome.after_commit.unchanged_nodes.iter())
            .copied()
            .collect()
    }
    fn body_output_f64(interpreter: &Interpreter, arm_index: usize) -> f64 {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm_index];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        for id in (arm.body_node_start..arm.body_node_end).rev() {
            if let Ok(value) = plan.node(id).unwrap().function.out().as_f64() {
                return *value.borrow();
            }
        }
        panic!("no f64 output")
    }
    fn body_output(interpreter: &Interpreter, arm_index: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm_index];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        detached(
            &plan
                .node(arm.body_node_end - 1)
                .expect("missing activation body node")
                .function
                .out(),
        )
    }
    fn set_enum_event(interpreter: &Interpreter, variant: &str, payload: f64) {
        let Value::Enum(event) = symbol(interpreter, "event") else {
            panic!("event is not an enum");
        };
        let enum_id = event.borrow().id;
        let names = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .expect("event enum definition is missing")
            .names
            .clone();
        *event.borrow_mut() = MechEnum {
            id: enum_id,
            variants: vec![(hash_str(variant), Some(Value::F64(Ref::new(payload))))],
            names,
        };
    }
    fn set_unit_enum_event(interpreter: &Interpreter, variant: &str) {
        let event_value = symbol(interpreter, "event");
        if let Value::Atom(event) = &event_value {
            *event.borrow_mut() = MechAtom::from_name(variant);
            return;
        }
        let Value::Enum(event) = event_value else {
            panic!("event is neither an atom nor an enum");
        };
        let enum_id = event.borrow().id;
        let names = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .expect("event enum definition is missing")
            .names
            .clone();
        *event.borrow_mut() = MechEnum {
            id: enum_id,
            variants: vec![(hash_str(variant), None)],
            names,
        };
    }
    fn set_atom_tuple_event(interpreter: &Interpreter, tag: &str, payload: f64) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(vec![
            Value::Atom(Ref::new(MechAtom::from_name(tag))),
            Value::F64(Ref::new(payload)),
        ]);
    }
    fn set_tuple_event(interpreter: &Interpreter, values: Vec<Value>) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(values);
    }
    #[cfg(all(feature = "matrix", feature = "f64"))]
    fn set_f64_matrix_event(interpreter: &Interpreter, values: Vec<f64>) {
        let Value::MatrixF64(event) = symbol(interpreter, "event") else {
            panic!("event is not an f64 matrix")
        };
        event.set(values);
    }
    fn selected_arm_index(
        registration: &PatternActivationRegistration,
        outcome: &ReactiveTurnOutcome,
    ) -> usize {
        let changed = turn_changed_nodes(outcome);
        registration
            .arms
            .iter()
            .position(|arm| changed.contains(&arm.gate_node))
            .expect("no selected gate")
    }
    fn assert_dispatch_turn(
        interpreter: &Interpreter,
        topology: &PlanSnapshot,
        outcome: &ReactiveTurnOutcome,
        expected_arm: usize,
        output: f64,
    ) {
        let registration = registration(interpreter);
        let executed = turn_executed_nodes(outcome);
        let changed = turn_changed_nodes(outcome);
        let unchanged = turn_unchanged_nodes(outcome);
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.scope_pulse_node)
                .count(),
            1
        );
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.selector_node)
                .count(),
            1
        );
        assert_eq!(selected_arm_index(&registration, outcome), expected_arm);
        for (index, arm) in registration.arms.iter().enumerate() {
            for node in [arm.matcher_node, arm.finalizer_node, arm.gate_node] {
                assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
            }
            if index == expected_arm {
                assert!(changed.contains(&arm.gate_node));
                assert!(!unchanged.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
                }
            } else {
                assert!(unchanged.contains(&arm.gate_node));
                assert!(!changed.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert!(!executed.contains(&node));
                }
            }
        }
        assert_eq!(body_output_f64(interpreter, expected_arm), output);
        assert_eq!(&plan_snapshot(interpreter), topology);
    }

    const ENUM_ACTIVATION: &str = r#"
<event-kind> := :pressed<f64>
  | :released<f64>
  | :other<f64>

event<event-kind> := :pressed(0.0)

~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;
    fn load_enum_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let interpreter = interpret(ENUM_ACTIVATION);
        assert!(matches!(symbol(&interpreter, "event"), Value::Enum(_)));
        let enum_id = match symbol(&interpreter, "event") {
            Value::Enum(event) => event.borrow().id,
            value => panic!("expected enum event, found {:?}", value.kind()),
        };
        let enum_definition = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .cloned()
            .expect("event enum definition is missing");
        for variant in ["pressed", "released", "other"] {
            assert!(
                enum_definition
                    .variants
                    .iter()
                    .any(|(variant_id, _)| *variant_id == hash_str(variant)),
                "missing enum variant `{variant}`"
            );
        }
        let trigger = root_cell(&interpreter, "event");
        let registration = registration(&interpreter);
        assert_eq!(registration.arms.len(), 3);
        assert_eq!(registration.arms[0].captures.len(), 1);
        assert_eq!(registration.arms[1].captures.len(), 1);
        assert_eq!(registration.arms[0].captures[0].kind, ValueKind::F64);
        assert_eq!(registration.arms[1].captures[0].kind, ValueKind::F64);
        assert!(registration.arms[2].captures.is_empty());
        assert!(!interpreter.symbols().borrow().contains(hash_str("x")));
        assert!(
            !interpreter
                .symbols()
                .borrow()
                .contains(hash_str("selected"))
        );
        let topology = plan_snapshot(&interpreter);
        (interpreter, trigger, registration, topology)
    }

    #[test]
    fn activation_pattern_selects_pressed_released_and_wildcard() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        for (name, payload, arm, output) in [
            ("pressed", 10., 0, 10.),
            ("released", 20., 1, 1020.),
            ("other", 30., 2, -1.),
        ] {
            set_enum_event(&i, name, payload);
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, arm, output);
        }
    }
    #[test]
    fn activation_pattern_enum_arms_compile_independent_of_initial_variant() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
        set_enum_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    }
    #[test]
    fn activation_pattern_enum_payload_capture_is_available() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let cell = r.arms[0].captures[0].cell;
        assert!(
            i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
                .iter()
                .any(|n| n.inputs.iter().any(|d| d.cell == cell))
        );
        set_enum_event(&i, "pressed", 10.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 10.);
    }
    #[test]
    fn activation_pattern_equal_packets_dispatch_repeatedly() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        set_enum_event(&i, "pressed", 30.);
        for _ in 0..2 {
            let o = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &o, 0, 30.);
        }
    }
    #[test]
    fn activation_pattern_unselected_arm_nodes_do_not_execute() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        set_enum_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
        let executed = turn_executed_nodes(&o);
        for arm in [&r.arms[0], &r.arms[2]] {
            for node in arm.body_node_start..arm.body_node_end {
                assert!(!executed.contains(&node));
            }
        }
    }
    #[test]
    fn activation_only_selected_arm_commits_matching_captures() {
        let mut i = interpret(
            r#"
event := 1.0
~> event
  | first => {
      selected := first
    }
  | later => {
      rejected := later
    }
  | * => {
      fallback := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let later_before = committed_capture_value(&i, 1, 0);
        let committed_cell = activation.arms[0].captures[0].cell;
        let proposed_cell = proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0];
        assert_ne!(proposed_cell, committed_cell);
        {
            let plan = i.plan();
            let plan = plan.borrow();
            assert!(
                plan.nodes[activation.arms[0].matcher_node]
                    .outputs
                    .contains(&proposed_cell)
            );
            assert!(
                !plan.nodes[activation.arms[0].matcher_node]
                    .outputs
                    .contains(&committed_cell)
            );
            assert!(
                plan.nodes[activation.arms[0].gate_node]
                    .outputs
                    .contains(&committed_cell)
            );
            let body_inputs = plan.nodes
                [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
                .iter()
                .flat_map(|node| node.inputs.iter().map(|dependency| dependency.cell))
                .collect::<Vec<_>>();
            assert!(body_inputs.contains(&committed_cell));
            assert!(!body_inputs.contains(&proposed_cell));
        }
        let Value::F64(event) = symbol(&i, "event") else {
            panic!("event is not f64")
        };
        *event.borrow_mut() = 5.0;

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 0);
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), later_before);
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[1].body_node_start..activation.arms[1].body_node_end {
            assert!(!executed.contains(&node));
        }
    }
    #[test]
    fn activation_failed_repeated_binding_leaves_proposed_and_committed_unchanged() {
        let mut i = interpret(
            r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let proposed_before = proposed_capture_value(&i, 0, 0);
        let committed_before = committed_capture_value(&i, 0, 0);
        set_tuple_event(
            &i,
            vec![Value::F64(Ref::new(2.0)), Value::F64(Ref::new(3.0))],
        );

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 1);
        assert_eq!(proposed_capture_value(&i, 0, 0), proposed_before);
        assert_eq!(committed_capture_value(&i, 0, 0), committed_before);
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[0].body_node_start..activation.arms[0].body_node_end {
            assert!(!executed.contains(&node));
        }
    }
    #[test]
    fn activation_non_selected_composite_capture_keeps_last_committed_value() {
        let mut i = interpret(
            r#"
event := (0.0, 1.0)
~> event
  | (1.0, x) => {
      selected := x
    }
  | later => {
      rejected := later.2
    }
  | * => {
      fallback := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let committed_before = committed_capture_value(&i, 1, 0);
        set_tuple_event(
            &i,
            vec![Value::F64(Ref::new(1.0)), Value::F64(Ref::new(10.0))],
        );

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 0);
        assert_eq!(committed_capture_value(&i, 1, 0), committed_before);
        assert_eq!(
            proposed_capture_value(&i, 1, 0),
            Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                Value::F64(Ref::new(1.0)),
                Value::F64(Ref::new(10.0)),
            ])))
        );
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[1].body_node_start..activation.arms[1].body_node_end {
            assert!(!executed.contains(&node));
        }
    }
    #[test]
    fn activation_pattern_switching_arms_does_not_grow_plan() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        for (name, payload) in [
            ("pressed", 10.),
            ("released", 20.),
            ("other", 30.),
            ("pressed", 30.),
            ("pressed", 30.),
        ] {
            set_enum_event(&i, name, payload);
            i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    #[test]
    fn activation_pattern_matches_payload_free_enum_variant() {
        let mut i = interpret(
            r#"
<signal> := :ready | :other
event<signal> := :other
~> event
  | :ready => {
      selected := 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        set_unit_enum_event(&i, "ready");
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
    }
    #[test]
    fn activation_pattern_capture_storage_identity_is_stable() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let captures = r
            .arms
            .iter()
            .flat_map(|arm| arm.captures.iter())
            .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
            .collect::<Vec<_>>();
        for (name, payload) in [("pressed", 10.), ("released", 20.), ("other", 30.)] {
            set_enum_event(&i, name, payload);
            i.advance_reactive_turn(&[trigger]).unwrap();
            let current = registration(&i)
                .arms
                .iter()
                .flat_map(|arm| arm.captures.iter())
                .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
                .collect::<Vec<_>>();
            assert_eq!(current, captures);
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    const ATOM_TUPLE_ACTIVATION: &str = r#"
event := (:pressed, 0.0)
~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;
    fn load_atom_tuple_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let i = interpret(ATOM_TUPLE_ACTIVATION);
        let trigger = root_cell(&i, "event");
        let r = registration(&i);
        let topology = plan_snapshot(&i);
        (i, trigger, r, topology)
    }
    #[test]
    fn activation_pattern_atom_tagged_tuple_selects_arm() {
        let (mut i, trigger, _, topology) = load_atom_tuple_activation();
        for (tag, payload, arm, output) in [
            ("pressed", 10., 0, 10.),
            ("released", 20., 1, 1020.),
            ("other", 30., 2, -1.),
        ] {
            set_atom_tuple_event(&i, tag, payload);
            let o = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &o, arm, output);
        }
    }
    #[test]
    fn activation_pattern_atom_tagged_tuple_captures_payload() {
        let (mut i, trigger, r, topology) = load_atom_tuple_activation();
        assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
        let cell = r.arms[0].captures[0].cell;
        assert!(
            i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
                .iter()
                .any(|n| n.inputs.iter().any(|d| d.cell == cell))
        );
        set_atom_tuple_event(&i, "pressed", 10.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 10.);
    }
    #[test]
    fn activation_pattern_atom_tuple_arms_compile_independent_of_initial_tag() {
        let (mut i, trigger, r, topology) = load_atom_tuple_activation();
        assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
        assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
        set_atom_tuple_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    }

    const FLAT_TUPLE_ACTIVATION: &str = r#"
event := (1.0, 2.0)
~> event
  | (x, y) => {
      selected := x * 10.0 + y
    }
  | * => {
      selected := -1.0
    }
"#;
    const NESTED_TUPLE_ACTIVATION: &str = r#"
event := ((1.0, 2.0), 3.0)
~> event
  | ((x, y), z) => {
      selected := x * 100.0 + y * 10.0 + z
    }
  | * => {
      selected := -1.0
    }
"#;
    const REPEATED_CAPTURE_ACTIVATION: &str = r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#;
    fn tuple_fixture(source: &str) -> (Interpreter, ReactiveCellId, PlanSnapshot) {
        let i = interpret(source);
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        (i, trigger, topology)
    }
    #[test]
    fn activation_pattern_tuple_captures_elements() {
        let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 34.);
    }
    #[test]
    fn activation_pattern_nested_tuple_captures_elements() {
        let (mut i, trigger, topology) = tuple_fixture(NESTED_TUPLE_ACTIVATION);
        set_tuple_event(
            &i,
            vec![
                Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                    Value::F64(Ref::new(4.)),
                    Value::F64(Ref::new(5.)),
                ]))),
                Value::F64(Ref::new(6.)),
            ],
        );
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 456.);
    }
    #[test]
    fn activation_pattern_repeated_capture_requires_equal_values() {
        let (mut i, trigger, topology) = tuple_fixture(REPEATED_CAPTURE_ACTIVATION);
        set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(2.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 2.);
        set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(3.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, -1.);
    }

    #[test]
    fn activation_guards_fall_through_in_source_order_and_commit_only_the_selected_arm() {
        let mut i = interpret(
            r#"
event := 0.0
~> event
  | first, first > 10.0 => {
      selected := first + 100.0
    }
  | second, second > 0.0 => {
      selected := second + 200.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        let activation = registration(&i);
        assert!(activation.arms[0].guard.is_some());
        assert!(activation.arms[1].guard.is_some());
        assert!(activation.arms[2].guard.is_none());

        let Value::F64(event) = symbol(&i, "event") else {
            panic!("event is not f64")
        };
        *event.borrow_mut() = 20.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 120.0);
        let changed = turn_changed_nodes(&outcome);
        let unchanged = turn_unchanged_nodes(&outcome);
        for arm in &activation.arms[..2] {
            let guard = arm.guard.as_ref().unwrap();
            assert!(changed.contains(&guard.match_gate_node));
            assert!(changed.contains(&guard.guard_finalizer_node));
            assert!(unchanged.contains(&arm.finalizer_node));
        }
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(0.0)));

        *event.borrow_mut() = 5.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, 205.0);
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));

        *event.borrow_mut() = -5.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 2, -1.0);
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(-5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(-5.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
    }

    #[test]
    fn activation_guard_outer_dependencies_are_sampled_until_the_next_trigger() {
        let mut i = interpret(
            r#"
event := (:pressed, 0.0)
threshold := 10.0
~> event
  | :pressed(x), x > threshold => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        let activation = registration(&i);
        let guard = activation.arms[0].guard.as_ref().unwrap();
        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;

        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let executed = turn_executed_nodes(&dependency_turn);
        assert!(!executed.contains(&activation.scope_pulse_node));
        assert!(!executed.contains(&guard.match_gate_node));
        assert!(!executed.contains(&guard.guard_finalizer_node));
        for node in guard.guard_node_start..guard.guard_node_end {
            assert!(!executed.contains(&node));
        }
        assert_eq!(plan_snapshot(&i), topology);

        set_atom_tuple_event(&i, "pressed", 5.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 5.0);
    }

    #[test]
    fn activation_guard_user_function_refreshes_on_each_matching_trigger() {
        let mut i = interpret(
            r#"
passes(value<f64>, limit<f64>) => <bool>
  | value > limit.

event := (:pressed, 0.0)
threshold := 5.0
~> event
  | :pressed(x), passes(x, threshold) => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        {
            let functions = i.functions();
            let functions = functions.borrow();
            let passes = functions.user_functions.get(&hash_str("passes")).unwrap();
            assert_eq!(passes.code.match_arms.len(), 1);
            assert!(matches!(passes.code.match_arms[0].pattern, Pattern::Wildcard));
        }

        set_atom_tuple_event(&i, "pressed", 6.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 6.0);

        set_atom_tuple_event(&i, "pressed", 4.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        assert!(turn_executed_nodes(&dependency_turn).is_empty());

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 4.0);
    }

    #[test]
    fn activation_guard_initialization_commits_the_first_eligible_arm_without_pulsing_a_body() {
        let i = interpret(
            r#"
event := 5.0
~> event
  | first, first > 10.0 => {
      selected := first + 100.0
    }
  | second, second > 0.0 => {
      selected := second + 200.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );

        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(0.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(body_output_f64(&i, 1), 205.0);
        for arm in 0..3 {
            assert_eq!(arm_pulse_generation(&i, arm), 0);
        }
    }

    #[test]
    fn activation_guard_equal_packets_dispatch_again_without_changing_topology() {
        let mut i = interpret(
            r#"
event := 20.0
~> event
  | value, value > 10.0 => {
      selected := value + 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        for expected_pulse in [1usize, 2] {
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, 0, 21.0);
            let guard = registration(&i).arms[0].guard.clone().unwrap();
            let executed = turn_executed_nodes(&outcome);
            for guard_node in guard.guard_node_start..guard.guard_node_end {
                assert_eq!(
                    executed.iter().filter(|node| **node == guard_node).count(),
                    1
                );
            }
            assert_eq!(arm_pulse_generation(&i, 0), expected_pulse);
        }
    }

    #[test]
    fn activation_guard_non_boolean_result_rolls_back_plan_and_symbols() {
        let mut i = interpret("event := 1.0\nouter := 9.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | x, x + 1.0 => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "InvalidGuardExpression");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        assert_eq!(i.plan().activation_registration_depth(), 0);
    }

    #[test]
    fn activation_guard_rejects_unclassified_native_compiler_before_compile() {
        let mut i = interpret("event := (:released, 1.0)");
        let compile_calls = Arc::new(AtomicUsize::new(0));
        i.functions().borrow_mut().insert_function_compiler(
            "test/eager-guard",
            Arc::new(EagerGuardTestCompiler {
                compile_calls: compile_calls.clone(),
            }),
        );
        let symbols = i.symbols().borrow().snapshot();
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | :pressed(x), test/eager-guard(x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(compile_calls.load(Ordering::SeqCst), 0);
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(plan_snapshot(&i), topology);
        assert!(i.plan().pattern_activation_registrations().is_empty());
        assert_eq!(i.plan().activation_registration_depth(), 0);
    }

    #[test]
    fn activation_guard_rejects_eager_nested_match_control_flow() {
        let mut i = interpret("event := 1.0");
        let symbols = i.symbols().borrow().snapshot();
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | x, (x? | 0.0 => false | * => true.) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(plan_snapshot(&i), topology);
    }

    #[test]
    fn activation_guard_rejects_user_function_pattern_dispatch_that_cannot_refresh_statically() {
        let mut i = interpret(
            r#"
passes(value<f64>) => <bool>
  | 0.0 => false
  | * => true.

event := 1.0
"#,
        );
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | x, passes(x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(plan_snapshot(&i), topology);
    }

    #[test]
    fn activation_unmatched_guard_skips_runtime_error_and_guard_error_commits_nothing() {
        let mut i = interpret(
            r#"
event := (:pressed, 1.0)
text := "abc"
~index := 1.0
~> event
  | :pressed(x), text[index] == "a" => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);

        set_atom_tuple_event(&i, "pressed", 2.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 2.0);
        let committed_before = committed_capture_value(&i, 0, 0);
        let pulse_before = arm_pulse_generation(&i, 0);
        let body_before = body_output_f64(&i, 0);

        let Value::F64(index) = symbol(&i, "index") else {
            panic!("index is not f64")
        };
        *index.borrow_mut() = 4.0;
        set_atom_tuple_event(&i, "other", 3.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
        let guard = registration(&i).arms[0].guard.clone().unwrap();
        let executed = turn_executed_nodes(&outcome);
        let changed = turn_changed_nodes(&outcome);
        let unchanged = turn_unchanged_nodes(&outcome);
        assert!(executed.contains(&guard.match_gate_node));
        assert!(unchanged.contains(&guard.match_gate_node));
        assert!(changed.contains(&registration(&i).arms[0].finalizer_node));
        for node in guard.guard_node_start..guard.guard_node_end {
            assert!(!executed.contains(&node));
        }

        let pulses_before_error = (0..2)
            .map(|arm| arm_pulse_generation(&i, arm))
            .collect::<Vec<_>>();
        let bodies_before_error = (0..2)
            .map(|arm| body_output_f64(&i, arm))
            .collect::<Vec<_>>();
        set_atom_tuple_event(&i, "pressed", 3.0);
        let error = i.advance_reactive_turn(&[trigger]).unwrap_err();
        assert_eq!(error.kind_name(), "IndexOutOfBounds");
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(3.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), committed_before);
        assert_eq!(arm_pulse_generation(&i, 0), pulse_before);
        assert_eq!(body_output_f64(&i, 0), body_before);
        assert_eq!(
            (0..2)
                .map(|arm| arm_pulse_generation(&i, arm))
                .collect::<Vec<_>>(),
            pulses_before_error
        );
        assert_eq!(
            (0..2)
                .map(|arm| body_output_f64(&i, arm))
                .collect::<Vec<_>>(),
            bodies_before_error
        );
        assert_eq!(plan_snapshot(&i), topology);

        *index.borrow_mut() = 1.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 3.0);
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(3.0)));
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_pattern_expression_uses_outer_symbol_when_capture_name_collides() {
        let mut i = interpret(
            r#"
x := 9.0
event := [1.0 10.0]

~> event
  | [x, x + 1.0] => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let outer_cell = root_cell(&i, "x");
        let activation = registration(&i);
        let proposed_cell = proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0];
        let committed_cell = activation.arms[0].captures[0].cell;
        assert_ne!(proposed_cell, committed_cell);
        assert_ne!(outer_cell, proposed_cell);
        assert_ne!(outer_cell, committed_cell);
        let topology = plan_snapshot(&i);

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
        assert_eq!(symbol(&i, "x"), Value::F64(Ref::new(9.0)));
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(
            proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0],
            proposed_cell
        );
        assert_eq!(registration(&i).arms[0].captures[0].cell, committed_cell);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_guard_capture_shadows_outer_name_while_pattern_expression_keeps_outer_name() {
        let mut i = interpret(
            r#"
x := 9.0
event := [1.0 10.0]

~> event
  | [x, x + 1.0], x < 2.0 => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let outer_cell = root_cell(&i, "x");
        let activation = registration(&i);
        let proposed_cell = proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0];
        let committed_cell = activation.arms[0].captures[0].cell;
        assert_ne!(outer_cell, proposed_cell);
        assert_ne!(outer_cell, committed_cell);
        assert_ne!(proposed_cell, committed_cell);
        let guard = activation.arms[0].guard.as_ref().unwrap();
        {
            let plan = i.plan();
            let plan = plan.borrow();
            let guard_pulse_cell = plan.node(guard.match_gate_node).unwrap().outputs[0];
            for node in &plan.nodes[guard.guard_node_start..guard.guard_node_end] {
                assert!(node.inputs.iter().any(|dependency| {
                    dependency.cell == guard_pulse_cell
                        && dependency.kind == ReactiveDependencyKind::Reactive
                }));
                assert!(node.inputs.iter().any(|dependency| {
                    dependency.cell == proposed_cell
                        && dependency.kind == ReactiveDependencyKind::Sampled
                }));
                assert!(!node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == committed_cell));
            }
        }
        let topology = plan_snapshot(&i);

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
        assert_eq!(symbol(&i, "x"), Value::F64(Ref::new(9.0)));
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_guard_composite_rest_proposal_commits_only_when_the_guard_passes() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0]
~> event
  | [head | rest], head < rest[1] && rest[2] > rest[1] => {
      selected := head + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);

        set_f64_matrix_event(&i, vec![4.0, 5.0, 6.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&registration(&i), &outcome), 0);
        assert_eq!(body_output_f64(&i, 0), 4.0);
        assert_eq!(plan_snapshot(&i), topology);
        let committed_before = committed_capture_value(&i, 0, 1);
        let pulse_before = arm_pulse_generation(&i, 0);

        set_f64_matrix_event(&i, vec![7.0, 6.0, 5.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
        let Value::MatrixF64(proposed_rest) = proposed_capture_value(&i, 0, 1) else {
            panic!("proposed rest is not an f64 matrix")
        };
        assert_eq!(proposed_rest.as_vec(), vec![6.0, 5.0]);
        assert_eq!(committed_capture_value(&i, 0, 1), committed_before);
        assert_eq!(arm_pulse_generation(&i, 0), pulse_before);
        assert_eq!(body_output_f64(&i, 0), 4.0);
        assert_eq!(plan_snapshot(&i), topology);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_pattern_samples_expression_only_on_trigger() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 1.0]
threshold := 2.0
~> event
  | [x, threshold + 0.0, x] => {
      selected := x + 100.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        let registration = registration(&i);

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let dependency_nodes = turn_executed_nodes(&dependency_turn);
        assert!(!dependency_nodes.contains(&registration.scope_pulse_node));
        assert!(!dependency_nodes.contains(&registration.selector_node));
        for arm in &registration.arms {
            assert!(!dependency_nodes.contains(&arm.matcher_node));
            assert!(!dependency_nodes.contains(&arm.finalizer_node));
            assert!(!dependency_nodes.contains(&arm.gate_node));
        }

        set_f64_matrix_event(&i, vec![4.0, 3.0, 4.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 104.0);

        set_f64_matrix_event(&i, vec![4.0, 3.0, 5.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_pattern_samples_current_user_function_output_on_trigger() {
        let mut i = interpret(
            r#"
sample(value<f64>) => <f64>
  | value + 0.0.

event := [1.0 2.0 1.0]
threshold := 2.0
~> event
  | [x, sample(threshold), x] => {
      selected := x + 100.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        let activation = registration(&i);

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let dependency_nodes = turn_executed_nodes(&dependency_turn);
        assert!(!dependency_nodes.contains(&activation.scope_pulse_node));
        assert!(!dependency_nodes.contains(&activation.selector_node));
        for arm in &activation.arms {
            assert!(!dependency_nodes.contains(&arm.matcher_node));
            assert!(!dependency_nodes.contains(&arm.finalizer_node));
            assert!(!dependency_nodes.contains(&arm.gate_node));
        }

        set_f64_matrix_event(&i, vec![4.0, 3.0, 4.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 104.0);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_pattern_supports_prefix_suffix_and_anonymous_spread() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0 1.0]
~> event
  | [x, ..., x] => {
      selected := x + 10.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 11.0);

        set_f64_matrix_event(&i, vec![1.0, 2.0, 3.0, 4.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_rest_segment_accepts_nested_array_pattern() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0 4.0]
~> event
  | [head | [second, ..., last]] => {
      selected := head * 100.0 + second * 10.0 + last
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 124.0);
    }

    #[cfg(feature = "u64")]
    #[test]
    fn activation_typed_literal_pattern_uses_shared_value_matching() {
        let mut i = interpret(
            r#"
event := 1u64
~> event
  | 1u64 => {
      selected := 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
    }

    #[test]
    fn activation_whole_composite_capture_is_stable_and_visible_to_the_body() {
        let mut i = interpret(
            r#"
event := (1.0, 2.0)
~> event
  | whole => {
      selected := whole
    }
  | * => {
      selected := (-1.0, -1.0)
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let capture = &activation.arms[0].captures[0];
        assert_eq!(capture.kind, ValueKind::Tuple(vec![ValueKind::F64, ValueKind::F64]));
        let body_inputs = i.plan().borrow().nodes
            [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
            .iter()
            .flat_map(|node| node.inputs.iter().map(|dependency| dependency.cell))
            .collect::<Vec<_>>();
        assert!(
            body_inputs.contains(&capture.cell),
            "capture cell {:?} is absent from body inputs {:?}",
            capture.cell,
            body_inputs
        );
        let topology = plan_snapshot(&i);
        for values in [[3.0, 4.0], [5.0, 6.0]] {
            set_tuple_event(
                &i,
                values
                    .into_iter()
                    .map(|value| Value::F64(Ref::new(value)))
                    .collect(),
            );
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(selected_arm_index(&activation, &outcome), 0);
            assert_eq!(
                body_output(&i, 0),
                Value::Tuple(Ref::new(MechTuple::from_vec(
                    values
                        .into_iter()
                        .map(|value| Value::F64(Ref::new(value)))
                        .collect(),
                )))
            );
            assert_eq!(registration(&i).arms[0].captures[0].cell, capture.cell);
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    #[test]
    fn activation_whole_tuple_capture_keeps_element_access_attached() {
        let mut i = interpret(
            r#"
event := (1.0, 2.0)
~> event
  | whole => {
      selected := whole.1 * 10.0 + whole.2
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        for (values, expected) in [([3.0, 4.0], 34.0), ([5.0, 6.0], 56.0)] {
            set_tuple_event(
                &i,
                values
                    .into_iter()
                    .map(|value| Value::F64(Ref::new(value)))
                    .collect(),
            );
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, 0, expected);
        }
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_rest_capture_preserves_kind_payload_and_identity() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0 4.0 5.0]
~> event
  | [head | rest] => {
      selected := rest
    }
  | * => {
      selected := [-1.0]
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let rest_capture = &activation.arms[0].captures[1];
        assert_eq!(
            rest_capture.kind,
            ValueKind::Matrix(Box::new(ValueKind::F64), Vec::new())
        );
        assert!(
            i.plan().borrow().nodes
                [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
                .iter()
                .any(|node| node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == rest_capture.cell))
        );
        let topology = plan_snapshot(&i);
        let Value::MatrixF64(event) = symbol(&i, "event") else {
            panic!("event is not an f64 matrix")
        };
        for values in [
            vec![10.0, 20.0, 30.0, 40.0, 50.0],
            vec![11.0, 21.0, 31.0, 41.0, 51.0, 61.0],
        ] {
            let source = Matrix::from_vec(values.clone(), 1, values.len());
            assert!(event.replace_payload_from(&source));
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(selected_arm_index(&activation, &outcome), 0);
            let Value::MatrixF64(rest) = body_output(&i, 0) else {
                panic!("rest output is not an f64 matrix")
            };
            assert_eq!(rest.shape(), vec![1, values.len() - 1]);
            assert_eq!(rest.as_vec(), values[1..]);
            assert_eq!(
                registration(&i).arms[0].captures[1].cell,
                rest_capture.cell
            );
            assert_eq!(plan_snapshot(&i), topology);
        }
    }
    #[test]
    fn activation_pattern_repeated_capture_kind_mismatch_uses_canonical_error() {
        let mut i = interpret("event := (1.0, \"one\")");
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, x) => {
      selected := x
    }\n  | * => {
      selected := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "PatternCompileError");
        assert_eq!(plan_snapshot(&i), topology);
        assert!(!i.symbols().borrow().contains(hash_str("x")));
        assert!(!i.symbols().borrow().contains(hash_str("selected")));
    }

    #[test]
    fn activation_pattern_capture_does_not_leak() {
        let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
        for name in ["x", "y", "selected"] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 34.);
    }
    #[test]
    fn activation_pattern_capture_shadows_and_restores_outer_symbol() {
        let mut i = interpret("event := (1.0, 2.0)\nx := 99.0");
        let outer = symbol_ref(&i, "x");
        let address = outer.addr();
        interpret_more(
            &mut i,
            "~> event\n  | (x, y) => {
      selected := x + y
    }\n  | * => {
      selected := -1.0
    }",
        )
        .unwrap();
        assert_eq!(*symbol(&i, "x").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "x").addr(), address);
        assert!(!i.symbols().borrow().contains(hash_str("y")));
        assert!(!i.symbols().borrow().contains(hash_str("selected")));
        let topology = plan_snapshot(&i);
        let trigger = root_cell(&i, "event");
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 7.);
    }
    #[test]
    fn activation_pattern_arm_definitions_do_not_leak_between_arms() {
        let mut i = interpret("event := (1.0, 2.0)");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, y) => {
      first-local := x + y
    }\n  | (a, b) => {
      second-local := first-local + a + b
    }\n  | * => {
      fallback := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "UndefinedVariable");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        for name in [
            "first-local",
            "second-local",
            "fallback",
            "x",
            "y",
            "a",
            "b",
        ] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
    }

    fn failed_elaboration_fixture() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        ValRef,
        usize,
    ) {
        let i = interpret("event := (1.0, 2.0)\nouter := 99.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let outer = symbol_ref(&i, "outer");
        let address = outer.addr();
        (i, symbols, dictionary, topology, outer, address)
    }
    fn assert_failed_elaboration_restored() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        usize,
    ) {
        let (mut i, symbols, dictionary, topology, outer, address) = failed_elaboration_fixture();
        let error=interpret_more(&mut i,"~> event\n  | (x, y) => {\n      local-atom := :temporary\n      local-first := x + y\n      local-failure := function-that-does-not-exist(local-first)\n    }\n  | * => {
      fallback := 0.0
    }").unwrap_err();
        assert!(error.kind_name().contains("Function"));
        assert!(!i.dictionary().borrow().contains_key(&hash_str("temporary")));
        for name in [
            "local-atom",
            "local-first",
            "local-failure",
            "fallback",
            "x",
            "y",
        ] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        assert_eq!(*symbol(&i, "outer").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "outer").addr(), address);
        drop(outer);
        (i, symbols, dictionary, topology, address)
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_symbol_table() {
        let (i, symbols, _, _, _) = assert_failed_elaboration_restored();
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_program_dictionary() {
        let (i, _, dictionary, _, _) = assert_failed_elaboration_restored();
        assert_eq!(*i.dictionary().borrow(), dictionary);
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_plan() {
        let (i, _, _, topology, _) = assert_failed_elaboration_restored();
        assert_eq!(plan_snapshot(&i), topology);
    }
    #[test]
    fn activation_pattern_preflight_error_does_not_modify_plan() {
        let mut i = interpret("event := (1.0, \"one\")");
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, x) => {
      selected := x
    }\n  | * => {
      selected := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "PatternCompileError");
        assert_eq!(plan_snapshot(&i), topology);
    }
    #[test]
    fn activation_pattern_recursive_preflight_rejects_nested_activation() {
        let mut i = interpret("event := 1.0\ntick := 0.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | 1.0 => {\n      ~> tick {\n        nested := 1.0\n      }\n    }\n  | * => {\n      fallback := 0.0\n    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        assert!(!i.symbols().borrow().contains(hash_str("nested")));
        assert!(!i.symbols().borrow().contains(hash_str("fallback")));
    }
    #[test]
    fn activation_pattern_recursive_preflight_rejects_context_declaration() {
        let mut i = interpret("event := 1.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let context_bindings = i.context_bindings.borrow().clone();
        let error = interpret_more(
            &mut i,
            "~> event\n  | 1.0 => {
      @temporary := test://resource
    }\n  | * => {
      fallback := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        assert_eq!(*i.context_bindings.borrow(), context_bindings);
        assert!(
            !i.context_bindings
                .borrow()
                .contains_key(&hash_str("temporary"))
        );
        assert!(i.plan().pattern_activation_registrations().is_empty());
        assert!(!i.symbols().borrow().contains(hash_str("fallback")));
    }
}
