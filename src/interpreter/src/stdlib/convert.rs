#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Type Conversion Library
// ----------------------------------------------------------------------------

// Convert --------------------------------------------------------------------

#[macro_export]  
macro_rules! impl_convert_op {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $out_type2:ty, $op:ident) => {
    #[derive(Debug)]
    
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name
    where
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr,$out_type2)
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  }
}

macro_rules! convert_op1 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ *$out = *$arg as $out_type }
  };}

macro_rules! convert_op2 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ *$out = (*$arg).0 as $out_type }
  };}

macro_rules! convert_op3 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ (*$out).0 = (*$arg) as $out_type }
  };}

macro_rules! convert_op4 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ (*$out).0 = (*$arg).0 as $out_type }
  };}

macro_rules! impl_convert_op_group {
  ($from:ty, [$($to:ty),*], $func:ident) => {
    paste!{
      $(
        impl_convert_op!([<ConvertS $from:upper $to:upper>], $from, $to, [<$to:lower>], $func);
      )*
    }
  };
}

// From Type -> To Types
impl_convert_op_group!(i8,   [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i16,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i128, [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);

impl_convert_op_group!(u8,   [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u16,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u128, [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);

impl_convert_op_group!(F32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op2);
impl_convert_op_group!(F64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op2);

impl_convert_op_group!(i8,   [F32, F64], convert_op3);
impl_convert_op_group!(i16,  [F32, F64], convert_op3);
impl_convert_op_group!(i32,  [F32, F64], convert_op3);
impl_convert_op_group!(i64,  [F32, F64], convert_op3);
impl_convert_op_group!(i128, [F32, F64], convert_op3);
impl_convert_op_group!(u8,   [F32, F64], convert_op3);
impl_convert_op_group!(u16,  [F32, F64], convert_op3);
impl_convert_op_group!(u32,  [F32, F64], convert_op3);
impl_convert_op_group!(u64,  [F32, F64], convert_op3);
impl_convert_op_group!(u128, [F32, F64], convert_op3);

impl_convert_op_group!(F32,  [F32, F64], convert_op4);
impl_convert_op_group!(F64,  [F32, F64], convert_op4);

#[derive(Debug)]
struct ConvertSEnum {
  out: Value,
}
impl MechFunction for ConvertSEnum
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

macro_rules! convertm2m {
  ($from:tt, $to:expr) => {
    paste!{
      #[derive(Debug)]
      struct [<ConvertM $from:upper M $to:upper>] {
        arg: Matrix<$from>,
        out: Matrix<$to>,
      }
      impl MechFunction for [<ConvertM $from:upper M $to:upper>]
      {
        fn solve(&self) { 
          let arg_vec = self.arg.as_vec();
          self.out.set(arg_vec.iter().map(|x| *x as $to).collect::<Vec<$to>>());
        }
        fn out(&self) -> Value { Value::[<Matrix $to:upper>](self.out.clone()) }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  }
}

macro_rules! convertm2f {
  ($from:tt, $to:expr) => {
    paste!{
      #[derive(Debug)]
      struct [<ConvertM $from:upper M $to:upper>] {
        arg: Matrix<$from>,
        out: Matrix<$to>,
      }
      impl MechFunction for [<ConvertM $from:upper M $to:upper>]
      {
        fn solve(&self) { 
          let arg_vec = self.arg.as_vec();
          self.out.set(arg_vec.iter().map(|x| $from::into(*x)).collect::<Vec<$to>>());
        }
        fn out(&self) -> Value { Value::[<Matrix $to:upper>](self.out.clone()) }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  }
}

macro_rules! convertf2m {
  ($from:tt, $to:expr) => {
    paste!{
      #[derive(Debug)]
      struct [<ConvertM $from:upper M $to:upper>] {
        arg: Matrix<$from>,
        out: Matrix<$to>,
      }
      impl MechFunction for [<ConvertM $from:upper M $to:upper>]
      {
        fn solve(&self) { 
          let arg_vec = self.arg.as_vec();
          self.out.set(arg_vec.iter().map(|x| x.0 as $to).collect::<Vec<$to>>());
        }
        fn out(&self) -> Value { Value::[<Matrix $to:upper>](self.out.clone()) }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  }
}

macro_rules! convertf2f {
  ($from:tt, $to:expr) => {
    paste!{
      #[derive(Debug)]
      struct [<ConvertM $from:upper M $to:upper>] {
        arg: Matrix<$from>,
        out: Matrix<$to>,
      }
      impl MechFunction for [<ConvertM $from:upper M $to:upper>]
      {
        fn solve(&self) { 
          let arg_vec = self.arg.as_vec();
          self.out.set(arg_vec.iter().map(|x| $to::new(x.0 as [<$to:lower>])).collect::<Vec<$to>>());
        }
        fn out(&self) -> Value { Value::[<Matrix $to:upper>](self.out.clone()) }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  }
}

convertm2m!(u8, u8);
convertm2m!(u8, u16);
convertm2m!(u8, u32);
convertm2m!(u8, u64);
convertm2m!(u8, u128);
convertm2m!(u8, i8);
convertm2m!(u8, i16);
convertm2m!(u8, i32);
convertm2m!(u8, i64);
convertm2m!(u8, i128);
convertm2f!(u8, F32);
convertm2f!(u8, F64);

convertm2m!(u16, u8);
convertm2m!(u16, u16);
convertm2m!(u16, u32);
convertm2m!(u16, u64);
convertm2m!(u16, u128);
convertm2m!(u16, i8);
convertm2m!(u16, i16);
convertm2m!(u16, i32);
convertm2m!(u16, i64);
convertm2m!(u16, i128);
convertm2f!(u16, F32);
convertm2f!(u16, F64);

convertm2m!(u32, u8);
convertm2m!(u32, u16);
convertm2m!(u32, u32);
convertm2m!(u32, u64);
convertm2m!(u32, u128);
convertm2m!(u32, i8);
convertm2m!(u32, i16);
convertm2m!(u32, i32);
convertm2m!(u32, i64);
convertm2m!(u32, i128);
convertm2f!(u32, F32);
convertm2f!(u32, F64);

convertm2m!(u64, u8);
convertm2m!(u64, u16);
convertm2m!(u64, u32);
convertm2m!(u64, u64);
convertm2m!(u64, u128);
convertm2m!(u64, i8);
convertm2m!(u64, i16);
convertm2m!(u64, i32);
convertm2m!(u64, i64);
convertm2m!(u64, i128);
convertm2f!(u64, F32);
convertm2f!(u64, F64);

convertm2m!(u128, u8);
convertm2m!(u128, u16);
convertm2m!(u128, u32);
convertm2m!(u128, u64);
convertm2m!(u128, u128);
convertm2m!(u128, i8);
convertm2m!(u128, i16);
convertm2m!(u128, i32);
convertm2m!(u128, i64);
convertm2m!(u128, i128);
convertm2f!(u128, F32);
convertm2f!(u128, F64);

convertm2m!(i8, u8);
convertm2m!(i8, u16);
convertm2m!(i8, u32);
convertm2m!(i8, u64);
convertm2m!(i8, u128);
convertm2m!(i8, i8);
convertm2m!(i8, i16);
convertm2m!(i8, i32);
convertm2m!(i8, i64);
convertm2m!(i8, i128);
convertm2f!(i8, F32);
convertm2f!(i8, F64);

convertm2m!(i16, u8);
convertm2m!(i16, u16);
convertm2m!(i16, u32);
convertm2m!(i16, u64);
convertm2m!(i16, u128);
convertm2m!(i16, i8);
convertm2m!(i16, i16);
convertm2m!(i16, i32);
convertm2m!(i16, i64);
convertm2m!(i16, i128);
convertm2f!(i16, F32);
convertm2f!(i16, F64);

convertm2m!(i32, u8);
convertm2m!(i32, u16);
convertm2m!(i32, u32);
convertm2m!(i32, u64);
convertm2m!(i32, u128);
convertm2m!(i32, i8);
convertm2m!(i32, i16);
convertm2m!(i32, i32);
convertm2m!(i32, i64);
convertm2m!(i32, i128);
convertm2f!(i32, F32);
convertm2f!(i32, F64);

convertm2m!(i64, u8);
convertm2m!(i64, u16);
convertm2m!(i64, u32);
convertm2m!(i64, u64);
convertm2m!(i64, u128);
convertm2m!(i64, i8);
convertm2m!(i64, i16);
convertm2m!(i64, i32);
convertm2m!(i64, i64);
convertm2m!(i64, i128);
convertm2f!(i64, F32);
convertm2f!(i64, F64);

convertm2m!(i128, u8);
convertm2m!(i128, u16);
convertm2m!(i128, u32);
convertm2m!(i128, u64);
convertm2m!(i128, u128);
convertm2m!(i128, i8);
convertm2m!(i128, i16);
convertm2m!(i128, i32);
convertm2m!(i128, i64);
convertm2m!(i128, i128);
convertm2f!(i128, F32);
convertm2f!(i128, F64);

convertf2m!(F64, u8);
convertf2m!(F64, u16);
convertf2m!(F64, u32);
convertf2m!(F64, u64);
convertf2m!(F64, u128);
convertf2m!(F64, i8);
convertf2m!(F64, i16);
convertf2m!(F64, i32);
convertf2m!(F64, i64);
convertf2m!(F64, i128);
convertf2f!(F64, F32);
convertf2f!(F64, F64);

convertf2m!(F32, u8);
convertf2m!(F32, u16);
convertf2m!(F32, u32);
convertf2m!(F32, u64);
convertf2m!(F32, u128);
convertf2m!(F32, i8);
convertf2m!(F32, i16);
convertf2m!(F32, i32);
convertf2m!(F32, i64);
convertf2m!(F32, i128);
convertf2f!(F32, F32);
convertf2f!(F32, F64);

macro_rules! impl_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::[<$input_type:upper>](arg), ValueKind::[<$target_type:upper>]) => {Ok(Box::new([<ConvertS $input_type:upper $target_type:upper>]{arg: arg.clone(), out: new_ref($target_type::zero())}))},
            (Value::[<Matrix $input_type:upper>](arg), ValueKind::Matrix(kind,size)) => {
              match *kind {
                ValueKind::U8 => {let in_shape = arg.shape();let out = u8::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MU8>]{arg: arg.clone(), out}))}
                ValueKind::U16 => {let in_shape = arg.shape();let out = u16::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MU16>]{arg: arg.clone(), out}))}
                ValueKind::U32 => {let in_shape = arg.shape();let out = u32::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MU32>]{arg: arg.clone(), out}))}
                ValueKind::U64 => {let in_shape = arg.shape();let out = u64::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MU64>]{arg: arg.clone(), out}))}
                //ValueKind::U128 => {let in_shape = arg.shape();let out = u128::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MU128>]{arg: arg.clone(), out}))}
                ValueKind::I8 => {let in_shape = arg.shape();let out = i8::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MI8>]{arg: arg.clone(), out}))}
                ValueKind::I16 => {let in_shape = arg.shape();let out = i16::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MI16>]{arg: arg.clone(), out}))}
                ValueKind::I32 => {let in_shape = arg.shape();let out = i32::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MI32>]{arg: arg.clone(), out}))}
                ValueKind::I64 => {let in_shape = arg.shape();let out = i64::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MI64>]{arg: arg.clone(), out}))}
                //ValueKind::I128 => {let in_shape = arg.shape();let out = i128::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MI128>]{arg: arg.clone(), out}))}
                ValueKind::F32 => {let in_shape = arg.shape();let out = F32::to_matrix(vec![F32::zero(); in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MF32>]{arg: arg.clone(), out}))}
                ValueKind::F64 => {let in_shape = arg.shape();let out = F64::to_matrix(vec![F64::zero(); in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new([<ConvertM $input_type:upper MF64>]{arg: arg.clone(), out}))}
                _ => todo!(),
              }
            },
          )+
        )+
        (Value::Atom(varian_id), ValueKind::Enum(enum_id)) => {
          let variants = vec![(varian_id,None)];
          let enm = MechEnum{id: enum_id, variants};
          let val = Value::Enum(Box::new(enm.clone()));
          Ok(Box::new(ConvertSEnum{out: val}))
        }
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_conversion_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  impl_conversion_match_arms!(
    (source_value, target_kind),
    i8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
  )
}

