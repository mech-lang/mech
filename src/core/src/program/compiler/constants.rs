use super::*;
#[cfg(feature = "matrix")]
use crate::structures::Matrix;
use crate::*;

#[cfg(feature = "compiler")]
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
            let rows = dimensions
                .first()
                .copied()
                .unwrap_or(0)
                .try_into()
                .map_err(|_| {
                    unsupported_constant(
                        RuntimeType::Any,
                        kind.clone(),
                        "matrix row count exceeds u32",
                    )
                })?;
            let cols = dimensions
                .get(1)
                .copied()
                .unwrap_or(0)
                .try_into()
                .map_err(|_| {
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
        ValueKind::Table(columns, primary_key) => RuntimeType::Table {
            columns: columns
                .iter()
                .map(|(name, ty)| Ok((name.clone(), runtime_type_from_value_kind(ty)?)))
                .collect::<MResult<_>>()?,
            primary_key: (*primary_key).try_into().map_err(|_| {
                unsupported_constant(
                    RuntimeType::Any,
                    kind.clone(),
                    "table primary key exceeds u32",
                )
            })?,
        },
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

#[cfg(feature = "compiler")]
fn unsupported_value_kind(kind: ValueKind, reason: &'static str) -> MResult<u32> {
    Err(unsupported_constant(
        runtime_type_from_value_kind(&kind)?,
        kind,
        reason,
    ))
}

#[cfg(feature = "compiler")]
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

#[cfg(feature = "compiler")]
pub trait CompileConst {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32>;
}

#[cfg(feature = "compiler")]
impl CompileConst for Value {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        let reg = match self {
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            Value::Bool(x) => x.borrow().compile_const(ctx)?,
            #[cfg(any(feature = "string", feature = "variable_define"))]
            Value::String(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "u8")]
            Value::U8(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "u16")]
            Value::U16(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "u32")]
            Value::U32(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "u64")]
            Value::U64(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "u128")]
            Value::U128(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "i8")]
            Value::I8(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "i16")]
            Value::I16(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "i32")]
            Value::I32(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "i64")]
            Value::I64(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "i128")]
            Value::I128(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "f32")]
            Value::F32(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "f64")]
            Value::F64(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "atom")]
            Value::Atom(x) => x.borrow().compile_const(ctx)?,
            Value::Index(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "complex")]
            Value::C64(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "rational")]
            Value::R64(x) => x.borrow().compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(x) => x.compile_const(ctx)?,
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(x) => x.compile_const(ctx)?,
            #[cfg(feature = "matrix")]
            Value::MatrixIndex(x) => x.compile_const(ctx)?,
            #[cfg(feature = "matrix")]
            Value::MatrixValue(x) => x.compile_const(ctx)?,
            #[cfg(feature = "table")]
            Value::Table(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "record")]
            Value::Record(x) => x.borrow().compile_const(ctx)?,
            #[cfg(feature = "set")]
            Value::Set(x) => x.borrow().compile_const(ctx)?,
            Value::Typed(value, kind) => match value.as_ref() {
                Value::Empty if *kind == ValueKind::Empty => {
                    ctx.intern_constant(EncodedConstant {
                        runtime_type: RuntimeType::Empty,
                        alignment: 1,
                        bytes: Vec::new(),
                    })?
                }
                Value::Empty => {
                    return Err(unsupported_constant(
                        runtime_type_from_value_kind(kind)?,
                        kind.clone(),
                        "typed-empty constants do not have a Phase 1 canonical encoding",
                    ));
                }
                _ => {
                    let source_value_kind = value.kind();
                    if source_value_kind != *kind {
                        return Err(unsupported_constant(
                            runtime_type_from_value_kind(kind)?,
                            source_value_kind,
                            "typed constant annotation does not match its source value kind",
                        ));
                    }
                    value.compile_const(ctx)?
                }
            },
            Value::EmptyKind(kind) if *kind == ValueKind::Empty => {
                ctx.intern_constant(EncodedConstant {
                    runtime_type: RuntimeType::Empty,
                    alignment: 1,
                    bytes: Vec::new(),
                })?
            }
            Value::EmptyKind(kind) => {
                return Err(unsupported_constant(
                    runtime_type_from_value_kind(kind)?,
                    kind.clone(),
                    "typed-empty constants do not have a Phase 1 canonical encoding",
                ));
            }
            Value::Empty => ctx.intern_constant(EncodedConstant {
                runtime_type: RuntimeType::Empty,
                alignment: 1,
                bytes: Vec::new(),
            })?,
            value => {
                let source_value_kind = value.kind();
                return Err(unsupported_constant(
                    runtime_type_from_value_kind(&source_value_kind)?,
                    source_value_kind,
                    "the constant codec is not implemented in bytecode v1 Phase 1",
                ));
            }
        };
        Ok(reg)
    }
}

#[cfg(all(feature = "f64", feature = "compiler"))]
impl CompileConst for f64 {
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        ctx.intern_constant(EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: self.to_bits().to_le_bytes().to_vec(),
        })
    }
}

