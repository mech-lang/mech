use super::*;
#[cfg(feature = "matrix")]
use crate::structures::Matrix;
use crate::*;

#[cfg(all(feature = "semantic-compiler", feature = "no_std"))]
use alloc::collections::BTreeSet;
#[cfg(all(feature = "semantic-compiler", not(feature = "no_std")))]
use std::collections::BTreeSet;

#[cfg(feature = "semantic-compiler")]
const MAX_CONSTANT_NESTING: usize = 256;

#[cfg(feature = "semantic-compiler")]
struct ConstantCodecContext {
    active_references: BTreeSet<usize>,
    depth: usize,
}

#[cfg(feature = "semantic-compiler")]
enum AnnotatedChild {
    Concrete(EncodedConstant),
    AbsentOption { declared: RuntimeType },
}

#[cfg(feature = "semantic-compiler")]
impl ConstantCodecContext {
    fn new() -> Self {
        Self {
            active_references: BTreeSet::new(),
            depth: 0,
        }
    }

    fn nested<T>(&mut self, encode: impl FnOnce(&mut Self) -> MResult<T>) -> MResult<T> {
        if self.depth >= MAX_CONSTANT_NESTING {
            return Err(depth_exceeded(MAX_CONSTANT_NESTING));
        }
        self.depth += 1;
        let result = encode(self);
        self.depth -= 1;
        result
    }

    fn encode_child(&mut self, value: &LegacyValue) -> MResult<EncodedConstant> {
        self.nested(|context| encode_constant_value(value, context))
    }
}

#[cfg(feature = "semantic-compiler")]
fn encode_annotated_child(
    value: &LegacyValue,
    declared: &RuntimeType,
    context: &mut ConstantCodecContext,
) -> MResult<AnnotatedChild> {
    if let RuntimeType::Option(declared_inner) = declared {
        let explicit_option = match value {
            LegacyValue::Empty => None,
            LegacyValue::EmptyKind(ValueKind::Option(inner)) => Some(inner.as_ref()),
            LegacyValue::Typed(inner, ValueKind::Option(option))
                if matches!(inner.as_ref(), LegacyValue::Empty) =>
            {
                Some(option.as_ref())
            }
            _ => {
                let child = context.encode_child(value)?;
                if matches!(child.runtime_type, RuntimeType::Option(_))
                    || !runtime_type_matches_annotation(&child.runtime_type, declared_inner)
                {
                    return Ok(AnnotatedChild::Concrete(child));
                }

                let runtime_type = RuntimeType::Option(Box::new(child.runtime_type.clone()));
                let mut bytes = vec![1];
                append_child_payload(&mut bytes, &child)?;
                return Ok(AnnotatedChild::Concrete(encoded_constant(
                    runtime_type,
                    4,
                    bytes,
                )));
            }
        };

        if let Some(explicit_inner) = explicit_option {
            let explicit =
                RuntimeType::Option(Box::new(runtime_type_from_value_kind(explicit_inner)?));
            if !runtime_type_matches_annotation(&explicit, declared) {
                return Err(unsupported_constant(
                    declared.clone(),
                    value.kind(),
                    "explicit absent option does not match the declared composite schema",
                ));
            }
        }
        return Ok(AnnotatedChild::AbsentOption {
            declared: declared.clone(),
        });
    }

    Ok(AnnotatedChild::Concrete(context.encode_child(value)?))
}

#[cfg(feature = "semantic-compiler")]
fn encode_absent_option(runtime_type: RuntimeType) -> MResult<EncodedConstant> {
    if !matches!(runtime_type, RuntimeType::Option(_)) {
        return Err(unsupported_constant(
            runtime_type,
            ValueKind::Empty,
            "an absent option requires an Option runtime type",
        ));
    }
    Ok(encoded_constant(runtime_type, 1, vec![0]))
}

#[cfg(feature = "semantic-compiler")]
fn finalize_annotated_children(
    declared: &RuntimeType,
    children: Vec<AnnotatedChild>,
    source_kind: &ValueKind,
    mismatch_reason: &'static str,
) -> MResult<(RuntimeType, Vec<EncodedConstant>)> {
    let mut exact_type = None::<RuntimeType>;
    for child in &children {
        match child {
            AnnotatedChild::Concrete(child) => {
                if !runtime_type_matches_annotation(&child.runtime_type, declared) {
                    return Err(unsupported_constant(
                        declared.clone(),
                        source_kind.clone(),
                        mismatch_reason,
                    ));
                }
                if let Some(exact) = &exact_type {
                    if exact != &child.runtime_type {
                        return Err(unsupported_constant(
                            declared.clone(),
                            source_kind.clone(),
                            mismatch_reason,
                        ));
                    }
                } else {
                    exact_type = Some(child.runtime_type.clone());
                }
            }
            AnnotatedChild::AbsentOption {
                declared: absent_declared,
            } => {
                if absent_declared != declared {
                    return Err(unsupported_constant(
                        declared.clone(),
                        source_kind.clone(),
                        mismatch_reason,
                    ));
                }
            }
        }
    }

    let runtime_type = exact_type.unwrap_or_else(|| declared.clone());
    let mut encoded = Vec::new();
    encoded.try_reserve_exact(children.len()).map_err(|_| {
        invalid::<()>("unable to allocate annotated constant children").unwrap_err()
    })?;
    for child in children {
        encoded.push(match child {
            AnnotatedChild::Concrete(child) => child,
            AnnotatedChild::AbsentOption { .. } => encode_absent_option(runtime_type.clone())?,
        });
    }
    Ok((runtime_type, encoded))
}

#[cfg(feature = "semantic-compiler")]
fn runtime_table_type_from_columns(columns: &[(String, ValueKind)]) -> MResult<RuntimeType> {
    Ok(RuntimeType::Table {
        columns: columns
            .iter()
            .map(|(name, kind)| Ok((name.clone(), runtime_type_from_value_kind(kind)?)))
            .collect::<MResult<_>>()?,
        primary_key: 0,
    })
}

