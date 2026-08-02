// ----------------------------------------------------------------------------
// Access
// ----------------------------------------------------------------------------

#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(feature = "string")]
pub mod string;
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "tuple")]
pub mod tuple;

#[cfg(feature = "map")]
pub use self::map::*;
#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(feature = "string")]
pub use self::string::*;
#[cfg(feature = "table")]
pub use self::table::*;
#[cfg(feature = "tuple")]
pub use self::tuple::*;

#[macro_use]
use crate::stdlib::*;

/// Installs every enabled concrete access factory into the supplied catalog.
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix")]
    matrix::install_runtime(builder)?;
    #[cfg(feature = "tuple")]
    tuple::install_runtime(builder)?;
    Ok(())
}

#[cfg(feature = "matrix")]
fn matrix_access_index_is_scalar(index: &Value) -> bool {
    index.shape().as_slice() == [1, 1]
}

#[cfg(feature = "matrix")]
fn compile_matrix_access(arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
    match arguments.get(1..).unwrap_or_default() {
        [Value::IndexAll] => MatrixAccessAll {}.specialize(arguments),
        [index] if matrix_access_index_is_scalar(index) => {
            MatrixAccessScalar {}.specialize(arguments)
        }
        [_] => MatrixAccessRange {}.specialize(arguments),
        [Value::IndexAll, index] if matrix_access_index_is_scalar(index) => {
            MatrixAccessAllScalar {}.specialize(arguments)
        }
        [Value::IndexAll, _] => MatrixAccessAllRange {}.specialize(arguments),
        [index, Value::IndexAll] if matrix_access_index_is_scalar(index) => {
            MatrixAccessScalarAll {}.specialize(arguments)
        }
        [_, Value::IndexAll] => MatrixAccessRangeAll {}.specialize(arguments),
        [left, right]
            if matrix_access_index_is_scalar(left) && matrix_access_index_is_scalar(right) =>
        {
            MatrixAccessScalarScalar {}.specialize(arguments)
        }
        [left, _] if matrix_access_index_is_scalar(left) => {
            MatrixAccessScalarRange {}.specialize(arguments)
        }
        [_, right] if matrix_access_index_is_scalar(right) => {
            MatrixAccessRangeScalar {}.specialize(arguments)
        }
        [_, _] => MatrixAccessRangeRange {}.specialize(arguments),
        _ => Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 1,
                found: arguments.len(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

pub struct AccessScalar {}
impl FunctionSpecializer for AccessScalar {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if !(2..=3).contains(&arguments.len()) {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let src = &arguments[0];
        let index = &arguments[1];
        match src.kind().deref_kind() {
            #[cfg(feature = "matrix")]
            ValueKind::Matrix(..) => compile_matrix_access(arguments),
            #[cfg(feature = "table")]
            ValueKind::Table(..) => TableAccessScalar {}.specialize(arguments),
            #[cfg(feature = "map")]
            ValueKind::Map(..) => MapAccess {}.specialize(arguments),
            #[cfg(feature = "string")]
            ValueKind::String => StringAccessScalar {}.specialize(arguments),
            #[cfg(feature = "tuple")]
            ValueKind::Tuple(..) => TupleAccess {}.specialize(arguments),
            _ => Err(MechError::new(
                UnhandledFunctionArgumentKind2 {
                    arg: (src.kind(), index.kind()),
                    fxn_name: "access/scalar".to_string(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

pub struct AccessRange {}
impl FunctionSpecializer for AccessRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if !(2..=3).contains(&arguments.len()) {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let src = &arguments[0];
        let index = &arguments[1];
        match src.kind().deref_kind() {
            #[cfg(feature = "matrix")]
            ValueKind::Matrix(..) => compile_matrix_access(arguments),
            #[cfg(feature = "table")]
            ValueKind::Table(..) => TableAccessRange {}.specialize(arguments),
            _ => Err(MechError::new(
                UnhandledFunctionArgumentKind2 {
                    arg: (src.kind(), index.kind()),
                    fxn_name: "access/range".to_string(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

pub struct AccessSwizzle {}
impl FunctionSpecializer for AccessSwizzle {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() < 3 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let keys = &arguments[1..];
        let src = &arguments[0];
        match src {
            #[cfg(feature = "record")]
            Value::Record(rcrd) => {
                let mut values = vec![];
                for key in keys {
                    let k = key.as_u64().unwrap().borrow().clone();
                    match rcrd.borrow().get(&k) {
                        Some(value) => values.push(value.clone()),
                        None => {
                            return Err(MechError::new(
                                UndefinedRecordFieldError { id: k.clone() },
                                None,
                            )
                            .with_compiler_loc());
                        }
                    }
                }
                Ok(Box::new(RecordAccessSwizzle {
                    source: Value::Tuple(Ref::new(MechTuple::from_vec(values))),
                }))
            }
            #[cfg(feature = "table")]
            Value::Table(tbl) => {
                let mut elements = vec![];
                for k in keys {
                    match k {
                        Value::Id(k) => match tbl.borrow().get(&k) {
                            Some((kind, mat_values)) => {
                                elements.push(Box::new(mat_values.to_value()));
                            }
                            None => {
                                return Err(MechError::new(
                                    UndefinedRecordFieldError { id: k.clone() },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        },
                        _ => {
                            return Err(MechError::new(
                                UnhandledFunctionArgumentIxesMono {
                                    arg: (src.kind(), keys.iter().map(|x| x.kind()).collect()),
                                    fxn_name: "access/swizzle".to_string(),
                                },
                                None,
                            )
                            .with_compiler_loc());
                        }
                    }
                }
                todo!("Table swizzle needs to be fixed.");
                let tuple = Value::Tuple(Ref::new(MechTuple { elements }));
                Ok(Box::new(TableAccessSwizzle { out: tuple }))
            }
            Value::MutableReference(r) => match &*r.borrow() {
                #[cfg(feature = "record")]
                Value::Record(rcrd) => {
                    let mut values = vec![];
                    for key in keys {
                        let k = key.as_u64().unwrap().borrow().clone();
                        match rcrd.borrow().get(&k) {
                            Some(value) => values.push(value.clone()),
                            None => {
                                return Err(MechError::new(
                                    UndefinedRecordFieldError { id: k.clone() },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    Ok(Box::new(RecordAccessSwizzle {
                        source: Value::Tuple(Ref::new(MechTuple::from_vec(values))),
                    }))
                }
                #[cfg(feature = "table")]
                Value::Table(tbl) => {
                    let mut elements = vec![];
                    for key in keys {
                        let k = key.as_u64().unwrap().borrow().clone();
                        match tbl.borrow().get(&k) {
                            Some((kind, mat_values)) => {
                                elements.push(Box::new(mat_values.to_value()));
                            }
                            None => {
                                return Err(MechError::new(
                                    UndefinedTableColumnError { id: k.clone() },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    let tuple = Value::Tuple(Ref::new(MechTuple { elements }));
                    Ok(Box::new(TableAccessSwizzle { out: tuple }))
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
}

// ----------------------------------------------------------------------------

// Access Column

pub fn impl_access_column_fxn(source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
    match source.kind().deref_kind() {
        #[cfg(feature = "record")]
        ValueKind::Record(_) => RecordAccess {}.specialize(&vec![source, key]),
        #[cfg(feature = "map")]
        ValueKind::Map(..) => MapAccess {}.specialize(&vec![source, key]),
        #[cfg(feature = "table")]
        ValueKind::Table(_, _) => TableAccessColumn {}.specialize(&vec![source, key]),
        _ => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (source.kind(), key.kind()),
                fxn_name: "access/column".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

pub struct AccessColumn {}
impl FunctionSpecializer for AccessColumn {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let src = arguments[0].clone();
        let key = arguments[1].clone();
        match impl_access_column_fxn(src.clone(), key.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (src.clone(), &key.clone()) {
                (Value::MutableReference(src), _) => {
                    impl_access_column_fxn(src.borrow().clone(), key.clone())
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (src.kind(), key.kind()),
                        fxn_name: "access/column".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