#[cfg(all(feature = "f32", feature = "compiler"))]
impl CompileConst for f32 {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::F32,
            "F32 constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "u8", feature = "compiler"))]
impl CompileConst for u8 {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::U8,
            "U8 constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "i8", feature = "compiler"))]
impl CompileConst for i8 {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::I8,
            "I8 constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(feature = "compiler")]
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
    ($feature:literal, $t:tt) => {
        paste! {
          #[cfg(all(feature = $feature, feature = "compiler"))]
          impl CompileConst for $t {
            fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
              unsupported_value_kind(
                ValueKind::[<$t:upper>],
                concat!(stringify!($t), " constant encoding is deferred until bytecode v1 Phase 2"),
              )
            }
          }
        }
    };
}

#[cfg(feature = "u16")]
impl_compile_const!("u16", u16);
#[cfg(feature = "u32")]
impl_compile_const!("u32", u32);
#[cfg(feature = "u64")]
impl_compile_const!("u64", u64);
#[cfg(feature = "u128")]
impl_compile_const!("u128", u128);
#[cfg(feature = "i16")]
impl_compile_const!("i16", i16);
#[cfg(feature = "i32")]
impl_compile_const!("i32", i32);
#[cfg(feature = "i64")]
impl_compile_const!("i64", i64);
#[cfg(feature = "i128")]
impl_compile_const!("i128", i128);

#[cfg(all(
    feature = "compiler",
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
    feature = "compiler",
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

#[cfg(all(feature = "rational", feature = "compiler"))]
impl CompileConst for R64 {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::R64,
            "R64 constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "complex", feature = "compiler"))]
impl CompileConst for C64 {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::C64,
            "C64 constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

macro_rules! impl_compile_const_matrix {
    ($matrix_type:ty, $storage:expr) => {
        #[cfg(feature = "compiler")]
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
                let runtime_type = RuntimeType::Matrix {
                    element: Box::new(runtime_type_from_value_kind(&T::as_value_kind())?),
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
                if T::as_value_kind() != ValueKind::F64 {
                    return Err(unsupported_constant(
                        runtime_type,
                        source_value_kind,
                        "only F64 matrix constants are implemented in bytecode v1 Phase 1",
                    ));
                }
                let capacity = row_count
                    .checked_mul(column_count)
                    .and_then(|count| count.checked_mul(8))
                    .and_then(|bytes| bytes.checked_add(8))
                    .ok_or_else(|| {
                        unsupported_constant(
                            runtime_type.clone(),
                            source_value_kind.clone(),
                            "matrix constant payload size overflow",
                        )
                    })?;
                let mut payload = Vec::<u8>::with_capacity(capacity);
                payload.extend_from_slice(&rows.to_le_bytes());
                payload.extend_from_slice(&cols.to_le_bytes());

                for row in 0..row_count {
                    for column in 0..column_count {
                        let element = (&self[(row, column)] as &dyn core::any::Any)
                            .downcast_ref::<f64>()
                            .ok_or_else(|| {
                                unsupported_constant(
                                    runtime_type.clone(),
                                    source_value_kind.clone(),
                                    "F64 matrix kind did not contain F64 elements",
                                )
                            })?;
                        payload.extend_from_slice(&element.to_bits().to_le_bytes());
                    }
                }
                ctx.intern_constant(EncodedConstant {
                    runtime_type,
                    alignment: 8,
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

#[cfg(all(feature = "matrix", feature = "compiler"))]
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

#[cfg(all(feature = "matrixd", feature = "compiler"))]
impl<T> CompileConst for Ref<DMatrix<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "vectord", feature = "compiler"))]
impl<T> CompileConst for Ref<DVector<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "row_vectord", feature = "compiler"))]
impl<T> CompileConst for Ref<RowDVector<T>>
where
    T: CompileConst + ConstElem + AsValueKind + 'static,
{
    fn compile_const(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        self.borrow().compile_const(ctx)
    }
}

#[cfg(all(feature = "record", feature = "compiler"))]
impl CompileConst for MechRecord {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            self.kind(),
            "Record constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "enum", feature = "compiler"))]
impl CompileConst for MechEnum {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::Enum(self.id, self.name()),
            "Enum constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "atom", feature = "compiler"))]
impl CompileConst for MechAtom {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::Atom(self.id(), self.name().clone()),
            "Atom constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "set", feature = "compiler"))]
impl CompileConst for MechSet {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            self.kind(),
            "Set constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "tuple", feature = "compiler"))]
impl CompileConst for MechTuple {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            self.kind(),
            "Tuple constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "table", feature = "compiler"))]
impl CompileConst for MechTable {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            self.kind(),
            "Table constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}

#[cfg(all(feature = "map", feature = "compiler"))]
impl CompileConst for MechMap {
    fn compile_const(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        unsupported_value_kind(
            ValueKind::Map(
                Box::new(self.key_kind.clone()),
                Box::new(self.value_kind.clone()),
            ),
            "Map constant encoding is deferred until bytecode v1 Phase 2",
        )
    }
}
