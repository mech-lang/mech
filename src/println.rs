use crate::*;
use mech_core::*;


macro_rules! impl_io_println_matrix {
  ($matrix_shape:tt) => {
    paste!{
      #[derive(Debug)]
      struct [<IoPrintln $matrix_shape>]<T> {
        e0: Ref<$matrix_shape<T>>,
      }
      impl<T> MechFunction for [<IoPrintln $matrix_shape>]<T>
      where
        T: Clone + Sync + Send + 'static + std::fmt::Display + std::fmt::Debug
      {
        fn solve(&self) { 
          unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            for i in 0..e0_ptr.len() {
              println!("{}", e0_ptr[i]);
            }  
          }
        }
        fn out(&self) -> Value { Value::Empty }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  };
}

// Generate implementations for different matrix sizes
impl_io_println_matrix!(Matrix1);
impl_io_println_matrix!(Matrix2);
impl_io_println_matrix!(Matrix3);
impl_io_println_matrix!(Matrix4);
impl_io_println_matrix!(Matrix2x3);
impl_io_println_matrix!(Matrix3x2);
impl_io_println_matrix!(RowVector2);
impl_io_println_matrix!(RowVector3);
impl_io_println_matrix!(RowVector4);
impl_io_println_matrix!(Vector2);
impl_io_println_matrix!(Vector3);
impl_io_println_matrix!(Vector4);
impl_io_println_matrix!(DMatrix);
impl_io_println_matrix!(DVector);
impl_io_println_matrix!(RowDVector);

macro_rules! impl_println_match_arms {
  ($arg:expr, $($input_type:ident, $value_string:tt),+) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintlnMatrix1::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix2(input))) => Ok(Box::new(IoPrintlnMatrix2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix3(input))) => Ok(Box::new(IoPrintlnMatrix3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix4(input))) => Ok(Box::new(IoPrintlnMatrix4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector2(input))) => Ok(Box::new(IoPrintlnVector2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector3(input))) => Ok(Box::new(IoPrintlnVector3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector4(input))) => Ok(Box::new(IoPrintlnVector4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector2(input))) => Ok(Box::new(IoPrintlnRowVector2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector3(input))) => Ok(Box::new(IoPrintlnRowVector3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector4(input))) => Ok(Box::new(IoPrintlnRowVector4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowDVector(input))) => Ok(Box::new(IoPrintlnRowDVector::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DMatrix(input))) => Ok(Box::new(IoPrintlnDMatrix::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "VectorV"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DVector(input))) => Ok(Box::new(IoPrintlnDVector::<$input_type>{e0: input.clone()})),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[derive(Debug)]
struct IoPrintlnScalar<T> {
  e0: Ref<T>,
}

impl<T> MechFunction for IoPrintlnScalar<T> 
  where T: Clone + Sync + Send + 'static + std::fmt::Display + std::fmt::Debug {
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      println!("{}", e0_ptr);
    }
  }
  fn out(&self) -> Value { Value::Empty }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

fn impl_println_fxn(source_value: Value) -> MResult<Box<dyn MechFunction>>  {
  // Println the scalars.
  if source_value.is_scalar() {
    match source_value {
      Value::I8(value) => { return Ok(Box::new(IoPrintlnScalar::<i8> { e0: value })); }
      Value::I16(value) => { return Ok(Box::new(IoPrintlnScalar::<i16> { e0: value })); }
      Value::I32(value) => { return Ok(Box::new(IoPrintlnScalar::<i32> { e0: value })); }
      Value::I64(value) => { return Ok(Box::new(IoPrintlnScalar::<i64> { e0: value })); }
      Value::I128(value) => { return Ok(Box::new(IoPrintlnScalar::<i128> { e0: value })); }
      Value::U8(value) => { return Ok(Box::new(IoPrintlnScalar::<u8> { e0: value })); }
      Value::U16(value) => { return Ok(Box::new(IoPrintlnScalar::<u16> { e0: value })); }
      Value::U32(value) => { return Ok(Box::new(IoPrintlnScalar::<u32> { e0: value })); }
      Value::U64(value) => { return Ok(Box::new(IoPrintlnScalar::<u64> { e0: value })); }
      Value::U128(value) => { return Ok(Box::new(IoPrintlnScalar::<u128> { e0: value })); }
      Value::F32(value) => { return Ok(Box::new(IoPrintlnScalar::<F32> { e0: value })); }
      Value::F64(value) => { return Ok(Box::new(IoPrintlnScalar::<F64> { e0: value })); }
      Value::String(value) => { return Ok(Box::new(IoPrintlnScalar::<String> { e0: value })); }
      Value::Bool(value) => { return Ok(Box::new(IoPrintlnScalar::<bool> { e0: value })); }
      _ => (),
    }
  }


  impl_println_match_arms!(
    (source_value),
    i8, "I8",
    i16, "I16",
    i32, "I32",
    i64, "I64",
    i128, "I128",
    u8, "U8",
    u16, "U16",
    u32, "U32",
    u64, "U64",
    u128, "U128",
    F32, "F32",
    F64, "F64",
    String, "String",
    bool, "Bool"
  )
}

pub struct IoPrintln {}

impl NativeFunctionCompiler for IoPrintln {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_println_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_println_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}