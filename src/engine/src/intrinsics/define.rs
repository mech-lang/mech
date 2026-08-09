#[macro_use]
use crate::intrinsics::*;

/// Bytecode-visible marker that keeps integrity-constraint support in the
/// exact native dependency closure. Constraint identity and its live result
/// cell remain encoded by the immutable `!` symbol bound to the input
/// register; this no-op factory gives contract analysis an explicit linkage
/// requirement without adding a bytecode-v1 opcode or section.
#[cfg(feature = "invariant_define")]
#[derive(Debug)]
pub struct BytecodeIntegrityConstraintMarker {
    out: LegacyValue,
    arguments: Vec<LegacyValue>,
}

#[cfg(feature = "invariant_define")]
impl MechFunctionFactory for BytecodeIntegrityConstraintMarker {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Variadic(out, arguments) if arguments.len() == 6 => {
                Ok(Box::new(Self { out, arguments }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 6,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

#[cfg(feature = "invariant_define")]
impl MechFunctionImpl for BytecodeIntegrityConstraintMarker {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        self.out.clone()
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::None; argument_count])
    }

    fn to_string(&self) -> String {
        "integrity/constraint".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(Vec::new())
    }
}

#[cfg(all(feature = "invariant_define", feature = "compiler"))]
impl MechFunctionCompiler for BytecodeIntegrityConstraintMarker {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination =
            compile_value_register(&self.out, core::ptr::from_ref(&self.out).addr(), ctx)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| {
                compile_value_register(argument, core::ptr::from_ref(argument).addr(), ctx)
            })
            .collect::<MResult<Vec<_>>>()?;
        ctx.emit_varop(hash_str("integrity/constraint"), destination, arguments);
        Ok(destination)
    }
}

#[derive(Debug)]
pub struct VariableDefineMatrix<T, MatA> {
    pub id: u64,
    pub name: Ref<String>,
    pub mutable: Ref<bool>,
    pub var: Ref<MatA>,
    pub _marker: PhantomData<T>,
}
impl<T, MatA> MechFunctionFactory for VariableDefineMatrix<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + ConstElem + AsValueKind,
    #[cfg(feature = "compiler")]
    T: CompileConst,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + ConstElem + AsNaKind + FunctionRuntimeType + 'static,
    #[cfg(feature = "compiler")]
    MatA: CompileConst,
    Ref<MatA>: ToValue,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        MatA::REPRESENTATION,
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::Bool,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(var, arg1, arg2) => {
                let var: Ref<MatA> = var.try_function_ref(FunctionArgumentRole::Output)?;
                let name: Ref<String> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let mutable: Ref<bool> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let id = hash_str(&name.borrow());
                Ok(Box::new(Self {
                    id,
                    name,
                    mutable,
                    var,
                    _marker: PhantomData::default(),
                }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 3,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
impl<T, MatA> MechFunctionImpl for VariableDefineMatrix<T, MatA>
where
    Ref<MatA>: ToValue,
    T: Debug + Clone + Sync + Send + 'static + ConstElem + AsValueKind,
    MatA: Debug,
{
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.var.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<T, MatA> MechFunctionCompiler for VariableDefineMatrix<T, MatA>
where
    T: CompileConst + ConstElem + AsValueKind,
    MatA: CompileConst + ConstElem + AsNaKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let variable_register = compile_register_brrw!(self.var, ctx);
        let variable_name = self.name.borrow().clone();
        let variable_mutable = *self.mutable.borrow();
        ctx.define_symbol(
            self.var.addr(),
            variable_register,
            &variable_name,
            variable_mutable,
        );
        let name = format!(
            "VariableDefineMatrix<{}{}>",
            T::as_value_kind(),
            MatA::as_na_kind()
        );
        compile_binop!(name, self.var, self.name, self.mutable, ctx);
    }
}

#[macro_export]
macro_rules! impl_variable_define_fxn {
  ($kind:tt) => {
    paste! {
      #[derive(Debug, Clone)]
      pub struct [<VariableDefine $kind:camel>] {
        id: u64,
        name: Ref<String>,
        mutable: Ref<bool>,
        var: Ref<$kind>,
      }
      impl MechFunctionFactory for [<VariableDefine $kind:camel>] {
      const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
          <$kind as FunctionRuntimeType>::REPRESENTATION,
          FunctionValueRepresentation::String,
          FunctionValueRepresentation::Bool,
        );

      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
              let var: Ref<$kind> = out.try_function_ref(FunctionArgumentRole::Output)?;
              let name: Ref<String> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
              let mutable: Ref<bool> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
              let id = hash_str(&name.borrow());
              Ok(Box::new(Self { id, name, mutable, var }))
            },
            _ => Err(MechError::new(
                IncorrectNumberOfArguments { expected: 3, found: args.len() },
                None
              ).with_compiler_loc()
            ),
          }
        }
      }
      impl MechFunctionImpl for [<VariableDefine $kind:camel>] {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }
        fn out(&self) -> LegacyValue { self.var.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
          Ok(self.reactive_output_values())
        }
      }
      #[cfg(feature = "compiler")]
      impl MechFunctionCompiler for [<VariableDefine $kind:camel>] {
      fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let value = self.var.to_value();
          let variable_register = compile_value_register(&value, self.var.addr(), ctx)?;
          let variable_name = self.name.borrow().clone();
          let variable_mutable = *self.mutable.borrow();
          ctx.define_symbol(self.var.addr(), variable_register, &variable_name, variable_mutable);
          let name = format!(stringify!([<VariableDefine $kind:camel>]));
          compile_binop!(name, self.var, self.name, self.mutable, ctx );
        }
      }
    }
  };
}

