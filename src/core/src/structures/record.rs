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
    for (&field_id, _value) in &self.data {
      // Check field existence
      if !record.data.contains_key(&field_id) {
        return Err(MechError2::new(MissingFieldInRecordError { field_id }, None).with_compiler_loc());
      }
      // Get expected kind
      let expected_kind = self.kinds.get(self.key_index(field_id)?)
        .ok_or_else(|| MechError2::new(MissingKindForFieldError { field_id }, None).with_compiler_loc())?;
      // Get actual kind
      let actual_kind = record.kinds.get(record.key_index(field_id)?)
        .ok_or_else(|| MechError2::new(MissingKindInComparedRecordError { field_id }, None).with_compiler_loc())?;
      // Compare kinds
      if expected_kind != actual_kind {
        return Err(MechError2::new(
          RecordFieldKindMismatchError {
            field_id,
            expected_kind: expected_kind.clone(),
            actual_kind: actual_kind.clone(),
          },
          None
        ).with_compiler_loc());
      }
      // Check field names
      if self.field_names.get(&field_id) != record.field_names.get(&field_id) {
        return Err(MechError2::new(
          RecordFieldNameMismatchError {
            field_id,
            expected_name: self.field_names.get(&field_id).cloned(),
            actual_name: record.field_names.get(&field_id).cloned(),
          },
          None
        ).with_compiler_loc());
      }
    }
    Ok(())
  }

  fn key_index(&self, field_id: u64) -> MResult<usize> {
    self.data.keys().position(|&id| id == field_id).ok_or_else(|| {
      MechError2::new(
        KeyNotFoundInKeyIndexError { field_id },
        None
      ).with_compiler_loc()
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

  pub fn from_kind(fields: &Vec<(String,ValueKind)>) -> MResult<MechRecord> {
    let mut data = IndexMap::new();
    let mut field_names = HashMap::new();
    for (name, knd) in fields {
      let col_id = hash_str(name);
      field_names.insert(col_id, name.to_string());
      data.insert(col_id, Value::from_kind(knd));
    }
    let kinds = data.iter().map(|(_,v)| v.kind()).collect();
    Ok(MechRecord{cols: data.len(), kinds, data, field_names})
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

#[derive(Debug, Clone)]
pub struct MissingFieldInRecordError {
  pub field_id: u64,
}

impl MechErrorKind2 for MissingFieldInRecordError {
  fn name(&self) -> &str {
    "MissingFieldInRecord"
  }
  fn message(&self) -> String {
    format!("Record is missing required field `{}`.", self.field_id)
  }
}

#[derive(Debug, Clone)]
pub struct MissingKindForFieldError {
  pub field_id: u64,
}

impl MechErrorKind2 for MissingKindForFieldError {
  fn name(&self) -> &str {
    "MissingKindForField"
  }
  fn message(&self) -> String {
    format!("Missing expected kind for field `{}` in schema.", self.field_id)
  }
}

#[derive(Debug, Clone)]
pub struct MissingKindInComparedRecordError {
  pub field_id: u64,
}

impl MechErrorKind2 for MissingKindInComparedRecordError {
  fn name(&self) -> &str {
    "MissingKindInComparedRecord"
  }
  fn message(&self) -> String {
    format!("Missing kind for field `{}` in the compared record.", self.field_id)
  }
}

#[derive(Debug, Clone)]
pub struct RecordFieldKindMismatchError {
  pub field_id: u64,
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}

impl MechErrorKind2 for RecordFieldKindMismatchError {
  fn name(&self) -> &str {
    "RecordFieldKindMismatch"
  }
  fn message(&self) -> String {
    format!(
      "Kind mismatch for column `{}` (expected `{}`, found `{}`).",
      self.field_id, self.expected_kind, self.actual_kind
    )
  }
}

#[derive(Debug, Clone)]
pub struct RecordFieldNameMismatchError {
  pub field_id: u64,
  pub expected_name: Option<String>,
  pub actual_name: Option<String>,
}

impl MechErrorKind2 for RecordFieldNameMismatchError {
  fn name(&self) -> &str {
    "RecordFieldNameMismatch"
  }
  fn message(&self) -> String {
    match (&self.expected_name, &self.actual_name) {
      (Some(e), Some(a)) => format!(
        "Field name mismatch for field `{}` (expected `{}`, found `{}`).",
        self.field_id, e, a
      ),
      (Some(e), None) => format!(
        "Field name mismatch for field `{}` (expected `{}`, but no field found).",
        self.field_id, e
      ),
      _ => format!("Field name mismatch for field `{}`.", self.field_id),
    }
  }
}

#[derive(Debug, Clone)]
pub struct KeyNotFoundInKeyIndexError {
  pub field_id: u64,
}

impl MechErrorKind2 for KeyNotFoundInKeyIndexError {
  fn name(&self) -> &str {
    "KeyNotFoundInKeyIndex"
  }
  fn message(&self) -> String {
    format!("Key id `{}` not found in key_index.", self.field_id)
  }
}