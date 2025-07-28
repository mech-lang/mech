// ----------------------------------------------------------------------------
// Access 
// ----------------------------------------------------------------------------

pub mod matrix;
pub mod record;
pub mod table;
pub mod tuple;

pub use self::matrix::*;
pub use self::record::*;
pub use self::table::*;
pub use self::tuple::*;

#[macro_use]
use crate::stdlib::*;

pub struct AccessScalar {}
impl NativeFunctionCompiler for AccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let src = &arguments[0];
    let index = &arguments[1];
    match src.kind().deref_kind() {
      ValueKind::Matrix(mat,_) => {
        MatrixAccessScalar{}.compile(&arguments)
      },
      ValueKind::Table(tble,_) => TableAccessScalar{}.compile(&arguments),
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}

pub struct AccessRange {}
impl NativeFunctionCompiler for AccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let src = &arguments[0];
    let index = &arguments[1];
    match src.kind().deref_kind() {
      ValueKind::Matrix(mat,_) => {
        MatrixAccessRange{}.compile(&arguments)
      },
      ValueKind::Table(tble,_) => TableAccessRange{}.compile(&arguments),
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}


pub struct AccessSwizzle {}
impl NativeFunctionCompiler for AccessSwizzle {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let keys = &arguments.clone().split_off(1);
    let src = &arguments[0];
    match src {
      Value::Record(rcrd) => {
        let mut values = vec![];
        for key in keys {
          let k = key.as_u64().unwrap().borrow().clone();
          match rcrd.borrow().get(&k) {
            Some(value) => values.push(value.clone()),
            None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
          }
        }
        Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
      }
      Value::Table(tbl) => {
        let mut elements = vec![];
        for k in keys {
          match k {
            Value::Id(k) => {
              match tbl.borrow().get(&k) {
                Some((kind, mat_values)) => {
                  elements.push(Box::new(mat_values.to_value()));
                }
                None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k)});}
              }
            }
            _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
          }
        }
        todo!("Table swizzle needs to be fixed.");
        let tuple = Value::Tuple(MechTuple{elements});
        Ok(Box::new(TableAccessSwizzle{out: tuple}))
      }
      Value::MutableReference(r) => match &*r.borrow() {
        Value::Record(rcrd) => {
          let mut values = vec![];
          for key in keys {
            let k = key.as_u64().unwrap().borrow().clone();
            match rcrd.borrow().get(&k) {
              Some(value) => values.push(value.clone()),
              None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
            }
          }
          Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
        }
        Value::Table(tbl) => {
          let mut elements = vec![];
          for key in keys {
            let k = key.as_u64().unwrap().borrow().clone();
            match tbl.borrow().get(&k) {
              Some((kind, mat_values)) => {
                elements.push(Box::new(mat_values.to_value()));
              }
              None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
            }
          }
          let tuple = Value::Tuple(MechTuple{elements});
          Ok(Box::new(TableAccessSwizzle{out: tuple}))
        }
        _ => todo!(),
      }
      _ => todo!(),
    }
  }
}