#[macro_use]
use crate::intrinsics::*;

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
    MatA: Debug + ConstElem + AsNaKind + 'static,
    #[cfg(feature = "compiler")]
    MatA: CompileConst,
    Ref<MatA>: ToValue,
{
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(var, arg1, arg2) => {
                let var: Ref<MatA> = unsafe { var.as_unchecked() }.clone();
                let name: Ref<String> = unsafe { arg1.as_unchecked() }.clone();
                let mutable: Ref<bool> = unsafe { arg2.as_unchecked() }.clone();
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
    fn solve(&self) {}
    fn out(&self) -> Value {
        self.var.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
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
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
              let var: Ref<$kind> = unsafe { out.as_unchecked() }.clone();
              let name: Ref<String> = unsafe { arg1.as_unchecked() }.clone();
              let mutable: Ref<bool> = unsafe { arg2.as_unchecked() }.clone();
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
        fn solve(&self) {}
        fn out(&self) -> Value { self.var.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
          Ok(self.reactive_output_values())
        }
      }
      #[cfg(feature = "compiler")]
      impl MechFunctionCompiler for [<VariableDefine $kind:camel>] {
      fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let variable_register = compile_register_brrw!(self.var, ctx);
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
    factory: <VariableDefineF64 as MechFunctionFactory>::new,

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_f64",

    cargo_features: &[
        "bool",
        "f64",
        "native-link",
        "runtime",
        "string",
        "variable_define",
    ],
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
    ($feature:literal, $kind:ident, [$($cargo_feature:literal),+ $(,)?]) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "variable_define", feature = $feature),
                registration: [<register_variable_define_ $kind:lower>],
                installer: [<install_variable_define_ $kind:lower>],
                name: stringify!([<VariableDefine $kind:camel>]),
                factory: <[<VariableDefine $kind:camel>] as MechFunctionFactory>::new,
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_",
                    stringify!([<$kind:lower>]),
                ),
                cargo_features: [$($cargo_feature),+],
            }
        }
    };
}

