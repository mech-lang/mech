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
      fn to_string(&self) -> String { format!("{:?}", self) }
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
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! impl_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::[<$input_type:upper>](arg), ValueKind::[<$target_type:upper>]) => {Ok(Box::new([<ConvertS $input_type:upper $target_type:upper>]{arg: arg.clone(), out: new_ref($target_type::zero())}))},
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
          x => {
            println!("{:?}",x);
            todo!();
          }
        }
      }
    }
  }
}
