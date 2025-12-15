use crate::*;

// Enum -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechEnum {
  pub id: u64,
  pub variants: Vec<(u64, Option<Value>)>,
  pub names: Ref<Dictionary>,
}

impl MechEnum {

  pub fn name(&self) -> String {
    let names_brrw = self.names.borrow();
    names_brrw.get(&self.id).cloned().unwrap_or_else(|| format!("{}", self.id))
  }

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
    ValueKind::Enum(self.id, self.name())
  }

  pub fn size_of(&self) -> usize {
    self.variants.iter().map(|(_,v)| v.as_ref().map_or(0, |x| x.size_of())).sum()
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechEnum {
  fn pretty_print(&self) -> String {
    println!("Pretty printing enum...");
    println!("Enum ID: {}", self.id);
    println!("Variants: {:?}", self.variants);
    println!("Names: {:?}", self.names.borrow());
    let mut variants = Vec::new();
    let dict_brrw = self.names.borrow();
    let enum_name = dict_brrw.get(&self.id).unwrap();
    for (id, value) in &self.variants {
      let value_str = match value {
        Some(v) => v.pretty_print(),
        None => "None".to_string(),
      };
      let variant_name = dict_brrw.get(id).unwrap();
      variants.push(format!("{}: {}", variant_name, value_str));
    }
    format!("`{} {{ {} }}", enum_name, variants.join(" | "))
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