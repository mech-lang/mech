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
    "Patterned activations require a final unguarded irrefutable arm."
);
activation_error!(
    ActivationPatternWildcardMustBeLast,
    "An unguarded wildcard activation arm must be last."
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
    "Patterned activation register writes must target a whole local register."
);
activation_error!(
    ActivationScopeTriggerWriteUnsupported,
    "An activation scope cannot assign to its own trigger."
);
activation_error!(
    ActivationPatternContextEffectUnsupported,
    "Patterned activation context effects are not supported."
);
activation_error!(
    ActivationPatternTriggerInvariant,
    "Activation trigger root cells disagree with the resolved trigger."
);
activation_error!(
    ActivationPatternTransactionBoolStateUnsupported,
    "Patterned activation transaction state requires boolean values."
);

fn transaction_bool_state(value: &Ref<bool>) -> MResult<Value> {
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

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
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
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.matched)?);
        Ok(values)
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
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
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

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
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
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
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
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
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
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(Value::Index(self.selected.clone()));
        Ok(values)
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

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
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

fn pattern_is_irrefutable(pattern: &CompiledPattern, trigger_kind: &ValueKind) -> bool {
    fn check(
        pattern: &CompiledPattern,
        trigger_kind: &ValueKind,
        bindings: &mut HashSet<usize>,
    ) -> bool {
        match pattern {
            CompiledPattern::Wildcard => true,
            CompiledPattern::Binding { binding_index, .. } => {
                bindings.insert(*binding_index)
            }
            CompiledPattern::ExpressionValue { .. }
            | CompiledPattern::EnumVariant { .. }
            | CompiledPattern::AtomTuple { .. } => false,
            CompiledPattern::Tuple { elements } => {
                let ValueKind::Tuple(kinds) = trigger_kind.deref_kind() else {
                    return false;
                };
                elements.len() == kinds.len()
                    && elements
                        .iter()
                        .zip(&kinds)
                        .all(|(element, kind)| check(element, kind, bindings))
            }
            CompiledPattern::Array {
                prefix,
                spread,
                suffix,
            } => {
                let ValueKind::Matrix(element_kind, shape) = trigger_kind.deref_kind() else {
                    return false;
                };
                let minimum_len = prefix.len() + suffix.len();
                let known_len = (!shape.is_empty()).then(|| shape.iter().product::<usize>());
                match (known_len, spread) {
                    (Some(len), None) if len != minimum_len => return false,
                    (Some(len), Some(_)) if len < minimum_len => return false,
                    (None, None) => return false,
                    (None, Some(_)) if minimum_len != 0 => return false,
                    _ => {}
                }
                if !prefix
                    .iter()
                    .chain(suffix)
                    .all(|element| check(element, &element_kind, bindings))
                {
                    return false;
                }
                spread
                    .as_ref()
                    .and_then(|spread| spread.binding.as_deref())
                    .map_or(true, |binding| {
                        let middle_shape = known_len
                            .map(|len| vec![1, len - minimum_len])
                            .unwrap_or_default();
                        check(
                            binding,
                            &ValueKind::Matrix(element_kind, middle_shape),
                            bindings,
                        )
                    })
            }
        }
    }

    check(pattern, trigger_kind, &mut HashSet::new())
}

