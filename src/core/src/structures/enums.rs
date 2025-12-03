use crate::*;

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

#[derive(Debug, Clone)]
pub struct UnknownEnumVariantError {
  pub enum_id: u64,
  pub given_variant_id: u64,
}
impl MechErrorKind2 for UnknownEnumVariantError {
  fn name(&self) -> &str { "UnknownEnumVariant" }
  fn message(&self) -> String {
    format!(
      "Unknown variant {} for enum {}",
      self.given_variant_id, self.enum_id
    )
  }
}