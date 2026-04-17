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
    let dict_brrw = self.names.borrow();
    let mut variants = Vec::new();
    for (id, value) in &self.variants {
      let variant_name = dict_brrw
        .get(id)
        .map(|name| name.rsplit('/').next().unwrap_or(name).to_string())
        .unwrap_or_else(|| format!("{}", id));
      let variant_html = match value {
        Some(v) => format!(
          "<span class=\"mech-enum-variant-name\">:{}</span><span class=\"mech-enum-variant-payload\">(<span class=\"mech-enum-variant-value\">{}</span>)</span>",
          variant_name,
          v.to_html()
        ),
        None => format!("<span class=\"mech-enum-variant-name\">:{}</span>", variant_name),
      };
      variants.push(format!("<span class=\"mech-enum-variant\">{}</span>", variant_html));
    }
    format!(
      "<span class=\"mech-enum\">{}</span>",
      variants.join("<span class=\"mech-enum-variant-sep\"> | </span>")
    )
  }

  pub fn kind(&self) -> ValueKind {
    if self.variants.len() == 1 {
      let (variant_id, payload) = &self.variants[0];
      let names_brrw = self.names.borrow();
      if let Some(variant_name) = names_brrw.get(variant_id) {
        let short_variant_name = variant_name
          .rsplit('/')
          .next()
          .unwrap_or(variant_name)
          .to_string();
        if let Some(value) = payload {
          if !matches!(value, Value::Kind(_)) {
            return ValueKind::Enum(
              self.id,
              format!("{}({})", short_variant_name, enum_payload_kind(value)),
            );
          }
        }
        return ValueKind::Enum(self.id, short_variant_name);
      }
    }
    ValueKind::Enum(self.id, self.name())
  }

  pub fn size_of(&self) -> usize {
    self.variants.iter().map(|(_,v)| v.as_ref().map_or(0, |x| x.size_of())).sum()
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechEnum {
  fn pretty_print(&self) -> String {
    let mut variants = Vec::new();
    let dict_brrw = self.names.borrow();
    let enum_name = dict_brrw.get(&self.id).unwrap();
    for (id, value) in &self.variants {
      let value_str = match value {
        Some(v) => v.pretty_print(),
        None => "None".to_string(),
      };
      let variant_name = dict_brrw.get(id).unwrap();
      variants.push(format!("{}(\n{})", variant_name, value_str));
    }
    format!(":{}/{}", enum_name, variants.join(" | "))
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
impl MechErrorKind for UnknownEnumVariantError {
  fn name(&self) -> &str { "UnknownEnumVariant" }
  fn message(&self) -> String {
    format!(
      "Unknown variant {} for enum {}",
      self.given_variant_id, self.enum_id
    )
  }
}

fn enum_payload_kind(value: &Value) -> String {
  match value {
    Value::Enum(enum_value) => {
      let enum_brrw = enum_value.borrow();
      if enum_brrw.variants.len() == 1 {
        let (variant_id, payload) = &enum_brrw.variants[0];
        let names_brrw = enum_brrw.names.borrow();
        let variant_name = names_brrw
          .get(variant_id)
          .map(|name| name.rsplit('/').next().unwrap_or(name).to_string())
          .unwrap_or_else(|| format!("{}", variant_id));
        return match payload {
          Some(inner_payload) => format!(":{}({})", variant_name, enum_payload_kind(inner_payload)),
          None => format!(":{}", variant_name),
        };
      }
      format!("{}", enum_brrw.kind())
    }
    _ => format!("{}", value.kind()),
  }
}
