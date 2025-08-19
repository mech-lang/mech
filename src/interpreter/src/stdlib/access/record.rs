#[macro_use]
use crate::stdlib::*;

// Record Access --------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAccessField {
  pub source: Value,
}
impl MechFunction for RecordAccessField {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

pub fn impl_access_record_fxn(source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (source,key) {
    (Value::Record(rcd), Value::Id(id)) => {
      let k = id;
      match rcd.borrow().get(&k) {
        Some(value) => Ok(Box::new(RecordAccessField{source: value.clone()})),
        None => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)}),
      }
    }
    x => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Unhandled args {:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
  }
}

pub struct RecordAccess {}
impl NativeFunctionCompiler for RecordAccess {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let key = &arguments[1];
    let src = &arguments[0];
    match impl_access_record_fxn(src.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match src {
          Value::MutableReference(rcrd) => { impl_access_record_fxn(rcrd.borrow().clone(), key.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:#?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}


#[derive(Debug)]
pub struct RecordAccessSwizzle {
  pub source: Value,
}

impl MechFunction for RecordAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}
