use crate::*;
use super::*;

use std::cmp::Ordering;

#[cfg(feature = "atom")]
#[derive(Clone, Debug)]
pub struct MechAtom(pub (u64, Ref<Dictionary>));

impl PartialEq<MechAtom> for MechAtom {
  fn eq(&self, other: &MechAtom) -> bool {
    &self.id() == &other.id()
  }
}

impl Eq for MechAtom {}

impl PartialOrd for MechAtom {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.id().partial_cmp(&other.id())
  }
}

impl Ord for MechAtom {
  fn cmp(&self, other: &Self) -> Ordering {
    self.id().cmp(&other.id())
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
  pub fn from_name(name: &str) -> MechAtom {
    let id = hash_str(name);
    let dict = Ref::new(Dictionary::new());
    {
      let mut dict_brrw = dict.borrow_mut();
      dict_brrw.insert(id, name.to_string());
    }
    MechAtom((id, dict))
  }
  pub fn to_html(&self) -> String {
    format!("<span class=\"mech-atom\"><span class=\"mech-atom-name\">:{}</span></span>",self.name())
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
    let name = self.name();
    format!(":{}", name)
  }
}