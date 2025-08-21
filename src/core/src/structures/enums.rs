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

// Enum -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechEnum {
  pub id: u64,
  pub variants: Vec<(u64, Option<Value>)>,
}

impl MechEnum {

  #[cfg(feature = "pretty_print")]
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
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechEnum {
  fn pretty_print(&self) -> String {
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