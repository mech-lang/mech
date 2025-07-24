use crate::matrix::Matrix;
use crate::*;
use crate::nodes::Matrix as Mat;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use std::collections::HashMap;

use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
use indexmap::map::*;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use paste::paste;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use serde::de::{self, Deserialize, SeqAccess, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

// Set --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  pub kind: ValueKind,
  pub num_elements: usize,
  pub set: IndexSet<Value>,
}

impl MechSet {

  pub fn to_html(&self) -> String {
    let mut src = String::new();
    for (i, element) in self.set.iter().enumerate() {
      let e = element.to_html();
      if i == 0 {
        src = format!("{}", e);
      } else {
        src = format!("{}, {}", src, e);
      }
    }
    format!("<span class=\"mech-set\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",src)
  }

  pub fn kind(&self) -> ValueKind {
    let size = if self.num_elements > 0 { Some(self.num_elements) } else { None };
    ValueKind::Set(Box::new(self.kind.clone()), size)
  }

  pub fn size_of(&self) -> usize {
    self.set.iter().map(|x| x.size_of()).sum()
  }

  pub fn from_vec(vec: Vec<Value>) -> MechSet {
    let mut set = IndexSet::new();
    for v in vec {
      set.insert(v);
    }
    let kind = if set.len() > 0 { set.iter().next().unwrap().kind() } else { ValueKind::Empty };
    MechSet{
      kind,
      num_elements: set.len(),
      set}
  }

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let mut element_strings = vec![];
    for x in self.set.iter() {
      element_strings.push(x.pretty_print());
    }
    builder.push_record(element_strings);

    let style = Style::empty()
      .top(' ')
      .left('║')
      .right('║')
      .bottom(' ')
      .vertical(' ')
      .intersection_bottom(' ')
      .corner_top_left('╔')
      .corner_top_right('╗')
      .corner_bottom_left('╚')
      .corner_bottom_right('╝');
    let mut table = builder.build();
    table.with(style);
    format!("{table}")
  }

}

impl Hash for MechSet {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.set.iter() {
      x.hash(state)
    }
  }
}

// Map ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechMap {
  pub key_kind: ValueKind,
  pub value_kind: ValueKind,
  pub num_elements: usize,
  pub map: IndexMap<Value,Value>,
}

impl MechMap {

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

  pub fn pretty_print(&self) -> String {
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

impl Hash for MechMap {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.map.iter() {
      x.hash(state)
    }
  }
}

// Table ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTable {
  pub rows: usize,
  pub cols: usize,
  pub data: IndexMap<u64,(ValueKind,Matrix<Value>)>,
  pub col_names: HashMap<u64,String>,
}

impl MechTable {


  pub fn check_record_schema(&self, record: &MechRecord) -> MResult<bool> {

    for (&col_id, record_value) in &record.data {
      // Check that the column exists in the table
      // self.get data col id _or continue to the next column
      let (expected_kind, column_matrix) = match self.data.get(&col_id) {
        Some(data) => data,
        None => {
          continue;
        }
      };

      // Check actual value kind
      let actual_kind = record_value.kind();

      if expected_kind != &actual_kind {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} kind mismatch (expected: {:?}, found: {:?})",col_id, expected_kind, actual_kind),kind: MechErrorKind::None,});
      }

      // Check column name
      if let Some(expected_name) = self.col_names.get(&col_id) {
        if let Some(field_name) = record.field_names.get(&col_id) {
          if expected_name != field_name {
            return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} name mismatch (expected: '{}', found: '{}')",col_id, expected_name, field_name),kind: MechErrorKind::None,});
          }
        }
      }
    }

    Ok(true)
  }

  pub fn append_record(&mut self, record: MechRecord) -> MResult<()> {
    // Validate schema (this includes column count, types, and optional name checks)
    self.check_record_schema(&record)?;

    // Append each value to the corresponding column in the matrix
    for (&col_id, value) in &record.data {
      if let Some((_kind, column_matrix)) = self.data.get_mut(&col_id) {
        let result = column_matrix.push(value.clone());
      } else {
        continue;
      }
    }

    // Increment row count
    self.rows += 1;

    Ok(())
  }

  pub fn get_record(&self, ix: usize) -> Option<MechRecord> {
    if ix > self.rows {
      return None;
    }

    let mut data: IndexMap<u64, Value> = IndexMap::new();
    data = self.data.iter().map(|(key, (kind, matrix))| {
      let value = matrix.index1d(ix);
      let name = self.col_names.get(key).unwrap();
      (hash_str(name), value.clone())
    }).collect();

    let mut kinds = Vec::with_capacity(self.cols);
    kinds = self.data.iter().map(|(_, (kind, _))| kind.clone()).collect();

    let mut field_names = self.col_names.clone();
   
    Some(MechRecord{cols: self.cols, kinds, data, field_names})
  }


  pub fn to_html(&self) -> String {
    let mut html = String::new();

    // Start table
    html.push_str("<table class=\"mech-table\">");

    // Build thead
    html.push_str("<thead class=\"mech-table-header\"><tr>");
    for (key, (kind, _matrix)) in self.data.iter() {
        let col_name = self
            .col_names
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());

        let kind_str = format!(
            "<span class=\"mech-kind-annotation\">&lt;<span class=\"mech-kind\">{}</span>&gt;</span>",
            kind
        );

        html.push_str(&format!(
            "<th class=\"mech-table-field\">\
                <div class=\"mech-field\">\
                  <span class=\"mech-field-name\">{}</span>\
                  <span class=\"mech-field-kind\">{}</span>\
                </div>\
            </th>",
            col_name, kind_str
        ));
    }
    html.push_str("</tr></thead>");

    // Build tbody
    html.push_str("<tbody class=\"mech-table-body\">");
    for row_idx in 1..=self.rows {
        html.push_str("<tr class=\"mech-table-row\">");
        for (_key, (_kind, matrix)) in self.data.iter() {
            let value = matrix.index1d(row_idx);
            html.push_str(&format!(
                "<td class=\"mech-table-column\">{}</td>",
                value.to_html()
            ));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    html
  }

  pub fn new(rows: usize, cols: usize, data: IndexMap<u64,(ValueKind,Matrix<Value>)>, col_names: HashMap<u64,String>) -> MechTable {
    MechTable{rows, cols, data, col_names}
  }

  pub fn kind(&self) -> ValueKind {
    let column_kinds: Vec<(String, ValueKind)> = self.data.iter()
      .filter_map(|(key, (kind, _))| {
        let col_name = self.col_names.get(key)?;
        Some((col_name.clone(), kind.clone()))
      })
      .collect();
    ValueKind::Table(column_kinds, self.rows)
  }
  
  pub fn size_of(&self) -> usize {
    self.data.iter().map(|(_,(_,v))| v.size_of()).sum()
  }

  pub fn rows(&self) -> usize {
    self.rows
  }

  pub fn cols(&self) -> usize {
    self.cols
  }

  pub fn get(&self, key: &u64) -> Option<&(ValueKind,Matrix<Value>)> {
    self.data.get(key)
  }

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    for (k,(knd,val)) in &self.data {
      let name = self.col_names.get(k).unwrap();
      let val_string: String = val.as_vec().iter()
        .map(|x| x.pretty_print())
        .collect::<Vec<String>>()
        .join("\n");
      let mut col_string = vec![format!("{}<{}>", name.to_string(), knd), val_string];
      builder.push_column(col_string);
    }
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }

  pub fn shape(&self) -> Vec<usize> {
    vec![self.rows,self.cols]
  }
}

