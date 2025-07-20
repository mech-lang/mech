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
          let k = key.as_usize().unwrap() as u64;
          match rcrd.get(&k) {
            Some(value) => values.push(value.clone()),
            None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
          }
        }
        Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
      }
      Value::Table(tbl) => {
        let mut elements = vec![];
        for k in keys {
          match tbl.get(k) {
            Some((kind, mat_values)) => {
              elements.push(Box::new(mat_values.to_value()));
            }
            None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
          }
        }
        let tuple = Value::Tuple(MechTuple{elements});
        Ok(Box::new(TableAccessSwizzle{out: tuple}))
      }
      Value::MutableReference(r) => match &*r.borrow() {
        Value::Record(rcrd) => {
          let mut values = vec![];
          for key in keys {
            let k = key.as_usize().unwrap() as u64;
            match rcrd.get(&k) {
              Some(value) => values.push(value.clone()),
              None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
            }
          }
          Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
        }
        Value::Table(tbl) => {
          let mut elements = vec![];
          for k in keys {
            match tbl.get(k) {
              Some((kind, mat_values)) => {
                elements.push(Box::new(mat_values.to_value()));
              }
              None => { return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
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