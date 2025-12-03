use crate::*;

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

#[derive(Debug, Clone)]
pub struct TupleDestructureTooManyVarsError{pub value: ValueKind }
impl MechErrorKind2 for TupleDestructureTooManyVarsError {
  fn name(&self) -> &str { "TupleDestructureTooManyVars" }
  fn message(&self) -> String {
    format!("Attempted to destructure tuple into too many variables: {:?}", self.value)
  }
}

#[derive(Debug, Clone)]
pub struct DestructureExpectedTupleError{pub value: ValueKind }
impl MechErrorKind2 for DestructureExpectedTupleError {
  fn name(&self) -> &str { "DestructureExpectedTuple" }
  fn message(&self) -> String {
    format!("Expected a tuple value for destructuring, found: {:?}", self.value)
  }
}

#[derive(Debug, Clone)]
pub struct TupleIndexOutOfBoundsError{pub ix: usize, pub len: usize }
impl MechErrorKind2 for TupleIndexOutOfBoundsError {
  fn name(&self) -> &str { "TupleIndexOutOfBounds" }
  fn message(&self) -> String {
    format!("Tuple index {} out of bounds for tuple of length {}", self.ix, self.len)
  }
}
