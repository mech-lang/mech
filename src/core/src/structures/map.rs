use crate::*;
use indexmap::map::*;

// Map ------------------------------------------------------------------

#[cfg(feature = "map")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechMap {
  pub key_kind: ValueKind,
  pub value_kind: ValueKind,
  pub num_elements: usize,
  pub map: IndexMap<Value,Value>,
}

#[cfg(feature = "map")]
impl MechMap {

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    let mut src = String::new();
    for (i, (key, value)) in self.map.iter().enumerate() {
      let k = key.to_html();
      let v = value.to_html();
      if i == 0 {
        src = format!("{}: {}", k, v);
      } else {
        src = format!("{}, {}: {}", src, k, v);
      }
    }
    format!("<span class=\"mech-map\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",src)
  }

  pub fn kind(&self) -> ValueKind {
    ValueKind::Map(Box::new(self.key_kind.clone()), Box::new(self.value_kind.clone()))
  }

  pub fn size_of(&self) -> usize {
    self.map.iter().map(|(k,v)| k.size_of() + v.size_of()).sum()
  }

  pub fn from_vec(vec: Vec<(Value,Value)>) -> MechMap {
    let mut map = IndexMap::new();
    for (k,v) in vec {
      map.insert(k,v);
    }
    MechMap{
      key_kind: map.keys().next().unwrap().kind(),
      value_kind: map.values().next().unwrap().kind(),
      num_elements: map.len(),
      map}
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechMap {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let mut element_strings = vec![];
    let mut key_strings = vec![];
    for (k,v) in self.map.iter() {
      element_strings.push(v.pretty_print());
      key_strings.push(k.pretty_print());
    }    
    builder.push_record(key_strings);
    builder.push_record(element_strings);
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }
}

#[cfg(feature = "map")]
impl Hash for MechMap {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.map.iter() {
      x.hash(state)
    }
  }
}

#[derive(Debug)]
pub struct MapKeyKindMismatchError {
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}
impl MechErrorKind2 for MapKeyKindMismatchError {
  fn name(&self) -> &str {
    "MapKeyKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Map key kind mismatch (expected `{}`, found `{}`).",
      self.expected_kind, self.actual_kind
    )
  }
}

#[derive(Debug)]
pub struct MapValueKindMismatchError {
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}
impl MechErrorKind2 for MapValueKindMismatchError {
  fn name(&self) -> &str {
    "MapValueKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Map value kind mismatch (expected `{}`, found `{}`).",
      self.expected_kind, self.actual_kind
    )
  }
}