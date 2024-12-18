#[macro_use]
use crate::stdlib::*;

pub mod matrix;
pub mod record;
pub mod table;

pub use self::matrix::*;
pub use self::record::*;
pub use self::table::*;

// ----------------------------------------------------------------------------
// Assign 
// ----------------------------------------------------------------------------

// x = 1 ----------------------------------------------------------------------

#[derive(Debug)]
struct Assign<T> {
  sink: Ref<T>,
  source: Ref<T>,
}
impl<T> MechFunction for Assign<T> 
where
  T: Clone + Debug,
  Ref<T>: ToValue
{
  fn solve(&self) {
    let sink_ptr = self.sink.as_ptr();
    let source_ptr = self.source.as_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

pub struct AssignValue {}
impl NativeFunctionCompiler for AssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match (sink,source) {
      (Value::U8(sink),Value::U8(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::U16(sink),Value::U16(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::U32(sink),Value::U32(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::U64(sink),Value::U64(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::U128(sink),Value::U128(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::I8(sink),Value::I8(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::I16(sink),Value::I16(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::I32(sink),Value::I32(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::I64(sink),Value::I64(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::I128(sink),Value::I128(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::F32(sink),Value::F32(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::F64(sink),Value::F64(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::Bool(sink),Value::Bool(source)) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix1(sink)),Value::MatrixF64(Matrix::Matrix1(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix2(sink)),Value::MatrixF64(Matrix::Matrix2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix2x3(sink)),Value::MatrixF64(Matrix::Matrix2x3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix3x2(sink)),Value::MatrixF64(Matrix::Matrix3x2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix3(sink)),Value::MatrixF64(Matrix::Matrix3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Matrix4(sink)),Value::MatrixF64(Matrix::Matrix4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::DMatrix(sink)),Value::MatrixF64(Matrix::DMatrix(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Vector2(sink)),Value::MatrixF64(Matrix::Vector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Vector3(sink)),Value::MatrixF64(Matrix::Vector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::Vector4(sink)),Value::MatrixF64(Matrix::Vector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::DVector(sink)),Value::MatrixF64(Matrix::DVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::RowVector2(sink)),Value::MatrixF64(Matrix::RowVector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::RowVector3(sink)),Value::MatrixF64(Matrix::RowVector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::RowVector4(sink)),Value::MatrixF64(Matrix::RowVector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      (Value::MatrixF64(Matrix::RowDVector(sink)),Value::MatrixF64(Matrix::RowDVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
      x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}