#[cfg(feature = "f64")]
impl_variable_define_fxn!(f64);

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "f64", feature = "variable_define"),

    registration: register_variable_define_f64,
    installer: install_variable_define_f64,

    name: "VariableDefineF64",
    factory_type: VariableDefineF64,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_f64",

    extra_cargo_features: ["variable_define"],
}

#[cfg(feature = "f32")]
impl_variable_define_fxn!(f32);
#[cfg(feature = "u8")]
impl_variable_define_fxn!(u8);
#[cfg(feature = "u16")]
impl_variable_define_fxn!(u16);
#[cfg(feature = "u32")]
impl_variable_define_fxn!(u32);
#[cfg(feature = "u64")]
impl_variable_define_fxn!(u64);
#[cfg(feature = "u128")]
impl_variable_define_fxn!(u128);
#[cfg(feature = "i8")]
impl_variable_define_fxn!(i8);
#[cfg(feature = "i16")]
impl_variable_define_fxn!(i16);
#[cfg(feature = "i32")]
impl_variable_define_fxn!(i32);
#[cfg(feature = "i64")]
impl_variable_define_fxn!(i64);
#[cfg(feature = "i128")]
impl_variable_define_fxn!(i128);
#[cfg(feature = "r64")]
impl_variable_define_fxn!(R64);
#[cfg(feature = "c64")]
impl_variable_define_fxn!(C64);
#[cfg(feature = "bool")]
impl_variable_define_fxn!(bool);
#[cfg(feature = "string")]
impl_variable_define_fxn!(String);
#[cfg(feature = "table")]
impl_variable_define_fxn!(MechTable);
#[cfg(feature = "set")]
impl_variable_define_fxn!(MechSet);
#[cfg(feature = "tuple")]
impl_variable_define_fxn!(MechTuple);
#[cfg(feature = "record")]
impl_variable_define_fxn!(MechRecord);
#[cfg(feature = "map")]
impl_variable_define_fxn!(MechMap);
#[cfg(feature = "atom")]
impl_variable_define_fxn!(MechAtom);
#[cfg(feature = "enum")]
impl_variable_define_fxn!(MechEnum);

macro_rules! declare_variable_define_scalar_native {
    ($feature:literal, $kind:ident) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "variable_define", feature = $feature),
                registration: [<register_variable_define_ $kind:lower>],
                installer: [<install_variable_define_ $kind:lower>],
                name: stringify!([<VariableDefine $kind:camel>]),
                factory_type: [<VariableDefine $kind:camel>],
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_",
                    stringify!([<$kind:lower>]),
                ),
                extra_cargo_features: ["variable_define"],
            }
        }
    };
}

