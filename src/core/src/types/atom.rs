use crate::*;
use super::*;

#[cfg(feature = "atom")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Copy, PartialOrd, Debug)]
pub struct MechAtom(pub u64);

impl Hash for MechAtom {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl fmt::Display for MechAtom {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

impl PrettyPrint for MechAtom {
  fn pretty_print(&self) -> String {
    format!("`{}", emojify(&(self.0 as u16)))
  }
}