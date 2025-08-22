use crate::*;
use indexmap::map::*;

// Record ------------------------------------------------------------------

#[cfg(feature = "record")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechRecord {
  pub cols: usize,
  pub kinds: Vec<ValueKind>,
  pub data: IndexMap<u64,Value>,
  pub field_names: HashMap<u64,String>,
}

#[cfg(feature = "record")]
impl MechRecord {

  pub fn check_record_schema(&self, record: &MechRecord) -> MResult<()> {
    for (&col_id, _value) in &self.data {
      // Check column existence
      if !record.data.contains_key(&col_id) {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Missing column {} in record data", col_id),kind: MechErrorKind::None});
      }

      // Get expected kind
      let expected_kind = self.kinds.get(self.key_index(col_id)?).ok_or_else(|| MechError { id: line!(), file: file!().to_string(), tokens: vec![], msg: format!("Missing kind for column {}", col_id), kind: MechErrorKind::None})?;

      // Get actual kind
      let actual_kind = record.kinds.get(record.key_index(col_id)?).ok_or_else(|| MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Missing kind for column {} in compared record", col_id),kind: MechErrorKind::None,})?;

      // Compare kinds
      if expected_kind != actual_kind {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Kind mismatch for column {} (expected {:?}, found {:?})", col_id, expected_kind, actual_kind),kind: MechErrorKind::None,});
      }

      // Check field names
      let expected_name = self.field_names.get(&col_id);
      let actual_name = record.field_names.get(&col_id);

      if expected_name != actual_name {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: "".to_string(),kind: MechErrorKind::None,});
      }
    }

    Ok(())
  }

  fn key_index(&self, col_id: u64) -> MResult<usize> {
    self.data.keys().position(|&id| id == col_id).ok_or_else(|| MechError {
      id: line!(),
      file: file!().to_string(),
      tokens: vec![],
      msg: format!("Column id {} not found in key_index", col_id),
      kind: MechErrorKind::None,
    })
  }

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    let mut bindings = Vec::new();

    for (key, value) in &self.data {
      let name = self.field_names.get(key).unwrap();

      let binding_html = format!(
        "<span class=\"mech-binding\">\
          <span class=\"mech-binding-name\">{}</span>\
          <span class=\"mech-binding-colon-op\">:</span>\
          <span class=\"mech-binding-value\">{}</span>\
        </span>",
        name,
        value.to_html(),
      );

      bindings.push(binding_html);
    }

    format!(
      "<span class=\"mech-record\">\
        <span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span>\
      </span>",
      bindings.join("<span class=\"mech-separator\">, </span>")
    )
  }

  pub fn get(&self, key: &u64) -> Option<&Value> {
    self.data.get(key)
  }

  pub fn new(vec: Vec<(&str, Value)>) -> MechRecord {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = IndexMap::new();
    let mut field_names = HashMap::new();

    for (name, value) in vec {
      let col_id = hash_str(name);

      field_names.insert(col_id, name.to_string());
      data.insert(col_id, value);
    }

    let kinds = data.iter().map(|(_, v)| v.kind()).collect();

    MechRecord {
      cols: data.len(),
      kinds,
      data,
      field_names,
    }
  }

  pub fn from_vec(vec: Vec<((u64,String),Value)>) -> MechRecord {
    let mut data = IndexMap::new();
    let mut field_names = HashMap::new();
    for ((k,s),v) in vec {
      field_names.insert(k,s);
      data.insert(k,v);
    }
    let kinds = data.iter().map(|(_,v)| v.kind()).collect();
    MechRecord{cols: data.len(), kinds, data, field_names}
  }

  pub fn insert_field(&mut self, key: u64, value: Value) {
    self.cols += 1;
    self.kinds.push(value.kind());
    self.data.insert(key, value);
  }

  pub fn kind(&self) -> ValueKind {
    ValueKind::Record(self.data.iter()
      .map(|(k,v)| (self.field_names.get(k).unwrap().to_string(), v.kind()))
      .collect())
  }

  pub fn size_of(&self) -> usize {
    self.data.iter().map(|(_,v)| v.size_of()).sum()
  }

  pub fn shape(&self) -> Vec<usize> {
    vec![1,self.cols]
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechRecord {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let mut key_strings = vec![];
    let mut element_strings = vec![];
    for (k,v) in &self.data {
      let field_name = self.field_names.get(k).unwrap();
      key_strings.push(format!("{}<{}>",field_name, v.kind()));
      element_strings.push(v.pretty_print());
    }
    builder.push_record(key_strings);
    builder.push_record(element_strings);
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }
}

#[cfg(feature = "record")]
impl Hash for MechRecord {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,v) in self.data.iter() {
      k.hash(state);
      v.hash(state);
    }
  }
}