#[cfg(feature = "semantic-compiler")]
fn runtime_type_from_value_kind(kind: &ValueKind) -> MResult<RuntimeType> {
    Ok(match kind {
        ValueKind::U8 => RuntimeType::U8,
        ValueKind::U16 => RuntimeType::U16,
        ValueKind::U32 => RuntimeType::U32,
        ValueKind::U64 => RuntimeType::U64,
        ValueKind::U128 => RuntimeType::U128,
        ValueKind::I8 => RuntimeType::I8,
        ValueKind::I16 => RuntimeType::I16,
        ValueKind::I32 => RuntimeType::I32,
        ValueKind::I64 => RuntimeType::I64,
        ValueKind::I128 => RuntimeType::I128,
        ValueKind::F32 => RuntimeType::F32,
        ValueKind::F64 => RuntimeType::F64,
        ValueKind::C64 => RuntimeType::C64,
        ValueKind::R64 => RuntimeType::R64,
        ValueKind::String => RuntimeType::String,
        ValueKind::Bool => RuntimeType::Bool,
        ValueKind::Id => RuntimeType::Id,
        ValueKind::Index => RuntimeType::Index,
        ValueKind::Empty => RuntimeType::Empty,
        ValueKind::Any => RuntimeType::Any,
        ValueKind::None => RuntimeType::None,
        ValueKind::Matrix(element, dimensions) => {
            // Missing dimensions are schema holes, not zeros. Only an explicit
            // pair can describe a concrete matrix, including a real 0x0
            // shape.
            let [row_count, column_count] = dimensions.as_slice() else {
                return Err(unsupported_constant(
                    RuntimeType::Any,
                    kind.clone(),
                    "matrix bytecode types require an explicit two-dimensional shape",
                ));
            };
            let rows = (*row_count).try_into().map_err(|_| {
                unsupported_constant(
                    RuntimeType::Any,
                    kind.clone(),
                    "matrix row count exceeds u32",
                )
            })?;
            let cols = (*column_count).try_into().map_err(|_| {
                unsupported_constant(
                    RuntimeType::Any,
                    kind.clone(),
                    "matrix column count exceeds u32",
                )
            })?;
            RuntimeType::Matrix {
                element: Box::new(runtime_type_from_value_kind(element)?),
                storage: MatrixStorage::MatrixD,
                rows,
                cols,
            }
        }
        ValueKind::Enum(id, name) => RuntimeType::Enum {
            id: *id,
            name: name.clone(),
        },
        ValueKind::Record(fields) => RuntimeType::Record(
            fields
                .iter()
                .map(|(name, ty)| Ok((name.clone(), runtime_type_from_value_kind(ty)?)))
                .collect::<MResult<_>>()?,
        ),
        ValueKind::Map(key, value) => RuntimeType::Map {
            key: Box::new(runtime_type_from_value_kind(key)?),
            value: Box::new(runtime_type_from_value_kind(value)?),
        },
        ValueKind::Atom(id, name) => RuntimeType::Atom {
            id: *id,
            name: name.clone(),
        },
        ValueKind::Table(columns, _row_count) => runtime_table_type_from_columns(columns)?,
        ValueKind::Tuple(types) => RuntimeType::Tuple(
            types
                .iter()
                .map(runtime_type_from_value_kind)
                .collect::<MResult<_>>()?,
        ),
        ValueKind::Reference(inner) => {
            RuntimeType::Reference(Box::new(runtime_type_from_value_kind(inner)?))
        }
        ValueKind::Set(element, max_len) => RuntimeType::Set {
            element: Box::new(runtime_type_from_value_kind(element)?),
            max_len: max_len
                .map(|value| value.try_into())
                .transpose()
                .map_err(|_| {
                    unsupported_constant(RuntimeType::Any, kind.clone(), "set limit exceeds u32")
                })?,
        },
        ValueKind::Option(inner) => {
            RuntimeType::Option(Box::new(runtime_type_from_value_kind(inner)?))
        }
        ValueKind::Kind(inner) => RuntimeType::Kind(semantic_kind_from_value_kind(inner)?),
    })
}

#[cfg(feature = "semantic-compiler")]
fn runtime_type_matches_annotation(actual: &RuntimeType, declared: &RuntimeType) -> bool {
    match (actual, declared) {
        (
            RuntimeType::Matrix {
                element: actual_element,
                rows: actual_rows,
                cols: actual_cols,
                ..
            },
            RuntimeType::Matrix {
                element: declared_element,
                rows: declared_rows,
                cols: declared_cols,
                ..
            },
        ) => {
            actual_rows == declared_rows
                && actual_cols == declared_cols
                && runtime_type_matches_annotation(actual_element, declared_element)
        }
        (RuntimeType::Record(actual), RuntimeType::Record(declared)) => {
            actual.len() == declared.len()
                && actual.iter().zip(declared).all(
                    |((actual_name, actual_type), (declared_name, declared_type))| {
                        actual_name == declared_name
                            && runtime_type_matches_annotation(actual_type, declared_type)
                    },
                )
        }
        (
            RuntimeType::Map {
                key: actual_key,
                value: actual_value,
            },
            RuntimeType::Map {
                key: declared_key,
                value: declared_value,
            },
        ) => {
            runtime_type_matches_annotation(actual_key, declared_key)
                && runtime_type_matches_annotation(actual_value, declared_value)
        }
        (
            RuntimeType::Table {
                columns: actual_columns,
                primary_key: actual_primary_key,
            },
            RuntimeType::Table {
                columns: declared_columns,
                primary_key: declared_primary_key,
            },
        ) => {
            actual_primary_key == declared_primary_key
                && actual_columns.len() == declared_columns.len()
                && actual_columns.iter().zip(declared_columns).all(
                    |((actual_name, actual_type), (declared_name, declared_type))| {
                        actual_name == declared_name
                            && runtime_type_matches_annotation(actual_type, declared_type)
                    },
                )
        }
        (RuntimeType::Tuple(actual), RuntimeType::Tuple(declared)) => {
            actual.len() == declared.len()
                && actual
                    .iter()
                    .zip(declared)
                    .all(|(actual, declared)| runtime_type_matches_annotation(actual, declared))
        }
        (RuntimeType::Reference(actual), RuntimeType::Reference(declared))
        | (RuntimeType::Option(actual), RuntimeType::Option(declared)) => {
            runtime_type_matches_annotation(actual, declared)
        }
        (
            RuntimeType::Set {
                element: actual_element,
                max_len: actual_max_len,
            },
            RuntimeType::Set {
                element: declared_element,
                max_len: declared_max_len,
            },
        ) => {
            actual_max_len == declared_max_len
                && runtime_type_matches_annotation(actual_element, declared_element)
        }
        _ => actual == declared,
    }
}

#[cfg(feature = "semantic-compiler")]
fn unsupported_value_kind(kind: ValueKind, reason: &'static str) -> MResult<u32> {
    Err(unsupported_constant(
        runtime_type_from_value_kind(&kind)?,
        kind,
        reason,
    ))
}

#[cfg(feature = "semantic-compiler")]
fn semantic_kind_from_value_kind(kind: &ValueKind) -> MResult<crate::kind::Kind> {
    use crate::kind::Kind;

    Ok(match kind {
        ValueKind::Any => Kind::Any,
        ValueKind::None => Kind::None,
        ValueKind::Empty => Kind::Empty,
        ValueKind::Id => Kind::Id,
        ValueKind::Index => Kind::Index,
        ValueKind::Atom(id, name) => Kind::Atom(*id, name.clone()),
        ValueKind::Enum(id, name) => Kind::Enum(*id, name.clone()),
        ValueKind::Map(key, value) => Kind::Map(
            Box::new(semantic_kind_from_value_kind(key)?),
            Box::new(semantic_kind_from_value_kind(value)?),
        ),
        ValueKind::Matrix(element, dimensions) => Kind::Matrix(
            Box::new(semantic_kind_from_value_kind(element)?),
            dimensions.clone(),
        ),
        ValueKind::Option(inner) => Kind::Option(Box::new(semantic_kind_from_value_kind(inner)?)),
        ValueKind::Record(fields) => Kind::Record(
            fields
                .iter()
                .map(|(name, ty)| Ok((name.clone(), semantic_kind_from_value_kind(ty)?)))
                .collect::<MResult<_>>()?,
        ),
        ValueKind::Reference(inner) => {
            Kind::Reference(Box::new(semantic_kind_from_value_kind(inner)?))
        }
        ValueKind::Set(element, max_len) => {
            Kind::Set(Box::new(semantic_kind_from_value_kind(element)?), *max_len)
        }
        ValueKind::Table(columns, primary_key) => Kind::Table(
            columns
                .iter()
                .map(|(name, ty)| Ok((name.clone(), semantic_kind_from_value_kind(ty)?)))
                .collect::<MResult<_>>()?,
            *primary_key,
        ),
        ValueKind::Tuple(types) => Kind::Tuple(
            types
                .iter()
                .map(semantic_kind_from_value_kind)
                .collect::<MResult<_>>()?,
        ),
        ValueKind::Kind(inner) => Kind::Kind(Box::new(semantic_kind_from_value_kind(inner)?)),
        scalar => Kind::Scalar(hash_str(&scalar.to_string())),
    })
}

