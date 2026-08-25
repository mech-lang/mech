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

#[cfg(any(
    feature = "matrix",
    feature = "table",
    feature = "map",
    feature = "string",
    feature = "tuple",
    feature = "record"
))]
use crate::ValueKind;
#[cfg(feature = "native-plan")]
use crate::{
    BytecodeCompilerContext, CompileConst, FunctionArgs, FunctionValueRepresentation,
    MechFunctionCompiler, MechFunctionFactory, MechFunctionImpl, Register, RuntimeFunctionContract,
    RuntimeFunctionSignature, RuntimeOutputAliasPolicy, compile_register, hash_str,
};
use crate::{
    FunctionCatalogBuilder, FunctionSpecializer, IncorrectNumberOfArguments, LegacyValue, MResult,
    MechError, MechFunction, UnhandledFunctionArgumentKind2,
};
#[cfg(any(feature = "record", feature = "table"))]
use crate::{MechTuple, Ref, UndefinedRecordFieldError};
#[cfg(feature = "table")]
use crate::{ToValue, UndefinedTableColumnError, UnhandledFunctionArgumentIxesMono};

#[cfg(feature = "native-plan")]
macro_rules! declare_structural_access_alias {
    (
        $factory:ident,
        $registration:ident,
        $installer:ident,
        $name:literal,
        $path:literal
    ) => {
        #[derive(Debug)]
        struct $factory {
            out: LegacyValue,
        }

        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature =
                RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Nullary(out) => Ok(Box::new(Self { out })),
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 0,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }

        impl MechFunctionImpl for $factory {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }

            fn out(&self) -> LegacyValue {
                self.out.clone()
            }

            fn to_string(&self) -> String {
                format!("{self:#?}")
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $factory {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let register = compile_register!(self.out, ctx);
                ctx.emit_nullop(hash_str($name), register);
                Ok(register)
            }
        }

        mech_core::declare_native_runtime_factory! {
            cfg: feature = "access",
            registration: $registration,
            installer: $installer,
            name: $name,
            factory_type: $factory,
            contract: RuntimeFunctionContract::same_shape(
                RuntimeOutputAliasPolicy::DisallowInputAlias,
            ),
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: $path,
            extra_cargo_features: ["access"],
        }
    };
}

#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    RecordAccessFieldAliasFactory,
    register_record_access_field,
    install_record_access_field,
    "RecordAccessField",
    "mech_engine::__mech_native::install_record_access_field"
);
#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    RecordAccessSwizzleAliasFactory,
    register_record_access_swizzle,
    install_record_access_swizzle,
    "RecordAccessSwizzle",
    "mech_engine::__mech_native::install_record_access_swizzle"
);
#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    TableAccessSwizzleAliasFactory,
    register_table_access_swizzle,
    install_table_access_swizzle,
    "TableAccessSwizzle",
    "mech_engine::__mech_native::install_table_access_swizzle"
);

/// Installs every enabled concrete access factory into the supplied catalog.
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix")]
    matrix::install_runtime(builder)?;
    #[cfg(feature = "tuple")]
    tuple::install_runtime(builder)?;
    Ok(())
}

/// Installs structural access aliases emitted by the source compiler without
/// adding them to the frozen standard runtime surface.
#[cfg(feature = "native-plan")]
pub(crate) fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_record_access_field(builder)?;
    register_record_access_swizzle(builder)?;
    register_table_access_swizzle(builder)?;
    Ok(())
}

#[cfg(feature = "matrix")]
fn matrix_access_index_is_scalar(index: &LegacyValue) -> bool {
    index.shape().as_slice() == [1, 1]
}

#[cfg(feature = "matrix")]
fn compile_matrix_access(arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
    match arguments.get(1..).unwrap_or_default() {
        [LegacyValue::IndexAll] => MatrixAccessAll {}.specialize(arguments),
        [index] if matrix_access_index_is_scalar(index) => {
            MatrixAccessScalar {}.specialize(arguments)
        }
        [_] => MatrixAccessRange {}.specialize(arguments),
        [LegacyValue::IndexAll, index] if matrix_access_index_is_scalar(index) => {
            MatrixAccessAllScalar {}.specialize(arguments)
        }
        [LegacyValue::IndexAll, _] => MatrixAccessAllRange {}.specialize(arguments),
        [index, LegacyValue::IndexAll] if matrix_access_index_is_scalar(index) => {
            MatrixAccessScalarAll {}.specialize(arguments)
        }
        [_, LegacyValue::IndexAll] => MatrixAccessRangeAll {}.specialize(arguments),
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
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
            LegacyValue::Record(rcrd) => {
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
                    source: LegacyValue::Tuple(Ref::new(MechTuple::from_vec(values))),
                }))
            }
            #[cfg(feature = "table")]
            LegacyValue::Table(tbl) => {
                let mut elements = vec![];
                for k in keys {
                    match k {
                        LegacyValue::Id(k) => match tbl.borrow().get(&k) {
                            Some((_, mat_values)) => {
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
                let tuple = LegacyValue::Tuple(Ref::new(MechTuple { elements }));
                Ok(Box::new(TableAccessSwizzle { out: tuple }))
            }
            LegacyValue::MutableReference(r) => match &*r.borrow() {
                #[cfg(feature = "record")]
                LegacyValue::Record(rcrd) => {
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
                        source: LegacyValue::Tuple(Ref::new(MechTuple::from_vec(values))),
                    }))
                }
                #[cfg(feature = "table")]
                LegacyValue::Table(tbl) => {
                    let mut elements = vec![];
                    for key in keys {
                        let k = key.as_u64().unwrap().borrow().clone();
                        match tbl.borrow().get(&k) {
                            Some((_, mat_values)) => {
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
                    let tuple = LegacyValue::Tuple(Ref::new(MechTuple { elements }));
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

pub fn impl_access_column_fxn(
    source: LegacyValue,
    key: LegacyValue,
) -> MResult<Box<dyn MechFunction>> {
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
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
                (LegacyValue::MutableReference(src), _) => {
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
