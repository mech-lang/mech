#[macro_use]
use crate::stdlib::*;

macro_rules! impl_convert_scalar_to_vec {
  ($name:ident, $scalar_type:ty, $vector_type:ty) => {
    #[derive(Debug)]
    struct $name {
      arg: Ref<$scalar_type>,
      out: Ref<$vector_type>,
    }
    impl MechFunction for $name
    where
      Ref<$vector_type>: ToValue,
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        unsafe {
          let out_ptr_deref = &mut *out_ptr;
          for i in 0..out_ptr_deref.len() {
            out_ptr_deref[i] = (*arg_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  };
}

macro_rules! for_all_scalar_types {
  ($macro:ident) => {
    $macro!(bool);
    $macro!(i8);
    $macro!(i16);
    $macro!(i32);
    $macro!(i64);
    $macro!(i128);
    $macro!(u8);
    $macro!(u16);
    $macro!(u32);
    $macro!(u64);
    $macro!(u128);  
    $macro!(F32);  
    $macro!(F64);
    $macro!(String);
  };
}

macro_rules! define_convert_for_type {
  ($scalar:ident) => {
      paste!{
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToR2>],   $scalar, RowVector2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToR3>],   $scalar, RowVector3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToR4>],   $scalar, RowVector4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToV2>],   $scalar, Vector2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToV3>],   $scalar, Vector3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToV4>],   $scalar, Vector4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM1>],   $scalar, Matrix1<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM2>],   $scalar, Matrix2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM3>],   $scalar, Matrix3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM4>],   $scalar, Matrix4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM3x2>], $scalar, Matrix3x2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToM2x3>], $scalar, Matrix2x3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToMD>],   $scalar, DMatrix<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToRD>],   $scalar, RowDVector<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar:upper ToVD>],   $scalar, DVector<$scalar>);
    }
  };
}

for_all_scalar_types!(define_convert_for_type);

macro_rules! impl_conversion_scalar_to_mat_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$input_type(v), ValueKind::Matrix(box ValueKind::$target_type, dims)) => {
              match dims[..] {
                [1,1] => {let out = Matrix1::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToM1>]{arg: v, out: new_ref(out)}));},
                [2,2] => {let out = Matrix2::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToM2>]{arg: v, out: new_ref(out)}));},
                [3,3] => {let out = Matrix3::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToM3>]{arg: v, out: new_ref(out)}));},
                [4,4] => {let out = Matrix4::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToM4>]{arg: v, out: new_ref(out)}));},
                [2,3] => {let out = Matrix2x3::from_element(v.borrow().clone());    return Ok(Box::new([<Convert $input_type:upper ToM2x3>]{arg: v, out: new_ref(out)}));},
                [3,2] => {let out = Matrix3x2::from_element(v.borrow().clone());    return Ok(Box::new([<Convert $input_type:upper ToM3x2>]{arg: v, out: new_ref(out)}));},
                [1,2] => {let out = RowVector2::from_element(v.borrow().clone());   return Ok(Box::new([<Convert $input_type:upper ToR2>]{arg: v, out: new_ref(out)}));},
                [1,3] => {let out = RowVector3::from_element(v.borrow().clone());   return Ok(Box::new([<Convert $input_type:upper ToR3>]{arg: v, out: new_ref(out)}));},
                [1,4] => {let out = RowVector4::from_element(v.borrow().clone());   return Ok(Box::new([<Convert $input_type:upper ToR4>]{arg: v, out: new_ref(out)}));},
                [2,1] => {let out = Vector2::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToV2>]{arg: v, out: new_ref(out)}));},
                [3,1] => {let out = Vector3::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToV3>]{arg: v, out: new_ref(out)}));},
                [4,1] => {let out = Vector4::from_element(v.borrow().clone());      return Ok(Box::new([<Convert $input_type:upper ToV4>]{arg: v, out: new_ref(out)}));},
                [1,n] => {let out = RowDVector::from_element(n,v.borrow().clone()); return Ok(Box::new([<Convert $input_type:upper ToRD>]{arg: v, out: new_ref(out)}));},
                [n,1] => {let out = DVector::from_element(n,v.borrow().clone());    return Ok(Box::new([<Convert $input_type:upper ToVD>]{arg: v, out: new_ref(out)}));},
                [n,m] => {let out = DMatrix::from_element(n,m,v.borrow().clone());  return Ok(Box::new([<Convert $input_type:upper ToMD>]{arg: v, out: new_ref(out)}));},
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