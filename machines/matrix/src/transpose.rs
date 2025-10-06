use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
  ($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).transpose(); }
  };}

#[macro_export]  
macro_rules! impl_transpose {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + 'static + 
      ConstElem + CompileConst + AsValueKind +
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Unary(out, arg) => {
            let arg: Ref<$arg_type> = unsafe{ arg.as_unchecked().clone() };
            let out: Ref<$out_type> = unsafe{ out.as_unchecked().clone() };
            Ok(Box::new($struct_name{arg, out}))
          }
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Expected unary arguments, got {:#?}", args), id: line!(), kind: MechErrorKind::None }),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T>
    where
      T: ConstElem + CompileConst + AsValueKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx, $feature_flag);
      }
    }
    register_fxn_descriptor!($struct_name, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64", bool, "bool", String, "string", C64, "complex", R64, "rational");
  };
}

#[cfg(feature = "matrix1")]
impl_transpose!(TransposeM1, Matrix1<T>, Matrix1<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(feature = "matrix2")]
impl_transpose!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(feature = "matrix3")]
impl_transpose!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(feature = "matrix4")]
impl_transpose!(TransposeM4, Matrix4<T>, Matrix4<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))]
impl_transpose!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))]
impl_transpose!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(feature = "matrixd")]
impl_transpose!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "vector2", feature = "row_vector2"))]
impl_transpose!(TransposeV2, Vector2<T>, RowVector2<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "vector3", feature = "row_vector3"))]
impl_transpose!(TransposeV3, Vector3<T>, RowVector3<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "vector4", feature = "row_vector4"))]
impl_transpose!(TransposeV4, Vector4<T>, RowVector4<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "vectord", feature = "row_vectord"))]
impl_transpose!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "row_vector2", feature = "vector2"))]
impl_transpose!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "row_vector3", feature = "vector3"))]
impl_transpose!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "row_vector4", feature = "vector4"))]
impl_transpose!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));
#[cfg(all(feature = "row_vectord", feature = "vectord"))]
impl_transpose!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op, FeatureFlag::Builtin(FeatureKind::Transpose));

macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{ 
      match $arg {
        $(
          $(
            #[cfg(all(feature = "row_vector4", feature = "vector4", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(TransposeR4{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = "row_vector3", feature = "vector3", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(TransposeR3{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = "row_vector2", feature = "vector2", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(TransposeR2{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector4", feature = "row_vector4", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(TransposeV4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector3", feature = "row_vector3", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(TransposeV3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector2", feature = "row_vector2", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(TransposeV2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = "matrix4", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(TransposeM4{arg: arg.clone(), out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix3", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(TransposeM3{arg: arg.clone(), out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix2", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(TransposeM2{arg: arg.clone(), out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix1", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(TransposeM1{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(TransposeM2x3{arg: arg.clone(), out: Ref::new(Matrix3x2::from_element($target_type::default()))})),          
            #[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(TransposeM3x2{arg: arg.clone(), out: Ref::new(Matrix2x3::from_element($target_type::default()))})),          
            #[cfg(all(feature = "vectord", feature = "row_vectord", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$target_type::default())) })),
            #[cfg(all(feature = "vectord", feature = "row_vectord", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())) })),
            #[cfg(all(feature = "matrixd", feature = $value_string))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new(TransposeMD{arg, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:#?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_transpose_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_transpose_match_arms!(
    (lhs_value),
    Bool,   bool,   "bool";
    I8,     i8,     "i8";
    I16,    i16,    "i16";
    I32,    i32,    "i32";
    I64,    i64,    "i64";
    I128,   i128,   "i128";
    U8,     u8,     "u8";
    U16,    u16,    "u16";
    U32,    u32,    "u32";
    U64,    u64,    "u64";
    U128,   u128,   "u128";
    F32,    F32,    "f32";
    F64,    F64,    "f64";
    String, String, "string";
    C64, C64, "complex";
    R64, R64, "rational";
  )
}
  
impl_mech_urnop_fxn!(MatrixTranspose,impl_transpose_fxn,"matrix/transpose");  