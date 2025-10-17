#[macro_use]
use crate::stdlib::*;

#[macro_export]
macro_rules! register_define {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>>::new,
        }
      }
    }
  };
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
  T: Debug + Clone + Sync + Send + 'static + 
  CompileConst + ConstElem + AsValueKind,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  MatA: Debug + CompileConst + ConstElem + AsNaKind + 'static,
  Ref<MatA>: ToValue
{
  fn new(args: FunctionArgs) -> Result<Box<dyn MechFunction>, MechError> {
    match args {
      FunctionArgs::Binary(var, arg1, arg2) => {
        let var: Ref<MatA> = unsafe { var.as_unchecked() }.clone();
        let name: Ref<String> = unsafe { arg1.as_unchecked() }.clone();
        let mutable: Ref<bool> = unsafe { arg2.as_unchecked() }.clone();
        let id = hash_str(&name.borrow());
        Ok(Box::new(Self {id, name, mutable, var, _marker: PhantomData::default() }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T, MatA> MechFunctionImpl for VariableDefineMatrix<T, MatA>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + 
  CompileConst + ConstElem + AsValueKind,
  MatA: Debug,
{
  fn solve(&self) {}
  fn out(&self) -> Value {self.var.to_value()}
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T, MatA> MechFunctionCompiler for VariableDefineMatrix<T, MatA> 
where
  T: CompileConst + ConstElem + AsValueKind,
  MatA: CompileConst + ConstElem + AsNaKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("VariableDefineMatrix<{}{}>", T::as_value_kind(), MatA::as_na_kind());
    compile_binop!(name, self.var, self.name, self.mutable, ctx, FeatureFlag::Builtin(FeatureKind::VariableDefine) );
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
            _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "VariableDefine requires 3 arguments: (var, name, mutable)".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments}),
          }
        }
      }
      impl MechFunctionImpl for [<VariableDefine $kind:camel>] {
        fn solve(&self) {}
        fn out(&self) -> Value { self.var.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
      #[cfg(feature = "compiler")]
      impl MechFunctionCompiler for [<VariableDefine $kind:camel>] {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          let name = format!(stringify!([<VariableDefine $kind:camel>]));
          compile_binop!(name, self.var, self.name, self.mutable, ctx, FeatureFlag::Builtin(FeatureKind::VariableDefine) );
        }
      }
      inventory::submit! {
        FunctionDescriptor {
          name: stringify!([<VariableDefine $kind:camel>]),
          ptr: [<VariableDefine $kind:camel>]::new,
        }
      }
    }
  };
}

#[cfg(feature = "f64")]
impl_variable_define_fxn!(F64);
#[cfg(feature = "f32")]
impl_variable_define_fxn!(F32);
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
//#[cfg(feature = "tuple")]
//impl_variable_define_fxn!(MechTuple);
#[cfg(feature = "record")]
impl_variable_define_fxn!(MechRecord);

#[macro_export]
macro_rules! impl_variable_define_match_arms {
  ($arg:expr, $value_kind:ty, $feature:expr) => {
    paste::paste! {
      match $arg {
        #[cfg(feature = $feature)]
        (Value::[<$value_kind:camel>](sink), name, mutable, id) => box_mech_fxn(Ok(Box::new([<VariableDefine $value_kind:camel>]{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id } ))),
        #[cfg(all(feature = $feature, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix1);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix2);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix2x3);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix3x2);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix3);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Matrix4);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, DMatrix);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Vector2);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Vector3);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, Vector4);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, DVector);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, RowVector2);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, RowVector3);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, RowVector4);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        #[cfg(all(feature = $feature, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), name, mutable, id) => {
          register_define!(VariableDefineMatrix, $value_kind, $feature, RowDVector);
          box_mech_fxn(Ok(Box::new(VariableDefineMatrix{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id: *id, _marker: PhantomData::<$value_kind>::default() })))
        },
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}

fn impl_var_define_fxn(var: Value, name: Value, mutable: Value, id: u64) -> MResult<Box<dyn MechFunction>> {
  let arg = (var, name, mutable, id);
  match arg {
    #[cfg(feature = "table")]
    (Value::Table(sink), name, mutable, id) => return box_mech_fxn(Ok(Box::new(VariableDefineMechTable{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id } ))),
    #[cfg(feature = "set")]
    (Value::Set(sink), name, mutable, id) => return box_mech_fxn(Ok(Box::new(VariableDefineMechSet{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id } ))),
    //#[cfg(feature = "tuple")]
    //(Value::Tuple(sink), name, mutable, id) => return box_mech_fxn(Ok(Box::new(VariableDefineMechTuple{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id } ))),
    #[cfg(feature = "record")]
    (Value::Record(sink), name, mutable, id) => return box_mech_fxn(Ok(Box::new(VariableDefineMechRecord{ var: sink.clone(), name: name.as_string()?, mutable: mutable.as_bool()?, id } ))),
    _ => (),
  }


                 impl_variable_define_match_arms!(&arg, u8,   "u8")
  .or_else(|_| impl_variable_define_match_arms!(&arg, u16,  "u16"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, u32,  "u32"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, u64,  "u64"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, u128, "u128"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, i8,   "i8"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, i16,  "i16"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, i32,  "i32"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, i64,  "i64"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, i128, "i128"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, F32,  "f32"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, F64,  "f64"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, R64,  "rational"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, C64,  "complex"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, bool, "bool"))
  .or_else(|_| impl_variable_define_match_arms!(&arg, String, "string"))
  .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
}


pub struct VarDefine{}
impl NativeFunctionCompiler for VarDefine {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 3 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let var = arguments[0].clone();
    let name = &arguments[1].clone();
    let mutable = &arguments[2].clone();
    let name_string = name.as_string()?;
    let id = hash_str(&name_string.borrow());
    
    match impl_var_define_fxn(var.clone(), name.clone(), mutable.clone(), id) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (var) {
          (Value::MutableReference(input)) => {impl_var_define_fxn(input.borrow().clone(), name.clone(), mutable.clone(), id)}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}