declare_variable_define_scalar_native!("f32", f32);
declare_variable_define_scalar_native!("u8", u8);
declare_variable_define_scalar_native!("u16", u16);
declare_variable_define_scalar_native!("u32", u32);
declare_variable_define_scalar_native!("u64", u64);
declare_variable_define_scalar_native!("u128", u128);
declare_variable_define_scalar_native!("i8", i8);
declare_variable_define_scalar_native!("i16", i16);
declare_variable_define_scalar_native!("i32", i32);
declare_variable_define_scalar_native!("i64", i64);
declare_variable_define_scalar_native!("i128", i128);
declare_variable_define_scalar_native!("r64", R64);
declare_variable_define_scalar_native!("c64", C64);
declare_variable_define_scalar_native!("bool", bool);
declare_variable_define_scalar_native!("string", String);
declare_variable_define_scalar_native!("table", MechTable);
declare_variable_define_scalar_native!("set", MechSet);
declare_variable_define_scalar_native!("tuple", MechTuple);
declare_variable_define_scalar_native!("record", MechRecord);
declare_variable_define_scalar_native!("map", MechMap);
declare_variable_define_scalar_native!("atom", MechAtom);
declare_variable_define_scalar_native!("enum", MechEnum);

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(feature = "variable_define")]
    pub use super::install_variable_define_empty;

    macro_rules! export_variable_define_scalar_native {
        ($feature:literal, $kind:ident) => {
            #[cfg(all(feature = "variable_define", feature = $feature))]
            paste::paste! { pub use super::[<install_variable_define_ $kind:lower>]; }
        };
    }
    export_variable_define_scalar_native!("f32", f32);
    export_variable_define_scalar_native!("u8", u8);
    export_variable_define_scalar_native!("u16", u16);
    export_variable_define_scalar_native!("u32", u32);
    export_variable_define_scalar_native!("u64", u64);
    export_variable_define_scalar_native!("u128", u128);
    export_variable_define_scalar_native!("i8", i8);
    export_variable_define_scalar_native!("i16", i16);
    export_variable_define_scalar_native!("i32", i32);
    export_variable_define_scalar_native!("i64", i64);
    export_variable_define_scalar_native!("i128", i128);
    export_variable_define_scalar_native!("r64", R64);
    export_variable_define_scalar_native!("c64", C64);
    export_variable_define_scalar_native!("bool", bool);
    export_variable_define_scalar_native!("string", String);
    export_variable_define_scalar_native!("table", MechTable);
    export_variable_define_scalar_native!("set", MechSet);
    export_variable_define_scalar_native!("tuple", MechTuple);
    export_variable_define_scalar_native!("record", MechRecord);
    export_variable_define_scalar_native!("map", MechMap);
    export_variable_define_scalar_native!("atom", MechAtom);
    export_variable_define_scalar_native!("enum", MechEnum);
}

macro_rules! for_each_variable_define_matrix_shape {
    ($callback:path, $context:tt) => {
        $callback!(
            $context,
            Matrix1,
            "matrix1",
            any(feature = "matrix1", feature = "variable_define_matrix1")
        );
        $callback!($context, Matrix2, "matrix2", feature = "matrix2");
        $callback!($context, Matrix2x3, "matrix2x3", feature = "matrix2x3");
        $callback!($context, Matrix3x2, "matrix3x2", feature = "matrix3x2");
        $callback!($context, Matrix3, "matrix3", feature = "matrix3");
        $callback!($context, Matrix4, "matrix4", feature = "matrix4");
        $callback!($context, DMatrix, "matrixd", feature = "matrixd");
        $callback!($context, Vector2, "vector2", feature = "vector2");
        $callback!($context, Vector3, "vector3", feature = "vector3");
        $callback!($context, Vector4, "vector4", feature = "vector4");
        $callback!($context, DVector, "vectord", feature = "vectord");
        $callback!($context, RowVector2, "row_vector2", feature = "row_vector2");
        $callback!($context, RowVector3, "row_vector3", feature = "row_vector3");
        $callback!($context, RowVector4, "row_vector4", feature = "row_vector4");
        $callback!($context, RowDVector, "row_vectord", feature = "row_vectord");
    };
}

macro_rules! declare_variable_define_matrix_native {
    (
        ($value_feature:literal; $kind:ty; $kind_token:ident; $kind_name:literal),
        $shape:ident,
        $shape_feature:literal,
        $shape_cfg:meta
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "variable_define",
                    feature = $value_feature,
                    $shape_cfg
                ),
                registration: [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                installer: [<install_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                name: concat!("VariableDefineMatrix<", $kind_name, stringify!($shape), ">"),
                factory_type: VariableDefineMatrix<$kind, $shape<$kind>>,
                contract: RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_matrix_",
                    stringify!([<$kind_token:lower>]), "_", stringify!([<$shape:lower>]),
                ),
                extra_cargo_features: ["variable_define"],
            }
        }
    };
}

macro_rules! declare_variable_define_matrix_for_type {
    ($value_feature:literal, $kind:ty, $kind_token:ident, $kind_name:literal) => {
        for_each_variable_define_matrix_shape!(
            declare_variable_define_matrix_native,
            ($value_feature; $kind; $kind_token; $kind_name)
        );
    };
}

