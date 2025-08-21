use crate::*;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
use std::collections::HashMap;

use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
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

// Set --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  pub kind: ValueKind,
  pub num_elements: usize,
  pub set: IndexSet<Value>,
}

impl MechSet {

  #[cfg(feature = "pretty_print")]
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
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechSet {
  fn pretty_print(&self) -> String {
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