#[macro_use]
use crate::stdlib::*;

pub mod matrix;
pub mod record;
pub mod table;

pub use self::matrix::*;
pub use self::record::*;
pub use self::table::*;

// ----------------------------------------------------------------------------
// Set 
// ----------------------------------------------------------------------------

// x = 1 ----------------------------------------------------------------------

#[derive(Debug)]
struct SetF64{
  sink: Ref<F64>,
  source: Ref<F64>,
}
impl MechFunction for SetF64 {
  fn solve(&self) {
    let sink_ptr = self.sink.as_ptr();
    let source_ptr = self.source.as_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { Value::F64(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

pub struct SetValue {}
impl NativeFunctionCompiler for SetValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match (sink,source) {
      (Value::F64(sink),Value::F64(source)) => {
        Ok(Box::new(SetF64{sink: sink.clone(), source: source.clone()}))
      }
      x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}