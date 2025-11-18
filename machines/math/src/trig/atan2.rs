use crate::*;
use mech_core::*;
use libm::{atan2,atan2f};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Atan2 ------------------------------------------------------------------------

macro_rules! atan2_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = atan2((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! atan2_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = atan2(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! atan2f_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = atan2f((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! atan2f_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = atan2f(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! impl_two_arg_fxn {
  ($struct_name:ident, $kind1:ty, $kind2:ty, $out_kind:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg1: Ref<$kind1>,
      arg2: Ref<$kind2>,
      out: Ref<$out_kind>,
    }
    impl MechFunctionFactory for $struct_name {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let arg1: Ref<$kind1> = unsafe{ arg1.as_unchecked().clone() };
            let arg2: Ref<$kind2> = unsafe{ arg2.as_unchecked().clone() };
            let out: Ref<$out_kind> = unsafe{ out.as_unchecked().clone() };
            Ok(Box::new($struct_name {arg1, arg2, out}))
          },
          _ => Err(MechError2::new(
              IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
    impl MechFunctionImpl for $struct_name {
      fn solve(&self) {
        let arg1_ptr = self.arg1.as_ptr();
        let arg2_ptr = self.arg2.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg1_ptr,arg2_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for $struct_name {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let mut registers = [0,0,0];
  
        registers[0] = compile_register_brrw!(self.out,  ctx);
        registers[1] = compile_register_brrw!(self.arg1, ctx);
        registers[2] = compile_register_brrw!(self.arg2, ctx);

        ctx.features.insert(FeatureFlag::Custom(hash_str("math/atan2")));

        ctx.emit_binop(
          hash_str(stringify!($struct_name)),
          registers[0],
          registers[1],
          registers[2],
        );

        return Ok(registers[0])
      }
    }
    register_descriptor!{
      FunctionDescriptor {
        name: stringify!($struct_name),
        ptr: $struct_name::new,
      }
    }
  };}

macro_rules! impl_atan2 {
  ($type:tt, $type_string:tt, $op:ident, $($struct_name:ident, $kind:ty, $feature:literal);* $(;)?) => {
    paste!{
      $(
        #[cfg(all(feature = $type_string, feature = $feature))]
        impl_two_arg_fxn!([<$struct_name $type>], $kind<$type>, $kind<$type>, $kind<$type>, $op);
      )*
    }
  };
}

impl_atan2!(
  F64, "f64", atan2_vec_op,
  Atan2M1, Matrix1, "matrix1";
  Atan2M2, Matrix2, "matrix2";
  Atan2M3, Matrix3, "matrix3";
  Atan2M2x3, Matrix2x3, "matrix2x3";
  Atan2M3x2, Matrix3x2, "matrix3";
  Atan2M4, Matrix4, "matrix4";
  Atan2V2, Vector2, "vector2";
  Atan2V3, Vector3, "vector3";
  Atan2V4, Vector4, "vector4";
  Atan2R2, RowVector2, "row_vector2";
  Atan2R3, RowVector3, "row_vector3";
  Atan2R4, RowVector4, "row_vector4";
  Atan2RD, RowDVector, "row_vectord";
  Atan2VD, DVector, "vectord";
  Atan2MD, DMatrix, "matrixd";
);

impl_atan2!(
  F32, "f32", atan2f_vec_op,
  Atan2M1, Matrix1, "matrix1";
  Atan2M2, Matrix2, "matrix2";
  Atan2M3, Matrix3, "matrix3";
  Atan2M2x3, Matrix2x3, "matrix2x3";
  Atan2M3x2, Matrix3x2, "matrix3";
  Atan2M4, Matrix4, "matrix4";
  Atan2V2, Vector2, "vector2";
  Atan2V3, Vector3, "vector3";
  Atan2V4, Vector4, "vector4";
  Atan2R2, RowVector2, "row_vector2";
  Atan2R3, RowVector3, "row_vector3";
  Atan2R4, RowVector4, "row_vector4";
  Atan2RD, RowDVector, "row_vectord";
  Atan2VD, DVector, "vectord";
  Atan2MD, DMatrix, "matrixd";
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(Atan2F32, F32, F32, F32, atan2f_op);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(Atan2F64, F64, F64, F64, atan2_op);

#[macro_export]
macro_rules! impl_binop_atan2 {
  ($fxn:ident, $arg1:expr, $arg2:expr, $($t:ident, $zero_fn:expr, $feat:tt);+ $(;)?) => {
    paste! {
      match ($arg1, $arg2) {
        $(
          // Scalar
          #[cfg(feature = $feat)]
          (Value::$t(arg1), Value::$t(arg2)) => Ok(Box::new([<$fxn $t>]{arg1, arg2, out: Ref::new($t::from($zero_fn))})),

          // Fixed matrices
          #[cfg(all(feature = "matrix1", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Matrix1(arg1)), Value::[<Matrix $t>](Matrix::Matrix1(arg2))) =>
            Ok(Box::new([<$fxn M1 $t>]{arg1, arg2, out: Ref::new(Matrix1::from_element($zero_fn))})),
          #[cfg(all(feature = "matrix2", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Matrix2(arg1)), Value::[<Matrix $t>](Matrix::Matrix2(arg2))) =>
            Ok(Box::new([<$fxn M2 $t>]{arg1, arg2, out: Ref::new(Matrix2::from_element($zero_fn))})),
          #[cfg(all(feature = "matrix3", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Matrix3(arg1)), Value::[<Matrix $t>](Matrix::Matrix3(arg2))) =>
            Ok(Box::new([<$fxn M3 $t>]{arg1, arg2, out: Ref::new(Matrix3::from_element($zero_fn))})),
          #[cfg(all(feature = "matrix4", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Matrix4(arg1)), Value::[<Matrix $t>](Matrix::Matrix4(arg2))) =>
            Ok(Box::new([<$fxn M4 $t>]{arg1, arg2, out: Ref::new(Matrix4::from_element($zero_fn))})),

          // Fixed vectors
          #[cfg(all(feature = "vector2", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Vector2(arg1)), Value::[<Matrix $t>](Matrix::Vector2(arg2))) =>
            Ok(Box::new([<$fxn V2 $t>]{arg1, arg2, out: Ref::new(Vector2::from_element($zero_fn))})),
          #[cfg(all(feature = "vector3", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Vector3(arg1)), Value::[<Matrix $t>](Matrix::Vector3(arg2))) =>
            Ok(Box::new([<$fxn V3 $t>]{arg1, arg2, out: Ref::new(Vector3::from_element($zero_fn))})),
          #[cfg(all(feature = "vector4", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::Vector4(arg1)), Value::[<Matrix $t>](Matrix::Vector4(arg2))) =>
            Ok(Box::new([<$fxn V4 $t>]{arg1, arg2, out: Ref::new(Vector4::from_element($zero_fn))})),

          // Fixed row vectors
          #[cfg(all(feature = "row_vector2", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::RowVector2(arg1)), Value::[<Matrix $t>](Matrix::RowVector2(arg2))) =>
            Ok(Box::new([<$fxn R2 $t>]{arg1, arg2, out: Ref::new(RowVector2::from_element($zero_fn))})),
          #[cfg(all(feature = "row_vector3", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::RowVector3(arg1)), Value::[<Matrix $t>](Matrix::RowVector3(arg2))) =>
            Ok(Box::new([<$fxn R3 $t>]{arg1, arg2, out: Ref::new(RowVector3::from_element($zero_fn))})),
          #[cfg(all(feature = "row_vector4", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::RowVector4(arg1)), Value::[<Matrix $t>](Matrix::RowVector4(arg2))) =>
            Ok(Box::new([<$fxn R4 $t>]{arg1, arg2, out: Ref::new(RowVector4::from_element($zero_fn))})),

          // Dynamic vectors
          #[cfg(all(feature = "vectord", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::DVector(arg1)), Value::[<Matrix $t>](Matrix::DVector(arg2))) =>
            Ok(Box::new([<$fxn VD $t>]{arg1: arg1.clone(), arg2, out: Ref::new(DVector::from_element(arg1.borrow().nrows(), $zero_fn))})),
          #[cfg(all(feature = "row_vectord", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::RowDVector(arg1)), Value::[<Matrix $t>](Matrix::RowDVector(arg2))) =>
            Ok(Box::new([<$fxn RD $t>]{arg1: arg1.clone(), arg2, out: Ref::new(RowDVector::from_element(arg1.borrow().ncols(), $zero_fn))})),

          // Dynamic matrices
          #[cfg(all(feature = "matrixd", feature = $feat))]
          (Value::[<Matrix $t>](Matrix::DMatrix(arg1)), Value::[<Matrix $t>](Matrix::DMatrix(arg2))) => {
            let rows = arg1.borrow().nrows();
            let cols = arg1.borrow().ncols();
            Ok(Box::new([<$fxn MD $t>]{arg1, arg2, out: Ref::new(DMatrix::from_element(rows, cols, $zero_fn))}))
          },
        )+
        (arg1,arg2) => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

pub fn impl_atan2_fxn(arg1_value: Value, arg2_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_binop_atan2!(Atan2, arg1_value, arg2_value,
    F32, F32::default(), "f32";
    F64, F64::default(), "f64";
  )
}

pub struct MathAtan2 {}

impl NativeFunctionCompiler for MathAtan2 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() },None).with_compiler_loc());
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match impl_atan2_fxn(arg1.clone(), arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.borrow().clone())}
          (Value::MutableReference(arg1),arg2) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.clone())}
          (arg1,Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.clone(),arg2.borrow().clone())}
          (arg1,arg2) => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: "math/atan2".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/atan2",
    ptr: &MathAtan2{},
  }
}