// CompileConst Trait
// ----------------------------------------------------------------------------

#[cfg(feature = "semantic-compiler")]
pub trait CompileConst {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32>;
}

#[cfg(feature = "semantic-compiler")]
struct CapturingConstantContext {
    constant: Option<EncodedConstant>,
}

#[cfg(feature = "semantic-compiler")]
impl BytecodeCompilerContext for CapturingConstantContext {
    fn register_for_ptr_with_initialization_status(&mut self, _pointer: usize) -> (Register, bool) {
        (0, false)
    }

    fn intern_constant(&mut self, constant: EncodedConstant) -> MResult<u32> {
        if self.constant.replace(constant).is_some() {
            return invalid("a constant encoder attempted to intern more than one constant");
        }
        Ok(0)
    }

    fn emit_composite_pack(
        &mut self,
        _destination: Register,
        _template: u32,
        _children: Vec<Register>,
    ) {
        unreachable!("constant capture does not emit bytecode instructions")
    }

    fn define_symbol(
        &mut self,
        _pointer: usize,
        _register: Register,
        _name: &str,
        _mutable: bool,
    ) -> MResult<()> {
        Ok(())
    }
    fn intern_requirement(&mut self, _requirement: ApplicationRequirement) -> MResult<u32> {
        Ok(0)
    }
    fn emit_const_load(&mut self, _destination: Register, _constant: u32) {}
    fn emit_nullop(&mut self, _function: u64, _destination: Register) {}
    fn emit_unop(&mut self, _function: u64, _destination: Register, _source: Register) {}
    fn emit_binop(
        &mut self,
        _function: u64,
        _destination: Register,
        _lhs: Register,
        _rhs: Register,
    ) {
    }
    fn emit_ternop(
        &mut self,
        _function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
    ) {
    }
    fn emit_quadop(
        &mut self,
        _function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
        _d: Register,
    ) {
    }
    fn emit_varop(&mut self, _function: u64, _destination: Register, _arguments: Vec<Register>) {}
    fn emit_host_call(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _arguments: Vec<Register>,
    ) {
    }
    fn emit_resource_read(&mut self, _requirement: u32, _destination: Register) {}
    fn emit_resource_write(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _source: Register,
    ) {
    }
    fn emit_resource_send(&mut self, _requirement: u32, _destination: Register, _source: Register) {
    }
}

#[cfg(feature = "semantic-compiler")]
fn capture_constant<T: CompileConst + ?Sized>(value: &T) -> MResult<EncodedConstant> {
    let mut context = CapturingConstantContext { constant: None };
    value.compile_const(&mut context)?;
    context
        .constant
        .ok_or_else(|| invalid::<()>("constant encoder did not intern a constant").unwrap_err())
}

#[cfg(feature = "semantic-compiler")]
fn encoded_constant(runtime_type: RuntimeType, alignment: u8, bytes: Vec<u8>) -> EncodedConstant {
    EncodedConstant {
        runtime_type,
        alignment,
        bytes,
    }
}

