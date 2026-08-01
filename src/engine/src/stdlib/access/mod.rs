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

/// Installs every enabled concrete access factory without consulting the
/// legacy distributed inventory.
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
fn compile_matrix_access(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    match arguments.as_slice().get(1..).unwrap_or_default() {
        [Value::IndexAll] => MatrixAccessAll {}.compile(arguments),
        [index] if matrix_access_index_is_scalar(index) => MatrixAccessScalar {}.compile(arguments),
        [_] => MatrixAccessRange {}.compile(arguments),
        [Value::IndexAll, index] if matrix_access_index_is_scalar(index) => {
            MatrixAccessAllScalar {}.compile(arguments)
        }
        [Value::IndexAll, _] => MatrixAccessAllRange {}.compile(arguments),
        [index, Value::IndexAll] if matrix_access_index_is_scalar(index) => {
            MatrixAccessScalarAll {}.compile(arguments)
        }
        [_, Value::IndexAll] => MatrixAccessRangeAll {}.compile(arguments),
        [left, right]
            if matrix_access_index_is_scalar(left) && matrix_access_index_is_scalar(right) =>
        {
            MatrixAccessScalarScalar {}.compile(arguments)
        }
        [left, _] if matrix_access_index_is_scalar(left) => {
            MatrixAccessScalarRange {}.compile(arguments)
        }
        [_, right] if matrix_access_index_is_scalar(right) => {
            MatrixAccessRangeScalar {}.compile(arguments)
        }
        [_, _] => MatrixAccessRangeRange {}.compile(arguments),
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
impl NativeFunctionCompiler for AccessScalar {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
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
            ValueKind::Table(..) => TableAccessScalar {}.compile(arguments),
            #[cfg(feature = "map")]
            ValueKind::Map(..) => MapAccess {}.compile(arguments),
            #[cfg(feature = "string")]
            ValueKind::String => StringAccessScalar {}.compile(arguments),
            #[cfg(feature = "tuple")]
            ValueKind::Tuple(..) => TupleAccess {}.compile(arguments),
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
impl NativeFunctionCompiler for AccessRange {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
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
            ValueKind::Table(..) => TableAccessRange {}.compile(arguments),
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
impl NativeFunctionCompiler for AccessSwizzle {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
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
        let keys = &arguments.clone().split_off(1);
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
        ValueKind::Record(_) => RecordAccess {}.compile(&vec![source, key]),
        #[cfg(feature = "map")]
        ValueKind::Map(..) => MapAccess {}.compile(&vec![source, key]),
        #[cfg(feature = "table")]
        ValueKind::Table(_, _) => TableAccessColumn {}.compile(&vec![source, key]),
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
impl NativeFunctionCompiler for AccessColumn {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_catalog_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn explicit_runtime_factories_match_access_inventory_names_ids_and_pointers() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        let mut legacy = BTreeMap::new();
        for descriptor in inventory::iter::<FunctionDescriptor>
            .into_iter()
            .filter(|descriptor| {
                descriptor.name.starts_with("Access") || descriptor.name == "TupleAccessElement"
            })
        {
            if let Some(existing) = legacy.insert(descriptor.name, descriptor.ptr as usize) {
                assert_eq!(existing, descriptor.ptr as usize, "{}", descriptor.name);
            }
        }

        assert_eq!(catalog.runtime_factory_count(), legacy.len());
        for entry in catalog.runtime_entries() {
            assert_eq!(entry.id, RuntimeFunctionId::from_name(&entry.name));
            let legacy_factory = legacy
                .remove(entry.name.as_str())
                .unwrap_or_else(|| panic!("missing legacy access factory {}", entry.name));
            assert_eq!(entry.factory as usize, legacy_factory, "{}", entry.name);
        }
        assert!(
            legacy.is_empty(),
            "unmigrated legacy access factories: {legacy:?}"
        );
    }
}