pub struct ConvertKind {}

impl NativeFunctionCompiler for ConvertKind {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let target_kind = arguments[1].kind();
    match impl_conversion_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_fxn(rhs.borrow().clone(), target_kind.clone()),
          Value::Atom(atom_id) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          x => {
            println!("{:?}",x);
            todo!();
          }
        }
      }
    }
  }
}

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
    $macro!(F32);  
    $macro!(F64);
    $macro!(String);
  };
}

macro_rules! define_convert_for_type {
  ($scalar:ident) => {
      paste!{
      impl_convert_scalar_to_vec!([<Convert $scalar ToR2>],   $scalar, RowVector2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToR3>],   $scalar, RowVector3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToR4>],   $scalar, RowVector4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToV2>],   $scalar, Vector2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToV3>],   $scalar, Vector3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToV4>],   $scalar, Vector4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM1>],   $scalar, Matrix1<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM2>],   $scalar, Matrix2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM3>],   $scalar, Matrix3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM4>],   $scalar, Matrix4<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM3x2>], $scalar, Matrix3x2<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToM2x3>], $scalar, Matrix2x3<$scalar>);
      impl_convert_scalar_to_vec!([<Convert $scalar ToMD>], $scalar, DMatrix<$scalar>);
    }
  };
}

