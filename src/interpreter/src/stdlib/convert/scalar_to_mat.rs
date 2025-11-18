#[macro_use]
use crate::stdlib::*;
use nalgebra::Scalar;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ConvertScalarToMat2<F, T> {
  pub arg: Ref<F>,
  pub out: Ref<T>,
}

impl<F, T> MechFunctionImpl for ConvertScalarToMat2<F, T>
where
  Ref<T>: ToValue,
  F: Debug + Scalar + Clone,
  for<'a> &'a mut T: IntoIterator<Item = &'a mut F>,
  T: Debug,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      let arg_ref: &F = &*arg_ptr;
      let out_ref: &mut T = &mut *out_ptr;
      for dst in (&mut *out_ref).into_iter() {
        *dst = arg_ref.clone();
      }
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String { format!("{:#?}",self) }
}
#[cfg(feature = "compiler")]
impl<F, T> MechFunctionCompiler for ConvertScalarToMat2<F, T>
where
  T: CompileConst + ConstElem + AsValueKind,
  F: ConstElem + CompileConst + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertScalarToMat2<{},{}>", F::as_value_kind(), T::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}

macro_rules! impl_conversion_scalar_to_mat_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident, $type_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = "matrix", feature = $type_string))]
            (Value::$input_type(v), ValueKind::Matrix(box ValueKind::$target_type, dims)) => {
              match dims[..] {
                #[cfg(feature = "matrix1")]
                [1,1] => {let out = Matrix1::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix2")]
                [2,2] => {let out = Matrix2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix3")]
                [3,3] => {let out = Matrix3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix4")]
                [4,4] => {let out = Matrix4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix2x3")]
                [2,3] => {let out = Matrix2x3::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix3x2")]
                [3,2] => {let out = Matrix3x2::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector2")]
                [1,2] => {let out = RowVector2::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector3")]
                [1,3] => {let out = RowVector3::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector4")]
                [1,4] => {let out = RowVector4::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector2")]
                [2,1] => {let out = Vector2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector3")]
                [3,1] => {let out = Vector3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector4")]
                [4,1] => {let out = Vector4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vectord")]
                [1,n] => {let out = RowDVector::from_element(n,v.borrow().clone()); return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vectord")]
                [n,1] => {let out = DVector::from_element(n,v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrixd")]
                [n,m] => {let out = DMatrix::from_element(n,m,v.borrow().clone());  return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                [] => {return Err(MechError2::new(
                  UnsupportedConversionError{from: ValueKind::$input_type, to: ValueKind::Matrix(box ValueKind::$target_type, dims.clone())},
                  Some("Cannot convert scalar to matrix with zero dimensions".to_string())
                ).with_compiler_loc());}
                _ => todo!(),
              }
            }
          )+
        )+
        x => Err(MechError2::new(
            UnsupportedConversionError{from: x.0.kind(), to: x.1.clone()},
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_conversion_scalar_to_mat_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  impl_conversion_scalar_to_mat_match_arms!(
    (source_value, target_kind),
    Bool => Bool, "bool";
    U8 => U8, "u8";
    U16 => U16, "u16";
    U32 => U32, "u32";
    U64 => U64, "u64";
    U128 => U128, "u128";
    I8 => I8, "i8";
    I16 => I16, "i16";
    I32 => I32, "i32";
    I64 => I64, "i64";
    I128 => I128, "i128";
    F32 => F32, "f32";
    F64 => F64, "f64";
    String => String, "string";
    R64 => R64, "rational";
    C64 => C64, "complex";
  )
}

pub struct ConvertScalarToMat {}

impl NativeFunctionCompiler for ConvertScalarToMat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match impl_conversion_scalar_to_mat_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_scalar_to_mat_fxn(rhs.borrow().clone(), target_kind.clone()),
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (arguments[0].clone(), arguments[1].clone()), fxn_name: "convert/scalar-to-mat".to_string() },
              None,
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}