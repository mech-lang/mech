#[macro_use]
use crate::stdlib::*;

#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(feature = "table")]
pub mod table;

#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(feature = "table")]
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
impl<T> MechFunctionImpl for Assign<T> 
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
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for Assign<T> 
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("Assign<{}>", T::as_value_kind());
    compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
  }
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
        x => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 {arg: x, fxn_name: "assign".to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  };
}

fn assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
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
    R64, "rational";
    C64, "complex";
  )
}

pub struct AssignValue {}
impl NativeFunctionCompiler for AssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
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
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: x, fxn_name: "assign".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

pub struct AssignColumn {}
impl NativeFunctionCompiler for AssignColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let src = &arguments[0];
    match src.kind().deref_kind() {
      #[cfg(feature = "table")]
      ValueKind::Table(_,_) => AssignTableColumn{}.compile(&arguments),
      #[cfg(feature = "record")]
      ValueKind::Record(_) => AssignRecordColumn{}.compile(&arguments),
      _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind1 { arg: src.clone(), fxn_name: "assign/column".to_string() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}

// x += y ----------------------------------------------------------------------

pub fn add_assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
  match sink {
    #[cfg(feature = "table")]
    Value::Table(_) => add_assign_table_fxn(sink, source),
    #[cfg(feature = "math_add_assign")]
    _ => add_assign_math_fxn(sink, source),
    _ => Err(MechError2::new(
        UnhandledFunctionArgumentKind2 { arg: (sink, source), fxn_name: "assign/add".to_string() },
        None
      ).with_compiler_loc()
    ),
  }
}

pub struct AddAssignValue {}
impl NativeFunctionCompiler for AddAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match add_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { add_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { add_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { add_assign_value_fxn(sink.borrow().clone(),source.clone()) },
          (sink,source) => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (sink, source), fxn_name: "assign/add".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}