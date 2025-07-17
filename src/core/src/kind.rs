use crate::*;
use hashbrown::HashMap;

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Tuple(Vec<Kind>),
  Matrix(Box<Kind>,Vec<usize>),
  Set(Box<Kind>,usize),
  Map(Box<Kind>,Box<Kind>),
  Table(Vec<(String,Kind)>,usize),
  Record(Vec<Kind>),
  Enum(u64),
  Scalar(u64),
  Atom(u64),
  Function(Vec<Kind>,Vec<Kind>),
  Reference(Box<Kind>),
  Fsm(Vec<Kind>,Vec<Kind>),
  Id,
  Index,
  Empty,
  Any,
}

impl Kind {

  pub fn to_value_kind(&self, functions: FunctionsRef) -> MResult<ValueKind> {
    match self {
      Kind::Scalar(id) => {
        match functions.borrow().kinds.get(id).cloned() {
          Some(val_knd) => Ok(val_knd),
          None => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(*id)}),
        }
      },
      Kind::Matrix(knd,size) => {
        let val_knd = knd.to_value_kind(functions.clone())?;
        Ok(ValueKind::Matrix(Box::new(val_knd),size.clone()))
      },
      Kind::Tuple(elements) => {
        let val_knds = elements.iter().map(|k| k.to_value_kind(functions.clone())).collect::<MResult<Vec<ValueKind>>>()?;
        Ok(ValueKind::Tuple(val_knds))
      }
      Kind::Set(kind,size) => {
        let val_knd = kind.to_value_kind(functions.clone())?;
        Ok(ValueKind::Set(Box::new(val_knd),*size))
      }
      Kind::Map(keys,vals) => {
        let key_knd = keys.to_value_kind(functions.clone())?;
        let val_knd = vals.to_value_kind(functions.clone())?;
        Ok(ValueKind::Map(Box::new(key_knd),Box::new(val_knd)))
      },
      Kind::Table(elements, size) => {
        let val_knds: Vec<(String, ValueKind)> = elements.iter()
          .map(|(id, k)| k.to_value_kind(functions.clone()).map(|kind| (id.clone(), kind)))
          .collect::<MResult<_>>()?;
        Ok(ValueKind::Table(val_knds, *size))
      }
      Kind::Record(elements) => {
        let val_knds = elements.iter().map(|k| k.to_value_kind(functions.clone())).collect::<MResult<Vec<ValueKind>>>()?;
        Ok(ValueKind::Record(val_knds))
      },
      Kind::Enum(id) => Ok(ValueKind::Enum(*id)),
      Kind::Atom(id) => Ok(ValueKind::Atom(*id)),
      Kind::Reference(kind) => {
        let val_knd = kind.to_value_kind(functions.clone())?;
        Ok(ValueKind::Reference(Box::new(val_knd)))
      },
      Kind::Id => Ok(ValueKind::Id),
      Kind::Index => Ok(ValueKind::Index),
      Kind::Empty => Ok(ValueKind::Empty),
      Kind::Any => Ok(ValueKind::Any),
      _ => todo!(),
    }
  }
}