#[cfg(feature = "semantic-compiler")]
fn encode_constant_value(
    value: &LegacyValue,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let _ = context;
    match value {
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        LegacyValue::Bool(value) => Ok(encoded_constant(
            RuntimeType::Bool,
            1,
            vec![if *value.borrow() { 1 } else { 0 }],
        )),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        LegacyValue::String(value) => Ok(encoded_constant(
            RuntimeType::String,
            1,
            value.borrow().as_bytes().to_vec(),
        )),
        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Ok(encoded_constant(
            RuntimeType::U8,
            1,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Ok(encoded_constant(
            RuntimeType::U16,
            2,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Ok(encoded_constant(
            RuntimeType::U32,
            4,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Ok(encoded_constant(
            RuntimeType::U64,
            8,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Ok(encoded_constant(
            RuntimeType::U128,
            16,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => Ok(encoded_constant(
            RuntimeType::I8,
            1,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => Ok(encoded_constant(
            RuntimeType::I16,
            2,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => Ok(encoded_constant(
            RuntimeType::I32,
            4,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => Ok(encoded_constant(
            RuntimeType::I64,
            8,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => Ok(encoded_constant(
            RuntimeType::I128,
            16,
            value.borrow().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => Ok(encoded_constant(
            RuntimeType::F32,
            4,
            value.borrow().to_bits().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => Ok(encoded_constant(
            RuntimeType::F64,
            8,
            value.borrow().to_bits().to_le_bytes().to_vec(),
        )),
        #[cfg(feature = "complex")]
        LegacyValue::C64(value) => Ok(encoded_constant(
            RuntimeType::C64,
            8,
            [
                value.borrow().0.re.to_bits().to_le_bytes(),
                value.borrow().0.im.to_bits().to_le_bytes(),
            ]
            .concat(),
        )),
        #[cfg(feature = "rational")]
        LegacyValue::R64(value) => Ok(encoded_constant(
            RuntimeType::R64,
            8,
            [
                value.borrow().numer().to_le_bytes(),
                value.borrow().denom().to_le_bytes(),
            ]
            .concat(),
        )),
        LegacyValue::Id(value) => Ok(encoded_constant(
            RuntimeType::Id,
            8,
            value.to_le_bytes().to_vec(),
        )),
        LegacyValue::Index(value) => {
            let index = u64::try_from(*value.borrow()).map_err(|_| {
                unsupported_constant(
                    RuntimeType::Index,
                    ValueKind::Index,
                    "Index constant cannot be represented as u64",
                )
            })?;
            Ok(encoded_constant(
                RuntimeType::Index,
                8,
                index.to_le_bytes().to_vec(),
            ))
        }
        LegacyValue::Empty => Ok(encoded_constant(RuntimeType::Empty, 1, Vec::new())),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        LegacyValue::MatrixF64(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        LegacyValue::MatrixF32(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        LegacyValue::MatrixU8(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        LegacyValue::MatrixU16(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        LegacyValue::MatrixU32(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        LegacyValue::MatrixU64(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        LegacyValue::MatrixU128(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        LegacyValue::MatrixI8(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        LegacyValue::MatrixI16(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        LegacyValue::MatrixI32(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        LegacyValue::MatrixI64(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        LegacyValue::MatrixI128(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        LegacyValue::MatrixBool(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        LegacyValue::MatrixR64(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        LegacyValue::MatrixC64(value) => capture_constant(value),
        #[cfg(all(feature = "matrix", feature = "string"))]
        LegacyValue::MatrixString(value) => capture_constant(value),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixIndex(value) => capture_constant(value),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(value) if value.as_vec().is_empty() => {
            let kind =
                ValueKind::Matrix(Box::new(ValueKind::Any), vec![value.rows(), value.cols()]);
            let rows = u32::try_from(value.rows()).map_err(|_| {
                unsupported_constant(
                    RuntimeType::Any,
                    kind.clone(),
                    "empty value-matrix row count exceeds u32",
                )
            })?;
            let cols = u32::try_from(value.cols()).map_err(|_| {
                unsupported_constant(
                    RuntimeType::Any,
                    kind,
                    "empty value-matrix column count exceeds u32",
                )
            })?;
            let runtime_type = RuntimeType::Matrix {
                element: Box::new(RuntimeType::Any),
                storage: MatrixStorage::MatrixD,
                rows,
                cols,
            };
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&rows.to_le_bytes());
            bytes.extend_from_slice(&cols.to_le_bytes());
            Ok(encoded_constant(runtime_type, 4, bytes))
        }
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(value) => Err(unsupported_constant(
            RuntimeType::Any,
            ValueKind::Matrix(Box::new(ValueKind::Any), vec![value.rows(), value.cols()]),
            "nonempty value-matrix constants require heterogeneous element encoding, which bytecode v1 does not define",
        )),
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(value) => encode_tuple_constant(&value.borrow(), context),
        #[cfg(feature = "record")]
        LegacyValue::Record(value) => encode_record_constant(&value.borrow(), context),
        #[cfg(feature = "map")]
        LegacyValue::Map(value) => encode_map_constant(&value.borrow(), context),
        #[cfg(feature = "set")]
        LegacyValue::Set(value) => encode_set_constant(&value.borrow(), context),
        #[cfg(feature = "table")]
        LegacyValue::Table(value) => encode_table_constant(&value.borrow(), context),
        #[cfg(feature = "atom")]
        LegacyValue::Atom(value) => encode_atom_constant(&value.borrow()),
        #[cfg(feature = "enum")]
        LegacyValue::Enum(value) => encode_enum_constant(&value.borrow(), context),
        LegacyValue::MutableReference(value) => encode_reference_constant(value, context),
        LegacyValue::Typed(value, kind) => encode_typed_constant(value, kind, context),
        LegacyValue::EmptyKind(kind) => encode_empty_kind_constant(kind),
        LegacyValue::Kind(kind) => Ok(encoded_constant(
            RuntimeType::Kind(semantic_kind_from_value_kind(kind)?),
            1,
            Vec::new(),
        )),
        LegacyValue::IndexAll => Err(unsupported_constant(
            RuntimeType::Any,
            ValueKind::Empty,
            "IndexAll constants do not have a bytecode-v1 encoding",
        )),
        other => {
            let kind = other.kind();
            Err(unsupported_constant(
                runtime_type_from_value_kind(&kind)?,
                kind,
                "the constant value is not yet supported by the bytecode-v1 codec",
            ))
        }
    }
}

#[cfg(feature = "semantic-compiler")]
fn append_child_payload(payload: &mut Vec<u8>, child: &EncodedConstant) -> MResult<()> {
    let length = u32::try_from(child.bytes.len()).map_err(|_| {
        unsupported_constant(
            child.runtime_type.clone(),
            ValueKind::Any,
            "nested constant payload length exceeds u32",
        )
    })?;
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(&child.bytes);
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
fn checked_count(
    count: usize,
    runtime_type: RuntimeType,
    kind: ValueKind,
    what: &'static str,
) -> MResult<u32> {
    u32::try_from(count).map_err(|_| unsupported_constant(runtime_type, kind, what))
}

#[cfg(all(feature = "tuple", feature = "semantic-compiler"))]
fn encode_tuple_constant(
    value: &MechTuple,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let mut children = Vec::new();
    children
        .try_reserve_exact(value.elements.len())
        .map_err(|_| invalid::<()>("unable to allocate tuple constant children").unwrap_err())?;
    for element in &value.elements {
        children.push(context.encode_child(element)?);
    }
    let runtime_type = RuntimeType::Tuple(
        children
            .iter()
            .map(|child| child.runtime_type.clone())
            .collect(),
    );
    let kind = value.kind();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            children.len(),
            runtime_type.clone(),
            kind.clone(),
            "tuple element count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for child in &children {
        append_child_payload(&mut bytes, child)?;
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(all(feature = "record", feature = "semantic-compiler"))]
fn encode_record_constant(
    value: &MechRecord,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let mut fields = Vec::new();
    let mut children = Vec::new();
    for (field_index, (id, child_value)) in value.data.iter().enumerate() {
        let name = value.field_names.get(id).ok_or_else(|| {
            unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "record field is missing its canonical name",
            )
        })?;
        if hash_str(name) != *id {
            return Err(unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "record field name does not match its stable ID",
            ));
        }
        if let Some(annotation) = value.kinds.get(field_index) {
            let declared = runtime_type_from_value_kind(annotation)?;
            let annotated = encode_annotated_child(child_value, &declared, context)?;
            let (field_type, mut encoded) = finalize_annotated_children(
                &declared,
                vec![annotated],
                &value.kind(),
                "record field type does not match its declared schema",
            )?;
            fields.push((name.clone(), field_type));
            children.push(encoded.pop().expect("one annotated record field"));
        } else {
            let child = context.encode_child(child_value)?;
            fields.push((name.clone(), child.runtime_type.clone()));
            children.push(child);
        }
    }
    let runtime_type = RuntimeType::Record(fields);
    let kind = value.kind();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            children.len(),
            runtime_type.clone(),
            kind,
            "record field count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for child in &children {
        append_child_payload(&mut bytes, child)?;
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(all(feature = "map", feature = "semantic-compiler"))]
fn encode_map_constant(
    value: &MechMap,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let declared_key_type = runtime_type_from_value_kind(&value.key_kind)?;
    let declared_value_type = runtime_type_from_value_kind(&value.value_kind)?;
    let source_kind = value.kind();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for (key, entry_value) in &value.map {
        keys.push(encode_annotated_child(key, &declared_key_type, context)?);
        values.push(encode_annotated_child(
            entry_value,
            &declared_value_type,
            context,
        )?);
    }
    let (key_type, keys) = finalize_annotated_children(
        &declared_key_type,
        keys,
        &source_kind,
        "map key type does not match the declared map schema",
    )?;
    let (value_type, values) = finalize_annotated_children(
        &declared_value_type,
        values,
        &source_kind,
        "map value type does not match the declared map schema",
    )?;
    let runtime_type = RuntimeType::Map {
        key: Box::new(key_type.clone()),
        value: Box::new(value_type.clone()),
    };
    let mut entries = keys.into_iter().zip(values).collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| (&lhs.0.bytes, &lhs.1.bytes).cmp(&(&rhs.0.bytes, &rhs.1.bytes)));
    if entries
        .windows(2)
        .any(|pair| pair[0].0.bytes == pair[1].0.bytes)
    {
        return Err(unsupported_constant(
            runtime_type,
            source_kind,
            "map contains duplicate canonical key payloads",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            entries.len(),
            runtime_type.clone(),
            source_kind,
            "map entry count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for (key, entry_value) in entries {
        append_child_payload(&mut bytes, &key)?;
        append_child_payload(&mut bytes, &entry_value)?;
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(all(feature = "set", feature = "semantic-compiler"))]
fn encode_set_constant(
    value: &MechSet,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let declared_element_type = runtime_type_from_value_kind(&value.kind)?;
    let max_len = value
        .max_elements
        .map(|limit| {
            checked_count(
                limit,
                RuntimeType::Any,
                value.kind(),
                "set maximum length exceeds u32",
            )
        })
        .transpose()?;
    let source_kind = value.kind();
    let mut elements = Vec::new();
    for element in &value.set {
        elements.push(encode_annotated_child(
            element,
            &declared_element_type,
            context,
        )?);
    }
    let (element_type, mut elements) = finalize_annotated_children(
        &declared_element_type,
        elements,
        &source_kind,
        "set element type does not match the declared set schema",
    )?;
    let runtime_type = RuntimeType::Set {
        element: Box::new(element_type.clone()),
        max_len,
    };
    elements.sort_by(|lhs, rhs| lhs.bytes.cmp(&rhs.bytes));
    if elements
        .windows(2)
        .any(|pair| pair[0].bytes == pair[1].bytes)
    {
        return Err(unsupported_constant(
            runtime_type,
            source_kind,
            "set contains duplicate canonical element payloads",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            elements.len(),
            runtime_type.clone(),
            source_kind,
            "set element count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for element in elements {
        append_child_payload(&mut bytes, &element)?;
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(all(feature = "table", feature = "semantic-compiler", feature = "vectord"))]
fn encode_table_constant(
    value: &MechTable,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    if value.rows > crate::program::bytecode::constants::MAX_TABLE_CONSTANT_ROWS {
        return Err(unsupported_constant(
            RuntimeType::Any,
            value.kind(),
            "table row count exceeds bytecode v1 limit",
        ));
    }
    let cells = value.rows.checked_mul(value.cols).ok_or_else(|| {
        unsupported_constant(RuntimeType::Any, value.kind(), "table cell count overflow")
    })?;
    if cells > crate::program::bytecode::constants::MAX_TABLE_CONSTANT_CELLS {
        return Err(unsupported_constant(
            RuntimeType::Any,
            value.kind(),
            "table cell count exceeds bytecode v1 limit",
        ));
    }
    let mut columns = Vec::new();
    let mut column_values = Vec::new();
    for (id, (kind, column)) in &value.data {
        let name = value.col_names.get(id).ok_or_else(|| {
            unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "table column is missing its canonical name",
            )
        })?;
        if hash_str(name) != *id {
            return Err(unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "table column name does not match its stable ID",
            ));
        }
        let Matrix::DVector(values) = column else {
            return Err(unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "table columns must use dynamic value vectors",
            ));
        };
        if values.borrow().len() != value.rows {
            return Err(unsupported_constant(
                RuntimeType::Any,
                value.kind(),
                "table column length does not match row count",
            ));
        }
        let declared_type = runtime_type_from_value_kind(kind)?;
        let mut annotated_cells = Vec::new();
        annotated_cells
            .try_reserve_exact(value.rows)
            .map_err(|_| invalid::<()>("unable to allocate table constant cells").unwrap_err())?;
        for cell in values.borrow().iter() {
            annotated_cells.push(encode_annotated_child(cell, &declared_type, context)?);
        }
        let column_kind = ValueKind::Table(vec![(name.clone(), kind.clone())], value.rows);
        let (column_type, encoded_cells) = finalize_annotated_children(
            &declared_type,
            annotated_cells,
            &column_kind,
            "table cell type does not match its declared column schema",
        )?;
        columns.push((name.clone(), column_type));
        column_values.push(encoded_cells);
    }
    let primary_key = 0;
    let runtime_type = RuntimeType::Table {
        columns: columns.clone(),
        primary_key,
    };
    let source_kind = value.kind();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            value.rows,
            runtime_type.clone(),
            source_kind.clone(),
            "table row count exceeds u32",
        )?
        .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &checked_count(
            columns.len(),
            runtime_type.clone(),
            source_kind.clone(),
            "table column count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for row in 0..value.rows {
        for cells in &column_values {
            append_child_payload(&mut bytes, &cells[row])?;
        }
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(all(
    feature = "table",
    feature = "semantic-compiler",
    not(feature = "vectord")
))]
fn encode_table_constant(
    value: &MechTable,
    _context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    unsupported_value_kind(
        value.kind(),
        "table constants require the dynamic vector feature",
    )
}

#[cfg(all(feature = "atom", feature = "semantic-compiler"))]
fn encode_atom_constant(value: &MechAtom) -> MResult<EncodedConstant> {
    let id = value.id();
    let name = value.name();
    if hash_str(&name) != id {
        return Err(unsupported_constant(
            RuntimeType::Atom { id, name },
            ValueKind::Atom(id, value.name()),
            "atom name does not match its stable ID",
        ));
    }
    Ok(encoded_constant(
        RuntimeType::Atom { id, name },
        1,
        Vec::new(),
    ))
}

#[cfg(all(feature = "enum", feature = "semantic-compiler"))]
fn encode_enum_constant(
    value: &MechEnum,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let enum_name = value.name();
    if hash_str(&enum_name) != value.id {
        return Err(unsupported_constant(
            RuntimeType::Enum {
                id: value.id,
                name: enum_name,
            },
            value.kind(),
            "enum name does not match its stable ID",
        ));
    }
    let runtime_type = RuntimeType::Enum {
        id: value.id,
        name: enum_name,
    };
    let source_kind = value.kind();
    let names = value.names.borrow();
    let mut variants = Vec::new();
    for (id, payload) in &value.variants {
        let name = names.get(id).cloned().ok_or_else(|| {
            unsupported_constant(
                runtime_type.clone(),
                source_kind.clone(),
                "enum variant is missing its canonical name",
            )
        })?;
        if hash_str(&name) != *id {
            return Err(unsupported_constant(
                runtime_type,
                source_kind,
                "enum variant name does not match its stable ID",
            ));
        }
        let payload = payload
            .as_ref()
            .map(|payload| context.encode_child(payload))
            .transpose()?;
        variants.push((*id, name, payload));
    }
    variants.sort_by_key(|(id, _, _)| *id);
    if variants.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(unsupported_constant(
            runtime_type,
            source_kind,
            "enum contains duplicate variant IDs",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &checked_count(
            variants.len(),
            runtime_type.clone(),
            source_kind.clone(),
            "enum variant count exceeds u32",
        )?
        .to_le_bytes(),
    );
    for (id, name, payload) in variants {
        bytes.extend_from_slice(&id.to_le_bytes());
        let name_length = u32::try_from(name.len()).map_err(|_| {
            unsupported_constant(
                runtime_type.clone(),
                source_kind.clone(),
                "enum variant name length exceeds u32",
            )
        })?;
        bytes.extend_from_slice(&name_length.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        match payload {
            None => bytes.push(0),
            Some(payload) => {
                bytes.push(1);
                let type_key = crate::program::bytecode::constants::inline_type::encode(
                    &payload.runtime_type,
                )?;
                let key_length = u32::try_from(type_key.len()).map_err(|_| {
                    unsupported_constant(
                        runtime_type.clone(),
                        source_kind.clone(),
                        "enum inline type key length exceeds u32",
                    )
                })?;
                bytes.extend_from_slice(&key_length.to_le_bytes());
                bytes.extend_from_slice(&type_key);
                append_child_payload(&mut bytes, &payload)?;
            }
        }
    }
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(feature = "semantic-compiler")]
fn encode_reference_constant(
    value: &MutableReference,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let address = value.addr();
    if !context.active_references.insert(address) {
        return Err(unsupported_constant(
            RuntimeType::Any,
            ValueKind::Any,
            "cyclic mutable reference graph cannot be encoded",
        ));
    }
    let child = context.encode_child(&value.borrow());
    context.active_references.remove(&address);
    let child = child?;
    let mut bytes = Vec::new();
    append_child_payload(&mut bytes, &child)?;
    Ok(encoded_constant(
        RuntimeType::Reference(Box::new(child.runtime_type)),
        4,
        bytes,
    ))
}

#[cfg(feature = "semantic-compiler")]
fn encode_empty_kind_constant(kind: &ValueKind) -> MResult<EncodedConstant> {
    match kind {
        ValueKind::Any => Ok(encoded_constant(RuntimeType::Any, 1, Vec::new())),
        ValueKind::None => Ok(encoded_constant(RuntimeType::None, 1, Vec::new())),
        ValueKind::Option(inner) => Ok(encoded_constant(
            RuntimeType::Option(Box::new(runtime_type_from_value_kind(inner)?)),
            1,
            vec![0],
        )),
        _ => Err(unsupported_constant(
            runtime_type_from_value_kind(kind)?,
            kind.clone(),
            "nonempty EmptyKind values do not have a bytecode-v1 encoding",
        )),
    }
}

#[cfg(feature = "semantic-compiler")]
fn encode_typed_constant(
    value: &LegacyValue,
    kind: &ValueKind,
    context: &mut ConstantCodecContext,
) -> MResult<EncodedConstant> {
    let ValueKind::Option(inner) = kind else {
        return Err(unsupported_constant(
            runtime_type_from_value_kind(kind)?,
            value.kind(),
            "typed constant annotation does not match its source value kind; only Option wrappers are canonical",
        ));
    };
    let declared_inner_type = runtime_type_from_value_kind(inner)?;
    let declared_runtime_type = RuntimeType::Option(Box::new(declared_inner_type.clone()));
    if matches!(value, LegacyValue::Empty) {
        return Ok(encoded_constant(declared_runtime_type, 1, vec![0]));
    }
    let child = context.encode_child(value)?;
    if !runtime_type_matches_annotation(&child.runtime_type, &declared_inner_type) {
        return Err(unsupported_constant(
            declared_runtime_type,
            value.kind(),
            "typed option child does not match its declared inner type",
        ));
    }
    let runtime_type = RuntimeType::Option(Box::new(child.runtime_type.clone()));
    let mut bytes = vec![1];
    append_child_payload(&mut bytes, &child)?;
    Ok(encoded_constant(runtime_type, 4, bytes))
}

#[cfg(feature = "semantic-compiler")]
impl CompileConst for LegacyValue {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        let mut codec = ConstantCodecContext::new();
        ctx.intern_constant(encode_constant_value(self, &mut codec)?)
    }
}

#[cfg(all(feature = "f64", feature = "semantic-compiler"))]
impl CompileConst for f64 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: self.to_bits().to_le_bytes().to_vec(),
        })
    }
}

#[cfg(all(feature = "f32", feature = "semantic-compiler"))]
impl CompileConst for f32 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::F32,
            alignment: 4,
            bytes: self.to_bits().to_le_bytes().to_vec(),
        })
    }
}

#[cfg(all(feature = "u8", feature = "semantic-compiler"))]
impl CompileConst for u8 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::U8,
            alignment: 1,
            bytes: vec![*self],
        })
    }
}

#[cfg(all(feature = "i8", feature = "semantic-compiler"))]
impl CompileConst for i8 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::I8,
            alignment: 1,
            bytes: vec![*self as u8],
        })
    }
}

#[cfg(feature = "semantic-compiler")]
impl CompileConst for usize {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        let value = u64::try_from(*self).map_err(|_| {
            unsupported_constant(
                RuntimeType::Index,
                ValueKind::Index,
                "Index constant cannot be represented as u64",
            )
        })?;
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::Index,
            alignment: 8,
            bytes: value.to_le_bytes().to_vec(),
        })
    }
}

macro_rules! impl_compile_const {
    ($feature:literal, $t:tt, $runtime_type:ident, $alignment:literal) => {
        paste! {
          #[cfg(all(feature = $feature, feature = "semantic-compiler"))]
          impl CompileConst for $t {
            fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
              ctx.intern_constant(EncodedConstant {
                runtime_type: RuntimeType::$runtime_type,
                alignment: $alignment,
                bytes: self.to_le_bytes().to_vec(),
              })
            }
          }
        }
    };
}

#[cfg(feature = "u16")]
impl_compile_const!("u16", u16, U16, 2);
#[cfg(feature = "u32")]
impl_compile_const!("u32", u32, U32, 4);
#[cfg(feature = "u64")]
impl_compile_const!("u64", u64, U64, 8);
#[cfg(feature = "u128")]
impl_compile_const!("u128", u128, U128, 16);
#[cfg(feature = "i16")]
impl_compile_const!("i16", i16, I16, 2);
#[cfg(feature = "i32")]
impl_compile_const!("i32", i32, I32, 4);
#[cfg(feature = "i64")]
impl_compile_const!("i64", i64, I64, 8);
#[cfg(feature = "i128")]
impl_compile_const!("i128", i128, I128, 16);

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "bool", feature = "variable_define")
))]
impl CompileConst for bool {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::Bool,
            alignment: 1,
            bytes: vec![if *self { 1 } else { 0 }],
        })
    }
}

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "string", feature = "variable_define")
))]
impl CompileConst for String {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: self.as_bytes().to_vec(),
        })
    }
}

#[cfg(all(feature = "rational", feature = "semantic-compiler"))]
impl CompileConst for R64 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        let numerator = *self.numer();
        let denominator = *self.denom();
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::R64,
            alignment: 8,
            bytes: [numerator.to_le_bytes(), denominator.to_le_bytes()].concat(),
        })
    }
}

