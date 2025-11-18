use crate::*;
use indexmap::set::IndexSet;

// Set --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  pub kind: ValueKind,
  pub num_elements: usize,
  pub set: IndexSet<Value>,
}

impl MechSet {

  pub fn new(kind: ValueKind, size: usize) -> MechSet {
    MechSet{
      kind,
      num_elements: size,
      set: IndexSet::with_capacity(size)
    }
  }


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

#[derive(Debug, Clone)]
pub struct SetKindMismatchError {
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}
impl MechErrorKind2 for SetKindMismatchError {
  fn name(&self) -> &str { "SetKindMismatch" }
  fn message(&self) -> String {
    format!("Schema mismatch: set kind mismatch (expected: {}, found: {}).",
            self.expected_kind, self.actual_kind)
  }
}