for_all_scalar_types!(define_convert_for_type);

pub struct ConvertScalarToVec {}

impl NativeFunctionCompiler for ConvertScalarToVec {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match (source_value,target_kind) {
      (Value::String(v), ValueKind::Matrix(box ValueKind::String, dims)) => {
        let out = match dims[..] {
          [1,1] => {
            let out = Matrix1::from_element(v.borrow().clone());
            return Ok(Box::new(ConvertStringToM1{arg: v, out: new_ref(out)}));
          },
          [2,2] => {let out = Matrix2::from_element(v.clone());},
          [3,3] => {let out = Matrix3::from_element(v.clone());},
          [4,4] => {let out = Matrix4::from_element(v.clone());},
          [2,3] => {let out = Matrix2x3::from_element(v.clone());},
          [3,2] => {let out = Matrix3x2::from_element(v.clone());},
          [1,2] => {let out = RowVector2::from_element(v.clone());},
          [1,3] => {let out = RowVector3::from_element(v.clone());},
          [1,4] => {
            let out = RowVector4::from_element(v.borrow().clone());
            return Ok(Box::new(ConvertStringToR4{arg: v, out: new_ref(out)}));
          },
          [2,1] => {let out = Vector2::from_element(v.clone());},
          [3,1] => {let out = Vector3::from_element(v.clone());},
          [4,1] => {let out = Vector4::from_element(v.clone());},
          _ => todo!(),
        };
      }
      _ => todo!(),
    }
    
  
    todo!();

  }
}