impl Hash for MechTable {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,(knd,val)) in self.data.iter() {
      k.hash(state);
      knd.hash(state);
      val.hash(state);
    }
  }
}

// Record ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechRecord {
  pub cols: usize,
  pub kinds: Vec<ValueKind>,
  pub data: IndexMap<u64,Value>,
  pub field_names: HashMap<u64,String>,
}

impl MechRecord {

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

  pub fn pretty_print(&self) -> String {
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

  pub fn shape(&self) -> Vec<usize> {
    vec![1,self.cols]
  }
}

impl Hash for MechRecord {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,v) in self.data.iter() {
      k.hash(state);
      v.hash(state);
    }
  }
}

// Tuple ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTuple {
  pub elements: Vec<Box<Value>>
}

impl MechTuple {

  pub fn to_html(&self) -> String {
    let mut elements = Vec::new();
    for element in &self.elements {
      elements.push(element.to_html());
    }
    format!("<span class=\"mech-tuple\"><span class=\"mech-start-brace\">(</span>{}<span class=\"mech-end-brace\">)</span></span>", elements.join(", "))
  }

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let string_elements: Vec<String> = self.elements.iter().map(|e| e.pretty_print()).collect::<Vec<String>>();
    builder.push_record(string_elements);
    let mut table = builder.build();
    let style = Style::empty()
      .top(' ')
      .left('│')
      .right('│')
      .bottom(' ')
      .vertical(' ')
      .intersection_bottom('ʼ')
      .corner_top_left('╭')
      .corner_top_right('╮')
      .corner_bottom_left('╰')
      .corner_bottom_right('╯');
    table.with(style);
    format!("{table}")
  }

  pub fn get(&self, index: usize) -> Option<&Value> {
    if index < self.elements.len() {
      Some(self.elements[index].as_ref())
    } else {
      None
    }
  }

  pub fn from_vec(elements: Vec<Value>) -> Self {
    MechTuple{elements: elements.iter().map(|m| Box::new(m.clone())).collect::<Vec<Box<Value>>>()}
  }

  pub fn size(&self) -> usize {
    self.elements.len()
  }

  pub fn kind(&self) -> ValueKind {
    ValueKind::Tuple(self.elements.iter().map(|x| x.kind()).collect())
  }

  pub fn size_of(&self) -> usize {
    self.elements.iter().map(|x| x.size_of()).sum()
  }

}

impl Hash for MechTuple {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.elements.iter() {
        x.hash(state)
    }
  }
}

// Enum -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechEnum {
  pub id: u64,
  pub variants: Vec<(u64, Option<Value>)>,
}

impl MechEnum {

  pub fn to_html(&self) -> String {
    let mut variants = Vec::new();
    for (id, value) in &self.variants {
      let value_html = match value {
        Some(v) => v.to_html(),
        None => "None".to_string(),
      };
      variants.push(format!("<span class=\"mech-enum-variant\">{}: {}</span>", id, value_html));
    }
    format!("<span class=\"mech-enum\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>", variants.join(", "))
  }

  pub fn kind(&self) -> ValueKind {
    ValueKind::Enum(self.id)
  }

  pub fn size_of(&self) -> usize {
    self.variants.iter().map(|(_,v)| v.as_ref().map_or(0, |x| x.size_of())).sum()
  }

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let string_elements: Vec<String> = vec![format!("{}{:?}",self.id,self.variants)];
    builder.push_record(string_elements);
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }

}

impl Hash for MechEnum {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.id.hash(state);
    self.variants.hash(state);
  }
}