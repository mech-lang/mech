// ----------------------------------------------------------------------------
// Access 
// ----------------------------------------------------------------------------

#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "tuple")]
pub mod tuple;

#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(feature = "table")]
pub use self::table::*;
#[cfg(feature = "tuple")]
pub use self::tuple::*;

#[macro_use]
use crate::stdlib::*;

pub struct AccessScalar {}
impl NativeFunctionCompiler for AccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let src = &arguments[0];
    let index = &arguments[1];
    match src.kind().deref_kind() {
      #[cfg(feature = "matrix")]
      ValueKind::Matrix(mat,_) => MatrixAccessScalar{}.compile(&arguments),
      #[cfg(feature = "table")]
      ValueKind::Table(tble,_) => TableAccessScalar{}.compile(&arguments),
      _ => Err(MechError2::new(UnhandledFunctionArgumentKind2 { arg: (src.clone(), index.clone()), fxn_name: "access/scalar".to_string() }, None).with_compiler_loc()),
    }
  }
}

pub struct AccessRange {}
impl NativeFunctionCompiler for AccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let src = &arguments[0];
    let index = &arguments[1];
    match src.kind().deref_kind() {
      #[cfg(feature = "matrix")]
      ValueKind::Matrix(mat,_) => MatrixAccessRange{}.compile(&arguments),
      #[cfg(feature = "table")]
      ValueKind::Table(tble,_) => TableAccessRange{}.compile(&arguments),
      _ => Err(MechError2::new(UnhandledFunctionArgumentKind2 { arg: (src.clone(), index.clone()), fxn_name: "access/range".to_string() }, None).with_compiler_loc()),
    }
  }
}

pub struct AccessSwizzle {}
impl NativeFunctionCompiler for AccessSwizzle {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let keys = &arguments.clone().split_off(1);
    let src = &arguments[0];
    match src {
      #[cfg(feature = "record")]
      Value::Record(rcrd) => {
        let mut values = vec![];
        for key in keys {
          let k = key.as_u64().unwrap().borrow().clone();
          match rcrd.borrow().get(&k) {
            Some(value) => values.push(value.clone()),
            None => { 
              return Err(MechError2::new(
                UndefinedRecordFieldError { id: k.clone() },
                None
              ).with_compiler_loc());
            }
          }
        }
        Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(Ref::new(MechTuple::from_vec(values)))}))
      }
      #[cfg(feature = "table")]
      Value::Table(tbl) => {
        let mut elements = vec![];
        for k in keys {
          match k {
            Value::Id(k) => {
              match tbl.borrow().get(&k) {
                Some((kind, mat_values)) => {
                  elements.push(Box::new(mat_values.to_value()));
                }
                None => { return Err(MechError2::new(
                  UndefinedRecordFieldError { id: k.clone() },
                  None
                ).with_compiler_loc()); }
              }
            }
            _ => return Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), keys.clone()), fxn_name: "access/swizzle".to_string() }, None).with_compiler_loc()),
          }
        }
        todo!("Table swizzle needs to be fixed.");
        let tuple = Value::Tuple(Ref::new(MechTuple{elements}));
        Ok(Box::new(TableAccessSwizzle{out: tuple}))
      }
      Value::MutableReference(r) => match &*r.borrow() {
        #[cfg(feature = "record")]
        Value::Record(rcrd) => {
          let mut values = vec![];
          for key in keys {
            let k = key.as_u64().unwrap().borrow().clone();
            match rcrd.borrow().get(&k) {
              Some(value) => values.push(value.clone()),
              None => { return Err(MechError2::new(
                  UndefinedRecordFieldError { id: k.clone() },
                  None
                ).with_compiler_loc());
              }
            }
          }
          Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(Ref::new(MechTuple::from_vec(values)))}))
        }
        #[cfg(feature = "table")]
        Value::Table(tbl) => {
          let mut elements = vec![];
          for key in keys {
            let k = key.as_u64().unwrap().borrow().clone();
            match tbl.borrow().get(&k) {
              Some((kind, mat_values)) => {
                elements.push(Box::new(mat_values.to_value()));
              }
              None => { return Err(MechError2::new(
                  UndefinedTableColumnError { id: k.clone() },
                  None
                ).with_compiler_loc());
              }
            }
          }
          let tuple = Value::Tuple(Ref::new(MechTuple{elements}));
          Ok(Box::new(TableAccessSwizzle{out: tuple}))
        }
        _ => todo!(),
      }
      _ => todo!(),
    }
  }
}

// ----------------------------------------------------------------------------

// Access Column

pub fn impl_access_column_fxn(source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
  match source.kind().deref_kind() {
    #[cfg(feature = "record")]
    ValueKind::Record(_) => RecordAccess{}.compile(&vec![source,key]),
    #[cfg(feature = "table")]
    ValueKind::Table(_,_) => TableAccessColumn{}.compile(&vec![source,key]),
    _ => Err(MechError2::new(
        UnhandledFunctionArgumentKind2 { arg: (source.clone(), key.clone()), fxn_name: "access/column".to_string() },
        None
      ).with_compiler_loc()
    ),
  }
}

pub struct AccessColumn {}
impl NativeFunctionCompiler for AccessColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let src = arguments[0].clone();
    let key = arguments[1].clone();
    match impl_access_column_fxn(src.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (src.clone(),&key.clone()) {
          (Value::MutableReference(src),_) => { impl_access_column_fxn(src.borrow().clone(), key.clone()) }
          _ => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (src.clone(), key.clone()), fxn_name: "access/column".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}