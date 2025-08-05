#[macro_use]
use crate::stdlib::*;
use nalgebra::Scalar;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ConvertScalarToMat2<F, T> {
  pub arg: Ref<F>,
  pub out: Ref<T>,
}

impl<F, T> MechFunction for ConvertScalarToMat2<F, T>
where
  Ref<T>: ToValue,
  F: Debug + Scalar + Clone,
  for<'a> &'a mut T: IntoIterator<Item = &'a mut F>,
  T: Debug,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let arg_ref: &F = &*arg_ptr;
      let out_ref: &mut T = &mut *out_ptr;
      for dst in (&mut *out_ref).into_iter() {
        *dst = arg_ref.clone();
      }
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String {format!("{:#?}",self)}
}

macro_rules! impl_conversion_scalar_to_mat_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$input_type(v), ValueKind::Matrix(box ValueKind::$target_type, dims)) => {
              match dims[..] {
                [1,1] => {let out = Matrix1::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [2,2] => {let out = Matrix2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [3,3] => {let out = Matrix3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [4,4] => {let out = Matrix4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [2,3] => {let out = Matrix2x3::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [3,2] => {let out = Matrix3x2::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [1,2] => {let out = RowVector2::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [1,3] => {let out = RowVector3::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [1,4] => {let out = RowVector4::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [2,1] => {let out = Vector2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [3,1] => {let out = Vector3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [4,1] => {let out = Vector4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [1,n] => {let out = RowDVector::from_element(n,v.borrow().clone()); return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [n,1] => {let out = DVector::from_element(n,v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [n,m] => {let out = DMatrix::from_element(n,m,v.borrow().clone());  return Ok(Box::new(ConvertScalarToMat2{arg: v, out: new_ref(out)}));},
                [] => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Cannot convert to zero-dimension matrix".to_string(), id: line!(), kind: MechErrorKind::None});},
                _ => todo!(),
              }
            }
          )+
        )+
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_conversion_scalar_to_mat_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  impl_conversion_scalar_to_mat_match_arms!(
    (source_value, target_kind),
    Bool => Bool;
    U8 => U8;
    U16 => U16;
    U32 => U32;
    U64 => U64;
    U128 => U128;
    I8 => I8;
    I16 => I16;
    I32 => I32;
    I64 => I64;
    I128 => I128;
    F32 => F32;
    F64 => F64;
    String => String;
    RationalNumber => RationalNumber;
    ComplexNumber => ComplexNumber;
  )
}

pub struct ConvertScalarToMat {}

impl NativeFunctionCompiler for ConvertScalarToMat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match impl_conversion_scalar_to_mat_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_scalar_to_mat_fxn(rhs.borrow().clone(), target_kind.clone()),
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}