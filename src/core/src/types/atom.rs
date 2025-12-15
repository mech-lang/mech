use crate::*;
use super::*;

#[cfg(feature = "atom")]
#[derive(Clone, Debug)]
pub struct MechAtom(pub (u64, Ref<Dictionary>));

impl PartialEq<MechAtom> for MechAtom {
  fn eq(&self, other: &MechAtom) -> bool {
    &self.id() == &other.id()
  }
}

impl MechAtom {
  pub fn id(&self) -> u64 {
    self.0.0
  }
  pub fn name(&self) -> String {
    let names_brrw = self.0.1.borrow();
    names_brrw.get(&self.0.0).cloned().unwrap_or_else(|| format!("{}", emojify(&(self.0.0 as u16))))
  }
  pub fn dictionary(&self) -> Ref<Dictionary> {
    self.0.1.clone()
  }
  pub fn new(id: u64) -> MechAtom {
    let dict = Ref::new(Dictionary::new());
    MechAtom((id, dict))
  }

}

impl Hash for MechAtom {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0 .0.hash(state);
  }
}

impl fmt::Display for MechAtom {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

impl PrettyPrint for MechAtom {
  fn pretty_print(&self) -> String {
    let names_brrw = self.0.1.borrow();
    if let Some(name) = names_brrw.get(&self.0.0) {
      return format!("`{}", name);
    } 
    format!("`{}", emojify(&(self.0.0 as u16)))
  }
}