declare_variable_define_matrix_for_type!("u8", u8, u8, "u8");
declare_variable_define_matrix_for_type!("u16", u16, u16, "u16");
declare_variable_define_matrix_for_type!("u32", u32, u32, "u32");
declare_variable_define_matrix_for_type!("u64", u64, u64, "u64");
declare_variable_define_matrix_for_type!("u128", u128, u128, "u128");
declare_variable_define_matrix_for_type!("i8", i8, i8, "i8");
declare_variable_define_matrix_for_type!("i16", i16, i16, "i16");
declare_variable_define_matrix_for_type!("i32", i32, i32, "i32");
declare_variable_define_matrix_for_type!("i64", i64, i64, "i64");
declare_variable_define_matrix_for_type!("i128", i128, i128, "i128");
declare_variable_define_matrix_for_type!("f32", f32, f32, "f32");
declare_variable_define_matrix_for_type!("f64", f64, f64, "f64");
declare_variable_define_matrix_for_type!("r64", R64, R64, "rational");
declare_variable_define_matrix_for_type!("c64", C64, C64, "complex");
declare_variable_define_matrix_for_type!("bool", bool, bool, "bool");
declare_variable_define_matrix_for_type!("string", String, String, "string");

macro_rules! register_variable_define_matrix_native {
    (
        ($builder:ident; $kind_token:ident),
        Matrix1,
        "matrix1",
        $shape_cfg:meta
    ) => {
        #[cfg(feature = "matrix1")]
        paste! {
            [<register_variable_define_matrix_ $kind_token:lower _matrix1>]($builder)?;
        }
    };
    (
        ($builder:ident; $kind_token:ident),
        $shape:ident,
        $_shape_feature:literal,
        $shape_cfg:meta
    ) => {
        #[cfg($shape_cfg)]
        paste! {
            [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>]($builder)?;
        }
    };
}