fn preflight_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: &Value,
    trigger_cells: &[ReactiveCellId],
    i: &Interpreter,
) -> MResult<PreflightPatternedActivation> {
    arms.last().ok_or_else(|| {
        MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
    })?;
    let trigger_id = match &scope.trigger {
        Expression::Var(var) => var.name.hash(),
        _ => {
            return Err(
                MechError::new(ActivationPatternTriggerInvariant, None)
                    .with_tokens(scope.trigger.tokens()),
            );
        }
    };
    for arm in arms {
        if let Some(guard) = &arm.guard {
            validate_patterned_guard_expression(guard, i)?;
        }
        validate_patterned_arm_body(&arm.body, trigger_id, trigger_cells, i)?;
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
    let last = arms.last().unwrap();
    if last.guard.is_some()
        || !pattern_is_irrefutable(&compiled.last().unwrap().pattern, &trigger_kind)
    {
        return Err(
            MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
        );
    }
    if arms[..arms.len() - 1]
        .iter()
        .any(|arm| arm.guard.is_none() && matches!(arm.pattern, Pattern::Wildcard))
    {
        return Err(
            MechError::new(ActivationPatternWildcardMustBeLast, None)
                .with_tokens(scope.tokens()),
        );
    }
    Ok(PreflightPatternedActivation {
        trigger_kind,
        arms: compiled,
    })
}

fn validation_error(kind: impl MechErrorKind + 'static, tokens: Vec<Token>) -> MResult<()> {
    Err(MechError::new(kind, None).with_tokens(tokens))
}

fn validate_patterned_arm_body(
    body: &ActivationArmBody,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
    match body {
        ActivationArmBody::Block(body) => {
            for (code, _) in body {
                validate_patterned_code(code, trigger_id, trigger_cells, interpreter)?;
            }
            Ok(())
        }
        ActivationArmBody::Expression(expression) => validate_patterned_expression(expression),
    }
}
fn validate_patterned_code(
    code: &MechCode,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
    match code {
        MechCode::Comment(_) => Ok(()),
        MechCode::Expression(expression) => validate_patterned_expression(expression),
        MechCode::Statement(statement) => {
            validate_patterned_statement(statement, trigger_id, trigger_cells, interpreter)
        }
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
fn validate_patterned_register_write(
    target: &SliceRef,
    expression: &Expression,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
    tokens: Vec<Token>,
) -> MResult<()> {
    if target.context.is_some() {
        return validation_error(ActivationPatternContextEffectUnsupported, tokens);
    }
    let target_id = target.name.hash();
    let aliases_trigger = interpreter
        .symbols()
        .borrow()
        .get(target_id)
        .is_some_and(|value| {
            value
                .borrow()
                .reactive_root_cell_ids()
                .iter()
                .any(|cell| trigger_cells.contains(cell))
        });
    if target_id == trigger_id || aliases_trigger {
        return validation_error(ActivationScopeTriggerWriteUnsupported, tokens);
    }
    // Indexed assignment implementations still mutate eagerly and do not
    // implement the reactive-register staging contract.
    if target.subscript.is_some() {
        return validation_error(ActivationPatternRegisterWriteUnsupported, tokens);
    }
    validate_patterned_expression(expression)
}

fn validate_patterned_statement(
    statement: &Statement,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
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
        Statement::VariableAssign(assignment) => validate_patterned_register_write(
            &assignment.target,
            &assignment.expression,
            trigger_id,
            trigger_cells,
            interpreter,
            statement.tokens(),
        ),
        Statement::OpAssign(assignment) => validate_patterned_register_write(
            &assignment.target,
            &assignment.expression,
            trigger_id,
            trigger_cells,
            interpreter,
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

pub(crate) fn activation_scope_entry_cells(
    interpreter: &Interpreter,
) -> Vec<ReactiveCellId> {
    let symbols = interpreter.symbols();
    let symbols = symbols.borrow();
    let mut cells = Vec::new();
    for symbol in symbols.symbols.values() {
        for cell in symbol.borrow().reactive_cell_ids() {
            if !cells.contains(&cell) {
                cells.push(cell);
            }
        }
    }
    cells
}

fn elaborate_patterned_arm_guard(
    guard: &Expression,
    captures: &[ActivationPatternCapture],
    pulse: &Value,
    eligible: &Ref<bool>,
    completion: Ref<usize>,
    interpreter: &InterpreterExecution<'_>,
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
    plan.push_activation_registration_scope_with_sampled_cells(
        pulse_cells.clone(),
        activation_scope_entry_cells(interpreter),
    );
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
    interpreter: &InterpreterExecution<'_>,
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
    plan.push_activation_registration_scope_with_sampled_cells(
        pulse.reactive_root_cell_ids(),
        activation_scope_entry_cells(interpreter),
    );
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
    i: &InterpreterExecution<'_>,
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
    interpreter: &InterpreterExecution<'_>,
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
mod tests;
