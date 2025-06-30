#[macro_use]
use crate::stdlib::*;

// Tuple Access --------------------------------------------------------------

#[derive(Debug)]
struct TupleAccessElement {
  out: Value,
}

impl MechFunction for TupleAccessElement {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
  
pub struct TupleAccess {}
impl NativeFunctionCompiler for TupleAccess{
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ix = &arguments[1];
    let src = &arguments[0];
    match (src,ix) {
      (Value::Tuple(tpl), Value::Index(ix)) => {
        let ix_brrw = ix.borrow();
        if *ix_brrw > tpl.elements.len() || *ix_brrw < 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IndexOutOfBounds});
        }
        let element = tpl.elements[*ix_brrw - 1].clone();
        let new_fxn = TupleAccessElement{ out: *element };
        Ok(Box::new(new_fxn))
      },
      (Value::MutableReference(tpl), Value::Index(ix)) => {
        match &*tpl.borrow() {
          Value::Tuple(ref tpl) => {
            let ix_brrw = ix.borrow();
            if *ix_brrw > tpl.elements.len() || *ix_brrw < 1 {
              return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IndexOutOfBounds});
            }
            let element = tpl.elements[*ix_brrw - 1].clone();
            let new_fxn = TupleAccessElement{ out: *element };
            Ok(Box::new(new_fxn))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      },
      _ => todo!(),
    }
  }
}