macro_rules! export_variable_define_matrix_native {
    (($kind_token:ident), $shape:ident, $_shape_feature:literal, $shape_cfg:meta) => {
        #[cfg($shape_cfg)]
        paste::paste! {
            pub use super::[<install_variable_define_matrix_ $kind_token:lower _ $shape:lower>];
        }
    };
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native_matrix {
    macro_rules! export_variable_define_matrix_for_type {
        ($feature:literal, $kind_token:ident) => {
            #[cfg(all(feature = "variable_define", feature = $feature))]
            for_each_variable_define_matrix_shape!(
                export_variable_define_matrix_native,
                ($kind_token)
            );
        };
    }
    export_variable_define_matrix_for_type!("u8", u8);
    export_variable_define_matrix_for_type!("u16", u16);
    export_variable_define_matrix_for_type!("u32", u32);
    export_variable_define_matrix_for_type!("u64", u64);
    export_variable_define_matrix_for_type!("u128", u128);
    export_variable_define_matrix_for_type!("i8", i8);
    export_variable_define_matrix_for_type!("i16", i16);
    export_variable_define_matrix_for_type!("i32", i32);
    export_variable_define_matrix_for_type!("i64", i64);
    export_variable_define_matrix_for_type!("i128", i128);
    export_variable_define_matrix_for_type!("f32", f32);
    export_variable_define_matrix_for_type!("f64", f64);
    export_variable_define_matrix_for_type!("r64", R64);
    export_variable_define_matrix_for_type!("c64", C64);
    export_variable_define_matrix_for_type!("bool", bool);
    export_variable_define_matrix_for_type!("string", String);
}

#[derive(Debug, Clone)]
pub struct VariableDefineEmpty {
    id: u64,
    name: Ref<String>,
    mutable: Ref<bool>,
    var: Ref<LegacyValue>,
}

impl MechFunctionFactory for VariableDefineEmpty {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::MutableValueCell,
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::Bool,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(var, arg1, arg2) => {
                let var: Ref<LegacyValue> = var.try_function_ref(FunctionArgumentRole::Output)?;
                let name: Ref<String> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let mutable: Ref<bool> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let id = hash_str(&name.borrow());
                Ok(Box::new(Self {
                    id,
                    name,
                    mutable,
                    var,
                }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 3,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "variable_define",
    registration: register_variable_define_empty,
    installer: install_variable_define_empty,
    name: "VariableDefineEmpty",
    factory_type: VariableDefineEmpty,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_empty",
    extra_cargo_features: ["variable_define"],
}
impl MechFunctionImpl for VariableDefineEmpty {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.var.borrow().clone()
    }
    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(vec![LegacyValue::MutableReference(self.var.clone())])
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for VariableDefineEmpty {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let value = self.var.to_value();
        let variable_register = compile_value_register(&value, self.var.addr(), ctx)?;
        let variable_name = self.name.borrow().clone();
        let variable_mutable = *self.mutable.borrow();
        ctx.define_symbol(
            self.var.addr(),
            variable_register,
            &variable_name,
            variable_mutable,
        );
        let name = "VariableDefineEmpty".to_string();
        compile_binop!(name, self.var, self.name, self.mutable, ctx);
    }
}

#[cfg(test)]
mod empty_transaction_state_tests {
    use super::*;

    #[test]
    fn variable_define_empty_exposes_original_outer_value_cell() {
        let var = Ref::new(LegacyValue::Empty);
        let function = VariableDefineEmpty {
            id: 1,
            name: Ref::new("value".to_string()),
            mutable: Ref::new(true),
            var: var.clone(),
        };
        let values = function.transaction_state_values().unwrap();
        assert_eq!(values.len(), 1);
        match &values[0] {
            LegacyValue::MutableReference(value) => assert_eq!(value.addr(), var.addr()),
            value => panic!("expected mutable-reference transaction state, got {value:?}"),
        }
    }
}

#[macro_export]
macro_rules! impl_variable_define_match_arms {
  ($arg:expr, $value_kind:ty, $feature:expr) => {
    paste::paste! {
      match $arg {
        #[cfg(feature = $feature)]
        (LegacyValue::[<$value_kind:camel>](sink), name, mutable, id) => box_mech_fxn(Ok(Box::new([<VariableDefine $value_kind:camel>]{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id } ))),
        #[cfg(all(
          feature = $feature,
          any(feature = "matrix1", feature = "variable_define_matrix1")
        ))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        (sink, name, mutable, id) => Err(MechError::new(
            UnhandledFunctionArgumentKind3 {arg: (sink.kind(), name.kind(), mutable.kind()), fxn_name: "var/define".to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  };
}

fn impl_var_define_fxn(
    var: LegacyValue,
    name: LegacyValue,
    mutable: LegacyValue,
    id: u64,
) -> MResult<Box<dyn MechFunction>> {
    let arg = (var.clone(), name.clone(), mutable.clone(), id);
    match arg {
        (LegacyValue::Kind(kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(LegacyValue::Kind(kind)),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (LegacyValue::Empty, name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(LegacyValue::Empty),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (LegacyValue::Typed(value, kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(LegacyValue::Typed(value.clone(), kind.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (LegacyValue::EmptyKind(kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(LegacyValue::EmptyKind(kind.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "matrix")]
        (LegacyValue::MatrixValue(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(LegacyValue::MatrixValue(sink.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "table")]
        (LegacyValue::Table(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechTable {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "set")]
        (LegacyValue::Set(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechSet {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "tuple")]
        (LegacyValue::Tuple(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechTuple {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "record")]
        (LegacyValue::Record(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechRecord {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "map")]
        (LegacyValue::Map(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechMap {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "atom")]
        (LegacyValue::Atom(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechAtom {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "enum")]
        (LegacyValue::Enum(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechEnum {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        _ => (),
    }

    impl_variable_define_match_arms!(&arg, u8, "u8")
        .or_else(|_| impl_variable_define_match_arms!(&arg, u16, "u16"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, u32, "u32"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, u64, "u64"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, u128, "u128"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, i8, "i8"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, i16, "i16"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, i32, "i32"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, i64, "i64"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, i128, "i128"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, f32, "f32"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, f64, "f64"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, R64, "rational"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, C64, "complex"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, bool, "bool"))
        .or_else(|_| impl_variable_define_match_arms!(&arg, String, "string"))
        .map_err(|_| {
            MechError::new(
                UnhandledFunctionArgumentKind3 {
                    arg: (var.kind(), name.kind(), mutable.kind()),
                    fxn_name: "var/define".to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

macro_rules! install_variable_define_scalar_runtime {
    ($builder:expr, $kind:ident) => {
        paste! {
            [<register_variable_define_ $kind:lower>]($builder)?;
        }
    };
}

macro_rules! install_variable_define_matrix_runtime {
    ($builder:ident, $kind_token:ident) => {
        for_each_variable_define_matrix_shape!(
            register_variable_define_matrix_native,
            ($builder; $kind_token)
        );
    };
}

#[cfg(feature = "native-plan")]
pub(super) fn install_native_plan_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(all(feature = "variable_define_matrix1", not(feature = "matrix1")))]
    {
        macro_rules! install_matrix1_kind {
            ($feature:literal, $kind_token:ident) => {
                #[cfg(feature = $feature)]
                paste! {
                    [<register_variable_define_matrix_ $kind_token:lower _matrix1>](builder)?;
                }
            };
        }

        install_matrix1_kind!("u8", u8);
        install_matrix1_kind!("u16", u16);
        install_matrix1_kind!("u32", u32);
        install_matrix1_kind!("u64", u64);
        install_matrix1_kind!("u128", u128);
        install_matrix1_kind!("i8", i8);
        install_matrix1_kind!("i16", i16);
        install_matrix1_kind!("i32", i32);
        install_matrix1_kind!("i64", i64);
        install_matrix1_kind!("i128", i128);
        install_matrix1_kind!("f32", f32);
        install_matrix1_kind!("f64", f64);
        install_matrix1_kind!("r64", R64);
        install_matrix1_kind!("c64", C64);
        install_matrix1_kind!("bool", bool);
        install_matrix1_kind!("string", String);
    }

    Ok(())
}

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! install_kind {
        ($feature:literal, $kind:ty, $kind_token:ident, $kind_name:literal) => {
            #[cfg(feature = $feature)]
            {
                install_variable_define_scalar_runtime!(builder, $kind_token);
                #[cfg(feature = "matrix")]
                install_variable_define_matrix_runtime!(builder, $kind_token);
            }
        };
    }

    install_kind!("u8", u8, u8, "u8");
    install_kind!("u16", u16, u16, "u16");
    install_kind!("u32", u32, u32, "u32");
    install_kind!("u64", u64, u64, "u64");
    install_kind!("u128", u128, u128, "u128");
    install_kind!("i8", i8, i8, "i8");
    install_kind!("i16", i16, i16, "i16");
    install_kind!("i32", i32, i32, "i32");
    install_kind!("i64", i64, i64, "i64");
    install_kind!("i128", i128, i128, "i128");
    install_kind!("f32", f32, f32, "f32");
    #[cfg(feature = "f64")]
    {
        register_variable_define_f64(builder)?;
        #[cfg(feature = "matrix")]
        install_variable_define_matrix_runtime!(builder, f64);
    }
    #[cfg(feature = "r64")]
    install_variable_define_scalar_runtime!(builder, R64);
    #[cfg(all(feature = "matrix", feature = "rational"))]
    install_variable_define_matrix_runtime!(builder, R64);

    #[cfg(feature = "c64")]
    install_variable_define_scalar_runtime!(builder, C64);
    #[cfg(all(feature = "matrix", feature = "complex"))]
    install_variable_define_matrix_runtime!(builder, C64);

    install_kind!("bool", bool, bool, "bool");
    install_kind!("string", String, String, "string");

    #[cfg(feature = "table")]
    install_variable_define_scalar_runtime!(builder, MechTable);
    #[cfg(feature = "set")]
    install_variable_define_scalar_runtime!(builder, MechSet);
    #[cfg(feature = "tuple")]
    install_variable_define_scalar_runtime!(builder, MechTuple);
    #[cfg(feature = "record")]
    install_variable_define_scalar_runtime!(builder, MechRecord);
    #[cfg(feature = "map")]
    install_variable_define_scalar_runtime!(builder, MechMap);
    #[cfg(feature = "atom")]
    install_variable_define_scalar_runtime!(builder, MechAtom);
    #[cfg(feature = "enum")]
    install_variable_define_scalar_runtime!(builder, MechEnum);

    register_variable_define_empty(builder)?;
    Ok(())
}

pub struct VarDefine {}
impl FunctionSpecializer for VarDefine {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 3 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let var = arguments[0].clone();
        let name = &arguments[1].clone();
        let mutable = &arguments[2].clone();
        let name_string = name.as_string()?;
        let id = hash_str(&name_string.borrow());

        match impl_var_define_fxn(var.clone(), name.clone(), mutable.clone(), id) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (var) {
                (LegacyValue::MutableReference(input)) => {
                    impl_var_define_fxn(input.borrow().clone(), name.clone(), mutable.clone(), id)
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind3 {
                        arg: (var.kind(), name.kind(), mutable.kind()),
                        fxn_name: "var/define".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
