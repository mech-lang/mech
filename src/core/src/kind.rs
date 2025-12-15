use crate::*;

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Any,
  None,
  Atom(u64, String),
  Empty,
  Enum(u64, String),
  //Fsm(Vec<Kind>,Vec<Kind>),
  //Function(Vec<Kind>,Vec<Kind>),
  Id,
  Index,
  Map(Box<Kind>,Box<Kind>),
  Matrix(Box<Kind>,Vec<usize>),
  Option(Box<Kind>),
  Record((Vec<(String,Kind)>)),
  Reference(Box<Kind>),
  Scalar(u64),
  Set(Box<Kind>,Option<usize>),
  Table(Vec<(String,Kind)>,usize),
  Tuple(Vec<Kind>),
}

impl Kind {

  #[cfg(feature = "kind_annotation")]
  pub fn to_value(&self, kinds: &KindTable) -> MResult<Value> {
    let value_kind = self.to_value_kind(kinds)?;
    Ok(Value::Kind(value_kind))
  }

  #[cfg(feature = "kind_annotation")]
  pub fn to_value_kind(&self, kinds: &KindTable) -> MResult<ValueKind> {
    match self {
      Kind::None => Ok(ValueKind::None),
      Kind::Any => Ok(ValueKind::Any),
      Kind::Empty => Ok(ValueKind::Empty),
      Kind::Atom(id, name) => Ok(ValueKind::Atom(*id, name.clone())),
      Kind::Enum(id, name) => Ok(ValueKind::Enum(*id, name.clone())),
      Kind::Id => Ok(ValueKind::Id),
      Kind::Index => Ok(ValueKind::Index),
      Kind::Map(keys, vals) => {
        let key_knd = keys.to_value_kind(kinds)?;
        let val_knd = vals.to_value_kind(kinds)?;
        Ok(ValueKind::Map(Box::new(key_knd), Box::new(val_knd)))
      },
      Kind::Matrix(knd, size) => {
        let val_knd = knd.to_value_kind(kinds)?;
        Ok(ValueKind::Matrix(Box::new(val_knd), size.clone()))
      },
      Kind::Option(knd) => {
        let val_knd = knd.to_value_kind(kinds)?;
        Ok(ValueKind::Option(Box::new(val_knd)))
      },
      Kind::Record(elements) => {
        let val_knds: Vec<(String, ValueKind)> = elements.iter()
          .map(|(id, k)| k.to_value_kind(kinds).map(|kind| (id.clone(), kind)))
          .collect::<MResult<_>>()?;
        Ok(ValueKind::Record(val_knds))
      },
      Kind::Reference(kind) => {
        let val_knd = kind.to_value_kind(kinds)?;
        Ok(ValueKind::Reference(Box::new(val_knd)))
      },
      Kind::Scalar(id) => {
        match kinds.get(id).cloned() {
          Some(val_knd) => Ok(val_knd),
          None => Err(
            MechError2::new(
              UndefinedKindError { kind_id: *id },
              None,
            )
            .with_compiler_loc()
          ),
        }
      },
      Kind::Set(kind, size) => {
        let val_knd = kind.to_value_kind(kinds)?;
        Ok(ValueKind::Set(Box::new(val_knd), *size))
      },
      Kind::Table(elements, size) => {
        let val_knds: Vec<(String, ValueKind)> = elements.iter()
          .map(|(id, k)| k.to_value_kind(kinds).map(|kind| (id.clone(), kind)))
          .collect::<MResult<_>>()?;
        Ok(ValueKind::Table(val_knds, *size))
      },
      Kind::Tuple(elements) => {
        let val_knds = elements.iter().map(|k| k.to_value_kind(kinds)).collect::<MResult<Vec<ValueKind>>>()?;
        Ok(ValueKind::Tuple(val_knds))
      }
    }
  }
}