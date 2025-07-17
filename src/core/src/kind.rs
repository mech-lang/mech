use crate::*;
use hashbrown::HashMap;

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Any,
  Atom(u64),
  Empty,
  Enum(u64),
  //Fsm(Vec<Kind>,Vec<Kind>),
  //Function(Vec<Kind>,Vec<Kind>),
  Id,
  Index,
  Map(Box<Kind>,Box<Kind>),
  Matrix(Box<Kind>,Vec<usize>),
  Record((Vec<(String,Kind)>)),
  Reference(Box<Kind>),
  Scalar(u64),
  Set(Box<Kind>,Option<usize>),
  Table(Vec<(String,Kind)>,usize),
  Tuple(Vec<Kind>),
}

impl Kind {

  pub fn to_value_kind(&self, functions: FunctionsRef) -> MResult<ValueKind> {
    match self {
      Kind::Any => Ok(ValueKind::Any),
      Kind::Atom(id) => Ok(ValueKind::Atom(*id)),
      Kind::Empty => Ok(ValueKind::Empty),
      Kind::Enum(id) => Ok(ValueKind::Enum(*id)),
      Kind::Id => Ok(ValueKind::Id),
      Kind::Index => Ok(ValueKind::Index),
      Kind::Map(keys, vals) => {
        let key_knd = keys.to_value_kind(functions.clone())?;
        let val_knd = vals.to_value_kind(functions.clone())?;
        Ok(ValueKind::Map(Box::new(key_knd), Box::new(val_knd)))
      },
      Kind::Matrix(knd, size) => {
        let val_knd = knd.to_value_kind(functions.clone())?;
        Ok(ValueKind::Matrix(Box::new(val_knd), size.clone()))
      },
      Kind::Record(elements) => {
        let val_knds: Vec<(String, ValueKind)> = elements.iter()
          .map(|(id, k)| k.to_value_kind(functions.clone()).map(|kind| (id.clone(), kind)))
          .collect::<MResult<_>>()?;
        Ok(ValueKind::Record(val_knds))
      },
      Kind::Reference(kind) => {
        let val_knd = kind.to_value_kind(functions.clone())?;
        Ok(ValueKind::Reference(Box::new(val_knd)))
      },
      Kind::Scalar(id) => {
        match functions.borrow().kinds.get(id).cloned() {
          Some(val_knd) => Ok(val_knd),
          None => Err(MechError {
            file: file!().to_string(),
            tokens: vec![],
            msg: "".to_string(),
            id: line!(),
            kind: MechErrorKind::UndefinedKind(*id),
          }),
        }
      },
      Kind::Set(kind, size) => {
        let val_knd = kind.to_value_kind(functions.clone())?;
        Ok(ValueKind::Set(Box::new(val_knd), *size))
      }
      Kind::Table(elements, size) => {
        let val_knds: Vec<(String, ValueKind)> = elements.iter()
          .map(|(id, k)| k.to_value_kind(functions.clone()).map(|kind| (id.clone(), kind)))
          .collect::<MResult<_>>()?;
        Ok(ValueKind::Table(val_knds, *size))
      }
      Kind::Tuple(elements) => {
        let val_knds = elements.iter().map(|k| k.to_value_kind(functions.clone())).collect::<MResult<Vec<ValueKind>>>()?;
        Ok(ValueKind::Tuple(val_knds))
      }
    }
  }
}