#[macro_use]
use crate::stdlib::*;

macro_rules! impl_convert_mat_to_string_mat {
  ($from_type:ty, $to:tt, $mat_shape:tt, $shape:tt) => {
    paste!{
      #[derive(Debug)]
      struct [<ConvertMatToMat $from_type:camel $to:camel $shape>]{
        arg: Ref<$mat_shape<$from_type>>,
        out: Ref<$mat_shape<$to>>,
      }
      impl MechFunction for [<ConvertMatToMat $from_type:camel $to:camel $shape>]
      where
        Ref<$mat_shape<$to>>: ToValue,
        $from_type: LosslessInto<$to>,
      {
        fn solve(&self) {
          let arg_ptr = self.arg.as_ptr();
          let out_ptr = self.out.as_ptr();
          unsafe {
            let out_ptr_deref = &mut *out_ptr;
            let arg_ptr_deref = &*arg_ptr;
            for i in 0..out_ptr_deref.len() {
              out_ptr_deref[i] = arg_ptr_deref[i].lossless_into();
            }
          }
        }
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  };
}

macro_rules! for_all_matrix_shapes {
  ($macro:ident, $from:ident, $to:ident) => {
      $macro!($from, $to, Matrix1,     M1);
      $macro!($from, $to, Matrix2,     M2);
      $macro!($from, $to, Matrix3,     M3);
      $macro!($from, $to, Matrix4,     M4);
      $macro!($from, $to, Matrix3x2,   M3x2);
      $macro!($from, $to, Matrix2x3,   M2x3);
      $macro!($from, $to, RowVector2,  R2);
      $macro!($from, $to, RowVector3,  R3);
      $macro!($from, $to, RowVector4,  R4);
      $macro!($from, $to, Vector2,     V2);
      $macro!($from, $to, Vector3,     V3);
      $macro!($from, $to, Vector4,     V4);
      $macro!($from, $to, DVector,     VD);
      $macro!($from, $to, RowDVector,  RD);
      $macro!($from, $to, DMatrix,     MD);
  };
}

macro_rules! for_all_scalar_types {
  ($macro:ident) => {
      $macro!(u8,u16);
      $macro!(u8,u32);
      $macro!(u8,u64);
      $macro!(u8,u128);
      $macro!(u8,i8);
      $macro!(u8,i16);
      $macro!(u8,i32);
      $macro!(u8,i64);
      $macro!(u8,i128);
      $macro!(u8,F32);
      $macro!(u8,F64);
      $macro!(u8,String);

      $macro!(u16,u8);
      $macro!(u16,u32);
      $macro!(u16,u64);
      $macro!(u16,u128);
      $macro!(u16,i8);
      $macro!(u16,i16);
      $macro!(u16,i32);
      $macro!(u16,i64);
      $macro!(u16,i128);
      $macro!(u16,F32);
      $macro!(u16,F64);
      $macro!(u16,String);

      $macro!(u32,u8);
      $macro!(u32,u16);
      $macro!(u32,u64);
      $macro!(u32,u128);
      $macro!(u32,i8);
      $macro!(u32,i16);
      $macro!(u32,i32);
      $macro!(u32,i64);
      $macro!(u32,i128);
      $macro!(u32,F32);
      $macro!(u32,F64);
      $macro!(u32,String);

      $macro!(u64,u8);
      $macro!(u64,u16);
      $macro!(u64,u32);
      $macro!(u64,u128);
      $macro!(u64,i8);
      $macro!(u64,i16);
      $macro!(u64,i32);
      $macro!(u64,i64);
      $macro!(u64,i128);
      $macro!(u64,F32);
      $macro!(u64,F64);
      $macro!(u64,String);

      $macro!(u128,u8);
      $macro!(u128,u16);
      $macro!(u128,u32);
      $macro!(u128,u64);
      $macro!(u128,i8);
      $macro!(u128,i16);
      $macro!(u128,i32);
      $macro!(u128,i64);
      $macro!(u128,i128);
      $macro!(u128,F32);
      $macro!(u128,F64);
      $macro!(u128,String);

      $macro!(i8,i16);
      $macro!(i8,i32);
      $macro!(i8,i64);
      $macro!(i8,i128);
      $macro!(i8,u8);
      $macro!(i8,u16);
      $macro!(i8,u32);
      $macro!(i8,u64);
      $macro!(i8,u128);
      $macro!(i8,F32);
      $macro!(i8,F64);
      $macro!(i8,String);

      $macro!(i16,i8);
      $macro!(i16,i32);
      $macro!(i16,i64);
      $macro!(i16,i128);
      $macro!(i16,u8);
      $macro!(i16,u16);
      $macro!(i16,u32);
      $macro!(i16,u64);
      $macro!(i16,u128);
      $macro!(i16,F32);
      $macro!(i16,F64);
      $macro!(i16,String);

      $macro!(i32,i8);
      $macro!(i32,i16);
      $macro!(i32,i64);
      $macro!(i32,i128);
      $macro!(i32,u8);
      $macro!(i32,u16);
      $macro!(i32,u32);
      $macro!(i32,u64);
      $macro!(i32,u128);
      $macro!(i32,F32);
      $macro!(i32,F64);
      $macro!(i32,String);

      $macro!(i64,i8);
      $macro!(i64,i16);
      $macro!(i64,i32);
      $macro!(i64,i128);
      $macro!(i64,u8);
      $macro!(i64,u16);
      $macro!(i64,u32);
      $macro!(i64,u64);
      $macro!(i64,u128);
      $macro!(i64,F32);
      $macro!(i64,F64);
      $macro!(i64,String);

      $macro!(i128,i8);
      $macro!(i128,i16);
      $macro!(i128,i32);
      $macro!(i128,i64);
      $macro!(i128,u8);
      $macro!(i128,u16);
      $macro!(i128,u32);
      $macro!(i128,u64);
      $macro!(i128,u128);
      $macro!(i128,F32);
      $macro!(i128,F64);
      $macro!(i128,String);

      $macro!(F64,u8);
      $macro!(F64,u16);
      $macro!(F64,u32);
      $macro!(F64,u64);
      $macro!(F64,u128);
      $macro!(F64,i8);
      $macro!(F64,i16);
      $macro!(F64,i32);
      $macro!(F64,i64);
      $macro!(F64,i128);
      $macro!(F64,F32);
      $macro!(F64,String);

      $macro!(F32,u8);
      $macro!(F32,u16);
      $macro!(F32,u32);
      $macro!(F32,u64);
      $macro!(F32,u128);
      $macro!(F32,i8);
      $macro!(F32,i16);
      $macro!(F32,i32);
      $macro!(F32,i64);
      $macro!(F32,i128);
      $macro!(F32,F64);
      $macro!(F32,String);
  };
}

macro_rules! define_all_mat_to_string_mat {
  ($from:ident, $to:ident) => {
      for_all_matrix_shapes!(impl_convert_mat_to_string_mat, $from, $to);
  };
}

for_all_scalar_types!(define_all_mat_to_string_mat);

macro_rules! impl_conversion_mat_to_mat_match_arms {
  (
    $arg:expr,
    $(
      $input_type:ident => $(
        $target_type:ident, $zero:expr
      ),+ $(,)?
    );+ $(;)?
  ) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::[<Matrix $input_type>](v), ValueKind::Matrix(box ValueKind::$target_type, dims)) => {
              let shape = v.shape();
              if dims.is_empty() || ((shape[0] == dims[0]) && (shape[1] == dims[1])) {
                match v {
                  Matrix::Matrix1(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M1>]{arg: v, out: new_ref(Matrix1::from_element($zero))})); },
                  Matrix::Matrix2(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M2>]{arg: v, out: new_ref(Matrix2::from_element($zero))})); },
                  Matrix::Matrix3(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M3>]{arg: v, out: new_ref(Matrix3::from_element($zero))})); },
                  Matrix::Matrix4(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M4>]{arg: v, out: new_ref(Matrix4::from_element($zero))})); },
                  Matrix::Matrix3x2(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M3x2>]{arg: v, out: new_ref(Matrix3x2::from_element($zero))})); },
                  Matrix::Matrix2x3(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel M2x3>]{arg: v, out: new_ref(Matrix2x3::from_element($zero))})); },
                  Matrix::RowVector2(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel R2>]{arg: v, out: new_ref(RowVector2::from_element($zero))})); },
                  Matrix::RowVector3(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel R3>]{arg: v, out: new_ref(RowVector3::from_element($zero))})); },
                  Matrix::RowVector4(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel R4>]{arg: v, out: new_ref(RowVector4::from_element($zero))})); },
                  Matrix::Vector2(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel V2>]{arg: v, out: new_ref(Vector2::from_element($zero))})); },
                  Matrix::Vector3(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel V3>]{arg: v, out: new_ref(Vector3::from_element($zero))})); },
                  Matrix::Vector4(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel V4>]{arg: v, out: new_ref(Vector4::from_element($zero))})); },
                  Matrix::DVector(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel VD>]{arg: v, out: new_ref(DVector::from_element(shape[0], $zero))})); },
                  Matrix::RowDVector(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel RD>]{arg: v, out: new_ref(RowDVector::from_element(shape[1], $zero))})); },
                  Matrix::DMatrix(v) => { return Ok(Box::new([<ConvertMatToMat $input_type:camel $target_type:camel MD>]{arg: v, out: new_ref(DMatrix::from_element(shape[0], shape[1], $zero))})); },
                }
              } else {
                return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix dimensions do not match".to_string(), id: line!(), kind: MechErrorKind::None});
              }
            }
          )+
        )+
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_conversion_mat_to_mat_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  impl_conversion_mat_to_mat_match_arms!(
    (source_value, target_kind),
    F64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero();
    F32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F64, F64::zero();
    U8 => String, String::new(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    U16 => String, String::new(), U8, u8::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    U32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    U64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    U128 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    I8 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    I16 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    I32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    I64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    I128 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), F32, F32::zero(), F64, F64::zero();
  )
}

pub struct ConvertMatToMat {}

impl NativeFunctionCompiler for ConvertMatToMat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: String::new(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match impl_conversion_mat_to_mat_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_mat_to_mat_fxn(rhs.borrow().clone(), target_kind.clone()),
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}