#[cfg(all(feature = "complex", feature = "semantic-compiler"))]
impl CompileConst for C64 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::C64,
            alignment: 8,
            bytes: [
                self.0.re.to_bits().to_le_bytes(),
                self.0.im.to_bits().to_le_bytes(),
            ]
            .concat(),
        })
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
trait MatrixConstantElement: AsValueKind + 'static {
    fn runtime_type() -> Option<RuntimeType>;
    fn alignment() -> u8;
    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()>;
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
macro_rules! impl_matrix_constant_element {
    ($feature:literal, $type:ty, $runtime_type:ident, $alignment:literal) => {
        #[cfg(feature = $feature)]
        impl MatrixConstantElement for $type {
            fn runtime_type() -> Option<RuntimeType> {
                Some(RuntimeType::$runtime_type)
            }

            fn alignment() -> u8 {
                $alignment
            }

            fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
                payload.extend_from_slice(&self.to_le_bytes());
                Ok(())
            }
        }
    };
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler", feature = "bool"))]
impl MatrixConstantElement for bool {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::Bool)
    }

    fn alignment() -> u8 {
        1
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        payload.push(if *self { 1 } else { 0 });
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("u8", u8, U8, 1);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("u16", u16, U16, 2);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("u32", u32, U32, 4);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("u64", u64, U64, 8);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("u128", u128, U128, 16);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("i8", i8, I8, 1);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("i16", i16, I16, 2);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("i32", i32, I32, 4);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("i64", i64, I64, 8);
#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl_matrix_constant_element!("i128", i128, I128, 16);

#[cfg(all(feature = "matrix", feature = "semantic-compiler", feature = "f32"))]
impl MatrixConstantElement for f32 {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::F32)
    }

    fn alignment() -> u8 {
        4
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        payload.extend_from_slice(&self.to_bits().to_le_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler", feature = "f64"))]
impl MatrixConstantElement for f64 {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::F64)
    }

    fn alignment() -> u8 {
        8
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        payload.extend_from_slice(&self.to_bits().to_le_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler", feature = "string"))]
impl MatrixConstantElement for String {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::String)
    }

    fn alignment() -> u8 {
        4
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        let length = u32::try_from(self.len()).map_err(|_| {
            unsupported_constant(
                RuntimeType::String,
                ValueKind::String,
                "String matrix element length exceeds u32",
            )
        })?;
        payload.extend_from_slice(&length.to_le_bytes());
        payload.extend_from_slice(self.as_bytes());
        Ok(())
    }
}

#[cfg(all(
    feature = "matrix",
    feature = "semantic-compiler",
    feature = "rational"
))]
impl MatrixConstantElement for R64 {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::R64)
    }

    fn alignment() -> u8 {
        8
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        payload.extend_from_slice(&self.numer().to_le_bytes());
        payload.extend_from_slice(&self.denom().to_le_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler", feature = "complex"))]
impl MatrixConstantElement for C64 {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::C64)
    }

    fn alignment() -> u8 {
        8
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        payload.extend_from_slice(&self.0.re.to_bits().to_le_bytes());
        payload.extend_from_slice(&self.0.im.to_bits().to_le_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl MatrixConstantElement for usize {
    fn runtime_type() -> Option<RuntimeType> {
        Some(RuntimeType::Index)
    }

    fn alignment() -> u8 {
        8
    }

    fn encode_matrix_element(&self, payload: &mut Vec<u8>) -> MResult<()> {
        let value = u64::try_from(*self).map_err(|_| {
            unsupported_constant(
                RuntimeType::Index,
                ValueKind::Index,
                "Index matrix element cannot be represented as u64",
            )
        })?;
        payload.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl MatrixConstantElement for LegacyValue {
    fn runtime_type() -> Option<RuntimeType> {
        None
    }

    fn alignment() -> u8 {
        1
    }

    fn encode_matrix_element(&self, _payload: &mut Vec<u8>) -> MResult<()> {
        unreachable!("Matrix<Value> constants are rejected before their elements are encoded")
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
fn matrix_element_alignment(element_type: &RuntimeType) -> u8 {
    match element_type {
        RuntimeType::Bool | RuntimeType::U8 | RuntimeType::I8 => 1,
        RuntimeType::U16 | RuntimeType::I16 => 2,
        RuntimeType::U32 | RuntimeType::I32 | RuntimeType::F32 | RuntimeType::String => 4,
        RuntimeType::U64
        | RuntimeType::I64
        | RuntimeType::F64
        | RuntimeType::C64
        | RuntimeType::R64
        | RuntimeType::Index => 8,
        RuntimeType::U128 | RuntimeType::I128 => 16,
        _ => 1,
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
fn encode_matrix_element<T: 'static>(
    element: &T,
    element_type: &RuntimeType,
    matrix_type: &RuntimeType,
    source_value_kind: &ValueKind,
    payload: &mut Vec<u8>,
) -> MResult<()> {
    macro_rules! fixed_element {
        ($type:ty, $value:ident, $encode:block) => {{
            let $value = (element as &dyn core::any::Any)
                .downcast_ref::<$type>()
                .ok_or_else(|| {
                    unsupported_constant(
                        matrix_type.clone(),
                        source_value_kind.clone(),
                        "matrix element does not match its declared bytecode runtime type",
                    )
                })?;
            $encode
            Ok(())
        }};
    }

    match element_type {
        RuntimeType::Bool => fixed_element!(bool, value, {
            payload.push(if *value { 1 } else { 0 });
        }),
        RuntimeType::U8 => fixed_element!(u8, value, {
            payload.push(*value);
        }),
        RuntimeType::U16 => fixed_element!(u16, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::U32 => fixed_element!(u32, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::U64 => fixed_element!(u64, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::U128 => fixed_element!(u128, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::I8 => fixed_element!(i8, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::I16 => fixed_element!(i16, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::I32 => fixed_element!(i32, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::I64 => fixed_element!(i64, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::I128 => fixed_element!(i128, value, {
            payload.extend_from_slice(&value.to_le_bytes());
        }),
        RuntimeType::F32 => fixed_element!(f32, value, {
            payload.extend_from_slice(&value.to_bits().to_le_bytes());
        }),
        RuntimeType::F64 => fixed_element!(f64, value, {
            payload.extend_from_slice(&value.to_bits().to_le_bytes());
        }),
        RuntimeType::Index => fixed_element!(usize, value, {
            let index = u64::try_from(*value).map_err(|_| {
                unsupported_constant(
                    matrix_type.clone(),
                    source_value_kind.clone(),
                    "Index matrix element cannot be represented as u64",
                )
            })?;
            payload.extend_from_slice(&index.to_le_bytes());
        }),
        RuntimeType::String => fixed_element!(String, value, {
            let length = u32::try_from(value.len()).map_err(|_| {
                unsupported_constant(
                    matrix_type.clone(),
                    source_value_kind.clone(),
                    "String matrix element length exceeds u32",
                )
            })?;
            payload.extend_from_slice(&length.to_le_bytes());
            payload.extend_from_slice(value.as_bytes());
        }),
        RuntimeType::R64 => {
            #[cfg(feature = "rational")]
            {
                fixed_element!(R64, value, {
                    payload.extend_from_slice(&value.numer().to_le_bytes());
                    payload.extend_from_slice(&value.denom().to_le_bytes());
                })
            }
            #[cfg(not(feature = "rational"))]
            {
                Err(unsupported_constant(
                    matrix_type.clone(),
                    source_value_kind.clone(),
                    "R64 matrix constants are unavailable in this runtime",
                ))
            }
        }
        RuntimeType::C64 => {
            #[cfg(feature = "complex")]
            {
                fixed_element!(C64, value, {
                    payload.extend_from_slice(&value.0.re.to_bits().to_le_bytes());
                    payload.extend_from_slice(&value.0.im.to_bits().to_le_bytes());
                })
            }
            #[cfg(not(feature = "complex"))]
            {
                Err(unsupported_constant(
                    matrix_type.clone(),
                    source_value_kind.clone(),
                    "C64 matrix constants are unavailable in this runtime",
                ))
            }
        }
        _ => Err(unsupported_constant(
            matrix_type.clone(),
            source_value_kind.clone(),
            "MatrixValue and non-scalar matrix constants do not have a bytecode-v1 encoding",
        )),
    }
}

macro_rules! impl_compile_const_matrix {
    ($matrix_type:ty, $storage:expr) => {
        #[cfg(feature = "semantic-compiler")]
        impl<T> CompileConst for $matrix_type
        where
            T: ConstElem + AsValueKind + 'static,
        {
            fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
                let row_count = self.nrows();
                let column_count = self.ncols();
                let rows = u32::try_from(row_count).map_err(|_| {
                    unsupported_constant(
                        RuntimeType::Any,
                        ValueKind::Matrix(
                            Box::new(T::as_value_kind()),
                            vec![row_count, column_count],
                        ),
                        "matrix row count exceeds u32",
                    )
                })?;
                let cols = u32::try_from(column_count).map_err(|_| {
                    unsupported_constant(
                        RuntimeType::Any,
                        ValueKind::Matrix(
                            Box::new(T::as_value_kind()),
                            vec![row_count, column_count],
                        ),
                        "matrix column count exceeds u32",
                    )
                })?;
                let source_value_kind = ValueKind::Matrix(
                    Box::new(T::as_value_kind()),
                    vec![row_count, column_count],
                );
                let element_type = runtime_type_from_value_kind(&T::as_value_kind())?;
                let runtime_type = RuntimeType::Matrix {
                    element: Box::new(element_type.clone()),
                    storage: $storage,
                    rows,
                    cols,
                };
                if !$storage.validate_dimensions(rows, cols) {
                    return Err(unsupported_constant(
                        runtime_type,
                        source_value_kind,
                        "matrix storage and dimensions do not form a valid bytecode v1 runtime type",
                    ));
                }
                let mut payload = Vec::<u8>::new();
                payload.extend_from_slice(&rows.to_le_bytes());
                payload.extend_from_slice(&cols.to_le_bytes());

                for row in 0..row_count {
                    for column in 0..column_count {
                        encode_matrix_element(
                            &self[(row, column)],
                            &element_type,
                            &runtime_type,
                            &source_value_kind,
                            &mut payload,
                        )?;
                    }
                }
                ctx.intern_constant(EncodedConstant {
                    runtime_type,
                    alignment: matrix_element_alignment(&element_type),
                    bytes: payload,
                })
            }
        }
    };
}

#[cfg(feature = "matrix1")]
impl_compile_const_matrix!(na::Matrix1<T>, MatrixStorage::Matrix1);
#[cfg(feature = "matrix2")]
impl_compile_const_matrix!(na::Matrix2<T>, MatrixStorage::Matrix2);
#[cfg(feature = "matrix3")]
impl_compile_const_matrix!(na::Matrix3<T>, MatrixStorage::Matrix3);
#[cfg(feature = "matrix4")]
impl_compile_const_matrix!(na::Matrix4<T>, MatrixStorage::Matrix4);
#[cfg(feature = "matrix2x3")]
impl_compile_const_matrix!(na::Matrix2x3<T>, MatrixStorage::Matrix2x3);
#[cfg(feature = "matrix3x2")]
impl_compile_const_matrix!(na::Matrix3x2<T>, MatrixStorage::Matrix3x2);
#[cfg(feature = "row_vector2")]
impl_compile_const_matrix!(na::RowVector2<T>, MatrixStorage::RowVector2);
#[cfg(feature = "row_vector3")]
impl_compile_const_matrix!(na::RowVector3<T>, MatrixStorage::RowVector3);
#[cfg(feature = "row_vector4")]
impl_compile_const_matrix!(na::RowVector4<T>, MatrixStorage::RowVector4);
#[cfg(feature = "vector2")]
impl_compile_const_matrix!(na::Vector2<T>, MatrixStorage::Vector2);
#[cfg(feature = "vector3")]
impl_compile_const_matrix!(na::Vector3<T>, MatrixStorage::Vector3);
#[cfg(feature = "vector4")]
impl_compile_const_matrix!(na::Vector4<T>, MatrixStorage::Vector4);
#[cfg(feature = "matrixd")]
impl_compile_const_matrix!(na::DMatrix<T>, MatrixStorage::MatrixD);
#[cfg(feature = "vectord")]
impl_compile_const_matrix!(na::DVector<T>, MatrixStorage::VectorD);
#[cfg(feature = "row_vectord")]
impl_compile_const_matrix!(na::RowDVector<T>, MatrixStorage::RowVectorD);

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl<T> CompileConst for Matrix<T>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        match self {
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "vectord")]
            Matrix::DVector(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(mat) => mat.borrow().compile_const(ctx),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(mat) => mat.borrow().compile_const(ctx),
        }
    }
}

#[cfg(all(feature = "matrixd", feature = "semantic-compiler"))]
impl<T> CompileConst for Ref<DMatrix<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "vectord", feature = "semantic-compiler"))]
impl<T> CompileConst for Ref<DVector<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "row_vectord", feature = "semantic-compiler"))]
impl<T> CompileConst for Ref<RowDVector<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "record", feature = "semantic-compiler"))]
impl CompileConst for MechRecord {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Record(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "enum", feature = "semantic-compiler"))]
impl CompileConst for MechEnum {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Enum(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "atom", feature = "semantic-compiler"))]
impl CompileConst for MechAtom {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Atom(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "set", feature = "semantic-compiler"))]
impl CompileConst for MechSet {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Set(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "tuple", feature = "semantic-compiler"))]
impl CompileConst for MechTuple {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Tuple(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "table", feature = "semantic-compiler"))]
impl CompileConst for MechTable {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Table(Ref::new(self.clone())).compile_const(ctx)
    }
}

#[cfg(all(feature = "map", feature = "semantic-compiler"))]
impl CompileConst for MechMap {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        LegacyValue::Map(Ref::new(self.clone())).compile_const(ctx)
    }
}
