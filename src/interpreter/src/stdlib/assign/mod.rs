#[macro_use]
use crate::stdlib::*;

#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(feature = "table")]
pub mod table;
pub mod op_assign;

#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(feature = "table")]
pub use self::table::*;
pub use self::op_assign::*;

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
    let source_ptr = self.source.as_ptr();
    let sink_ptr = self.sink.as_mut_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[macro_export]
macro_rules! impl_assign_value_match_arms {
  ($arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new(Assign{ sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}

fn assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_assign_value_match_arms!(
    (sink, source),
    Bool,   "bool";
    String, "string";
    U8,     "u8";
    U16,    "u16";
    U32,    "u32";
    U64,    "u64";
    U128,   "u128";
    I8,     "i8";
    I16,    "i16";
    I32,    "i32";
    I64,    "i64";
    U128,   "u128";
    F32,    "f32";
    F64,    "f64";
    RationalNumber, "rational";
    ComplexNumber, "complex";
  )
}

pub struct AssignValue {}
impl NativeFunctionCompiler for AssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { assign_value_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

pub struct AssignColumn {}
impl NativeFunctionCompiler for AssignColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let src = &arguments[0];
    match src.kind().deref_kind() {
      #[cfg(feature = "table")]
      ValueKind::Table(_,_) => AssignTableColumn{}.compile(&arguments),
      #[cfg(feature = "record")]
      ValueKind::Record(_) => AssignRecordColumn{}.compile(&arguments),
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}