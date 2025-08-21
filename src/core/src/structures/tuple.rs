#[cfg(feature = "matrix")]
use crate::matrix::Matrix;
use crate::*;
use crate::nodes::Matrix as Mat;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use std::collections::HashMap;

#[cfg(feature = "matrix")]
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::hash::{Hash, Hasher};
#[cfg(feature = "set")]
use indexmap::set::IndexSet;
#[cfg(any(feature = "map", feature = "table", feature = "record"))]
use indexmap::map::*;
#[cfg(feature = "pretty_print")]
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use paste::paste;
#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, SeqAccess, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

// Tuple ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTuple {
  pub elements: Vec<Box<Value>>
}

impl MechTuple {

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    let mut elements = Vec::new();
    for element in &self.elements {
      elements.push(element.to_html());
    }
    format!("<span class=\"mech-tuple\"><span class=\"mech-start-brace\">(</span>{}<span class=\"mech-end-brace\">)</span></span>", elements.join(", "))
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

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechTuple {
  fn pretty_print(&self) -> String {
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
}

impl Hash for MechTuple {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.elements.iter() {
        x.hash(state)
    }
  }
}
