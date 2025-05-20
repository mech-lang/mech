use crate::*;
use mech_core::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::console;


#[cfg(target_arch = "wasm32")]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

macro_rules! impl_io_print_matrix {
  ($matrix_shape:tt) => {
    paste!{
      #[derive(Debug)]
      struct [<IoPrint $matrix_shape>]<T> {
        e0: Ref<$matrix_shape<T>>,
      }
      impl<T> MechFunction for [<IoPrint $matrix_shape>]<T>
      where
        T: Clone + Sync + Send + 'static + std::fmt::Display + std::fmt::Debug
      {
        fn solve(&self) { 
          unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            for i in 0..e0_ptr.len() {
              #[cfg(not(target_arch = "wasm32"))]
              print!("{} ", e0_ptr[i]);
              #[cfg(target_arch = "wasm32")]
              log!("{} ", e0_ptr[i]);
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
impl_io_print_matrix!(Matrix1);
impl_io_print_matrix!(Matrix2);
impl_io_print_matrix!(Matrix3);
impl_io_print_matrix!(Matrix4);
impl_io_print_matrix!(Matrix2x3);
impl_io_print_matrix!(Matrix3x2);
impl_io_print_matrix!(RowVector2);
impl_io_print_matrix!(RowVector3);
impl_io_print_matrix!(RowVector4);
impl_io_print_matrix!(Vector2);
impl_io_print_matrix!(Vector3);
impl_io_print_matrix!(Vector4);
impl_io_print_matrix!(DMatrix);
impl_io_print_matrix!(DVector);
impl_io_print_matrix!(RowDVector);

macro_rules! impl_print_match_arms {
  ($arg:expr, $($input_type:ident, $value_string:tt),+) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintMatrix1::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix2(input))) => Ok(Box::new(IoPrintMatrix2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix3(input))) => Ok(Box::new(IoPrintMatrix3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix4(input))) => Ok(Box::new(IoPrintMatrix4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector2(input))) => Ok(Box::new(IoPrintVector2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector3(input))) => Ok(Box::new(IoPrintVector3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector4(input))) => Ok(Box::new(IoPrintVector4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector2(input))) => Ok(Box::new(IoPrintRowVector2::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector3(input))) => Ok(Box::new(IoPrintRowVector3::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector4(input))) => Ok(Box::new(IoPrintRowVector4::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowDVector(input))) => Ok(Box::new(IoPrintRowDVector::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DMatrix(input))) => Ok(Box::new(IoPrintDMatrix::<$input_type>{e0: input.clone()})),
          #[cfg(all(feature = $value_string, feature = "VectorV"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DVector(input))) => Ok(Box::new(IoPrintDVector::<$input_type>{e0: input.clone()})),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[derive(Debug)]
struct IoPrintScalar<T> {
  e0: Ref<T>,
}

impl<T> MechFunction for IoPrintScalar<T> 
  where T: Clone + Sync + Send + 'static + std::fmt::Display + std::fmt::Debug {
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      print!("{}", e0_ptr);
    }
  }
  fn out(&self) -> Value { Value::Empty }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

fn impl_print_fxn(source_value: Value) -> MResult<Box<dyn MechFunction>>  {
  // Print the scalars.
  if source_value.is_scalar() {
    match source_value {
      Value::I8(value) => { return Ok(Box::new(IoPrintScalar::<i8> { e0: value })); }
      Value::I16(value) => { return Ok(Box::new(IoPrintScalar::<i16> { e0: value })); }
      Value::I32(value) => { return Ok(Box::new(IoPrintScalar::<i32> { e0: value })); }
      Value::I64(value) => { return Ok(Box::new(IoPrintScalar::<i64> { e0: value })); }
      Value::I128(value) => { return Ok(Box::new(IoPrintScalar::<i128> { e0: value })); }
      Value::U8(value) => { return Ok(Box::new(IoPrintScalar::<u8> { e0: value })); }
      Value::U16(value) => { return Ok(Box::new(IoPrintScalar::<u16> { e0: value })); }
      Value::U32(value) => { return Ok(Box::new(IoPrintScalar::<u32> { e0: value })); }
      Value::U64(value) => { return Ok(Box::new(IoPrintScalar::<u64> { e0: value })); }
      Value::U128(value) => { return Ok(Box::new(IoPrintScalar::<u128> { e0: value })); }
      Value::F32(value) => { return Ok(Box::new(IoPrintScalar::<F32> { e0: value })); }
      Value::F64(value) => { return Ok(Box::new(IoPrintScalar::<F64> { e0: value })); }
      Value::String(value) => { return Ok(Box::new(IoPrintScalar::<String> { e0: value })); }
      Value::Bool(value) => { return Ok(Box::new(IoPrintScalar::<bool> { e0: value })); }
      _ => (),
    }
  }


  impl_print_match_arms!(
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

pub struct IoPrint {}

impl NativeFunctionCompiler for IoPrint {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_print_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_print_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}