declare_variable_define_scalar_native!(
    "f32",
    f32,
    [
        "bool",
        "f32",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "u8",
    u8,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u8",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "u16",
    u16,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u16",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "u32",
    u32,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u32",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "u64",
    u64,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u64",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "u128",
    u128,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u128",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "i8",
    i8,
    [
        "bool",
        "i8",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "i16",
    i16,
    [
        "bool",
        "i16",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "i32",
    i32,
    [
        "bool",
        "i32",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "i64",
    i64,
    [
        "bool",
        "i64",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "i128",
    i128,
    [
        "bool",
        "i128",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "r64",
    R64,
    [
        "bool",
        "native-link",
        "r64",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "c64",
    C64,
    [
        "bool",
        "c64",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "bool",
    bool,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "string",
    String,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "table",
    MechTable,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "table",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "set",
    MechSet,
    [
        "bool",
        "native-link",
        "runtime",
        "set",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "tuple",
    MechTuple,
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "tuple",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "record",
    MechRecord,
    [
        "bool",
        "native-link",
        "record",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "map",
    MechMap,
    [
        "bool",
        "map",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "atom",
    MechAtom,
    [
        "atom",
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_scalar_native!(
    "enum",
    MechEnum,
    [
        "bool",
        "enum",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);

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
        #[cfg(feature = "matrix1")]
        $callback!($context, Matrix1, "matrix1");
        #[cfg(feature = "matrix2")]
        $callback!($context, Matrix2, "matrix2");
        #[cfg(feature = "matrix2x3")]
        $callback!($context, Matrix2x3, "matrix2x3");
        #[cfg(feature = "matrix3x2")]
        $callback!($context, Matrix3x2, "matrix3x2");
        #[cfg(feature = "matrix3")]
        $callback!($context, Matrix3, "matrix3");
        #[cfg(feature = "matrix4")]
        $callback!($context, Matrix4, "matrix4");
        #[cfg(feature = "matrixd")]
        $callback!($context, DMatrix, "matrixd");
        #[cfg(feature = "vector2")]
        $callback!($context, Vector2, "vector2");
        #[cfg(feature = "vector3")]
        $callback!($context, Vector3, "vector3");
        #[cfg(feature = "vector4")]
        $callback!($context, Vector4, "vector4");
        #[cfg(feature = "vectord")]
        $callback!($context, DVector, "vectord");
        #[cfg(feature = "row_vector2")]
        $callback!($context, RowVector2, "row_vector2");
        #[cfg(feature = "row_vector3")]
        $callback!($context, RowVector3, "row_vector3");
        #[cfg(feature = "row_vector4")]
        $callback!($context, RowVector4, "row_vector4");
        #[cfg(feature = "row_vectord")]
        $callback!($context, RowDVector, "row_vectord");
    };
}

macro_rules! declare_variable_define_matrix_native {
    (
        ($value_feature:literal; $kind:ty; $kind_token:ident; $kind_name:literal; [$($cargo_feature:literal),+]),
        $shape:ident,
        $shape_feature:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "variable_define",
                    feature = $value_feature,
                    feature = $shape_feature
                ),
                registration: [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                installer: [<install_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                name: concat!("VariableDefineMatrix<", $kind_name, stringify!($shape), ">"),
                factory: VariableDefineMatrix::<$kind, $shape<$kind>>::new,
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_matrix_",
                    stringify!([<$kind_token:lower>]), "_", stringify!([<$shape:lower>]),
                ),
                cargo_features: [$($cargo_feature,)+ $shape_feature],
            }
        }
    };
}

macro_rules! declare_variable_define_matrix_for_type {
    ($value_feature:literal, $kind:ty, $kind_token:ident, $kind_name:literal, [$($cargo_feature:literal),+ $(,)?]) => {
        for_each_variable_define_matrix_shape!(
            declare_variable_define_matrix_native,
            ($value_feature; $kind; $kind_token; $kind_name; [$($cargo_feature),+])
        );
    };
}

declare_variable_define_matrix_for_type!(
    "u8",
    u8,
    u8,
    "u8",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u8",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "u16",
    u16,
    u16,
    "u16",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u16",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "u32",
    u32,
    u32,
    "u32",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u32",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "u64",
    u64,
    u64,
    "u64",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u64",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "u128",
    u128,
    u128,
    "u128",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "u128",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "i8",
    i8,
    i8,
    "i8",
    [
        "bool",
        "i8",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "i16",
    i16,
    i16,
    "i16",
    [
        "bool",
        "i16",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "i32",
    i32,
    i32,
    "i32",
    [
        "bool",
        "i32",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "i64",
    i64,
    i64,
    "i64",
    [
        "bool",
        "i64",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "i128",
    i128,
    i128,
    "i128",
    [
        "bool",
        "i128",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "f32",
    f32,
    f32,
    "f32",
    [
        "bool",
        "f32",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "f64",
    f64,
    f64,
    "f64",
    [
        "bool",
        "f64",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "r64",
    R64,
    R64,
    "rational",
    [
        "bool",
        "native-link",
        "r64",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "c64",
    C64,
    C64,
    "complex",
    [
        "bool",
        "c64",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "bool",
    bool,
    bool,
    "bool",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);
declare_variable_define_matrix_for_type!(
    "string",
    String,
    String,
    "string",
    [
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define"
    ]
);

macro_rules! register_variable_define_matrix_native {
    (
        ($builder:ident; $kind_token:ident),
        $shape:ident,
        $_shape_feature:literal
    ) => {
        paste! {
            [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>]($builder)?;
        }
    };
}

macro_rules! export_variable_define_matrix_native {
    (($kind_token:ident), $shape:ident, $_shape_feature:literal) => {
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
    var: Ref<Value>,
}

fn variable_define_empty_factory(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
        FunctionArgs::Binary(var, arg1, arg2) => {
            let var: Ref<Value> = unsafe { var.as_unchecked() }.clone();
            let name: Ref<String> = unsafe { arg1.as_unchecked() }.clone();
            let mutable: Ref<bool> = unsafe { arg2.as_unchecked() }.clone();
            let id = hash_str(&name.borrow());
            Ok(Box::new(VariableDefineEmpty {
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

mech_core::declare_native_runtime_factory! {
    cfg: feature = "variable_define",
    registration: register_variable_define_empty,
    installer: install_variable_define_empty,
    name: "VariableDefineEmpty",
    factory: variable_define_empty_factory,
    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_empty",
    cargo_features: [
        "bool",
        "native-link",
        "runtime",
        "string",
        "variable_define",
    ],
}
impl MechFunctionImpl for VariableDefineEmpty {
    fn solve(&self) {}
    fn out(&self) -> Value {
        self.var.borrow().clone()
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::MutableReference(self.var.clone())])
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for VariableDefineEmpty {
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
        let name = "VariableDefineEmpty".to_string();
        compile_binop!(name, self.var, self.name, self.mutable, ctx);
    }
}

#[cfg(test)]
mod empty_transaction_state_tests {
    use super::*;

    #[test]
    fn variable_define_empty_exposes_original_outer_value_cell() {
        let var = Ref::new(Value::Empty);
        let function = VariableDefineEmpty {
            id: 1,
            name: Ref::new("value".to_string()),
            mutable: Ref::new(true),
            var: var.clone(),
        };
        let values = function.transaction_state_values().unwrap();
        assert_eq!(values.len(), 1);
        match &values[0] {
            Value::MutableReference(value) => assert_eq!(value.addr(), var.addr()),
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
        (Value::[<$value_kind:camel>](sink), name, mutable, id) => box_mech_fxn(Ok(Box::new([<VariableDefine $value_kind:camel>]{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id } ))),
        #[cfg(all(feature = $feature, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), name, mutable, id) => {
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), name, mutable, id) => {
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
    var: Value,
    name: Value,
    mutable: Value,
    id: u64,
) -> MResult<Box<dyn MechFunction>> {
    let arg = (var.clone(), name.clone(), mutable.clone(), id);
    match arg {
        (Value::Kind(kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(Value::Kind(kind)),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (Value::Empty, name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(Value::Empty),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (Value::Typed(value, kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(Value::Typed(value.clone(), kind.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        (Value::EmptyKind(kind), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(Value::EmptyKind(kind.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "matrix")]
        (Value::MatrixValue(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineEmpty {
                var: Ref::new(Value::MatrixValue(sink.clone())),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "table")]
        (Value::Table(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechTable {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "set")]
        (Value::Set(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechSet {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "tuple")]
        (Value::Tuple(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechTuple {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "record")]
        (Value::Record(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechRecord {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "map")]
        (Value::Map(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechMap {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "atom")]
        (Value::Atom(sink), name, mutable, id) => {
            return box_mech_fxn(Ok(Box::new(VariableDefineMechAtom {
                var: sink.clone(),
                name: name.as_string()?,
                mutable: mutable.as_bool()?,
                id,
            })));
        }
        #[cfg(feature = "enum")]
        (Value::Enum(sink), name, mutable, id) => {
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
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
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
                (Value::MutableReference(input)) => {
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
