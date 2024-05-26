use hashbrown::HashSet;

#[derive(Debug,Clone,PartialEq)]
pub struct EnumDefinition {
  pub kind: u64,
  pub variants: HashSet<u64>,
}

#[derive(Debug,Clone,PartialEq,PartialOrd,Serialize,Deserialize)]
pub struct Enum {
  pub kind: u64,
  pub variant: u64,
}