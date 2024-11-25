use crate::*;

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Scalar(u64),
  Matrix(Box<Kind>,Vec<usize>),
  Tuple,
  Brace,
  Map,
  Atom,
  Function,
  Fsm,
  Empty,
}

impl Kind {

  pub fn to_value_kind(&self, functions: FunctionsRef) -> MResult<ValueKind> {
    match self {
      Kind::Scalar(id) => {
        match functions.borrow().kinds.get(id).cloned() {
          Some(val_knd) => Ok(val_knd),
          None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(*id)}),
        }
      },
      Kind::Matrix(knd,size) => {
        let val_knd = knd.to_value_kind(functions.clone())?;
        Ok(ValueKind::Matrix(Box::new(val_knd),(size[0],size[1])))
      },
      Kind::Tuple => todo!(),
      Kind::Brace => todo!(),
      Kind::Map => todo!(),
      Kind::Atom => todo!(),
      Kind::Function => todo!(),
      Kind::Fsm => todo!(),
      Kind::Empty => Ok(ValueKind::Empty),
    }
  }
}